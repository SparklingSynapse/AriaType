use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use hound::WavReader;
use parking_lot::Mutex as ParkingMutex;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};
use tracing::{debug, error, info, instrument, warn};

use crate::events::{
    EventName, RecordingStateEvent, TranscriptionCompleteEvent, TranscriptionPartialEvent,
};
use crate::state::app_state::AppState;
use crate::state::unified_state::{AudioStorage, StreamingSttState};
use crate::stt_engine::cloud::StreamingSttClient;
use crate::stt_engine::traits::StreamingSttEngine;
use crate::utils::AppPaths;

// Audio activity thresholds (0-100 normalized scale)
// ON: ~-36 dB, above typical office noise
// OFF: ~-45 dB, provides hysteresis
const AUDIO_ACTIVITY_ON_THRESHOLD: u32 = 40;
const AUDIO_ACTIVITY_OFF_THRESHOLD: u32 = 25;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingState {
    pub is_recording: bool,
    pub is_transcribing: bool,
    pub audio_level: u32,
    pub output_path: Option<String>,
}

#[tauri::command]
#[instrument(skip(app, state), ret, err)]
pub async fn start_recording(app: AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    start_recording_sync(app)?;
    let path = state.output_path.lock().clone().unwrap_or_default();
    Ok(path)
}

pub fn start_recording_sync(app: AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();

    if state.is_recording.load(Ordering::SeqCst) {
        return Err("Already recording".to_string());
    }

    // Show pill immediately on hotkey press
    {
        let settings = state.settings.lock();
        let preset = settings.pill_position.clone();
        drop(settings);
        crate::commands::window::position_pill_window(&app, &preset);
    }
    state.is_recording.store(true, Ordering::SeqCst);
    state.is_transcribing.store(false, Ordering::SeqCst);
    crate::commands::window::update_pill_visibility(&app);

    {
        let settings = state.settings.lock();
        let beep_enabled = settings.beep_on_record;
        drop(settings);

        debug!(beep_enabled, "beep_check-start_recording");
        if beep_enabled {
            debug!("beep_play-start");
            crate::audio::beep::play_start_beep();
        }
    }

    let (cloud_stt_enabled, cloud_stt_config, language) = {
        let settings = state.settings.lock();
        (
            #[allow(deprecated)]
            settings.is_volcengine_streaming_active(),
            settings.get_active_cloud_stt_config(),
            settings.stt_engine_language.clone(),
        )
    };

    let task_id = state.task_counter.fetch_add(1, Ordering::SeqCst) + 1;
    state.start_session(task_id);

    let start_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    state.recording_start_time.store(start_ms, Ordering::SeqCst);

    if cloud_stt_enabled {
        start_streaming_recording(&app, task_id, cloud_stt_config, language)?;
    } else {
        start_chunked_recording(&app, task_id)?;
    }

    if let Some(tx) = state.level_monitor_tx.lock().as_ref() {
        let _ = tx.send(true);
    }

    info!(task_id, streaming = cloud_stt_enabled, "recording_started");
    let _ = app.emit(
        EventName::RECORDING_STATE_CHANGED,
        RecordingStateEvent {
            status: "recording".to_string(),
            task_id,
        },
    );

    Ok(())
}

/// Memory accumulation for cloud STT (volcengine).
/// Audio is accumulated in memory and sent to API when recording stops.
fn start_streaming_recording(
    app: &AppHandle,
    task_id: u64,
    config: crate::commands::settings::CloudSttConfig,
    language: String,
) -> Result<(), String> {
    let state = app.state::<AppState>();
    let audio_device = {
        let settings = state.settings.lock();
        settings.audio_device.clone()
    };

    let (denoise_enabled, vad_enabled, stt_context) = {
        let settings = state.settings.lock();
        let denoise = matches!(settings.denoise_mode.as_str(), "on" | "auto");
        let ctx = crate::stt_engine::traits::SttContext {
            domain: {
                let d = settings.stt_engine_work_domain.trim().to_string();
                if d.is_empty() {
                    None
                } else {
                    Some(d)
                }
            },
            subdomain: {
                let s = settings.stt_engine_work_subdomain.trim().to_string();
                if s.is_empty() {
                    None
                } else {
                    Some(s)
                }
            },
            glossary: {
                let g = settings.stt_engine_user_glossary.trim().to_string();
                if g.is_empty() {
                    None
                } else {
                    Some(g)
                }
            },
            ..Default::default()
        };
        (denoise, settings.vad_enabled, ctx)
    };

    let (app_tx, mut app_rx) = tokio::sync::mpsc::channel::<Vec<i16>>(100);

    *state.streaming_stt.lock() = Some(StreamingSttState {
        audio_tx: app_tx.clone(),
        accumulated_text: String::new(),
        task_id,
        streaming_task: Arc::new(ParkingMutex::new(None)),
    });

    *state.audio_storage.lock() = Some(AudioStorage::Streaming);

    let device_name = if audio_device == "default" {
        None
    } else {
        Some(audio_device)
    };

    let sample_rate: Arc<parking_lot::Mutex<u32>> = Arc::new(parking_lot::Mutex::new(0));
    let channels: Arc<parking_lot::Mutex<u16>> = Arc::new(parking_lot::Mutex::new(0));
    let chunk_buffer: Arc<parking_lot::Mutex<Vec<i16>>> =
        Arc::new(parking_lot::Mutex::new(Vec::new()));
    let app_tx_clone = app_tx.clone();

    let sample_rate_clone = sample_rate.clone();
    let channels_clone = channels.clone();

    let processor: Arc<ParkingMutex<crate::audio::stream_processor::StreamAudioProcessor>> =
        Arc::new(ParkingMutex::new(
            crate::audio::stream_processor::StreamAudioProcessor::new(denoise_enabled, vad_enabled),
        ));

    let (sr, ch) = {
        let recorder = state.recorder.lock();
        recorder
            .start_streaming(device_name, move |pcm, sr, ch| {
                if *sample_rate_clone.lock() == 0 {
                    *sample_rate_clone.lock() = sr;
                    *channels_clone.lock() = ch;
                }

                let mut buffer = chunk_buffer.lock();
                buffer.extend_from_slice(pcm);

                let chunk_size = (sr as f32 * ch as f32 * 0.5) as usize;

                if buffer.len() >= chunk_size {
                    let chunk_data = buffer.drain(..).collect::<Vec<i16>>();

                    let audio_f32: Vec<f32> =
                        chunk_data.iter().map(|&s| s as f32 / 32768.0).collect();

                    let mono_f32 = if ch == 2 {
                        audio_f32
                            .chunks(2)
                            .map(|stereo| (stereo[0] + stereo.get(1).copied().unwrap_or(0.0)) / 2.0)
                            .collect()
                    } else {
                        audio_f32
                    };

                    let result = processor.lock().process_chunk(&mono_f32, sr);

                    if result.has_speech {
                        if let Err(e) = app_tx_clone.try_send(result.pcm_16khz_mono) {
                            warn!(task_id, error = %e, "audio_chunk_enqueue_failed-streaming");
                        } else {
                            debug!(task_id, "audio_chunk_enqueued-streaming");
                        }
                    } else {
                        debug!(task_id, "audio_chunk_skipped-silent");
                    }
                }
            })
            .map_err(|e| {
                error!(error = %e, "recorder_start_failed-cloud");
                state.is_recording.store(false, Ordering::SeqCst);
                crate::commands::window::update_pill_visibility(app);
                e.to_string()
            })?
    };

    let app_clone = app.clone();
    let handle = tauri::async_runtime::spawn(async move {
        let context_for_log = (
            stt_context
                .domain
                .clone()
                .unwrap_or_else(|| "none".to_owned()),
            stt_context
                .subdomain
                .clone()
                .unwrap_or_else(|| "none".to_owned()),
            stt_context
                .glossary
                .clone()
                .unwrap_or_else(|| "none".to_owned()),
        );
        let mut client = match StreamingSttClient::new(config, Some(&language), stt_context) {
            Ok(c) => c,
            Err(e) => {
                error!(task_id, error = %e, "streaming_client_create_failed");
                let _ = app_clone.emit(
                    EventName::RECORDING_STATE_CHANGED,
                    RecordingStateEvent {
                        status: "error".to_string(),
                        task_id,
                    },
                );
                return;
            }
        };
        let provider_name = client.provider_name();

        let (domain, subdomain, glossary) = context_for_log;
        info!(
            task_id,
            provider = %provider_name,
            domain,
            subdomain,
            glossary,
            "streaming_client_connected"
        );

        let app_event_clone = app_clone.clone();

        client.set_partial_callback(Arc::new(move |result| {
            if !result.is_final && !result.text.is_empty() {
                let _ = app_event_clone.emit(
                    EventName::TRANSCRIPTION_PARTIAL,
                    TranscriptionPartialEvent {
                        text: result.text,
                        is_definite: result.is_definite,
                        task_id,
                    },
                );
            }
        }));

        if let Err(e) = client.connect().await {
            error!(task_id, provider = %provider_name, error = %e, "streaming_client_connect_failed");
            let _ = app_clone.emit(
                EventName::RECORDING_STATE_CHANGED,
                RecordingStateEvent {
                    status: "error".to_string(),
                    task_id,
                },
            );
            return;
        }

        let audio_tx = match client.get_audio_sender().await {
            Some(tx) => tx,
            None => {
                error!(task_id, "audio_sender_unavailable-streaming_client");
                return;
            }
        };

        // Forward chunks
        let mut chunks_sent = 0;
        while let Some(chunk) = app_rx.recv().await {
            if let Err(e) = audio_tx.send(chunk).await {
                error!(task_id, provider = %provider_name, error = %e, "audio_chunk_send_failed-streaming");
                break;
            }
            chunks_sent += 1;
            info!(task_id, provider = %provider_name, chunk_index = chunks_sent, "audio_chunk_sent-streaming");
        }
        info!(
            task_id,
            total_chunks = chunks_sent,
            "audio_chunks_sent_all-awaiting_final"
        );

        // The client's internal audio sender task only exits after every clone of
        // its mpsc sender is dropped. This forwarder owns one clone, so release it
        // before calling finish() or finish() will wait on itself forever.
        drop(audio_tx);

        // When app_rx is closed (recording stopped), we call finish()
        let _ = app_clone.emit(
            EventName::RECORDING_STATE_CHANGED,
            RecordingStateEvent {
                status: "transcribing".to_string(),
                task_id,
            },
        );

        debug!(task_id, "streaming_client_finish_invoked");
        match client.finish().await {
            Ok(text) => {
                let raw_text = text.clone();
                let state = app_clone.state::<AppState>();
                let (final_text, polish_time_ms) = if text.is_empty() {
                    (String::new(), 0)
                } else {
                    maybe_polish_transcription_text(Some(&app_clone), &state, task_id, text).await
                };

                info!(
                    task_id,
                    text_len = final_text.len(),
                    "transcription_final_received-streaming"
                );
                if !final_text.is_empty() {
                    save_to_history(
                        &state,
                        &raw_text,
                        &final_text,
                        None,
                        if polish_time_ms > 0 {
                            Some(polish_time_ms as i64)
                        } else {
                            None
                        },
                        polish_time_ms > 0,
                    );
                    let _ = app_clone.emit(
                        EventName::TRANSCRIPTION_COMPLETE,
                        TranscriptionCompleteEvent {
                            text: final_text.clone(),
                            task_id,
                        },
                    );
                    let _ = app_clone.emit(
                        EventName::RECORDING_STATE_CHANGED,
                        RecordingStateEvent {
                            status: "idle".to_string(),
                            task_id,
                        },
                    );
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    let _ =
                        crate::commands::text::do_insert_text(app_clone.clone(), final_text).await;
                } else {
                    let _ = app_clone.emit(
                        EventName::RECORDING_STATE_CHANGED,
                        RecordingStateEvent {
                            status: "idle".to_string(),
                            task_id,
                        },
                    );
                }

                let _ = state.finish_session(task_id);
            }
            Err(e) => {
                let state = app_clone.state::<AppState>();
                error!(task_id, error = %e, "streaming_stt_finish_failed");
                let _ = app_clone.emit(
                    EventName::RECORDING_STATE_CHANGED,
                    RecordingStateEvent {
                        status: "error".to_string(),
                        task_id,
                    },
                );
                let _ = state.finish_session(task_id);
            }
        }
    });

    if let Some(stt) = state.streaming_stt.lock().as_mut() {
        stt.streaming_task.lock().replace(handle);
    }

    info!(
        task_id,
        sample_rate = sr,
        channels = ch,
        "recording_started-cloud_streaming"
    );
    Ok(())
}

/// Memory accumulation for local STT engines (Whisper, SenseVoice).
/// Audio is accumulated in memory and transcribed as a whole when recording stops.
fn start_chunked_recording(app: &AppHandle, task_id: u64) -> Result<(), String> {
    let state = app.state::<AppState>();
    let audio_device = {
        let settings = state.settings.lock();
        settings.audio_device.clone()
    };

    let samples: Arc<ParkingMutex<Vec<i16>>> = Arc::new(ParkingMutex::new(Vec::new()));
    let sample_rate: Arc<ParkingMutex<u32>> = Arc::new(ParkingMutex::new(0));
    let channels: Arc<ParkingMutex<u16>> = Arc::new(ParkingMutex::new(0));

    *state.audio_storage.lock() = Some(AudioStorage::Local {
        samples: samples.clone(),
        sample_rate: sample_rate.clone(),
        channels: channels.clone(),
    });

    let device_name = if audio_device == "default" {
        None
    } else {
        Some(audio_device)
    };

    let samples_clone = samples.clone();
    let sample_rate_clone = sample_rate.clone();
    let channels_clone = channels.clone();

    let (sr, ch) = {
        let recorder = state.recorder.lock();
        recorder
            .start_streaming(device_name, move |pcm, sr, ch| {
                samples_clone.lock().extend_from_slice(pcm);
                if *sample_rate_clone.lock() == 0 {
                    *sample_rate_clone.lock() = sr;
                    *channels_clone.lock() = ch;
                }
            })
            .map_err(|e| {
                error!(error = %e, "recorder_start_failed-local");
                state.is_recording.store(false, Ordering::SeqCst);
                crate::commands::window::update_pill_visibility(app);
                e.to_string()
            })?
    };

    info!(
        task_id,
        sample_rate = sr,
        channels = ch,
        "recording_started-local_memory"
    );
    Ok(())
}

#[tauri::command]
#[instrument(skip(app, _state), ret, err)]
pub async fn stop_recording(
    app: AppHandle,
    _state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let output_path = stop_recording_sync(app.clone())?;
    Ok(output_path)
}

fn await_streaming_task_in_background(task_id: u64, handle: tauri::async_runtime::JoinHandle<()>) {
    tauri::async_runtime::spawn(async move {
        if let Err(e) = handle.await {
            error!(task_id, error = %e, "streaming_stt_task_panicked");
        }
    });
}

pub fn stop_recording_sync(app: AppHandle) -> Result<Option<String>, String> {
    let state = app.state::<AppState>();

    if !state.is_recording.load(Ordering::SeqCst) {
        return Ok(None);
    }

    {
        let recorder = state.recorder.lock();
        recorder.stop().map_err(|e| e.to_string())?;
    }

    let task_id = state.task_counter.load(Ordering::SeqCst);
    let audio_storage = state.audio_storage.lock().take();

    let (cloud_stt_enabled, cloud_stt_config, language) = {
        let settings = state.settings.lock();
        (
            #[allow(deprecated)]
            settings.is_volcengine_streaming_active(),
            settings.get_active_cloud_stt_config(),
            settings.stt_engine_language.clone(),
        )
    };

    state.is_recording.store(false, Ordering::SeqCst);

    if let Some(tx) = state.level_monitor_tx.lock().as_ref() {
        let _ = tx.send(false);
    }

    {
        let settings = state.settings.lock();
        let beep_enabled = settings.beep_on_record;
        drop(settings);

        debug!(beep_enabled, "beep_check-stop_recording");
        if beep_enabled {
            debug!("beep_play-stop");
            crate::audio::beep::play_stop_beep();
        }
    }

    match audio_storage {
        Some(AudioStorage::Local {
            samples,
            sample_rate,
            channels,
        }) => {
            let s = samples.lock().clone();
            let sr = *sample_rate.lock();
            let ch = *channels.lock();

            if cloud_stt_enabled {
                finish_cloud_stt_recording(&app, task_id, s, sr, ch, cloud_stt_config, language)?;
            } else {
                finish_local_recording(&app, task_id, s, sr, ch)?;
            }
            Ok(None)
        }
        Some(AudioStorage::Streaming) => {
            let streaming_state = state.streaming_stt.lock().take();
            if let Some(stt) = streaming_state {
                info!(task_id, "streaming_stt_stopping-awaiting_final");
                if let Some(handle) = stt.streaming_task.lock().take() {
                    await_streaming_task_in_background(task_id, handle);
                }
                drop(stt);
            }
            Ok(None)
        }
        None => {
            warn!(task_id, "audio_storage_missing-recording_interrupted");
            let _ = app.emit(
                EventName::RECORDING_STATE_CHANGED,
                RecordingStateEvent {
                    status: "idle".to_string(),
                    task_id,
                },
            );
            Ok(None)
        }
    }
}

/// Finalize local STT recording - write accumulated audio to WAV and transcribe.
#[instrument(skip(app, samples), fields(task_id))]
fn finish_local_recording(
    app: &AppHandle,
    task_id: u64,
    samples: Vec<i16>,
    sample_rate: u32,
    channels: u16,
) -> Result<(), String> {
    let state = app.state::<AppState>();

    if samples.is_empty() {
        warn!(task_id, "audio_samples_empty-local");
        let _ = app.emit(
            EventName::RECORDING_STATE_CHANGED,
            RecordingStateEvent {
                status: "idle".to_string(),
                task_id,
            },
        );
        return Ok(());
    }

    info!(
        task_id,
        samples = samples.len(),
        sample_rate,
        channels,
        "recording_finalizing-local"
    );

    let _ = app.emit(
        EventName::RECORDING_STATE_CHANGED,
        RecordingStateEvent {
            status: "transcribing".to_string(),
            task_id,
        },
    );

    let app_dir = AppPaths::recordings_dir();
    std::fs::create_dir_all(&app_dir).map_err(|e| e.to_string())?;

    let filename = format!("recording_{}.wav", uuid::Uuid::new_v4());
    let output_path = app_dir.join(&filename);

    let spec = hound::WavSpec {
        channels,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    {
        let mut writer = hound::WavWriter::create(&output_path, spec)
            .map_err(|e| format!("Failed to create WAV file: {}", e))?;
        for sample in &samples {
            writer
                .write_sample(*sample)
                .map_err(|e| format!("Failed to write sample: {}", e))?;
        }
        writer
            .finalize()
            .map_err(|e| format!("Failed to finalize WAV: {}", e))?;
    }

    info!(task_id, path = %output_path.display(), "wav_written-local_transcription_starting");

    let job = crate::state::unified_state::TranscriptionJob {
        audio_path: output_path.to_string_lossy().to_string(),
        timestamp: std::time::SystemTime::now(),
        task_id,
    };

    *state.output_path.lock() = Some(output_path.to_string_lossy().to_string());

    let app_clone = app.clone();
    tauri::async_runtime::spawn(async move {
        run_transcription(app_clone, job.audio_path, task_id).await;
    });

    Ok(())
}

async fn run_local_polish(
    app: Option<&AppHandle>,
    state: &AppState,
    task_id: u64,
    accumulated_text: String,
    context: LocalPolishContext,
) -> (String, u64) {
    let LocalPolishContext {
        system_prompt,
        language,
        model_id,
        log_context,
    } = context;

    match crate::polish_engine::UnifiedPolishManager::get_engine_by_model_id(&model_id) {
        Some(engine_type) => {
            let model_filename = state
                .polish_manager
                .get_model_filename(engine_type, &model_id);

            if let Some(model_filename) = model_filename.filter(|_| {
                state
                    .polish_manager
                    .is_model_downloaded(engine_type, &model_id)
            }) {
                info!(task_id, engine = ?engine_type, model_id = %model_id, context = log_context, "polish_started-local");

                let request = crate::polish_engine::PolishRequest::new(
                    accumulated_text.clone(),
                    system_prompt,
                    language,
                )
                .with_model(model_filename);

                if let Some(app_handle) = app {
                    let _ = app_handle.emit(
                        EventName::RECORDING_STATE_CHANGED,
                        RecordingStateEvent {
                            status: "polishing".to_string(),
                            task_id,
                        },
                    );
                }

                match state.polish_manager.polish(engine_type, request).await {
                    Ok(result) if !result.text.is_empty() => {
                        info!(
                            task_id,
                            chars = result.text.len(),
                            polish_ms = result.total_ms,
                            context = log_context,
                            "polish_completed-local"
                        );
                        (result.text, result.total_ms)
                    }
                    Ok(_) => {
                        warn!(
                            task_id,
                            context = log_context,
                            "polish_empty_result-local_using_raw"
                        );
                        (accumulated_text, 0)
                    }
                    Err(e) => {
                        warn!(task_id, error = %e, context = log_context, "polish_failed-local_using_raw");
                        (accumulated_text, 0)
                    }
                }
            } else {
                warn!(
                    task_id,
                    context = log_context,
                    "polish_model_not_downloaded-using_raw"
                );
                (accumulated_text, 0)
            }
        }
        None => {
            warn!(task_id, model_id = %model_id, context = log_context, "polish_model_unknown-engine_undetermined");
            (accumulated_text, 0)
        }
    }
}

struct LocalPolishContext {
    system_prompt: String,
    language: String,
    model_id: String,
    log_context: &'static str,
}

fn save_to_history(
    state: &AppState,
    raw_text: &str,
    final_text: &str,
    stt_duration_ms: Option<i64>,
    polish_duration_ms: Option<i64>,
    polish_applied: bool,
) {
    let (stt_engine, stt_model, language, is_cloud) = {
        let settings = state.settings.lock();
        let cloud_config = settings.get_active_cloud_stt_config();
        let is_cloud = cloud_config.enabled;
        let engine_str = if is_cloud {
            format!("cloud-{}", cloud_config.provider_type)
        } else {
            crate::stt_engine::UnifiedEngineManager::get_engine_by_model_name(&settings.model)
                .map(|et| et.as_str().to_string())
                .unwrap_or_else(|| "unknown".to_string())
        };
        (
            engine_str,
            if is_cloud {
                Some(cloud_config.model.clone())
            } else {
                Some(settings.model.clone())
            },
            if settings.stt_engine_language.is_empty() {
                None
            } else {
                Some(settings.stt_engine_language.clone())
            },
            is_cloud,
        )
    };

    let recording_duration_ms = {
        let start = state
            .recording_start_time
            .load(std::sync::atomic::Ordering::SeqCst);
        if start > 0 {
            Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as i64
                    - start as i64,
            )
        } else {
            None
        }
    };

    let total_ms = match (stt_duration_ms, polish_duration_ms) {
        (Some(stt), Some(pol)) => Some(stt + pol),
        (Some(stt), None) => Some(stt),
        _ => None,
    };

    let entry = crate::history::NewTranscriptionEntry {
        raw_text: raw_text.to_string(),
        final_text: final_text.to_string(),
        stt_engine,
        stt_model,
        language,
        audio_duration_ms: recording_duration_ms,
        stt_duration_ms,
        polish_duration_ms,
        total_duration_ms: total_ms,
        polish_applied,
        polish_engine: None,
        is_cloud,
    };

    let store = state.history_store.lock();
    if let Err(e) = crate::history::save_history_entry(&store, entry) {
        tracing::warn!(error = %e, "failed_to_save_history");
    }
}

#[instrument(skip(app, state, accumulated_text), fields(task_id))]
async fn maybe_polish_transcription_text(
    app: Option<&AppHandle>,
    state: &AppState,
    task_id: u64,
    accumulated_text: String,
) -> (String, u64) {
    let (polish_enabled, cloud_polish_enabled) = {
        let settings = state.settings.lock();
        (settings.polish_enabled, settings.cloud_polish_enabled)
    };

    if !polish_enabled && !cloud_polish_enabled {
        return (accumulated_text, 0);
    }

    let (polish_system_prompt, polish_language, polish_model_id, cloud_polish_config) = {
        let settings = state.settings.lock();
        let prompt = settings.polish_system_prompt.clone();
        (
            if prompt.is_empty() {
                crate::polish_engine::DEFAULT_POLISH_PROMPT.to_string()
            } else {
                prompt
            },
            settings.stt_engine_language.clone(),
            settings.polish_model.clone(),
            settings.get_active_cloud_polish_config(),
        )
    };

    if cloud_polish_config.enabled {
        if cloud_polish_config.api_key.is_empty() || cloud_polish_config.model.is_empty() {
            warn!(task_id, provider = %cloud_polish_config.provider_type, api_key_empty = cloud_polish_config.api_key.is_empty(), model_empty = cloud_polish_config.model.is_empty(), "cloud_polish_config_incomplete-fallback_local");

            return run_local_polish(
                app,
                state,
                task_id,
                accumulated_text,
                LocalPolishContext {
                    system_prompt: polish_system_prompt,
                    language: polish_language,
                    model_id: polish_model_id,
                    log_context: "cloud_fallback",
                },
            )
            .await;
        }

        info!(task_id, provider = %cloud_polish_config.provider_type, model = %cloud_polish_config.model, "polish_started-cloud");

        let request = crate::polish_engine::PolishRequest::new(
            accumulated_text.clone(),
            polish_system_prompt,
            polish_language,
        );

        if let Some(app_handle) = app {
            let _ = app_handle.emit(
                EventName::RECORDING_STATE_CHANGED,
                RecordingStateEvent {
                    status: "polishing".to_string(),
                    task_id,
                },
            );
        }

        return match state
            .polish_manager
            .polish_cloud(
                request,
                &cloud_polish_config.provider_type,
                &cloud_polish_config.api_key,
                &cloud_polish_config.base_url,
                &cloud_polish_config.model,
                cloud_polish_config.enable_thinking,
            )
            .await
        {
            Ok(result) if !result.text.is_empty() => {
                info!(
                    task_id,
                    chars = result.text.len(),
                    polish_ms = result.total_ms,
                    "polish_completed-cloud"
                );
                (result.text, result.total_ms)
            }
            Ok(_) => {
                warn!(task_id, provider = %cloud_polish_config.provider_type, "polish_empty_result-cloud_using_raw");
                (accumulated_text, 0)
            }
            Err(e) => {
                warn!(task_id, provider = %cloud_polish_config.provider_type, error = %e, "polish_failed-cloud_using_raw");
                (accumulated_text, 0)
            }
        };
    }

    run_local_polish(
        app,
        state,
        task_id,
        accumulated_text,
        LocalPolishContext {
            system_prompt: polish_system_prompt,
            language: polish_language,
            model_id: polish_model_id,
            log_context: "local",
        },
    )
    .await
}

/// Finalize cloud STT recording - write accumulated audio to WAV and send to volcengine API.
#[instrument(skip(app, samples), fields(task_id))]
fn finish_cloud_stt_recording(
    app: &AppHandle,
    task_id: u64,
    samples: Vec<i16>,
    sample_rate: u32,
    channels: u16,
    config: crate::commands::settings::CloudSttConfig,
    language: String,
) -> Result<(), String> {
    let state = app.state::<AppState>();

    if samples.is_empty() {
        warn!(task_id, "audio_samples_empty-local");
        let _ = app.emit(
            EventName::RECORDING_STATE_CHANGED,
            RecordingStateEvent {
                status: "idle".to_string(),
                task_id,
            },
        );
        return Ok(());
    }

    info!(
        task_id,
        samples = samples.len(),
        sample_rate,
        channels,
        "recording_finalizing-cloud"
    );

    let _ = app.emit(
        EventName::RECORDING_STATE_CHANGED,
        RecordingStateEvent {
            status: "transcribing".to_string(),
            task_id,
        },
    );

    let app_dir = AppPaths::recordings_dir();
    std::fs::create_dir_all(&app_dir).map_err(|e| e.to_string())?;

    let filename = format!("recording_{}.wav", uuid::Uuid::new_v4());
    let output_path = app_dir.join(&filename);

    let spec = hound::WavSpec {
        channels,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    {
        let mut writer = hound::WavWriter::create(&output_path, spec)
            .map_err(|e| format!("Failed to create WAV file: {}", e))?;
        for sample in &samples {
            writer
                .write_sample(*sample)
                .map_err(|e| format!("Failed to write sample: {}", e))?;
        }
        writer
            .finalize()
            .map_err(|e| format!("Failed to finalize WAV: {}", e))?;
    }

    info!(task_id, path = %output_path.display(), "wav_written-cloud_transcription_starting");

    *state.output_path.lock() = Some(output_path.to_string_lossy().to_string());

    let audio_path = output_path.to_string_lossy().to_string();
    let app_clone = app.clone();

    tauri::async_runtime::spawn(async move {
        let result = run_cloud_transcription_with_streaming(
            &app_clone,
            &audio_path,
            task_id,
            config,
            language,
        )
        .await;

        match result {
            Ok(text) if !text.is_empty() => {
                let raw_text = text.clone();
                let state = app_clone.state::<AppState>();
                let (final_text, polish_time_ms) =
                    maybe_polish_transcription_text(Some(&app_clone), &state, task_id, text).await;

                save_to_history(
                    &state,
                    &raw_text,
                    &final_text,
                    None,
                    if polish_time_ms > 0 {
                        Some(polish_time_ms as i64)
                    } else {
                        None
                    },
                    polish_time_ms > 0,
                );

                let _ = app_clone.emit(
                    EventName::TRANSCRIPTION_COMPLETE,
                    TranscriptionCompleteEvent {
                        text: final_text.clone(),
                        task_id,
                    },
                );

                let _ = app_clone.emit(
                    EventName::RECORDING_STATE_CHANGED,
                    RecordingStateEvent {
                        status: "idle".to_string(),
                        task_id,
                    },
                );

                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                let _ = crate::commands::text::do_insert_text(app_clone.clone(), final_text).await;
                let _ = state.finish_session(task_id);
            }
            Ok(_) => {
                let state = app_clone.state::<AppState>();
                let _ = app_clone.emit(
                    EventName::RECORDING_STATE_CHANGED,
                    RecordingStateEvent {
                        status: "idle".to_string(),
                        task_id,
                    },
                );
                let _ = state.finish_session(task_id);
            }
            Err(e) => {
                let state = app_clone.state::<AppState>();
                error!(task_id, error = %e, "transcription_failed-cloud");
                let _ = app_clone.emit(
                    EventName::RECORDING_STATE_CHANGED,
                    RecordingStateEvent {
                        status: "idle".to_string(),
                        task_id,
                    },
                );
                let _ = state.finish_session(task_id);
            }
        }
    });

    Ok(())
}

/// Run cloud STT transcription using streaming API
async fn run_cloud_transcription_with_streaming(
    app: &AppHandle,
    audio_path: &str,
    task_id: u64,
    config: crate::commands::settings::CloudSttConfig,
    language: String,
) -> Result<String, String> {
    let stt_context = {
        let state = app.state::<AppState>();
        let settings = state.settings.lock();
        crate::stt_engine::traits::SttContext {
            domain: {
                let d = settings.stt_engine_work_domain.trim().to_string();
                if d.is_empty() {
                    None
                } else {
                    Some(d)
                }
            },
            subdomain: {
                let s = settings.stt_engine_work_subdomain.trim().to_string();
                if s.is_empty() {
                    None
                } else {
                    Some(s)
                }
            },
            glossary: {
                let g = settings.stt_engine_user_glossary.trim().to_string();
                if g.is_empty() {
                    None
                } else {
                    Some(g)
                }
            },
            ..Default::default()
        }
    };

    let mut client = StreamingSttClient::new(config, Some(&language), stt_context)?;

    let app_clone = app.clone();
    client.set_partial_callback(Arc::new(move |result| {
        if !result.is_final && !result.text.is_empty() {
            let _ = app_clone.emit(
                EventName::TRANSCRIPTION_PARTIAL,
                TranscriptionPartialEvent {
                    text: result.text,
                    is_definite: result.is_definite,
                    task_id,
                },
            );
        }
    }));

    client.connect().await?;

    let audio_tx = client
        .get_audio_sender()
        .await
        .ok_or("Failed to get audio sender")?;

    let reader = hound::WavReader::open(audio_path)
        .map_err(|e| format!("Failed to read WAV file: {}", e))?;
    let spec = reader.spec();
    let input_sample_rate = spec.sample_rate;
    let input_channels = spec.channels;

    let samples_i16: Vec<i16> = reader
        .into_samples::<i16>()
        .filter_map(|s| s.ok())
        .collect();

    let samples_16khz_mono: Vec<i16> = {
        let mut audio_f32: Vec<f32> = samples_i16.iter().map(|&s| s as f32 / 32768.0).collect();

        if input_channels == 2 {
            let mono: Vec<f32> = audio_f32
                .chunks(2)
                .map(|stereo| (stereo[0] + stereo.get(1).copied().unwrap_or(0.0)) / 2.0)
                .collect();
            audio_f32 = mono;
        }

        if input_sample_rate != 16000 {
            let resampled =
                crate::audio::resampler::resample_to_16khz(&audio_f32, input_sample_rate)
                    .map_err(|e| format!("Resampling failed: {}", e))?;
            resampled
                .iter()
                .map(|&s| (s * 32767.0).clamp(-32768.0, 32767.0) as i16)
                .collect()
        } else {
            audio_f32
                .iter()
                .map(|&s| (s * 32767.0).clamp(-32768.0, 32767.0) as i16)
                .collect()
        }
    };

    const CHUNK_SIZE: usize = 3200;
    for chunk in samples_16khz_mono.chunks(CHUNK_SIZE) {
        audio_tx
            .send(chunk.to_vec())
            .await
            .map_err(|e| format!("Failed to send audio chunk: {}", e))?;
    }

    drop(audio_tx);

    client.finish().await
}

#[allow(dead_code)]
/// Process transcription queue in FIFO order
fn process_transcription_queue(app: AppHandle) {
    let state = app.state::<AppState>();

    // Check if already processing
    let current_count = state.processing_count.load(Ordering::SeqCst);
    if current_count > 0 {
        debug!(
            jobs = current_count,
            "transcription_queue_processor_running"
        );
        return;
    }

    // Get next job from queue
    let job = {
        let mut queue = state.transcription_queue.lock();
        queue.pop_front()
    };

    if let Some(job) = job {
        state.processing_count.fetch_add(1, Ordering::SeqCst);
        state.is_transcribing.store(true, Ordering::SeqCst);

        // Emit processing state; frontend will filter by task_id
        info!(task_id = job.task_id, "transcription_processing_started");
        let _ = app.emit(
            EventName::RECORDING_STATE_CHANGED,
            RecordingStateEvent {
                status: "processing".to_string(),
                task_id: job.task_id,
            },
        );

        let app_clone = app.clone();
        let job_task_id = job.task_id;
        tauri::async_runtime::spawn(async move {
            run_transcription(app_clone.clone(), job.audio_path, job_task_id).await;

            // Decrement counter
            let state = app_clone.state::<AppState>();
            state.processing_count.fetch_sub(1, Ordering::SeqCst);
            state.is_transcribing.store(false, Ordering::SeqCst);

            // Process next job in queue
            process_transcription_queue(app_clone);
        });
    } else {
        debug!("transcription_queue_empty");
    }
}

#[instrument(skip(app), fields(task_id, audio_path))]
async fn run_transcription(app: AppHandle, audio_path: String, task_id: u64) {
    let state = app.state::<AppState>();

    // Log wav file information for debugging
    log_wav_file_info(&audio_path, task_id);

    let (
        mut model_name,
        language,
        initial_prompt,
        domain,
        subdomain,
        glossary,
        denoise_mode,
        vad_enabled,
        cloud_config,
    ) = {
        let settings = state.settings.lock();
        debug!(task_id, model = %settings.model, language = %settings.stt_engine_language, "transcription_settings");
        debug!(task_id, domain = %settings.stt_engine_work_domain, subdomain = %settings.stt_engine_work_subdomain, glossary_len = settings.stt_engine_user_glossary.len(), "domain_glossary_settings");
        (
            settings.model.clone(),
            settings.stt_engine_language.clone(),
            settings.stt_engine_initial_prompt.clone(),
            settings.stt_engine_work_domain.clone(),
            settings.stt_engine_work_subdomain.clone(),
            settings.stt_engine_user_glossary.clone(),
            settings.denoise_mode.clone(),
            settings.vad_enabled,
            settings.get_active_cloud_stt_config(),
        )
    };

    if cloud_config.enabled {
        model_name = "cloud".to_string();
    }

    // Auto-detect engine type from model name (more reliable than settings.stt_engine)
    let engine_type =
        match crate::stt_engine::UnifiedEngineManager::get_engine_by_model_name(&model_name) {
            Some(et) => et,
            None => {
                error!(task_id, model = %model_name, "model_unknown-engine_undetermined");
                let _ = app.emit(
                    EventName::TRANSCRIPTION_ERROR,
                    &format!("Unknown model: {}", model_name),
                );
                let _ = app.emit(
                    EventName::RECORDING_STATE_CHANGED,
                    RecordingStateEvent {
                        status: "error".to_string(),
                        task_id,
                    },
                );
                state.is_transcribing.store(false, Ordering::SeqCst);
                return;
            }
        };

    debug!(task_id, engine = ?engine_type, model = %model_name, "engine_detected-from_model");

    // Check if model is downloaded using engine manager
    if !state
        .engine_manager
        .is_model_downloaded(engine_type, &model_name)
    {
        let msg = format!(
            "Model '{}' not downloaded. Please download it in Settings > Model.",
            model_name
        );
        warn!(task_id, model = %model_name, engine = ?engine_type, "model_not_downloaded");
        let _ = app.emit(EventName::TRANSCRIPTION_ERROR, &msg);
        let _ = app.emit(
            EventName::RECORDING_STATE_CHANGED,
            RecordingStateEvent {
                status: "error".to_string(),
                task_id,
            },
        );

        let app_error = app.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(3));
            let state = app_error.state::<AppState>();
            let next_status = if state.is_recording.load(Ordering::SeqCst) {
                "recording"
            } else {
                "idle"
            };
            let _ = app_error.emit(
                EventName::RECORDING_STATE_CHANGED,
                RecordingStateEvent {
                    status: next_status.to_string(),
                    task_id,
                },
            );
        });
        state.is_transcribing.store(false, Ordering::SeqCst);
        return;
    }

    // Build transcription request
    let prompt =
        build_stt_engine_prompt(&initial_prompt, &language, &domain, &subdomain, &glossary);
    let mut request = crate::stt_engine::TranscriptionRequest::new(audio_path.clone())
        .with_model(model_name.clone())
        .with_language(language.clone())
        .with_denoise_mode(denoise_mode)
        .with_vad_enabled(vad_enabled)
        .with_cloud_config(cloud_config);
    if let Some(p) = prompt {
        request = request.with_prompt(p);
    }

    // Execute transcription using engine manager
    let result = state.engine_manager.transcribe(engine_type, request).await;

    let (text, mut metrics) = match result {
        Ok(result) => (
            result.text,
            crate::events::TranscriptionMetrics {
                load_time_ms: result.model_load_ms.unwrap_or(0),
                preprocess_time_ms: result.preprocess_ms.unwrap_or(0),
                inference_time_ms: result.inference_ms.unwrap_or(0),
                polish_time_ms: 0,
                total_time_ms: result.total_ms,
            },
        ),
        Err(e) => {
            error!(task_id, error = %e, "transcription_failed");
            let _ = app.emit(EventName::TRANSCRIPTION_ERROR, &e);
            let _ = app.emit(
                EventName::RECORDING_STATE_CHANGED,
                RecordingStateEvent {
                    status: "error".to_string(),
                    task_id,
                },
            );
            state.is_transcribing.store(false, Ordering::SeqCst);
            let _ = std::fs::remove_file(&audio_path);
            return;
        }
    };

    state.is_transcribing.store(false, Ordering::SeqCst);
    let _ = std::fs::remove_file(&audio_path);

    if !text.is_empty() {
        info!(task_id, chars = text.len(), "chunk_transcribed");

        // Append text to session state for accumulation
        state.append_session_text(task_id, &text);

        // Check if this is the last chunk
        let has_more = !state.transcription_queue.lock().is_empty();
        let is_still_recording = state.is_recording.load(Ordering::SeqCst);
        let is_last_chunk = !has_more && !is_still_recording;

        if is_last_chunk {
            // This is the last chunk - get accumulated text and do final polish
            let (accumulated_text, chunk_count) =
                state.get_session_text(task_id).unwrap_or((text.clone(), 1));

            info!(
                task_id,
                chars = accumulated_text.len(),
                chunks = chunk_count,
                "session_complete-polish_starting"
            );

            let raw_text = accumulated_text.clone();
            let (final_text, polish_time_ms) =
                maybe_polish_transcription_text(Some(&app), &state, task_id, accumulated_text)
                    .await;
            metrics.polish_time_ms = polish_time_ms;
            metrics.total_time_ms += polish_time_ms;
            let _ = app.emit(EventName::TRANSCRIPTION_METRICS, &metrics);
            info!(
                task_id,
                load_ms = metrics.load_time_ms,
                preprocess_ms = metrics.preprocess_time_ms,
                inference_ms = metrics.inference_time_ms,
                polish_ms = metrics.polish_time_ms,
                total_ms = metrics.total_time_ms,
                "transcription_metrics"
            );

            save_to_history(
                &state,
                &raw_text,
                &final_text,
                Some(metrics.total_time_ms as i64),
                if polish_time_ms > 0 {
                    Some(polish_time_ms as i64)
                } else {
                    None
                },
                polish_time_ms > 0,
            );

            let _ = app.emit(
                EventName::TRANSCRIPTION_COMPLETE,
                TranscriptionCompleteEvent {
                    text: final_text.clone(),
                    task_id,
                },
            );

            let _ = app.emit(
                EventName::RECORDING_STATE_CHANGED,
                RecordingStateEvent {
                    status: "idle".to_string(),
                    task_id,
                },
            );

            let _ = state.finish_session(task_id);

            let app_clone = app.clone();
            let text_clone = final_text.clone();
            tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                crate::commands::text::do_insert_text(app_clone, text_clone).await;
            });
        } else {
            // Intermediate chunk - just update status, don't inject yet
            debug!(
                task_id,
                has_more, is_still_recording, "chunk_intermediate_processed"
            );
            let next = if has_more {
                "processing"
            } else if is_still_recording {
                "recording"
            } else {
                "idle"
            };
            let _ = app.emit(
                EventName::RECORDING_STATE_CHANGED,
                RecordingStateEvent {
                    status: next.to_string(),
                    task_id,
                },
            );
        }
    } else {
        let _ = app.emit(EventName::TRANSCRIPTION_METRICS, &metrics);
        info!(
            task_id,
            load_ms = metrics.load_time_ms,
            preprocess_ms = metrics.preprocess_time_ms,
            inference_ms = metrics.inference_time_ms,
            total_ms = metrics.total_time_ms,
            "transcription metrics"
        );

        warn!(task_id, "transcription_empty_result");
        let has_more = !state.transcription_queue.lock().is_empty();
        let is_still_recording = state.is_recording.load(Ordering::SeqCst);
        let next = if has_more {
            "processing"
        } else if is_still_recording {
            "recording"
        } else {
            "idle"
        };
        let _ = app.emit(
            EventName::RECORDING_STATE_CHANGED,
            RecordingStateEvent {
                status: next.to_string(),
                task_id,
            },
        );
    }
}

#[tauri::command]
pub fn get_audio_level(state: State<'_, AppState>) -> u32 {
    state.audio_level.load(Ordering::SeqCst)
}

#[tauri::command]
pub fn get_recording_state(state: State<'_, AppState>) -> RecordingState {
    RecordingState {
        is_recording: state.is_recording.load(Ordering::SeqCst),
        is_transcribing: state.is_transcribing.load(Ordering::SeqCst),
        audio_level: state.audio_level.load(Ordering::SeqCst),
        output_path: state.output_path.lock().clone(),
    }
}

pub fn start_audio_level_monitor(app: AppHandle) -> Result<(), String> {
    info!("audio_level_monitor_started");

    // cpal::Stream is !Send on macOS — it must live on this thread.
    // Commands (open/close) arrive via a channel sent from start/stop_recording_sync.
    let state = app.state::<AppState>();

    // Take the receiver out of AppState — it belongs to this thread from now on.
    let rx = state
        .level_monitor_rx
        .lock()
        .take()
        .ok_or("level monitor receiver already taken")?;

    let audio_level = Arc::new(AtomicU32::new(0));
    let mut stream: Option<cpal::Stream> = None;
    let mut last_activity = false;
    let mut last_seen_start_ms: u64 = 0;

    loop {
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Detect new recording session and reset last_activity so the
        // diff-guard doesn't suppress the first audio-activity event.
        let current_start_ms = state.recording_start_time.load(Ordering::SeqCst);
        if current_start_ms != last_seen_start_ms {
            last_seen_start_ms = current_start_ms;
            last_activity = false;
        }

        // Drain all pending commands; only the last one matters.
        let mut cmd: Option<bool> = None;
        while let Ok(v) = rx.try_recv() {
            cmd = Some(v);
        }

        if let Some(should_open) = cmd {
            if should_open && stream.is_none() {
                // Open the mic stream
                let host = cpal::default_host();
                let audio_device = {
                    let settings = state.settings.lock();
                    settings.audio_device.clone()
                };
                let device = if audio_device == "default" {
                    host.default_input_device()
                } else {
                    host.input_devices()
                        .ok()
                        .and_then(|mut devs| {
                            devs.find(|d| d.name().ok().as_deref() == Some(&audio_device))
                        })
                        .or_else(|| host.default_input_device())
                };

                if let Some(device) = device {
                    match device.default_input_config() {
                        Ok(config) => {
                            let level_clone = audio_level.clone();
                            let err_fn = |err| error!(error = %err, "audio_stream_error");
                            match device.build_input_stream(
                                &config.into(),
                                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                                    let sum: f32 = data.iter().map(|&s| s * s).sum::<f32>()
                                        / data.len() as f32;
                                    let rms = sum.sqrt();
                                    let db = 20.0 * rms.log10();
                                    let normalized =
                                        ((db + 60.0) / 60.0 * 100.0).clamp(0.0, 100.0) as u32;
                                    level_clone.store(normalized, Ordering::SeqCst);
                                },
                                err_fn,
                                None,
                            ) {
                                Ok(s) => match s.play() {
                                    Ok(()) => {
                                        info!("audio_level_stream_opened");
                                        stream = Some(s);
                                    }
                                    Err(e) => {
                                        error!(error = %e, "audio_level_stream_play_failed")
                                    }
                                },
                                Err(e) => error!(error = %e, "audio_level_stream_build_failed"),
                            }
                        }
                        Err(e) => {
                            error!(error = %e, "audio_level_input_config_failed")
                        }
                    }
                } else {
                    warn!("audio_level_input_device_not_found");
                }
            } else if !should_open && stream.is_some() {
                // Close the mic stream
                drop(stream.take());
                audio_level.store(0, Ordering::SeqCst);
                info!("audio_level_stream_closed");
            }
        }

        let level = if stream.is_some() {
            audio_level.load(Ordering::SeqCst)
        } else {
            0
        };

        state.audio_level.store(level, Ordering::SeqCst);
        let _ = app.emit(EventName::AUDIO_LEVEL, level);

        // Emit audio activity with hysteresis to avoid rapid flickering.
        let has_activity = if last_activity {
            level >= AUDIO_ACTIVITY_OFF_THRESHOLD
        } else {
            level > AUDIO_ACTIVITY_ON_THRESHOLD
        };
        // Suppress `true` for the first 400ms after recording starts so the
        // start-beep picked up by the mic doesn't flash the indicator green.
        let suppressed = if has_activity {
            let start_ms = state.recording_start_time.load(Ordering::SeqCst);
            if start_ms > 0 {
                let now_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64;
                now_ms.saturating_sub(start_ms) < 400
            } else {
                false
            }
        } else {
            false
        };
        let effective_activity = has_activity && !suppressed;
        if effective_activity != last_activity {
            let _ = app.emit(EventName::AUDIO_ACTIVITY, effective_activity);
            last_activity = effective_activity;
        }
    }
}

fn build_stt_engine_prompt(
    initial_prompt: &str,
    language: &str,
    domain: &str,
    subdomain: &str,
    glossary: &str,
) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();

    // Language-specific base prompt (most important for output language)
    let lang_code = language.split('-').next().unwrap_or(language);
    let base_prompt = get_language_base_prompt(lang_code);
    if !base_prompt.is_empty() {
        parts.push(base_prompt.to_string());
    }

    // User's custom initial_prompt (if provided, takes precedence)
    if !initial_prompt.is_empty() {
        parts.push(initial_prompt.to_string());
    }

    // Domain-specific prompt in target language
    if domain != "general" {
        let domain_prompt = get_domain_prompt(lang_code, domain, subdomain);
        if !domain_prompt.is_empty() {
            parts.push(domain_prompt);
        }
    }

    // Glossary in target language format
    if !glossary.is_empty() {
        let glossary_formatted = format_glossary(lang_code, glossary);
        parts.push(glossary_formatted);
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n"))
    }
}

fn get_language_base_prompt(lang: &str) -> &'static str {
    match lang {
        "zh" => "以下是普通话的句子，请用简体中文输出。",
        "en" => "The following is an English sentence.",
        "ja" => "以下は日本語の文章です。",
        "ko" => "다음은 한국어 문장입니다.",
        "fr" => "Voici une phrase en français.",
        "de" => "Das ist ein deutscher Satz.",
        "es" => "Esta es una oración en español.",
        "ru" => "Это предложение на русском языке.",
        "pt" => "Esta é uma frase em português.",
        "it" => "Questa è una frase in italiano.",
        _ => "",
    }
}

fn get_domain_prompt(lang: &str, domain: &str, subdomain: &str) -> String {
    if domain == "general" || domain.is_empty() {
        return String::new();
    }

    match lang {
        "zh" => {
            if subdomain == "general" || subdomain.is_empty() {
                format!(
                    "请优先识别{}领域的专业术语和技术词汇。",
                    get_domain_name_zh(domain)
                )
            } else {
                format!(
                    "请优先识别{}领域中{}相关的专业术语。",
                    get_domain_name_zh(domain),
                    subdomain
                )
            }
        }
        "en" => {
            if subdomain == "general" || subdomain.is_empty() {
                format!(
                    "Prioritize recognition of {} terminology and technical terms.",
                    get_domain_name_en(domain)
                )
            } else {
                format!(
                    "Prioritize recognition of {} terminology, focusing on {}.",
                    get_domain_name_en(domain),
                    subdomain
                )
            }
        }
        _ => {
            if subdomain == "general" || subdomain.is_empty() {
                format!(
                    "Prioritize recognition of {} terminology and technical terms.",
                    get_domain_name_en(domain)
                )
            } else {
                format!(
                    "Prioritize recognition of {} terminology, focusing on {}.",
                    get_domain_name_en(domain),
                    subdomain
                )
            }
        }
    }
}

fn get_domain_name_zh(domain: &str) -> String {
    match domain {
        "it" => "IT技术".to_string(),
        "legal" => "法律".to_string(),
        "medical" => "医学".to_string(),
        "finance" => "金融".to_string(),
        "education" => "教育".to_string(),
        _ => domain.to_string(),
    }
}

fn get_domain_name_en(domain: &str) -> String {
    match domain {
        "it" => "IT".to_string(),
        "legal" => "legal".to_string(),
        "medical" => "medical".to_string(),
        "finance" => "finance".to_string(),
        "education" => "education".to_string(),
        _ => domain.to_string(),
    }
}

fn format_glossary(lang: &str, glossary: &str) -> String {
    match lang {
        "zh" => format!("专业词汇：{}", glossary),
        "ja" => format!("専門用語：{}", glossary),
        "ko" => format!("전문 용어：{}", glossary),
        "en" => format!("Terminology: {}", glossary),
        _ => format!("Terminology: {}", glossary),
    }
}

fn log_wav_file_info(audio_path: &str, task_id: u64) {
    let path = PathBuf::from(audio_path);

    let file_size = match std::fs::metadata(&path) {
        Ok(meta) => meta.len(),
        Err(e) => {
            warn!(task_id, error = %e, path = %audio_path, "wav_file_size_failed");
            return;
        }
    };

    match WavReader::open(&path) {
        Ok(reader) => {
            let spec = reader.spec();
            let duration_secs = reader.duration() as f64 / spec.sample_rate as f64;

            info!(task_id, path = %audio_path, file_size_bytes = file_size, sample_rate = spec.sample_rate, channels = spec.channels, bits_per_sample = spec.bits_per_sample, duration_secs = duration_secs, "wav_file_info");
        }
        Err(e) => {
            warn!(task_id, error = %e, path = %audio_path, "wav_file_read_failed");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{await_streaming_task_in_background, maybe_polish_transcription_text};
    use crate::commands::settings::CloudProviderConfig;
    use crate::state::app_state::AppState;
    use std::time::Duration;
    use wiremock::matchers::{body_partial_json, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn waiting_for_streaming_task_does_not_block_the_active_runtime() {
        let (done_tx, done_rx) = tokio::sync::oneshot::channel();
        let handle = tauri::async_runtime::spawn(async move {
            let _ = done_tx.send(());
        });

        await_streaming_task_in_background(1, handle);

        tokio::time::timeout(Duration::from_secs(1), done_rx)
            .await
            .expect("streaming task should complete without blocking the runtime")
            .expect("streaming task completion signal should be delivered");
    }

    #[tokio::test]
    async fn streaming_finalization_honors_cloud_polish_settings() {
        let mock_server = MockServer::start().await;

        let user_prompt = "System instruction here";
        let expected_system_content = format!(
            "{}\n\nUSER RULES:\n{}",
            crate::polish_engine::cloud::engine::CORE_POLISH_CONSTRAINT,
            user_prompt
        );

        let expected_body = serde_json::json!({
            "model": "gpt-4o-mini",
            "max_tokens": 4096,
            "messages": [
                {
                    "role": "system",
                    "content": expected_system_content
                },
                {
                    "role": "user",
                    "content": "User text here"
                }
            ]
        });

        let response_body = serde_json::json!({
            "choices": [
                {
                    "message": {
                        "content": "Polished streaming text"
                    }
                }
            ]
        });

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .and(header("Authorization", "Bearer test_openai_api_key"))
            .and(header("Content-Type", "application/json"))
            .and(body_partial_json(expected_body))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .mount(&mock_server)
            .await;

        let state = AppState::new();
        {
            let mut settings = state.settings.lock();
            settings.polish_enabled = false;
            settings.cloud_polish_enabled = true;
            settings.active_cloud_polish_provider = "openai".to_string();
            settings.stt_engine_language = "en-US".to_string();
            settings.polish_system_prompt = user_prompt.to_string();
            settings.cloud_polish_configs.insert(
                "openai".to_string(),
                CloudProviderConfig {
                    enabled: true,
                    provider_type: "openai".to_string(),
                    api_key: "test_openai_api_key".to_string(),
                    base_url: mock_server.uri(),
                    model: "gpt-4o-mini".to_string(),
                    enable_thinking: false,
                },
            );
        }

        let (final_text, _polish_time_ms) =
            maybe_polish_transcription_text(None, &state, 1, "User text here".to_string()).await;

        assert_eq!(final_text, "Polished streaming text");
    }
}
