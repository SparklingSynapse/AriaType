use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingStateEvent {
    pub status: String,
    pub task_id: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordingStatus {
    Recording,
    Transcribing,
    Polishing,
    Idle,
    Error,
}

impl RecordingStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            RecordingStatus::Recording => "recording",
            RecordingStatus::Transcribing => "transcribing",
            RecordingStatus::Polishing => "polishing",
            RecordingStatus::Idle => "idle",
            RecordingStatus::Error => "error",
        }
    }
}

pub fn recording_state_event(status: RecordingStatus, task_id: u64) -> RecordingStateEvent {
    RecordingStateEvent {
        status: status.as_str().to_string(),
        task_id,
    }
}

pub fn emit_recording_state(app: &AppHandle, status: RecordingStatus, task_id: u64) {
    let _ = app.emit(
        EventName::RECORDING_STATE_CHANGED,
        recording_state_event(status, task_id),
    );
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryStateEvent {
    pub entry_id: String,
    pub status: String,
    pub task_id: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryStatus {
    Transcribing,
    Polishing,
    Completed,
    Error,
}

impl RetryStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            RetryStatus::Transcribing => "transcribing",
            RetryStatus::Polishing => "polishing",
            RetryStatus::Completed => "completed",
            RetryStatus::Error => "error",
        }
    }
}

pub fn retry_state_event(entry_id: &str, status: RetryStatus, task_id: u64) -> RetryStateEvent {
    RetryStateEvent {
        entry_id: entry_id.to_string(),
        status: status.as_str().to_string(),
        task_id,
    }
}

pub fn emit_retry_state(app: &AppHandle, entry_id: &str, status: RetryStatus, task_id: u64) {
    let _ = app.emit(
        EventName::RETRY_STATE_CHANGED,
        retry_state_event(entry_id, status, task_id),
    );
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionCompleteEvent {
    pub text: String,
    pub task_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryCompleteEvent {
    pub entry_id: String,
    pub text: String,
    pub task_id: u64,
}

pub fn emit_retry_complete(app: &AppHandle, entry_id: &str, task_id: u64, text: &str) {
    let _ = app.emit(
        EventName::RETRY_COMPLETE,
        RetryCompleteEvent {
            entry_id: entry_id.to_string(),
            text: text.to_string(),
            task_id,
        },
    );
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionPartialEvent {
    pub text: String,
    pub is_definite: bool,
    pub task_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryErrorEvent {
    pub entry_id: String,
    pub error: String,
    pub task_id: u64,
}

pub fn emit_retry_error(app: &AppHandle, entry_id: &str, task_id: u64, error: &str) {
    let _ = app.emit(
        EventName::RETRY_ERROR,
        RetryErrorEvent {
            entry_id: entry_id.to_string(),
            error: error.to_string(),
            task_id,
        },
    );
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionMetrics {
    pub load_time_ms: u64,
    pub preprocess_time_ms: u64,
    pub inference_time_ms: u64,
    pub polish_time_ms: u64,
    pub total_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelDownloadProgressEvent {
    pub model: String,
    pub downloaded: u64,
    pub total: u64,
    pub progress: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelDownloadCompleteEvent {
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelDownloadCancelledEvent {
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolishModelDownloadProgressEvent {
    pub model_id: String,
    pub downloaded: u64,
    pub total: u64,
    pub progress: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolishModelDownloadCompleteEvent {
    pub model_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolishModelDownloadCancelledEvent {
    pub model_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelLoadedEvent {
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelResolvedEvent {
    pub requested: String,
    pub resolved: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelUnloadedEvent {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsChangedEvent {
    pub hotkey: String,
    pub model: String,
    pub stt_engine: String,
    pub pill_position: String,
    pub pill_indicator_mode: String,
    pub auto_start: bool,
    pub gpu_acceleration: bool,
    pub language: String,
    pub stt_engine_language: String,
    pub beep_on_record: bool,
    pub audio_device: String,
    pub polish_enabled: bool,
    pub polish_system_prompt: String,
    pub polish_model: String,
    pub theme_mode: String,
    pub stt_engine_initial_prompt: String,
    pub model_resident: bool,
    pub idle_unload_minutes: u32,
    pub denoise_mode: String,
    pub stt_engine_work_domain: String,
    pub stt_engine_work_domain_prompt: String,
    pub stt_engine_user_glossary: String,
}

#[allow(non_snake_case)]
pub mod EventName {
    pub const RECORDING_STATE_CHANGED: &str = "recording-state-changed";
    pub const RETRY_STATE_CHANGED: &str = "retry-state-changed";
    pub const AUDIO_LEVEL: &str = "audio-level";
    pub const AUDIO_ACTIVITY: &str = "audio-activity";
    pub const TRANSCRIPTION_COMPLETE: &str = "transcription-complete";
    pub const RETRY_COMPLETE: &str = "retry-complete";
    pub const TRANSCRIPTION_PARTIAL: &str = "transcription-partial";
    pub const TRANSCRIPTION_ERROR: &str = "transcription-error";
    pub const RETRY_ERROR: &str = "retry-error";
    pub const TRANSCRIPTION_METRICS: &str = "transcription-metrics";
    pub const MODEL_DOWNLOAD_PROGRESS: &str = "model-download-progress";
    pub const MODEL_DOWNLOAD_COMPLETE: &str = "model-download-complete";
    pub const MODEL_DOWNLOAD_CANCELLED: &str = "model-download-cancelled";
    pub const MODEL_DELETED: &str = "model-deleted";
    pub const POLISH_MODEL_DOWNLOAD_PROGRESS: &str = "polish-model-download-progress";
    pub const POLISH_MODEL_DOWNLOAD_COMPLETE: &str = "polish-model-download-complete";
    pub const POLISH_MODEL_DOWNLOAD_CANCELLED: &str = "polish-model-download-cancelled";
    pub const POLISH_MODEL_DELETED: &str = "polish-model-deleted";
    pub const MODEL_LOADED: &str = "model-loaded";
    pub const MODEL_RESOLVED: &str = "model-resolved";
    pub const MODEL_UNLOADED: &str = "model-unloaded";
    pub const SETTINGS_CHANGED: &str = "settings-changed";
    pub const TOAST_MESSAGE: &str = "toast-message";
    pub const SHORTCUT_REGISTRATION_FAILED: &str = "shortcut-registration-failed";
    pub const SHORTCUT_TRIGGERED: &str = "shortcut-triggered";
    pub const HOTKEY_CAPTURED: &str = "hotkey-captured";
}

#[cfg(test)]
mod tests {
    use super::{recording_state_event, retry_state_event, RecordingStatus, RetryStatus};

    #[test]
    fn recording_status_as_str_matches_frontend_contract() {
        assert_eq!(RecordingStatus::Recording.as_str(), "recording");
        assert_eq!(RecordingStatus::Transcribing.as_str(), "transcribing");
        assert_eq!(RecordingStatus::Polishing.as_str(), "polishing");
        assert_eq!(RecordingStatus::Idle.as_str(), "idle");
        assert_eq!(RecordingStatus::Error.as_str(), "error");
    }

    #[test]
    fn recording_state_event_uses_status_mapping_and_task_id() {
        let event = recording_state_event(RecordingStatus::Polishing, 42);

        assert_eq!(event.status, "polishing");
        assert_eq!(event.task_id, 42);
    }

    #[test]
    fn retry_status_as_str_matches_frontend_contract() {
        assert_eq!(RetryStatus::Transcribing.as_str(), "transcribing");
        assert_eq!(RetryStatus::Polishing.as_str(), "polishing");
        assert_eq!(RetryStatus::Completed.as_str(), "completed");
        assert_eq!(RetryStatus::Error.as_str(), "error");
    }

    #[test]
    fn retry_state_event_uses_status_mapping_entry_id_and_task_id() {
        let event = retry_state_event("entry-42", RetryStatus::Polishing, 7);

        assert_eq!(event.entry_id, "entry-42");
        assert_eq!(event.status, "polishing");
        assert_eq!(event.task_id, 7);
    }
}
