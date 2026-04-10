# Rust Coding Style Guide

Project-specific conventions for the AriaType Rust backend (`apps/desktop/src-tauri/`). Clippy enforces most rules; this document covers patterns and conventions not mechanically enforced.

---

## 1. Error Handling

Use `thiserror` for library errors, `anyhow` for application errors. For trait boundaries, use `Result<T, String>` for simplicity across implementations.

```rust
#[async_trait]
pub trait SttEngine: Send + Sync {
    fn engine_type(&self) -> EngineType;
    async fn send_chunk(&self, pcm_data: Vec<i16>) -> Result<(), String>;
    async fn finish(&self) -> Result<String, String>;
    fn set_partial_callback(&mut self, callback: PartialResultCallback);
}
```

Error messages: lowercase, no trailing period, include context.

```rust
// ✅ Correct
.map_err(|e| format!("Failed to create WAV file: {}", e))?;
Err("Model '{}' not downloaded. Please download it in Settings > Model.".to_string())

// ❌ Wrong
.map_err(|e| e.to_string())?;  // No context
```

---

## 2. Logging

Use `tracing` with `tracing-subscriber`. Never use `println!`/`eprintln!` except before tracing initialization.

Message pattern: `event_name-description` (snake_case, lowercase). Use `%` for Display, `?` for Debug.

```rust
// ✅ Correct
info!(task_id, engine = ?engine_type, model = %model_name, "model_loaded-startup_warmup");
error!(task_id, error = %e, "transcription_failed-model_not_found");

// ❌ Wrong
info!("Model loaded: {}", name);
error!("transcription failed: {}", e);
```

Add `#[instrument]` to Tauri commands and critical pipeline functions:

```rust
#[tauri::command]
#[instrument(skip(app, state), ret, err)]
pub async fn start_recording(app: AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    // ...
}
```

See [`docs/spec/logs.md`](../spec/logs.md) for complete logging standard.

---

## 3. Type Design

### Enums for State Machines

Use enums for state machines, not boolean flags:

```rust
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
    pub fn can_transition_to(&self, next: RecordingState) -> bool {
        matches!(
            (self, next),
            (RecordingState::Idle, RecordingState::Starting)
                | (RecordingState::Starting, RecordingState::Recording)
                | (RecordingState::Recording, RecordingState::Stopping)
                // ... explicit transitions
        )
    }
}
```

### Engine Type Enums

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EngineType {
    Whisper,
    SenseVoice,
    Cloud,
}

impl EngineType {
    pub fn as_str(&self) -> &'static str {
        match self {
            EngineType::Whisper => "whisper",
            EngineType::SenseVoice => "sensevoice",
            EngineType::Cloud => "cloud",
        }
    }
}
```

---

## 4. Module Organization

```
src-tauri/src/
├── lib.rs              # Entry point, command registration
├── audio/              # Audio capture and processing
├── commands/           # Tauri IPC handlers
├── state/              # Application state management
├── stt_engine/         # Speech-to-text engines
│   ├── mod.rs
│   ├── traits.rs
│   ├── unified_manager.rs
│   ├── buffering_engine.rs
│   ├── models.rs
│   ├── sherpa_onnx/
│   └── cloud/
├── polish_engine/      # Text polishing engines
├── text_injector/      # Platform-specific text injection
└── utils/              # Shared utilities
```

Re-export public API from `mod.rs`:

```rust
// stt_engine/mod.rs
pub use traits::{EngineType, SttEngine, TranscriptionRequest, TranscriptionResult};
pub use unified_manager::{ModelInfo, UnifiedEngineManager};
```

---

## 5. Naming Conventions

- **Files/modules**: `snake_case`
- **Types**: `PascalCase`
- **Functions/variables**: `snake_case`
- **Constants**: `SCREAMING_SNAKE_CASE`
- **Booleans**: `is_`, `has_`, `should_` prefixes

Event names use kebab-case in a dedicated module:

```rust
#[allow(non_snake_case)]
pub mod EventName {
    pub const RECORDING_STATE_CHANGED: &str = "recording-state-changed";
    pub const TRANSCRIPTION_COMPLETE: &str = "transcription-complete";
}
```

---

## 6. State Management

Use `parking_lot::Mutex` for sync access, atomics for flags, `Arc` for shared ownership.

```rust
pub struct AppState {
    pub recording_state: UnifiedRecordingState,
    pub settings: Mutex<AppSettings>,
    pub engine_manager: Arc<UnifiedEngineManager>,
    pub is_recording: AtomicBool,
    pub task_counter: AtomicU64,
}

pub struct UnifiedRecordingState {
    current: Mutex<RecordingState>,
    error: Mutex<Option<String>>,
}
```

Encapsulate state transitions with validation and logging:

```rust
pub fn transition_to(&self, new_state: RecordingState) -> Result<(), String> {
    let mut current = self.current.lock();
    let old_state = *current;

    if !current.can_transition_to(new_state) {
        tracing::warn!(from = %old_state.as_str(), to = %new_state.as_str(), "state_transition_rejected");
        return Err(format!("Invalid state transition from {:?} to {:?}", old_state, new_state));
    }

    *current = new_state;
    tracing::info!(from = %old_state.as_str(), to = %new_state.as_str(), "state_transition");
    Ok(())
}
```

---

## 7. Tauri Commands

```rust
#[tauri::command]
#[instrument(skip(app, state), ret, err)]
pub async fn start_recording(app: AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    if state.is_recording.load(Ordering::SeqCst) {
        return Err("Already recording".to_string());
    }
    // ...
}
```

Register in `lib.rs`:

```rust
.invoke_handler(tauri::generate_handler![
    window::show_main_window,
    start_recording,
    stop_recording,
    // ... all commands
])
```

---

## 8. Async Patterns

Use `tokio::sync::mpsc` for async channels, `std::sync::mpsc` for sync. Use `async_trait` for trait definitions:

```rust
#[async_trait]
pub trait SttEngine: Send + Sync {
    fn engine_type(&self) -> EngineType;
    async fn send_chunk(&self, pcm_data: Vec<i16>) -> Result<(), String>;
    async fn finish(&self) -> Result<String, String>;
    fn set_partial_callback(&mut self, callback: PartialResultCallback);
}
```

Spawn with `tauri::async_runtime`:

```rust
tauri::async_runtime::spawn(async move {
    run_transcription(app_clone, job.audio_path, task_id).await;
});
```

---

## 9. Platform-Specific Code

```rust
#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "windows")]
mod windows;

pub fn create_injector() -> Box<dyn TextInjector> {
    #[cfg(target_os = "macos")]
    return Box::new(macos::MacosInjector);

    #[cfg(target_os = "windows")]
    return Box::new(windows::WindowsInjector);

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    compile_error!("text_injector: unsupported platform");
}
```

---

## 10. Tests

Colocate tests in `#[cfg(test)] mod tests` blocks:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transcription_request_new() {
        let request = TranscriptionRequest::new("/path/to/audio.wav");
        assert_eq!(request.audio_path, PathBuf::from("/path/to/audio.wav"));
    }

    fn test_store() -> HistoryStore {
        HistoryStore::from_connection(Connection::open_in_memory().unwrap()).unwrap()
    }
}
```

Test naming: `test_<unit>_<scenario>_<expected_result>`.

---

## 11. Builder Pattern

```rust
impl TranscriptionRequest {
    pub fn new(audio_path: impl Into<PathBuf>) -> Self {
        Self { audio_path: audio_path.into(), language: None, model_name: None }
    }

    pub fn with_language(mut self, language: impl Into<String>) -> Self {
        self.language = Some(language.into());
        self
    }
}

// Usage
let request = TranscriptionRequest::new(path).with_model("base").with_language("en-US");
```

---

## 12. Clippy Lints

Configured in `Cargo.toml`:

```toml
[lints.clippy]
pedantic = { level = "warn", priority = -1 }
unwrap_used = "deny"
expect_used = "warn"
panic = "deny"
dbg_macro = "deny"
todo = "deny"
print_stdout = "deny"
print_stderr = "deny"
```

---

## 13. Key Dependencies

| Crate | Purpose |
|-------|---------|
| `tauri` | Desktop framework |
| `tokio` | Async runtime |
| `tracing` | Logging |
| `parking_lot` | Mutex/sync |
| `serde` | Serialization |
| `thiserror` | Error types (library) |
| `anyhow` | Error handling (application) |
| `cpal` | Audio I/O |
| `hound` | WAV handling |
| `rusqlite` | SQLite |

---

## 14. Forbidden Patterns

| Pattern | Reason |
|---------|--------|
| `unwrap()` in production | Use `?` or error handling |
| `println!`/`eprintln!` | Use `tracing` |
| `dbg!` macro | Use `debug!` |
| `todo!` macro | Incomplete code |
| Boolean flags for state | Use enums |
| Wildcard imports | Be explicit |
| `matches!` macro | Prefer explicit destructuring |