use parking_lot::Mutex;
use std::collections::HashSet;
use std::sync::{
    atomic::{AtomicBool, AtomicU64},
    mpsc, Arc,
};
use tauri::async_runtime::JoinHandle;
use tokio::sync::mpsc as async_mpsc;

#[derive(Debug, Clone, Default)]
pub struct SessionState {
    pub task_id: u64,
    pub accumulated_text: String,
    pub chunk_count: usize,
}

/// Recording consumer state for all STT engines.
pub struct StreamingSttState {
    pub audio_tx: async_mpsc::Sender<Vec<i16>>,
    pub accumulated_text: String,
    pub task_id: u64,
    pub streaming_task: Arc<Mutex<Option<JoinHandle<()>>>>,
    /// Path where the raw audio will be saved (for retry functionality).
    pub audio_save_path: Option<std::path::PathBuf>,
    /// Accumulated raw PCM samples (before VAD/processing) for saving to file.
    pub raw_audio_buffer: Arc<Mutex<Vec<i16>>>,
    /// Buffered raw device PCM that has not yet reached the chunk threshold.
    pub chunk_buffer: Arc<Mutex<Vec<i16>>>,
    /// Streaming processor used for chunk preprocessing and VAD.
    pub processor: Arc<Mutex<crate::audio::stream_processor::StreamAudioProcessor>>,
    /// Sample rate of the recording.
    pub sample_rate: u32,
    /// Number of channels.
    pub channels: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, Default)]
pub enum RecordingState {
    #[default]
    Idle,
    Starting,
    Recording,
    Stopping,
    Transcribing,
    Error,
}

impl RecordingState {
    pub fn as_str(&self) -> &'static str {
        match self {
            RecordingState::Idle => "idle",
            RecordingState::Starting => "starting",
            RecordingState::Recording => "recording",
            RecordingState::Stopping => "stopping",
            RecordingState::Transcribing => "transcribing",
            RecordingState::Error => "error",
        }
    }

    pub fn can_transition_to(&self, next: RecordingState) -> bool {
        matches!(
            (self, next),
            (RecordingState::Idle, RecordingState::Starting)
                | (RecordingState::Starting, RecordingState::Recording)
                | (RecordingState::Starting, RecordingState::Error)
                | (RecordingState::Starting, RecordingState::Idle)
                | (RecordingState::Recording, RecordingState::Stopping)
                | (RecordingState::Recording, RecordingState::Error)
                | (RecordingState::Stopping, RecordingState::Transcribing)
                | (RecordingState::Stopping, RecordingState::Idle)
                | (RecordingState::Stopping, RecordingState::Error)
                | (RecordingState::Transcribing, RecordingState::Idle)
                | (RecordingState::Transcribing, RecordingState::Error)
                | (RecordingState::Error, RecordingState::Idle)
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordingMode {
    Toggle,
    PushToTalk,
}

pub struct UnifiedRecordingState {
    current: Mutex<RecordingState>,
    error: Mutex<Option<String>>,
}

impl UnifiedRecordingState {
    pub fn new() -> Self {
        Self {
            current: Mutex::new(RecordingState::Idle),
            error: Mutex::new(None),
        }
    }

    pub fn current(&self) -> RecordingState {
        *self.current.lock()
    }

    pub fn get_error(&self) -> Option<String> {
        self.error.lock().clone()
    }

    pub fn transition_to(&self, new_state: RecordingState) -> Result<(), String> {
        let mut current = self.current.lock();
        let old_state = *current;

        if current.can_transition_to(new_state) {
            *current = new_state;
            if new_state != RecordingState::Error {
                *self.error.lock() = None;
            }
            tracing::info!(
                from = %old_state.as_str(),
                to = %new_state.as_str(),
                "state_transition"
            );
            Ok(())
        } else {
            tracing::warn!(
                from = %old_state.as_str(),
                to = %new_state.as_str(),
                "state_transition_rejected"
            );
            Err(format!(
                "Invalid state transition from {:?} to {:?}",
                old_state, new_state
            ))
        }
    }

    pub fn transition_to_with_error(
        &self,
        new_state: RecordingState,
        error: Option<String>,
    ) -> Result<(), String> {
        let result = self.transition_to(new_state);
        if let Some(err) = error {
            *self.error.lock() = Some(err);
        }
        result
    }

    pub fn force_transition(&self, new_state: RecordingState) {
        let old_state = *self.current.lock();
        *self.current.lock() = new_state;
        if new_state != RecordingState::Error {
            *self.error.lock() = None;
        }
        tracing::warn!(
            from = %old_state.as_str(),
            to = %new_state.as_str(),
            "state_transition_forced"
        );
    }
}

impl Default for UnifiedRecordingState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct AppState {
    pub recording_state: UnifiedRecordingState,
    pub recording_mode: Mutex<RecordingMode>,
    pub canceled_tasks: Mutex<HashSet<u64>>,
    pub current_recording_path: Mutex<Option<std::path::PathBuf>>,
    pub transcription_task: Mutex<Option<tokio::task::JoinHandle<()>>>,
    pub audio_level: std::sync::atomic::AtomicU32,
    pub settings: Mutex<crate::commands::settings::AppSettings>,
    pub recorder: Mutex<crate::audio::recorder::AudioRecorder>,
    /// Unified engine manager for STT operations
    pub engine_manager: Arc<crate::stt_engine::UnifiedEngineManager>,
    /// Unified polish manager for text polishing operations
    pub polish_manager: Arc<crate::polish_engine::UnifiedPolishManager>,
    // Legacy fields used by audio commands
    pub is_recording: AtomicBool,
    pub is_transcribing: AtomicBool,
    pub output_path: Mutex<Option<String>>,
    /// Monotonically increasing task ID; incremented on each new recording session
    pub task_counter: AtomicU64,
    /// Timestamp (ms since UNIX epoch) when the current recording started; used to
    /// suppress spurious `audio-activity` events caused by the start beep
    pub recording_start_time: AtomicU64,
    /// Channel sender to command the audio level monitor thread to open/close the mic stream.
    /// The receiver lives exclusively on the monitor thread (taken out on first use).
    pub level_monitor_tx: Mutex<Option<mpsc::Sender<bool>>>,
    pub level_monitor_rx: Mutex<Option<mpsc::Receiver<bool>>>,
    /// Tracks model names currently being downloaded to prevent duplicate downloads
    pub downloading_models: Mutex<std::collections::HashSet<String>>,
    /// Cancellation flags for active model downloads, keyed by model name
    pub download_cancellations: Mutex<std::collections::HashMap<String, Arc<AtomicBool>>>,
    /// Cancellation flags for active polish model downloads, keyed by model ID
    pub polish_download_cancellations: Mutex<std::collections::HashMap<String, Arc<AtomicBool>>>,
    pub idle_timer_running: AtomicBool,
    /// Current recording session state for accumulating transcription text
    pub session_state: Mutex<Option<SessionState>>,
    /// Recording consumer state for all STT engines (None when idle)
    pub streaming_stt: Mutex<Option<StreamingSttState>>,
    /// Transcription history store (SQLite)
    pub history_store: Mutex<crate::history::HistoryStore>,
}

impl AppState {
    pub fn new() -> Self {
        let recorder = crate::audio::recorder::AudioRecorder::new();
        let settings = crate::commands::settings::load_settings_from_disk();
        let (level_tx, level_rx) = mpsc::channel::<bool>();

        // Initialize unified engine manager with default models directory
        let models_dir = crate::stt_engine::UnifiedEngineManager::default_models_dir();
        let engine_manager = Arc::new(crate::stt_engine::UnifiedEngineManager::new(models_dir));
        engine_manager.set_provider(settings.gpu_acceleration);

        // Initialize unified polish manager
        let polish_manager = Arc::new(crate::polish_engine::UnifiedPolishManager::new());

        Self {
            recording_state: UnifiedRecordingState::new(),
            recording_mode: Mutex::new(RecordingMode::Toggle),
            canceled_tasks: Mutex::new(HashSet::new()),
            current_recording_path: Mutex::new(None),
            transcription_task: Mutex::new(None),
            audio_level: std::sync::atomic::AtomicU32::new(0),
            settings: Mutex::new(settings),
            recorder: Mutex::new(recorder),
            engine_manager,
            polish_manager,
            is_recording: AtomicBool::new(false),
            is_transcribing: AtomicBool::new(false),
            output_path: Mutex::new(None),
            task_counter: AtomicU64::new(0),
            recording_start_time: AtomicU64::new(0),
            level_monitor_tx: Mutex::new(Some(level_tx)),
            level_monitor_rx: Mutex::new(Some(level_rx)),
            downloading_models: Mutex::new(std::collections::HashSet::new()),
            download_cancellations: Mutex::new(std::collections::HashMap::new()),
            polish_download_cancellations: Mutex::new(std::collections::HashMap::new()),
            idle_timer_running: AtomicBool::new(false),
            session_state: Mutex::new(None),
            streaming_stt: Mutex::new(None),
            history_store: Mutex::new(
                crate::history::HistoryStore::new().expect("failed to initialize history store"),
            ),
        }
    }

    pub fn request_cancellation(&self, task_id: u64) {
        self.canceled_tasks.lock().insert(task_id);
    }

    pub fn clear_cancellation(&self, task_id: u64) {
        self.canceled_tasks.lock().remove(&task_id);
    }

    pub fn is_cancellation_requested(&self, task_id: u64) -> bool {
        self.canceled_tasks.lock().contains(&task_id)
    }

    pub fn get_current_state(&self) -> RecordingState {
        self.recording_state.current()
    }

    pub fn start_session(&self, task_id: u64) {
        let mut session = self.session_state.lock();
        *session = Some(SessionState {
            task_id,
            accumulated_text: String::new(),
            chunk_count: 0,
        });
    }

    pub fn append_session_text(&self, task_id: u64, text: &str) {
        let mut session = self.session_state.lock();
        if let Some(s) = session.as_mut() {
            if s.task_id == task_id && !text.is_empty() {
                if !s.accumulated_text.is_empty() {
                    s.accumulated_text.push(' ');
                }
                s.accumulated_text.push_str(text);
                s.chunk_count += 1;
            }
        }
    }

    pub fn get_session_text(&self, task_id: u64) -> Option<(String, usize)> {
        let session = self.session_state.lock();
        session
            .as_ref()
            .filter(|s| s.task_id == task_id)
            .map(|s| (s.accumulated_text.clone(), s.chunk_count))
    }

    pub fn finish_session(&self, task_id: u64) -> Option<(String, usize)> {
        let mut session = self.session_state.lock();
        session
            .take()
            .filter(|s| s.task_id == task_id)
            .map(|s| (s.accumulated_text, s.chunk_count))
    }

    pub fn clear_session(&self) {
        *self.session_state.lock() = None;
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

unsafe impl Send for AppState {}
unsafe impl Sync for AppState {}

#[cfg(test)]
mod tests {
    use super::AppState;

    #[test]
    fn cancellation_is_tracked_per_task() {
        let state = AppState::new();

        state.request_cancellation(7);

        assert!(state.is_cancellation_requested(7));
        assert!(!state.is_cancellation_requested(8));
    }

    #[test]
    fn clearing_one_task_cancellation_does_not_affect_others() {
        let state = AppState::new();

        state.request_cancellation(3);
        state.request_cancellation(4);
        state.clear_cancellation(3);

        assert!(!state.is_cancellation_requested(3));
        assert!(state.is_cancellation_requested(4));
    }
}
