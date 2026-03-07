use parking_lot::Mutex;
use std::collections::VecDeque;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    mpsc, Arc,
};

#[derive(Debug, Clone)]
pub struct TranscriptionJob {
    pub audio_path: String,
    pub timestamp: std::time::SystemTime,
    pub task_id: u64,
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
        match (self, next) {
            (RecordingState::Idle, RecordingState::Starting) => true,
            (RecordingState::Starting, RecordingState::Recording) => true,
            (RecordingState::Starting, RecordingState::Error) => true,
            (RecordingState::Starting, RecordingState::Idle) => true,
            (RecordingState::Recording, RecordingState::Stopping) => true,
            (RecordingState::Recording, RecordingState::Error) => true,
            (RecordingState::Stopping, RecordingState::Transcribing) => true,
            (RecordingState::Stopping, RecordingState::Idle) => true,
            (RecordingState::Stopping, RecordingState::Error) => true,
            (RecordingState::Transcribing, RecordingState::Idle) => true,
            (RecordingState::Transcribing, RecordingState::Error) => true,
            (RecordingState::Error, RecordingState::Idle) => true,
            _ => false,
        }
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

        if current.can_transition_to(new_state) {
            *current = new_state;
            if new_state != RecordingState::Error {
                *self.error.lock() = None;
            }
            Ok(())
        } else {
            Err(format!(
                "Invalid state transition from {:?} to {:?}",
                *current, new_state
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
        *self.current.lock() = new_state;
        if new_state != RecordingState::Error {
            *self.error.lock() = None;
        }
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
    pub should_cancel: AtomicBool,
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
    /// When true, the global hotkey should not trigger recording (e.g. user is setting a new hotkey)
    pub hotkey_capture_mode: AtomicBool,
    /// FIFO queue for transcription jobs
    pub transcription_queue: Mutex<VecDeque<TranscriptionJob>>,
    /// Number of jobs currently being processed
    pub processing_count: std::sync::atomic::AtomicUsize,
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
}

impl AppState {
    pub fn new() -> Self {
        let recorder = crate::audio::recorder::AudioRecorder::new();
        let settings = crate::commands::settings::load_settings_from_disk();
        let (level_tx, level_rx) = mpsc::channel::<bool>();

        // Initialize unified engine manager with default models directory
        let models_dir = crate::stt_engine::UnifiedEngineManager::default_models_dir();
        let engine_manager = Arc::new(crate::stt_engine::UnifiedEngineManager::new(models_dir));

        // Initialize unified polish manager
        let polish_manager = Arc::new(crate::polish_engine::UnifiedPolishManager::new());

        Self {
            recording_state: UnifiedRecordingState::new(),
            recording_mode: Mutex::new(RecordingMode::Toggle),
            should_cancel: AtomicBool::new(false),
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
            hotkey_capture_mode: AtomicBool::new(false),
            transcription_queue: Mutex::new(VecDeque::new()),
            processing_count: std::sync::atomic::AtomicUsize::new(0),
            task_counter: AtomicU64::new(0),
            recording_start_time: AtomicU64::new(0),
            level_monitor_tx: Mutex::new(Some(level_tx)),
            level_monitor_rx: Mutex::new(Some(level_rx)),
            downloading_models: Mutex::new(std::collections::HashSet::new()),
            download_cancellations: Mutex::new(std::collections::HashMap::new()),
            polish_download_cancellations: Mutex::new(std::collections::HashMap::new()),
            idle_timer_running: AtomicBool::new(false),
        }
    }

    pub fn request_cancellation(&self) {
        self.should_cancel.store(true, Ordering::SeqCst);
    }

    pub fn clear_cancellation(&self) {
        self.should_cancel.store(false, Ordering::SeqCst);
    }

    pub fn is_cancellation_requested(&self) -> bool {
        self.should_cancel.load(Ordering::SeqCst)
    }

    pub fn get_current_state(&self) -> RecordingState {
        self.recording_state.current()
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

unsafe impl Send for AppState {}
unsafe impl Sync for AppState {}
