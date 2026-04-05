# Logging Specification

Conventions, format, and coverage requirements for AriaType logging.

---

## 1. Architecture

### Backend (Rust)

- `tracing` + `tracing-subscriber` + `tracing-appender`
- Outputs: stderr (human-readable) + rolling hourly file (no ANSI)
- File: `~/Library/Logs/ariatype/ariatype.log.YYYY-MM-DD-HH` (macOS)
- Retention: 7 days, cleaned on startup
- Default level: `info` (override via `RUST_LOG`)

### Frontend (TypeScript)

- Custom `logger` at `apps/desktop/src/lib/logger.ts`
- Format: `[ISO timestamp] [LEVEL] message {context JSON}`
- Levels: `error` / `warn` / `info` / `debug`
- All `invoke()` calls wrapped with request/response/timing at debug level

---

## 2. Level Taxonomy

| Level | When | Examples |
|-------|------|----------|
| `error` | Unrecoverable failure | Model load failure, API auth error |
| `warn` | Degraded but handled | Fallback after polish fails |
| `info` | Lifecycle events, key metrics | Recording start/stop, transcription complete |
| `debug` | Development internals | Per-chunk progress, config dumps |
| `trace` | Raw data (rare) | Audio buffer contents |

---

## 3. Message Format

### Standard Format

```
[timestamp] LEVEL [target]-[event_name]-[description] field1=val1
```

Message string pattern: `event_name-description` (snake_case, lowercase).

- `event_name`: identifies the event (required)
- `description`: optional qualifier (omit if not needed)

### Mandatory Rules

1. **Message**: `event_name-description` — snake_case, lowercase
2. **Structured fields**: never embed values in format strings
3. **Error fields**: `error = %e` (Display) — never inline `"failed: {}"`
4. **Field display**: `%` for Display, `?` for Debug

```rust
// ✅ Correct
info!(task_id, engine = ?engine_type, duration_ms, "model_loaded-startup_warmup");
info!(task_id, duration_ms, text_len, "transcription_completed");
error!(task_id, error = %e, "transcription_failed-model_not_found");

// ❌ Wrong
info!("starting ariatype application");
error!("transcription failed: {}", e);
info!("[{}] model loaded", prefix);
```

### Standard Field Names

| Field | Type | Used In |
|-------|------|---------|
| `task_id` | `u64` | Pipeline operations |
| `engine` | `?EngineType` | STT/Polish operations |
| `model` | `%str` | Model operations |
| `error` | `%Error` | Error paths |
| `duration_ms` | `u128` | Operations > 10ms |
| `provider` | `%str` | Cloud operations |
| `http.status_code` | `u16` | Cloud API responses (HTTP status) |
| `sample_rate` | `u32` | Audio operations |
| `channels` | `u16` | Audio operations |
| `text_len` | `usize` | Text operations |
| `path` | `?Path` | File operations |
| `hotkey` | `%str` | Shortcut registration |

Field ordering: `task_id` → entity identifiers → metrics → error.

### Additional Field Names (Context-Specific)

These fields are used in specific contexts and are acceptable when documented:

| Field | Type | Used In | Notes |
|-------|------|---------|-------|
| `mem_mb` | `u64` | Memory reporting | Startup and model load operations |
| `migrated` | `bool` | Settings migration | State migration events |
| `model_id` | `%str` | Polish model ops | Alternative to `model` for polish-specific ID |
| `context` | `%str` | Operation context | Prefer `engine` or `provider` when applicable |
| `log_context` | `%str` | Operation context | Prefer `engine` or `provider` when applicable |
| `chunk_index` | `usize` | Streaming audio | Cloud STT chunk tracking |
| `chunks` | `usize` | Streaming summary | Total chunks sent |
| `url` | `%str` | Connection logging | WebSocket/API endpoint URLs |
| `logid` | `?str` | Server log ID | Volcengine X-Tt-Logid header |

---

## 4. Instrumentation

### `#[instrument]` Usage

Add to all Tauri command handlers, engine trait implementations, and critical pipeline functions.

```rust
#[tauri::command]
#[instrument(skip(app, state), fields(task_id))]
pub async fn start_recording(app: AppHandle, state: State<'_, AppState>) -> Result<String, String> {
```

Rules: `skip(app, state)` to avoid logging stateful params. Use `ret` for non-sensitive returns, `err` for errors.

### Required Functions

**Command handlers**: `start_recording`, `stop_recording`, `insert_text`, `copy_to_clipboard`, `download_model`, `delete_model`, `preload_model`, `unload_model`, `preload_polish_model`, `unload_polish_model`

**Pipeline functions** (`commands/audio.rs`): `run_transcription`, `maybe_polish_transcription_text`, `finish_local_recording`, `finish_cloud_stt_recording`

**Engine managers**: `UnifiedEngineManager::transcribe`, `UnifiedEngineManager::load_model`, `UnifiedPolishManager::polish`, `UnifiedPolishManager::polish_cloud`

**Text injection**: `do_insert_text`

---

## 5. Coverage Requirements

### Pipeline Stages

Every stage MUST have entry (`info`), exit with metrics (`info`), failure (`error`), and fallback (`warn`) logging:

```
Hotkey → Recording → Audio Capture → Stop → WAV Write → STT → Polish → Inject → Session End
```

### Required Log Points by Module

| Module | Events |
|--------|--------|
| `lib.rs` | App start, logging init, plugin registration, window creation, shortcut registration |
| `commands/audio.rs` | Recording start/stop, STT start/complete/fail, polish start/complete/fail, injection, pipeline summary |
| `audio/recorder.rs` | Stream open/close, device, format |
| `stt_engine/unified_manager.rs` | Model load/unload with timing, engine selection, transcription with timing |
| `stt_engine/cloud/*` | Connection, chunk send, result, error recovery |
| `polish_engine/*` | Engine init, model load/unload, polish with timing |
| `text_injector/*` | Injection method, success/failure with timing |
| `state/unified_state.rs` | State transitions (from → to) |
| `commands/settings.rs` | Settings save/load, hotkey change, cloud config change |
| `commands/model.rs` | Download start/progress/complete/fail, delete |
| `tray.rs` | Tray creation, menu action |
| `utils/downloader.rs` | Download start/progress/complete/fail with bytes and timing |

### Anti-Patterns

| Anti-Pattern | Fix |
|--------------|-----|
| `println!` / `eprintln!` | Use `tracing` macros |
| `error = format!("...")` | Use `error = %e` |
| `info!("Model loaded: {}", name)` | `info!(model = %name, "model_loaded")` |
| `info!("[prefix] message")` | `info!(prefix = %p, "message")` |
| Missing `task_id` in pipeline | Always include where available |
| `error!("failed")` no context | Add source, operation, identifiers |
| Logging sensitive data | Truncate, hash, or omit |
| `.catch(console.error)` (frontend) | `logger.error('operation_failed', { context })` |

---

## 6. Frontend Logging

- **Logger**: `apps/desktop/src/lib/logger.ts` — level filtering, structured context, ISO timestamps
- **IPC**: All `invoke()` calls go through `invokeWithLogging()` — logs command, params, timing, errors
- **Components**: Never use bare `console.error` / `console.warn`. Use `logger.error` / `logger.warn`

---

## 7. Verification Checklist

- [ ] No `println!` / `eprintln!` in production code (pre-init acceptable)
- [ ] Messages are snake_case, lowercase, no trailing period
- [ ] Structured fields use standard names (Section 3)
- [ ] `task_id` in all pipeline-stage logs
- [ ] `error = %e` (not inline format)
- [ ] `duration_ms` for all operations > 10ms
- [ ] `#[instrument]` on all functions in Section 4
- [ ] Frontend uses `logger`, not bare `console.*`
- [ ] Pipeline coverage: every stage has entry + exit + failure log
