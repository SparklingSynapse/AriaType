use serde::{Deserialize, Serialize};

/// A single transcription history entry stored in the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionEntry {
    pub id: String,
    pub created_at: i64,
    pub raw_text: String,
    pub final_text: String,
    pub stt_engine: String,
    pub stt_model: Option<String>,
    pub language: Option<String>,
    pub audio_duration_ms: Option<i64>,
    pub stt_duration_ms: Option<i64>,
    pub polish_duration_ms: Option<i64>,
    pub total_duration_ms: Option<i64>,
    pub polish_applied: bool,
    pub polish_engine: Option<String>,
    pub is_cloud: bool,
}

/// Parameters for saving a new transcription history entry.
#[derive(Debug, Clone)]
pub struct NewTranscriptionEntry {
    pub raw_text: String,
    pub final_text: String,
    pub stt_engine: String,
    pub stt_model: Option<String>,
    pub language: Option<String>,
    pub audio_duration_ms: Option<i64>,
    pub stt_duration_ms: Option<i64>,
    pub polish_duration_ms: Option<i64>,
    pub total_duration_ms: Option<i64>,
    pub polish_applied: bool,
    pub polish_engine: Option<String>,
    pub is_cloud: bool,
}

/// Summary statistics for the dashboard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardStats {
    /// Total number of transcriptions
    pub total_count: i64,
    /// Number of transcriptions today
    pub today_count: i64,
    /// Total characters typed across all transcriptions
    pub total_chars: i64,
    /// Total cross-language output units across all transcriptions
    pub total_output_units: i64,
    /// Total audio duration in milliseconds
    pub total_audio_ms: i64,
    /// Average STT processing time in milliseconds
    pub avg_stt_ms: Option<i64>,
    /// Average speaking duration per transcription in milliseconds
    pub avg_audio_ms: Option<i64>,
    /// Average output units per transcription
    pub avg_output_units: Option<f64>,
    /// Number of transcriptions using local engines
    pub local_count: i64,
    /// Number of transcriptions using cloud engines
    pub cloud_count: i64,
    /// Number of transcriptions where polish was applied
    pub polish_count: i64,
    /// Number of active usage days
    pub active_days: i64,
    /// Current streak of active days, tolerant of the current partial day
    pub current_streak_days: i64,
    /// Longest streak of active days
    pub longest_streak_days: i64,
    /// Captures in the last 7 days
    pub last_7_days_count: i64,
    /// Audio duration in the last 7 days
    pub last_7_days_audio_ms: i64,
    /// Output units in the last 7 days
    pub last_7_days_output_units: i64,
}

/// Daily usage count for charting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyUsage {
    pub date: String,
    pub count: i64,
    pub audio_ms: i64,
    pub output_units: i64,
}

/// Engine usage breakdown for charting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineUsage {
    pub engine: String,
    pub count: i64,
    pub avg_stt_ms: Option<i64>,
}

/// Filter parameters for querying history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryFilter {
    pub search: Option<String>,
    pub engine: Option<String>,
    pub date_from: Option<i64>,
    pub date_to: Option<i64>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
