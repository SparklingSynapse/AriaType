use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use parking_lot::Mutex as ParkingMutex;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};
use tracing::{debug, error, info, instrument, warn};

use crate::events::{
    EventName, RecordingStateEvent, TranscriptionCompleteEvent, TranscriptionPartialEvent,
};
use crate::state::app_state::AppState;
use crate::state::unified_state::StreamingSttState;
use crate::stt_engine::cloud::StreamingSttClient;
use crate::stt_engine::traits::RecordingConsumer;
use crate::utils::AppPaths;

// Audio activity thresholds (0-100 normalized scale)
// ON: ~-36 dB, above typical office noise
// OFF: ~-45 dB, provides hysteresis
const AUDIO_ACTIVITY_ON_THRESHOLD: u32 = 40;
const AUDIO_ACTIVITY_OFF_THRESHOLD: u32 = 25;
const RECORDING_CHUNK_DURATION_MS: usize = 200;

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
    tracing::info!("start_recording_sync_entered");

    // Use try_state to avoid panic if state not available
    let state = app
        .try_state::<AppState>()
        .ok_or_else(|| "AppState not available".to_string())?;

    tracing::info!("start_recording_sync_state_acquired");

    if state.is_recording.load(Ordering::SeqCst) {
        tracing::warn!("start_recording_sync_already_recording");
        return Err("Already recording".to_string());
    }

    // Show pill immediately on hotkey press
    tracing::info!("start_recording_sync_positioning_pill");
    {
        let settings = state.settings.lock();
        let preset = settings.pill_position.clone();
        drop(settings);
        crate::commands::window::position_pill_window(&app, &preset);
    }

    tracing::info!("start_recording_sync_updating_visibility");
    state.is_recording.store(true, Ordering::SeqCst);
    state.is_transcribing.store(false, Ordering::SeqCst);
    crate::commands::window::update_pill_visibility(&app);

    tracing::info!("start_recording_sync_playing_beep");
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

    tracing::info!("start_recording_sync_reading_settings");
    let (cloud_stt_enabled, cloud_stt_config, language) = {
        let settings = state.settings.lock();
        (
            #[allow(deprecated)]
            settings.is_volcengine_streaming_active(),
            settings.get_active_cloud_stt_config(),
            settings.stt_engine_language.clone(),
        )
    };

    tracing::info!(cloud_stt_enabled, language = %language, "start_recording_sync_config");

    let task_id = state.task_counter.fetch_add(1, Ordering::SeqCst) + 1;
    tracing::info!(task_id, "start_recording_sync_starting_session");
    state.start_session(task_id);

    let start_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    state.recording_start_time.store(start_ms, Ordering::SeqCst);

    start_unified_recording(&app, task_id, cloud_stt_enabled, cloud_stt_config, language)?;

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

    // Register cancel hotkey
    if let Some(shortcut_manager) = app.try_state::<crate::shortcut::ShortcutManager>() {
        let _ = shortcut_manager.register_cancel(task_id);
    }

    Ok(())
}

#[tauri::command]
pub async fn cancel_recording(app: AppHandle) -> Result<(), String> {
    cancel_recording_sync(app)
}

pub fn cancel_recording_sync(app: AppHandle) -> Result<(), String> {
    cancel_recording_internal(app, true)
}

pub fn cancel_recording_from_hotkey_sync(app: AppHandle) -> Result<(), String> {
    cancel_recording_internal(app, false)
}

fn cancel_recording_internal(
    app: AppHandle,
    unregister_cancel_hotkey_immediately: bool,
) -> Result<(), String> {
    tracing::info!("cancel_recording_sync_entered");

    let state = app
        .try_state::<AppState>()
        .ok_or_else(|| "AppState not available".to_string())?;

    let task_id = state.task_counter.load(Ordering::SeqCst);

    // Hotkey-triggered cancellation must keep the hotkey registered until ESC is released.
    if unregister_cancel_hotkey_immediately {
        if let Some(shortcut_manager) = app.try_state::<crate::shortcut::ShortcutManager>() {
            let _ = shortcut_manager.unregister_cancel_for_task(task_id);
        }
    }

    // We should cancel if we are recording OR transcribing
    let is_recording = state.is_recording.load(Ordering::SeqCst);
    let is_transcribing = state.is_transcribing.load(Ordering::SeqCst);

    if !is_recording && !is_transcribing {
        return Ok(());
    }

    // Stop the recorder if it's running
    if is_recording {
        let recorder = state.recorder.lock();
        let _ = recorder.stop();
    }

    // Request cancellation for this specific task so stale async work
    // cannot publish results after a newer session starts.
    state.request_cancellation(task_id);

    // Update states
    state.is_recording.store(false, Ordering::SeqCst);
    state.is_transcribing.store(false, Ordering::SeqCst);

    if let Some(tx) = state.level_monitor_tx.lock().as_ref() {
        let _ = tx.send(false);
    }

    state.clear_session();

    if let Some(stt) = state.streaming_stt.lock().take() {
        if let Some(handle) = stt.streaming_task.lock().take() {
            await_streaming_task_in_background(task_id, handle);
        }
    }

    // Close pill window
    crate::commands::window::update_pill_visibility(&app);

    // Emit cancellation event as 'idle' to ensure frontend correctly hides pill window
    let _ = app.emit(
        EventName::RECORDING_STATE_CHANGED,
        RecordingStateEvent {
            status: "idle".to_string(),
            task_id,
        },
    );

    tracing::info!(task_id, "recording_canceled");
    Ok(())
}

/// Memory accumulation for cloud STT (volcengine).
/// Audio is accumulated in memory and sent to API when recording stops.
fn start_unified_recording(
    app: &AppHandle,
    task_id: u64,
    cloud_stt_enabled: bool,
    config: crate::commands::settings::CloudSttConfig,
    language: String,
) -> Result<(), String> {
    let state = app
        .try_state::<AppState>()
        .ok_or_else(|| "AppState not available".to_string())?;
    let audio_device = {
        let settings = state.settings.lock();
        settings.audio_device.clone()
    };

    let (denoise_mode, vad_enabled, stt_context) = {
        let settings = state.settings.lock();
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
        (settings.denoise_mode.clone(), settings.vad_enabled, ctx)
    };

    let (app_tx, mut app_rx) = tokio::sync::mpsc::channel::<Vec<i16>>(100);

    // Create audio save path for retry functionality
    let audio_save_path = AppPaths::recordings_dir().join(format!(
        "{}_{}.wav",
        chrono::Utc::now().format("%Y%m%d_%H%M%S"),
        task_id
    ));

    // Ensure recordings directory exists
    if let Err(e) = std::fs::create_dir_all(AppPaths::recordings_dir()) {
        warn!(error = %e, "recordings_directory_creation_failed");
    }

    // Raw audio buffer for saving original recording (before VAD/processing)
    let raw_audio_buffer: Arc<ParkingMutex<Vec<i16>>> = Arc::new(ParkingMutex::new(Vec::new()));
    let raw_audio_buffer_clone = raw_audio_buffer.clone();

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

    let vad_model_path = state.engine_manager.vad_model_path();
    let vad_model_exists = state.engine_manager.is_vad_model_downloaded();
    let vad_path_arg = if vad_enabled && vad_model_exists {
        Some(vad_model_path.as_path())
    } else {
        None
    };

    let processor: Arc<ParkingMutex<crate::audio::stream_processor::StreamAudioProcessor>> =
        Arc::new(ParkingMutex::new(
            crate::audio::stream_processor::StreamAudioProcessor::new(
                &denoise_mode,
                vad_enabled,
                vad_path_arg,
            ),
        ));

    *state.streaming_stt.lock() = Some(StreamingSttState {
        audio_tx: app_tx.clone(),
        accumulated_text: String::new(),
        task_id,
        streaming_task: Arc::new(ParkingMutex::new(None)),
        audio_save_path: Some(audio_save_path.clone()),
        raw_audio_buffer: raw_audio_buffer.clone(),
        chunk_buffer: chunk_buffer.clone(),
        processor: processor.clone(),
        sample_rate: 0, // Will be set after recorder starts
        channels: 0,    // Will be set after recorder starts
    });

    let (sr, ch) = {
        let recorder = state.recorder.lock();
        recorder
            .start_streaming(device_name, move |pcm, sr, ch| {
                if *sample_rate_clone.lock() == 0 {
                    *sample_rate_clone.lock() = sr;
                    *channels_clone.lock() = ch;
                }

                // Accumulate raw PCM for saving to file (before any processing)
                raw_audio_buffer_clone.lock().extend_from_slice(pcm);

                let mut buffer = chunk_buffer.lock();
                buffer.extend_from_slice(pcm);

                let chunk_size = recording_chunk_size_samples(sr, ch);

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

    // Update streaming_stt state with actual sample rate and channels from recording
    {
        let mut streaming_stt = state.streaming_stt.lock();
        if let Some(stt) = streaming_stt.as_mut() {
            stt.sample_rate = sr;
            stt.channels = ch;
        }
    }

    let sr_for_async = sr;
    let ch_for_async = ch;

    let app_clone = app.clone();
    let handle = tauri::async_runtime::spawn(async move {
        let consumer: Box<dyn RecordingConsumer> = if cloud_stt_enabled {
            let (domain, subdomain, glossary) = (
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

            let client = match StreamingSttClient::new(config, Some(&language), stt_context) {
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
            info!(task_id, provider = %provider_name, domain, subdomain, glossary, "streaming_client_created");

            let app_event_clone = app_clone.clone();
            let callback = Arc::new(move |result: crate::stt_engine::traits::PartialResult| {
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
            });

            match crate::stt_engine::cloud::StreamingConsumer::new(client, callback).await {
                Ok(consumer) => {
                    info!(task_id, provider = %provider_name, "streaming_consumer_connected");
                    Box::new(consumer) as Box<dyn RecordingConsumer>
                }
                Err(e) => {
                    error!(task_id, provider = %provider_name, error = %e, "streaming_consumer_connect_failed");
                    let _ = app_clone.emit(
                        EventName::RECORDING_STATE_CHANGED,
                        RecordingStateEvent {
                            status: "error".to_string(),
                            task_id,
                        },
                    );
                    return;
                }
            }
        } else {
            let state_inner = app_clone.state::<AppState>();
            let (model_name, lang, initial_prompt) = {
                let settings = state_inner.settings.lock();
                (
                    settings.model.clone(),
                    settings.stt_engine_language.clone(),
                    settings.stt_engine_initial_prompt.clone(),
                )
            };

            let (_resolved_engine_type, resolved_model_name) = state_inner
                .engine_manager
                .resolve_available_model(&model_name, &lang);

            if resolved_model_name != model_name {
                info!(
                    requested = %model_name,
                    resolved = %resolved_model_name,
                    "model_fallback_applied"
                );
                let _ = app_clone.emit(
                    EventName::MODEL_RESOLVED,
                    crate::events::ModelResolvedEvent {
                        requested: model_name.clone(),
                        resolved: resolved_model_name.clone(),
                    },
                );
            }

            let engine = crate::stt_engine::buffering_engine::BufferingConsumer::new(
                state_inner.engine_manager.clone(),
                resolved_model_name,
                lang,
                Some(initial_prompt),
                stt_context,
            );

            Box::new(engine) as Box<dyn RecordingConsumer>
        };

        let mut chunks_sent = 0;
        while let Some(chunk) = app_rx.recv().await {
            if let Err(e) = consumer.send_chunk(chunk).await {
                error!(task_id, error = %e, "audio_chunk_send_failed");
                break;
            }
            chunks_sent += 1;
        }
        info!(task_id, total_chunks = chunks_sent, "audio_chunks_all_sent");

        let _ = app_clone.emit(
            EventName::RECORDING_STATE_CHANGED,
            RecordingStateEvent {
                status: "transcribing".to_string(),
                task_id,
            },
        );
        app_clone
            .state::<AppState>()
            .is_transcribing
            .store(true, Ordering::SeqCst);

        debug!(task_id, "consumer_finish_invoked");
        let text_result: Result<String, String> = consumer.finish().await;

        let state_inner = app_clone.state::<AppState>();
        if state_inner.is_cancellation_requested(task_id) {
            discard_canceled_result(&state_inner, task_id, None);
            return;
        }

        // Save raw audio to file regardless of success or failure
        let audio_path = {
            let state = app_clone.state::<AppState>();
            let streaming_stt = state.streaming_stt.lock();
            streaming_stt.as_ref().and_then(|s| {
                crate::audio::wav_writer::save_raw_audio_to_file(s, sr_for_async, ch_for_async)
            })
        };

        match text_result {
            Ok(text) => {
                let raw_text = text.clone();
                let state = app_clone.state::<AppState>();
                if state.is_cancellation_requested(task_id) {
                    discard_canceled_result(&state, task_id, audio_path.as_ref());
                    return;
                }
                let (final_text, polish_time_ms) = if text.is_empty() {
                    (String::new(), 0)
                } else {
                    maybe_polish_transcription_text(Some(&app_clone), &state, task_id, text).await
                };

                if state.is_cancellation_requested(task_id) {
                    discard_canceled_result(&state, task_id, audio_path.as_ref());
                    return;
                }

                info!(
                    task_id,
                    text_len = final_text.len(),
                    audio_saved = audio_path.is_some(),
                    "transcription_final_received"
                );

                if !final_text.is_empty() {
                    crate::history::commands::save_to_history(
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
                        audio_path.clone(),
                    );
                    // Clean up audio file after successful save
                    if let Some(ref path) = audio_path {
                        if let Err(e) = std::fs::remove_file(path) {
                            warn!(error = %e, path = %path, "audio_cleanup_failed");
                        }
                    }
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
                    // Empty result - still save failed entry with audio for potential retry
                    let state = app_clone.state::<AppState>();
                    crate::history::commands::save_failed_history(
                        &state,
                        audio_path,
                        "Empty transcription result",
                    );
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
                if state.is_cancellation_requested(task_id) {
                    discard_canceled_result(&state, task_id, audio_path.as_ref());
                    return;
                }
                error!(task_id, error = %e, "stt_finish_failed");

                // Save failed entry with audio for retry functionality
                crate::history::commands::save_failed_history(&state, audio_path, &e);

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

        // Clean up transcribing state and hotkey after everything is done
        let final_state = app_clone.state::<AppState>();
        final_state.is_transcribing.store(false, Ordering::SeqCst);
        let active_task_id = final_state.task_counter.load(Ordering::SeqCst);
        let cancellation_requested = final_state.is_cancellation_requested(task_id);
        if should_unregister_cancel_hotkey_after_async_cleanup(
            active_task_id,
            task_id,
            cancellation_requested,
        ) {
            if let Some(sm) = app_clone.try_state::<crate::shortcut::ShortcutManager>() {
                let _ = sm.unregister_cancel_for_task(task_id);
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
        cloud = cloud_stt_enabled,
        "recording_started-unified"
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

fn discard_canceled_result(state: &AppState, task_id: u64, audio_path: Option<&String>) {
    tracing::info!(task_id, "recording_canceled_discarding_result");

    if let Some(path) = audio_path {
        if let Err(e) = std::fs::remove_file(path) {
            warn!(task_id, error = %e, path = %path, "canceled_audio_cleanup_failed");
        }
    }

    state.is_transcribing.store(false, Ordering::SeqCst);
    let _ = state.finish_session(task_id);
    state.clear_cancellation(task_id);
}

fn recording_chunk_size_samples(sample_rate: u32, channels: u16) -> usize {
    (sample_rate as usize * channels as usize * RECORDING_CHUNK_DURATION_MS) / 1000
}

fn flush_pending_chunk_for_stop(
    chunk_buffer: &Arc<ParkingMutex<Vec<i16>>>,
    processor: &Arc<ParkingMutex<crate::audio::stream_processor::StreamAudioProcessor>>,
    sample_rate: u32,
    channels: u16,
) -> Option<Vec<i16>> {
    let pending_pcm = {
        let mut buffer = chunk_buffer.lock();
        if buffer.is_empty() || sample_rate == 0 || channels == 0 {
            return None;
        }
        buffer.drain(..).collect::<Vec<i16>>()
    };

    let audio_f32: Vec<f32> = pending_pcm.iter().map(|&s| s as f32 / 32768.0).collect();

    let mono_f32 = if channels == 2 {
        audio_f32
            .chunks(2)
            .map(|stereo| (stereo[0] + stereo.get(1).copied().unwrap_or(0.0)) / 2.0)
            .collect()
    } else {
        audio_f32
    };

    let result = processor
        .lock()
        .process_chunk_for_stop_flush(&mono_f32, sample_rate);
    if result.pcm_16khz_mono.is_empty() {
        None
    } else {
        Some(result.pcm_16khz_mono)
    }
}

async fn send_flushed_chunk_for_stop(
    audio_tx: &tokio::sync::mpsc::Sender<Vec<i16>>,
    chunk: Vec<i16>,
) -> Result<(), tokio::sync::mpsc::error::SendError<Vec<i16>>> {
    audio_tx.send(chunk).await
}

fn should_unregister_cancel_hotkey_after_async_cleanup(
    active_task_id: u64,
    cleanup_task_id: u64,
    cancellation_requested: bool,
) -> bool {
    active_task_id == cleanup_task_id && !cancellation_requested
}

pub fn stop_recording_sync(app: AppHandle) -> Result<Option<String>, String> {
    // Use try_state to avoid panic if state not available
    let state = app
        .try_state::<AppState>()
        .ok_or_else(|| "AppState not available".to_string())?;

    if !state.is_recording.load(Ordering::SeqCst) {
        return Ok(None);
    }

    {
        let recorder = state.recorder.lock();
        recorder.stop().map_err(|e| e.to_string())?;
    }

    let task_id = state.task_counter.load(Ordering::SeqCst);

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

    let streaming_state = state.streaming_stt.lock().take();
    if let Some(stt) = streaming_state {
        if let Some(flushed_chunk) = flush_pending_chunk_for_stop(
            &stt.chunk_buffer,
            &stt.processor,
            stt.sample_rate,
            stt.channels,
        ) {
            if let Err(e) = tauri::async_runtime::block_on(send_flushed_chunk_for_stop(
                &stt.audio_tx,
                flushed_chunk,
            )) {
                warn!(task_id, error = %e, "audio_tail_flush_enqueue_failed");
            } else {
                info!(task_id, "audio_tail_flushed_before_finish");
            }
        }

        info!(task_id, "stt_stopping-awaiting_final");
        if let Some(handle) = stt.streaming_task.lock().take() {
            await_streaming_task_in_background(task_id, handle);
        }
    } else {
        warn!(task_id, "streaming_state_missing-recording_interrupted");
        let _ = app.emit(
            EventName::RECORDING_STATE_CHANGED,
            RecordingStateEvent {
                status: "idle".to_string(),
                task_id,
            },
        );
    }

    Ok(None)
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
    let state = app
        .try_state::<AppState>()
        .ok_or_else(|| "AppState not available".to_string())?;

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

/// Retry transcription for a failed entry.
/// Called from frontend when user clicks retry button.
pub async fn retry_transcription_internal(
    app: AppHandle,
    _state: State<'_, AppState>,
    id: String,
) -> Result<String, String> {
    let state = app
        .try_state::<AppState>()
        .ok_or_else(|| "AppState not available".to_string())?;

    // Get the entry to retry
    let entry = {
        let store = state.history_store.lock();
        store
            .get_entry(&id)
            .map_err(|e| format!("Failed to get entry: {e}"))?
    };

    let entry = entry.ok_or_else(|| "Entry not found".to_string())?;

    // Check if entry is in error state
    if entry.status != "error" {
        return Err("Entry is not in error state".to_string());
    }

    // Get audio path
    let audio_path = entry
        .audio_path
        .ok_or_else(|| "No audio file saved for this entry".to_string())?;

    // Check if audio file exists
    if !std::path::Path::new(&audio_path).exists() {
        return Err(format!("Audio file not found: {}", audio_path));
    }

    info!(entry_id = %id, audio_path = %audio_path, "retry_transcription_started");

    // Emit transcribing status
    let _ = app.emit(
        EventName::RECORDING_STATE_CHANGED,
        RecordingStateEvent {
            status: "transcribing".to_string(),
            task_id: 0, // No task_id for retry
        },
    );

    // Transcribe the audio file
    let text_result = transcribe_audio_file(&state, &audio_path).await;

    match text_result {
        Ok(raw_text) => {
            let app_clone = app.clone();
            // Apply polish if enabled
            let (final_text, polish_time_ms) = if raw_text.is_empty() {
                (String::new(), 0)
            } else {
                maybe_polish_transcription_text(Some(&app), &state, 0, raw_text.clone()).await
            };

            if final_text.is_empty() {
                // Still failed
                let store = state.history_store.lock();
                store
                    .mark_error(&id, "Retry produced empty transcription")
                    .map_err(|e| format!("Failed to update entry: {e}"))?;

                let _ = app_clone.emit(
                    EventName::RECORDING_STATE_CHANGED,
                    RecordingStateEvent {
                        status: "error".to_string(),
                        task_id: 0,
                    },
                );

                return Err("Retry produced empty transcription".to_string());
            }

            // Update entry with new text
            let (stt_engine, stt_model) = {
                let settings = state.settings.lock();
                let cloud_config = settings.get_active_cloud_stt_config();
                let is_cloud = cloud_config.enabled;
                (
                    if is_cloud {
                        format!("cloud-{}", cloud_config.provider_type)
                    } else {
                        crate::stt_engine::UnifiedEngineManager::get_engine_by_model_name(
                            &settings.model,
                        )
                        .map(|et| et.as_str().to_string())
                        .unwrap_or_else(|| "unknown".to_string())
                    },
                    if is_cloud {
                        Some(cloud_config.model.clone())
                    } else {
                        Some(settings.model.clone())
                    },
                )
            };

            let updates = crate::history::store::EntryUpdates {
                raw_text: raw_text.clone(),
                final_text: final_text.clone(),
                stt_engine,
                stt_model,
                stt_duration_ms: None,
                polish_duration_ms: if polish_time_ms > 0 {
                    Some(polish_time_ms as i64)
                } else {
                    None
                },
                polish_applied: polish_time_ms > 0,
                polish_engine: None,
            };

            {
                let store = state.history_store.lock();
                store
                    .update_entry(&id, updates)
                    .map_err(|e| format!("Failed to update entry: {e}"))?;
            }

            // Clean up audio file after successful retry
            if let Err(e) = std::fs::remove_file(&audio_path) {
                warn!(error = %e, path = %audio_path, "audio_cleanup_failed_after_retry");
            }

            info!(
                entry_id = %id,
                text_len = final_text.len(),
                "retry_transcription_completed"
            );

            let _ = app_clone.emit(
                EventName::TRANSCRIPTION_COMPLETE,
                TranscriptionCompleteEvent {
                    text: final_text.clone(),
                    task_id: 0,
                },
            );

            let _ = app_clone.emit(
                EventName::RECORDING_STATE_CHANGED,
                RecordingStateEvent {
                    status: "idle".to_string(),
                    task_id: 0,
                },
            );

            // Insert the text
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            let _ = crate::commands::text::do_insert_text(app_clone, final_text.clone()).await;

            Ok(final_text)
        }
        Err(e) => {
            // Update error message
            let store = state.history_store.lock();
            store
                .mark_error(&id, &e)
                .map_err(|e2| format!("Failed to update entry: {e2}"))?;

            let _ = app.emit(
                EventName::RECORDING_STATE_CHANGED,
                RecordingStateEvent {
                    status: "error".to_string(),
                    task_id: 0,
                },
            );

            Err(format!("Transcription failed: {}", e))
        }
    }
}

/// Transcribe an audio file for retry.
async fn transcribe_audio_file(state: &AppState, audio_path: &str) -> Result<String, String> {
    let (model_name, lang) = {
        let settings = state.settings.lock();
        (settings.model.clone(), settings.stt_engine_language.clone())
    };

    // Read audio file
    let reader = hound::WavReader::open(audio_path)
        .map_err(|e| format!("Failed to open audio file: {}", e))?;

    let spec = reader.spec();
    let samples: Vec<i16> = reader.into_samples().filter_map(|s| s.ok()).collect();

    if samples.is_empty() {
        return Err("Audio file is empty".to_string());
    }

    // Convert to f32 samples
    let samples_f32: Vec<f32> = samples.iter().map(|&s| s as f32 / 32768.0).collect();

    // Convert to mono if stereo
    let mono_f32 = if spec.channels == 2 {
        samples_f32
            .chunks(2)
            .map(|chunk| {
                let left = chunk.first().copied().unwrap_or(0.0);
                let right = chunk.get(1).copied().unwrap_or(0.0);
                (left + right) / 2.0
            })
            .collect::<Vec<f32>>()
    } else {
        samples_f32.clone()
    };

    // Resample to 16kHz if needed
    let samples_16k_f32 = if spec.sample_rate != 16000 {
        let ratio = 16000.0 / spec.sample_rate as f32;
        let target_len = (mono_f32.len() as f32 * ratio) as usize;
        mono_f32
            .iter()
            .enumerate()
            .filter_map(|(i, _)| {
                let src_idx = (i as f32 / ratio) as usize;
                mono_f32.get(src_idx).copied()
            })
            .take(target_len)
            .collect()
    } else {
        mono_f32
    };

    // Resolve model
    let (_engine_type, resolved_model_name) = state
        .engine_manager
        .resolve_available_model(&model_name, &lang);

    let engine_type =
        crate::stt_engine::UnifiedEngineManager::get_engine_by_model_name(&resolved_model_name)
            .ok_or_else(|| "Unknown engine type".to_string())?;

    // Create transcription request
    let request = crate::stt_engine::traits::TranscriptionRequest::new(samples_16k_f32)
        .with_model(resolved_model_name.clone())
        .with_language(lang.clone());

    // Transcribe
    let result = state
        .engine_manager
        .transcribe(engine_type, request)
        .await
        .map_err(|e| format!("Transcription failed: {}", e))?;

    Ok(result.text)
}

#[cfg(test)]
mod tests {
    use super::{
        await_streaming_task_in_background, discard_canceled_result,
        flush_pending_chunk_for_stop, maybe_polish_transcription_text,
        recording_chunk_size_samples,
        send_flushed_chunk_for_stop,
        should_unregister_cancel_hotkey_after_async_cleanup,
    };
    use crate::commands::settings::CloudProviderConfig;
    use crate::state::app_state::AppState;
    use std::sync::Arc;
    use std::sync::atomic::Ordering;
    use std::time::Duration;
    use wiremock::matchers::{body_partial_json, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    type ParkingMutex<T> = parking_lot::Mutex<T>;

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

    #[test]
    fn async_cleanup_keeps_cancel_hotkey_while_hotkey_cancel_is_still_active() {
        assert!(!should_unregister_cancel_hotkey_after_async_cleanup(1, 1, true));
    }

    #[test]
    fn async_cleanup_ignores_stale_task_after_a_new_recording_starts() {
        assert!(!should_unregister_cancel_hotkey_after_async_cleanup(2, 1, false));
    }

    #[test]
    fn async_cleanup_unregisters_cancel_hotkey_only_for_current_non_canceled_task() {
        assert!(should_unregister_cancel_hotkey_after_async_cleanup(3, 3, false));
    }

    #[test]
    fn recording_chunk_size_uses_200ms_of_device_audio() {
        assert_eq!(recording_chunk_size_samples(16_000, 1), 3_200);
        assert_eq!(recording_chunk_size_samples(48_000, 2), 19_200);
    }

    #[test]
    fn flush_pending_chunk_for_stop_processes_sub_threshold_tail_audio() {
        let chunk_buffer = Arc::new(ParkingMutex::new(vec![1_000; 1_600]));
        let processor = Arc::new(ParkingMutex::new(
            crate::audio::stream_processor::StreamAudioProcessor::new("off", false, None),
        ));

        let flushed = flush_pending_chunk_for_stop(&chunk_buffer, &processor, 16_000, 1);

        assert_eq!(flushed, Some(vec![999; 1_600]));
        assert!(chunk_buffer.lock().is_empty());
    }

    #[test]
    fn flush_pending_chunk_for_stop_does_not_drop_tail_when_vad_rejects_it() {
        let chunk_buffer = Arc::new(ParkingMutex::new(vec![1_000; 1_600]));
        let processor = Arc::new(ParkingMutex::new(
            crate::audio::stream_processor::StreamAudioProcessor::new("off", true, None),
        ));
        {
            let mut processor_guard = processor.lock();
            processor_guard.force_vad_result_for_test(false);
            processor_guard.set_last_send_time_for_test(std::time::Instant::now());
        }

        let flushed = flush_pending_chunk_for_stop(&chunk_buffer, &processor, 16_000, 1);

        assert_eq!(flushed, Some(vec![999; 1_600]));
        assert!(chunk_buffer.lock().is_empty());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn send_flushed_chunk_for_stop_waits_for_capacity_when_channel_is_full() {
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        tx.send(vec![1]).await.unwrap();

        let send_task = tokio::spawn({
            let tx = tx.clone();
            async move { send_flushed_chunk_for_stop(&tx, vec![2]).await }
        });

        assert_eq!(rx.recv().await, Some(vec![1]));
        send_task.await.unwrap().unwrap();
        assert_eq!(rx.recv().await, Some(vec![2]));
    }

    #[test]
    fn discarding_a_canceled_result_only_clears_that_task() {
        let state = AppState::new();
        state.request_cancellation(5);
        state.request_cancellation(6);
        state.start_session(5);
        state.is_transcribing.store(true, Ordering::SeqCst);

        discard_canceled_result(&state, 5, None);

        assert!(!state.is_cancellation_requested(5));
        assert!(state.is_cancellation_requested(6));
        assert!(!state.is_transcribing.load(Ordering::SeqCst));
        assert!(state.get_session_text(5).is_none());
    }
}
