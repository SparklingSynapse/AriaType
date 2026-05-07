use std::sync::atomic::Ordering;
use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager};
use tracing::{debug, error, info, warn};

use crate::commands::settings::CloudSttConfig;
use crate::events::{emit_recording_state, EventName, RecordingStatus, TranscriptionPartialEvent};
use crate::services::transcription_finalize::{
    finalize_empty_transcription, finalize_failed_transcription, finalize_silent_recording,
    finalize_successful_transcription,
};
use crate::state::app_state::AppState;
use crate::state::unified_state::StreamingSttState;
use crate::stt_engine::cloud::StreamingSttClient;
use crate::stt_engine::traits::RecordingConsumer;
use crate::utils::AppPaths;

use super::polish::maybe_polish_transcription_text;
use super::shared::{
    apply_finalize_result, discard_canceled_result, emit_recording_error_then_idle,
    recording_chunk_size_samples, should_unregister_cancel_hotkey_after_async_cleanup,
    ParkingMutex, ProcessingEventTarget,
};

pub(super) fn start_unified_recording(
    app: &AppHandle,
    task_id: u64,
    cloud_stt_enabled: bool,
    config: CloudSttConfig,
    language: String,
    resolved_polish_template_id: Option<String>,
) -> Result<(), String> {
    let state = app
        .try_state::<AppState>()
        .ok_or_else(|| "AppState not available".to_string())?;
    let audio_device = {
        let settings = state.settings.lock();
        settings.audio_device.clone()
    };

    let (denoise_mode, vad_enabled, domain, subdomain, glossary, window_context_enabled) = {
        let settings = state.settings.lock();
        let d = settings.stt_engine_work_domain.trim().to_string();
        let s = settings.stt_engine_work_subdomain.trim().to_string();
        let g = settings.stt_engine_user_glossary.trim().to_string();
        (
            settings.denoise_mode.clone(),
            settings.vad_enabled,
            if d.is_empty() { None } else { Some(d) },
            if s.is_empty() { None } else { Some(s) },
            if g.is_empty() { None } else { Some(g) },
            settings.window_context_enabled,
        )
    };

    let (app_tx, mut app_rx) = tokio::sync::mpsc::channel::<Vec<i16>>(100);

    let audio_save_path = AppPaths::recordings_dir().join(format!(
        "{}_{}.wav",
        chrono::Utc::now().format("%Y%m%d_%H%M%S"),
        task_id
    ));

    if let Err(e) = std::fs::create_dir_all(AppPaths::recordings_dir()) {
        warn!(error = %e, "recordings_directory_creation_failed");
    }

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
        sample_rate: 0,
        channels: 0,
    });

    let (sr, ch) = {
        let recorder = state.recorder.lock();
        recorder
            .start_streaming(device_name, move |pcm, sr, ch| {
                if *sample_rate_clone.lock() == 0 {
                    *sample_rate_clone.lock() = sr;
                    *channels_clone.lock() = ch;
                }

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
    let resolved_polish_template_id_clone = resolved_polish_template_id.clone();
    let handle = tauri::async_runtime::spawn(async move {
        let window_context = if window_context_enabled {
            tokio::time::timeout(
                tokio::time::Duration::from_millis(300),
                crate::sensors::window_context::capture_window_context(),
            )
            .await
            .ok()
            .flatten()
        } else {
            None
        };

        if let Some(ref ctx) = window_context {
            let state_for_session = app_clone.state::<AppState>();
            let mut session = state_for_session.session_state.lock();
            if let Some(s) = session.as_mut() {
                s.window_context = Some(ctx.clone());
            }
        }

        let stt_context = crate::stt_engine::traits::SttContext {
            domain,
            subdomain,
            glossary,
            initial_prompt: window_context,
        };

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
                    emit_recording_error_then_idle(&app_clone, task_id).await;
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
                    emit_recording_error_then_idle(&app_clone, task_id).await;
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

        let state_inner = app_clone.state::<AppState>();
        if state_inner.is_cancellation_requested(task_id) {
            discard_canceled_result(&state_inner, task_id, None);
            return;
        }

        if chunks_sent == 0 {
            info!(task_id, "transcription_skipped_no_audio_chunks");
            let action = finalize_silent_recording(None);
            let _ = state_inner.finish_session(task_id);
            apply_finalize_result(&app_clone, task_id, action).await;
        } else {
            emit_recording_state(&app_clone, RecordingStatus::Transcribing, task_id);
            state_inner.is_transcribing.store(true, Ordering::SeqCst);

            debug!(task_id, "consumer_finish_invoked");
            let text_result: Result<String, String> = consumer.finish().await;

            let state_inner = app_clone.state::<AppState>();
            if state_inner.is_cancellation_requested(task_id) {
                discard_canceled_result(&state_inner, task_id, None);
                return;
            }

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
                        maybe_polish_transcription_text(
                            &ProcessingEventTarget::Recording(&app_clone),
                            &state,
                            task_id,
                            text,
                            resolved_polish_template_id_clone.clone(),
                        )
                        .await
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

                    let action = if !final_text.is_empty() {
                        finalize_successful_transcription(
                            &state,
                            &raw_text,
                            &final_text,
                            polish_time_ms,
                            audio_path.clone(),
                        )
                    } else {
                        finalize_empty_transcription(&state, audio_path)
                    };
                    let _ = state.finish_session(task_id);
                    apply_finalize_result(&app_clone, task_id, action).await;
                }
                Err(e) => {
                    let state = app_clone.state::<AppState>();
                    if state.is_cancellation_requested(task_id) {
                        discard_canceled_result(&state, task_id, audio_path.as_ref());
                        return;
                    }
                    error!(task_id, error = %e, "stt_finish_failed");

                    let action = finalize_failed_transcription(&state, audio_path, &e);
                    let _ = state.finish_session(task_id);
                    apply_finalize_result(&app_clone, task_id, action).await;
                }
            }
        }

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
