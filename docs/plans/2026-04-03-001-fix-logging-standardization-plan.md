---
title: "fix: Standardize logging across Rust backend and TypeScript frontend"
type: fix
status: active
date: 2026-04-03
---

# fix: Standardize Logging Across Rust Backend and TypeScript Frontend

## Overview

Standardize logging to eliminate spec violations, add critical missing coverage, and align with OpenTelemetry conventions. Work is scoped to `apps/desktop/src-tauri/src/` (Rust) and `apps/desktop/src/` (TypeScript/React).

## Problem Frame

The existing logging audit found:
- **SPEC VIOLATION**: Frontend `logger.ts` lacks `task_id` support — spec §3.2 requires `task_id` in all pipeline-stage logs
- **CRITICAL**: Cloud STT engines (`qwen_omni_realtime.rs`, `elevenlabs.rs`) have zero `#[instrument]` attributes
- **CRITICAL**: `unload_polish_model` (model_cache.rs:113) missing `#[instrument]`
- **HIGH**: Frontend event listeners in `tauri.ts` (12 handlers) log zero pipeline context
- **MEDIUM**: Non-standard field names (`context`, `log_context`, `mem_mb`, `migrated`) not documented
- **MEDIUM**: Cloud API calls missing `http.status_code` and standardized `provider` fields

## Requirements Trace

- R1: Logger supports `task_id` field for frontend pipeline correlation (spec §3.2)
- R2: All cloud STT engine public methods have `#[instrument]` with correct fields (spec §4)
- R3: `unload_polish_model` has `#[instrument]` (spec §4 Required Functions)
- R4: Frontend event listeners log with pipeline context (spec §5 Pipeline Stages)
- R5: All cloud API responses include `http.status_code` field (OpenTelemetry parity)
- R6: `provider` field standardized on all cloud operations (spec §3 Standard Field Names)
- R7: Non-standard fields documented in spec or replaced

## Scope Boundaries

- **In scope**: `apps/desktop/src/lib/logger.ts`, `apps/desktop/src/lib/tauri.ts`, `apps/desktop/src-tauri/src/stt_engine/cloud/*.rs`, `apps/desktop/src-tauri/src/commands/model_cache.rs`, `apps/desktop/src-tauri/src/polish_engine/unified_manager.rs`, `docs/spec/logs.md`
- **Out of scope**: Other Rust files (already have good coverage per audit), website package, CI configuration

## Key Technical Decisions

- **task_id flow**: Frontend receives `task_id` from backend via event payload (`RecordingStateEvent.task_id`, `TranscriptionCompleteEvent.task_id`). Logger must accept and propagate this field so all frontend pipeline logs can be correlated to backend traces.
- **Cloud engine #[instrument]**: `connect()`, `finish()`, and `send_audio_async()` are the critical entry points. Internal helpers (`start_result_receiver`, `start_audio_sender`) are spawned tasks — they inherit fields via closure, not `#[instrument]`.
- **Non-standard fields**: Fields like `mem_mb`, `migrated` are acceptable local conventions but should be documented in spec §3. `context`/`log_context` in `commands/audio.rs` should be renamed to `engine` or `provider` per spec.

## Open Questions

### Resolved During Planning
- **Q**: Should `preload_polish_model` and `unload_polish_model` be in `model_cache.rs` or `unified_manager.rs`?  
  **A**: Both exist — `model_cache.rs` exposes them as Tauri commands; `unified_manager.rs` has the internal implementation. The spec targets the command layer in `model_cache.rs`.

### Deferred to Implementation
- Whether to add `http.status_code` to WebSocket-level errors (which don't have HTTP status) — decide during implementation based on OpenTelemetry semantic conventions

## Implementation Units

- [ ] **Unit 1: Add task_id support to frontend logger**

**Goal:** Enable frontend pipeline correlation via task_id field

**Files:**
- Modify: `apps/desktop/src/lib/logger.ts`

**Approach:**
Extend the logger interface to accept `task_id?: string` in context. This is a lightweight additive change — existing call sites don't need to change unless they have a task_id to pass.

**Patterns to follow:**
- Backend pattern: `task_id` as first field in structured logs
- Event payload already includes `task_id` on `RecordingStateEvent` and `TranscriptionCompleteEvent`

**Test scenarios:**
- Happy path: Logger accepts context with `task_id` and includes it in formatted output
- Edge case: Logger called without `task_id` (must not break)

**Verification:**
- `logger.info('test', { task_id: '123' })` outputs `[timestamp] [INFO] test {"task_id":"123"}`

---

- [ ] **Unit 2: Add pipeline logging to frontend event listeners**

**Goal:** Frontend event handlers log pipeline stages per spec §5

**Files:**
- Modify: `apps/desktop/src/lib/tauri.ts`

**Approach:**
Wrap each `listen()` callback body with logger calls at appropriate level. Use `debug` for frequent events (audio-level), `info` for state transitions, `error` for error events.

Key events requiring logging:
| Event | Level | Fields |
|-------|-------|--------|
| `recording-state-changed` | info | `task_id`, `status` |
| `audio-level` | debug | `level` |
| `transcription-complete` | info | `task_id`, `text_len` |
| `transcription-error` | error | `task_id`, `error` |
| `model-download-progress` | debug | `model`, `progress` |
| `settings-changed` | info | (no task_id) |

**Patterns to follow:**
- `invokeWithLogging()` pattern (lines 6-21) — log entry/exit with timing
- Backend pipeline logging in `commands/audio.rs`

**Test scenarios:**
- Happy path: Event callback logs on receipt
- Error path: `transcription-error` event logs error context

**Verification:**
- Manual: Trigger events via backend, observe frontend logs

---

- [ ] **Unit 3: Add #[instrument] to qwen_omni_realtime.rs public methods**

**Goal:** Cloud engine has proper tracing instrumentation per spec §4

**Files:**
- Modify: `apps/desktop/src-tauri/src/stt_engine/cloud/qwen_omni_realtime.rs`

**Approach:**
Add `#[instrument]` to `connect()`, `finish()`, `send_audio_async()`, and `close()`. Use `fields(provider = "qwen-omni-realtime")`. Already-existing `info!/debug!/error!` calls provide the detailed span content.

Methods needing `#[instrument]`:
```rust
#[instrument(skip(self), fields(provider = "qwen-omni-realtime"), ret, err)]
pub async fn connect(&mut self) -> Result<(), String>

#[instrument(skip(self), fields(provider = "qwen-omni-realtime"), ret, err)]
pub async fn finish(&self) -> Result<String, String>

#[instrument(skip(self), fields(provider = "qwen-omni-realtime"), ret, err)]
pub async fn send_audio_async(&self, pcm_data: Vec<i16>) -> Result<(), String>

#[instrument(skip(self), fields(provider = "qwen-omni-realtime"))]
pub async fn close(&self)
```

Also add `http.status_code` enrichment where available (websocket `connect_async` result gives us `response` with status).

**Patterns to follow:**
- Existing `volcengine_streaming.rs` `#[instrument]` pattern (lines 236-244)
- Backend cloud logging convention: `provider = "qwen-omni-realtime"`

**Test scenarios:**
- Error path: Auth failure (401/403) is traced with correct provider

**Verification:**
- `cargo clippy --all-features -- -D warnings` passes

---

- [ ] **Unit 4: Add #[instrument] to elevenlabs.rs public methods**

**Goal:** Cloud engine has proper tracing instrumentation per spec §4

**Files:**
- Modify: `apps/desktop/src-tauri/src/stt_engine/cloud/elevenlabs.rs`

**Approach:**
Mirror Unit 3 approach for ElevenLabs. Provider name: `"elevenlabs"`.

Methods needing `#[instrument]`:
```rust
#[instrument(skip(self), fields(provider = "elevenlabs"), ret, err)]
pub async fn connect(&mut self) -> Result<(), String>

#[instrument(skip(self), fields(provider = "elevenlabs"), ret, err)]
pub async fn finish(&self) -> Result<String, String>

#[instrument(skip(self), fields(provider = "elevenlabs"), ret, err)]
pub async fn send_audio_async(&self, pcm_data: Vec<i16>) -> Result<(), String>

#[instrument(skip(self), fields(provider = "elevenlabs"))]
pub async fn close(&self)
```

**Patterns to follow:** Same as Unit 3.

**Test scenarios:** Same as Unit 3.

**Verification:**
- `cargo clippy --all-features -- -D warnings` passes

---

- [ ] **Unit 5: Add #[instrument] to unload_polish_model**

**Goal:** Polish model unload command has tracing instrumentation per spec §4

**Files:**
- Modify: `apps/desktop/src-tauri/src/commands/model_cache.rs`

**Approach:**
Add `#[instrument(skip(state), err)]` to `unload_polish_model` at line 113 (currently only has `#[tauri::command]`). Already-present logging in `polish_manager.clear_cache()` provides the detailed span.

**Patterns to follow:**
- `preload_polish_model` at line 91 as reference: `#[instrument(skip(state), err)]`

**Test scenarios:**
- Happy path: Command returns Ok and span completes

**Verification:**
- `cargo clippy --all-features -- -D warnings` passes

---

- [ ] **Unit 6: Add http.status_code to cloud API responses**

**Goal:** OpenTelemetry parity for cloud API calls

**Files:**
- Modify: `apps/desktop/src-tauri/src/stt_engine/cloud/volcengine_streaming.rs`
- Modify: `apps/desktop/src-tauri/src/stt_engine/cloud/qwen_omni_realtime.rs`
- Modify: `apps/desktop/src-tauri/src/stt_engine/cloud/elevenlabs.rs`

**Approach:**
After each `connect_async_tls_with_config()` call, extract and log the HTTP status code from the response. For WebSocket connections, the `response` parameter contains headers.

```rust
// After successful connect
info!(
    provider = "volcengine",
    http.status_code = response.status().as_u16(),
    "websocket_connected"
);
```

For error paths, log the error type (401/403/etc) as `http.status_code` if extractable, or as part of the error message.

**Patterns to follow:**
- Existing `provider` field convention
- OpenTelemetry semantic conventions for HTTP

**Test scenarios:**
- Happy path: Successful connection logs 200-level status
- Error path: Auth failure logs 401/403

**Verification:**
- `cargo clippy --all-features -- -D warnings` passes

---

- [ ] **Unit 7: Document non-standard fields in spec**

**Goal:** Spec §3 accurately reflects field usage

**Files:**
- Modify: `docs/spec/logs.md`

**Approach:**
Add an "Additional Field Names" subsection to spec §3 documenting fields that don't appear in the standard table but are used in the codebase:

| Field | Type | Used In |
|-------|------|---------|
| `mem_mb` | `u64` | Memory usage reporting |
| `migrated` | `bool` | State migration events |
| `context`/`log_context` | `&str` | Operation context (rename to `engine`/`provider` preferred) |

Also add `http.status_code` (u16) to the Standard Field Names table for cloud API calls.

**Verification:**
- Spec file updated with no format breaking changes

---

- [ ] **Unit 8: Rename context/log_context to standard fields in commands/audio.rs**

**Goal:** Align with spec §3 field naming

**Files:**
- Modify: `apps/desktop/src-tauri/src/commands/audio.rs`

**Approach:**
Audit `commands/audio.rs` for `context`/`log_context` usage. The spec §3 Standard Field Names table has `engine`, `provider`, `model`, etc. Fields called `context` should be renamed to more specific standard names where applicable.

Note: Some `context` usage may be appropriate for nested operation context — evaluate per-callsite. Rename only where `engine` or `provider` is the correct semantic.

**Patterns to follow:**
- spec §3 Standard Field Names

**Test scenarios:**
- No behavioral change — purely renaming
- Verify build passes after rename

**Verification:**
- `cargo clippy --all-features -- -D warnings` passes

## System-Wide Impact

- **Logger interface**: `logger.ts` API gains optional `task_id` parameter — backward compatible
- **Event listener behavior**: Adding logging to event callbacks adds minimal overhead but enables full pipeline correlation
- **Tracing spans**: Cloud engine `#[instrument]` creates proper spans with provider field — enables distributed tracing across backend/frontend

## Risks & Dependencies

| Risk | Mitigation |
|------|------------|
| Breaking frontend logger API | task_id is optional — existing call sites unaffected |
| Instrument noise from debug-level spans | Use `debug!` for high-volume operations (audio-level events) |
| Rename breaking external consumers | None — internal to apps/desktop only |

## Documentation / Operational Notes

- After changes, verify with: `cargo clippy --all-features -- -D warnings` and `pnpm --filter @ariatype/desktop typecheck`
- No runtime config changes required — logging is always-on per spec

## Sources & References

- **Spec document**: `docs/spec/logs.md` (176 lines, definitions through line 176)
- **Backend patterns**: `apps/desktop/src-tauri/src/commands/audio.rs` (pipeline logging reference)
- **Frontend patterns**: `apps/desktop/src/lib/tauri.ts` (`invokeWithLogging` reference)
- **Cloud engine reference**: `apps/desktop/src-tauri/src/stt_engine/cloud/volcengine_streaming.rs` (existing `#[instrument]` example)
- **OpenTelemetry HTTP conventions**: https://opentelemetry.io/docs/specs/otel/compatibility/opentelemetry-http/
