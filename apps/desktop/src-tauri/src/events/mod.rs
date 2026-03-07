use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingStateEvent {
    pub status: String,
    pub task_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionCompleteEvent {
    pub text: String,
    pub task_id: u64,
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
    pub const AUDIO_LEVEL: &str = "audio-level";
    pub const AUDIO_ACTIVITY: &str = "audio-activity";
    pub const TRANSCRIPTION_COMPLETE: &str = "transcription-complete";
    pub const TRANSCRIPTION_ERROR: &str = "transcription-error";
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
    pub const MODEL_UNLOADED: &str = "model-unloaded";
    pub const SETTINGS_CHANGED: &str = "settings-changed";
    pub const TOAST_MESSAGE: &str = "toast-message";
    pub const SHORTCUT_REGISTRATION_FAILED: &str = "shortcut-registration-failed";
}
