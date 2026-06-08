use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};
use tracing::{error, info, warn};

use crate::events::{
    emit_pill_tooltip, emit_recording_state, emit_retry_complete, emit_retry_error,
    emit_retry_state, EventName, RecordingStatus, RetryStatus,
};
use crate::services::transcription_finalize::FinalizeResult;
use crate::state::app_state::AppState;

pub(crate) type ParkingMutex<T> = parking_lot::Mutex<T>;

pub(crate) const AUDIO_ACTIVITY_ON_THRESHOLD: u32 = 40;
pub(crate) const AUDIO_ACTIVITY_OFF_THRESHOLD: u32 = 25;
pub(crate) const RECORDING_CHUNK_DURATION_MS: usize = 200;
pub(crate) const ERROR_STATE_SETTLE_MS: u64 = 2000;
pub(crate) const PROCESSING_ERROR_TOOLTIP_DURATION_MS: u64 = 4000;
pub(crate) const TRANSCRIPTION_ERROR_TOOLTIP_MESSAGE: &str =
    "Transcription failed. Please try again.";
pub(crate) const POLISH_ERROR_TOOLTIP_MESSAGE: &str =
    "Polish failed. Using original transcription.";
pub(crate) const POLISH_ERROR_TOOLTIP_SUFFIX: &str = "Using original transcription.";
pub(crate) const LOCAL_POLISH_TIMEOUT_TOOLTIP_MESSAGE: &str =
    "Local polish timed out. Using original transcription.";
pub(crate) const POLISH_POLICY_TOOLTIP_MESSAGE: &str =
    "Polish result was rejected. Using original transcription.";

pub(crate) fn polish_error_tooltip_message(reason: Option<&str>) -> String {
    match reason.map(str::trim).filter(|reason| !reason.is_empty()) {
        Some(reason) => format!("Polish failed: {reason}. {POLISH_ERROR_TOOLTIP_SUFFIX}"),
        None => POLISH_ERROR_TOOLTIP_MESSAGE.to_string(),
    }
}

pub(crate) fn should_emit_error_recovery_idle(
    current_task_id: u64,
    error_task_id: u64,
    is_recording: bool,
    is_transcribing: bool,
) -> bool {
    current_task_id == error_task_id && !is_recording && !is_transcribing
}

pub(crate) async fn emit_recording_error_then_idle(app: &AppHandle, task_id: u64) {
    emit_recording_state(app, RecordingStatus::Error, task_id);
    emit_pill_tooltip(
        app,
        TRANSCRIPTION_ERROR_TOOLTIP_MESSAGE,
        PROCESSING_ERROR_TOOLTIP_DURATION_MS,
        Some(task_id),
    );
    tokio::time::sleep(tokio::time::Duration::from_millis(ERROR_STATE_SETTLE_MS)).await;

    let state = app.state::<AppState>();
    let current_task_id = state.task_counter.load(Ordering::SeqCst);
    let is_recording = state.is_recording.load(Ordering::SeqCst);
    let is_transcribing = state.is_transcribing.load(Ordering::SeqCst);
    if !should_emit_error_recovery_idle(current_task_id, task_id, is_recording, is_transcribing) {
        info!(
            task_id,
            current_task_id,
            is_recording,
            is_transcribing,
            "error_recovery_idle_skipped_for_stale_task"
        );
        return;
    }

    emit_recording_state(app, RecordingStatus::Idle, task_id);
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingState {
    pub is_recording: bool,
    pub is_transcribing: bool,
    pub audio_level: u32,
    pub output_path: Option<String>,
}

pub(crate) async fn apply_finalize_result(app: &AppHandle, task_id: u64, result: FinalizeResult) {
    match result {
        FinalizeResult::DeliverText(text) => {
            let _ = app.emit(
                EventName::TRANSCRIPTION_COMPLETE,
                crate::events::TranscriptionCompleteEvent {
                    text: text.clone(),
                    task_id,
                },
            );
            emit_recording_state(app, RecordingStatus::Idle, task_id);
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            crate::text_injector::insert_text(&text);
            let correction_memory_enabled = {
                let state = app.state::<AppState>();
                let settings = state.settings.lock();
                settings.correction_memory_enabled
            };
            if correction_memory_enabled {
                crate::correction_learning::observe_post_delivery_edit(app.clone(), text);
            }
        }
        FinalizeResult::TransitionToIdle => {
            emit_recording_state(app, RecordingStatus::Idle, task_id);
        }
        FinalizeResult::TransitionToErrorThenIdle => {
            emit_recording_error_then_idle(app, task_id).await;
        }
    }
}

pub(crate) async fn apply_retry_success(app: &AppHandle, entry_id: &str, task_id: u64, text: &str) {
    emit_retry_complete(app, entry_id, task_id, text);
    emit_retry_state(app, entry_id, RetryStatus::Completed, task_id);
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    crate::text_injector::insert_text(text);
    let correction_memory_enabled = {
        let state = app.state::<AppState>();
        let settings = state.settings.lock();
        settings.correction_memory_enabled
    };
    if correction_memory_enabled {
        crate::correction_learning::observe_post_delivery_edit(app.clone(), text.to_string());
    }
}

pub(crate) fn apply_retry_error(app: &AppHandle, entry_id: &str, task_id: u64, error: &str) {
    emit_retry_error(app, entry_id, task_id, error);
    emit_retry_state(app, entry_id, RetryStatus::Error, task_id);
    emit_pill_tooltip(
        app,
        TRANSCRIPTION_ERROR_TOOLTIP_MESSAGE,
        PROCESSING_ERROR_TOOLTIP_DURATION_MS,
        None,
    );
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug)]
pub(crate) enum ProcessingEventTarget<'a> {
    None,
    Recording(&'a AppHandle),
    Retry {
        app: &'a AppHandle,
        entry_id: &'a str,
    },
}

impl ProcessingEventTarget<'_> {
    pub(crate) fn emit_polishing(&self, task_id: u64) {
        match self {
            Self::None => {}
            Self::Recording(app) => emit_recording_state(app, RecordingStatus::Polishing, task_id),
            Self::Retry { app, entry_id } => {
                emit_retry_state(app, entry_id, RetryStatus::Polishing, task_id);
            }
        }
    }

    pub(crate) fn emit_polish_error_tooltip(&self, task_id: u64, reason: Option<&str>) {
        self.emit_processing_tooltip(task_id, polish_error_tooltip_message(reason));
    }

    pub(crate) fn emit_local_polish_timeout_tooltip(&self, task_id: u64) {
        self.emit_processing_tooltip(task_id, LOCAL_POLISH_TIMEOUT_TOOLTIP_MESSAGE.to_string());
    }

    pub(crate) fn emit_polish_policy_tooltip(&self, task_id: u64) {
        self.emit_processing_tooltip(task_id, POLISH_POLICY_TOOLTIP_MESSAGE.to_string());
    }

    fn emit_processing_tooltip(&self, task_id: u64, message: String) {
        match self {
            Self::None => {}
            Self::Recording(app) => emit_pill_tooltip(
                app,
                message,
                PROCESSING_ERROR_TOOLTIP_DURATION_MS,
                Some(task_id),
            ),
            Self::Retry { app, .. } => {
                emit_pill_tooltip(app, message, PROCESSING_ERROR_TOOLTIP_DURATION_MS, None)
            }
        }
    }
}

pub(crate) fn await_streaming_task_in_background(
    task_id: u64,
    handle: tauri::async_runtime::JoinHandle<()>,
) {
    tauri::async_runtime::spawn(async move {
        if let Err(e) = handle.await {
            error!(task_id, error = %e, "streaming_stt_task_panicked");
        }
    });
}

pub(crate) fn discard_canceled_result(state: &AppState, task_id: u64, audio_path: Option<&String>) {
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

pub(crate) fn recording_chunk_size_samples(sample_rate: u32, channels: u16) -> usize {
    (sample_rate as usize * channels as usize * RECORDING_CHUNK_DURATION_MS) / 1000
}

pub(crate) fn flush_pending_chunk_for_stop(
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

pub(crate) async fn send_flushed_chunk_for_stop(
    audio_tx: &tokio::sync::mpsc::Sender<Vec<i16>>,
    chunk: Vec<i16>,
) -> Result<(), tokio::sync::mpsc::error::SendError<Vec<i16>>> {
    audio_tx.send(chunk).await
}

pub(crate) fn should_unregister_cancel_hotkey_after_async_cleanup(
    active_task_id: u64,
    cleanup_task_id: u64,
    cancellation_requested: bool,
) -> bool {
    active_task_id == cleanup_task_id && !cancellation_requested
}

#[cfg(test)]
mod tests {
    use super::{
        polish_error_tooltip_message, LOCAL_POLISH_TIMEOUT_TOOLTIP_MESSAGE,
        POLISH_ERROR_TOOLTIP_MESSAGE,
    };

    #[test]
    fn polish_error_tooltip_includes_reason_when_available() {
        assert_eq!(
            polish_error_tooltip_message(Some("model load failed")),
            "Polish failed: model load failed. Using original transcription."
        );
    }

    #[test]
    fn polish_error_tooltip_keeps_legacy_message_without_reason() {
        assert_eq!(
            polish_error_tooltip_message(None),
            POLISH_ERROR_TOOLTIP_MESSAGE
        );
        assert_eq!(
            polish_error_tooltip_message(Some("   ")),
            POLISH_ERROR_TOOLTIP_MESSAGE
        );
    }

    #[test]
    fn local_polish_timeout_tooltip_is_specific() {
        assert_eq!(
            LOCAL_POLISH_TIMEOUT_TOOLTIP_MESSAGE,
            "Local polish timed out. Using original transcription."
        );
    }
}
