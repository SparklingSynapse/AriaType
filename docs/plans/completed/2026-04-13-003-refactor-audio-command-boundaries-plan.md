---
title: Audio Command Boundary Refactor
type: refactor
status: completed
date: 2026-04-13
---

# Audio Command Boundary Refactor Plan

**Goal:** Split `apps/desktop/src-tauri/src/commands/audio.rs` into testable command-facing coordinators and backend services while preserving recording, transcription, retry, and cancellation behavior through failing-first regression tests.

**Architecture:** The target shape follows `docs/architecture/layers.md`: `commands/` remains a thin IPC boundary, state stays in `state/`, provider logic stays in `stt_engine/` and `polish_engine/`, and event emission is routed through explicit helpers instead of being interleaved with recording orchestration.

**Tech Stack:** Rust, Tauri, CPAL, tokio, existing unified STT/polish managers, `AppState`.

---

## Overview

`audio.rs` currently mixes Tauri commands, recording lifecycle orchestration, session bookkeeping, retry policy, history updates, and frontend event emission in one file. That shape violates the backend layer rule that `commands/` must stay thin, and it makes regressions hard to isolate because one function often mutates state, triggers audio I/O, and emits UI events in the same branch.

This plan refactors by extracting one behavior seam at a time, always starting with a failing test or characterization test. The first slice already protects recording-start atomicity so a recorder startup failure cannot leave `AppState` stuck in a recording session.

## Problem Frame

- **Current state:** `start_recording_sync`, `stop_recording_sync`, and `retry_transcription_internal` directly combine command concerns with business rules and state transitions.
- **Current state:** Recording start mutates visibility and session state before recorder startup, so failures can strand backend state unless rollback is explicit.
- **Current state:** Retry transcription uses a separate local-file path that is semantically different from live cloud streaming and is difficult to reason about or swap safely.
- **Desired state:** Commands validate inputs and delegate to focused services/coordinators with narrow responsibilities.
- **Desired state:** Critical state transitions are protected by small unit tests before extraction work begins.
- **Desired state:** Retry, recording lifecycle, and event emission each have independent tests and seams.

## Scope Boundaries

### In scope

- Add regression tests around recording lifecycle state transitions before refactoring.
- Extract recording-start/stop lifecycle coordination from IPC handlers into service-style helpers.
- Isolate retry transcription preparation and persistence updates behind testable helpers.
- Reduce direct event emission scatter by introducing explicit emission helpers or a small event adapter.
- Document migration order and verification gates for each slice.

### Out of scope

- Replacing CPAL or changing recorder device selection behavior.
- Reworking the full STT engine contract or cloud provider protocols.
- Frontend UI changes beyond what existing backend events already require.
- Broad history-store redesign unrelated to audio command boundaries.

## Implementation Units

### Unit 1: Lock recording-start atomicity with TDD

**Files**
- Modify: `apps/desktop/src-tauri/src/commands/audio.rs`

**Approach**
- Add failing-first tests that describe atomic recording-start behavior.
- Introduce a tiny rollback guard so session state is reverted if startup exits early before the recorder is running.
- Keep the production diff minimal and avoid mixing this slice with broader file extraction.

**Verification**
- `cargo test --lib recording_start_guard_`

### Unit 2: Extract recording lifecycle coordinator

**Files**
- Modify: `apps/desktop/src-tauri/src/commands/audio.rs`
- Add: `apps/desktop/src-tauri/src/services/recording_lifecycle.rs` or equivalent backend service module
- Modify: `apps/desktop/src-tauri/src/lib.rs` only if new module wiring is required

**Approach**
- Move non-IPC logic from `start_recording_sync`, `stop_recording_sync`, and cancel flows into a coordinator that owns sequencing, not transport.
- Keep `#[tauri::command]` functions as thin adapters.
- Add tests for state transition rules and cancellation edge cases before moving code.

**Verification**
- `cargo test --lib commands::audio::tests::`
- `cargo test --lib services::recording_lifecycle::tests::`
- `cargo test --lib state::`

### Unit 3: Isolate retry transcription policy

**Files**
- Modify: `apps/desktop/src-tauri/src/commands/audio.rs`
- Add: `apps/desktop/src-tauri/src/services/retry_transcription.rs` or equivalent

**Approach**
- Separate entry validation, audio-file decoding, transcription execution, polish decision, and history persistence into explicit helpers.
- Add characterization tests for retry preconditions and metadata updates before extracting.
- Decide and document whether retry should stay local-only or become provider-aware; do not leave ambiguous semantics in command code.

**Verification**
- `cargo test --lib retry_transcription_`
- `cargo test --lib commands::audio::tests::`

### Unit 4: Centralize backend event emission

**Files**
- Modify: `apps/desktop/src-tauri/src/commands/audio.rs`
- Add or modify: dedicated helper under `events/` or a backend service adapter

**Approach**
- Replace repeated inline `app.emit(...)` branches with typed helpers that encode allowed state transitions.
- Add focused tests for event payload selection where practical, and keep command handlers free of duplicated status strings.

**Verification**
- `cargo test --lib recording_state_`
- `cargo test --lib commands::audio::tests::`

## System-Wide Impact

- Recording start/stop behavior becomes easier to verify without booting the full Tauri app.
- Future fixes to retry semantics no longer require editing the same file as audio capture code.
- Command handlers become smaller, which reduces the chance of hidden coupling across state, history, and events.
- Backend behavior remains headless-friendly because orchestration moves deeper into Rust services instead of frontend logic.

## Risks & Dependencies

- There is no dedicated `docs/feat/...` spec for recording lifecycle; the refactor must therefore treat current tested behavior plus architecture rules as the canonical baseline.
- `audio.rs` is tightly coupled to Tauri `AppHandle`, so some seams may require small adapters before they become independently testable.
- Retry semantics currently mix product intent and implementation shortcuts; clarifying the desired behavior may require a separate user-facing decision if cloud parity is expected.

## Verification Evidence

- Added failing-first regression tests around recording-start rollback in `apps/desktop/src-tauri/src/commands/audio.rs`.
- Introduced a minimal recording-start rollback guard so startup failure cannot leave session state stuck.
- Added `apps/desktop/src-tauri/src/services/recording_lifecycle.rs` and moved recording-start state preparation plus rollback guard out of `commands/audio.rs`.
- Added service-level tests for prepared recording state capture and guard commit/rollback semantics.
- Moved stop/cancel state transitions into `recording_lifecycle` helpers so `commands/audio.rs` keeps only transport concerns such as recorder stop, async streaming drain, window updates, and event emission.
- Added service-level tests for stop no-op behavior, stop level-monitor shutdown, cancel session clearing, and transcribe-only cancellation semantics.
- Added typed recording-state event helpers under `apps/desktop/src-tauri/src/events/mod.rs` so backend status emission no longer duplicates raw string literals across `audio.rs`.
- Added focused tests for recording status to payload mapping and replaced inline `RECORDING_STATE_CHANGED` payload construction in `commands/audio.rs`.
- Added `apps/desktop/src-tauri/src/services/transcription_finalize.rs` to centralize successful/empty/failed transcription persistence plus the shared completion delivery flow that emits `TRANSCRIPTION_COMPLETE`, returns to idle, and inserts text.
- Added `apps/desktop/src-tauri/src/services/retry_transcription.rs` to isolate retry precondition validation, retry history metadata construction, history update/error marking, and retry audio cleanup out of `commands/audio.rs`.
- Replaced inline retry-entry validation, retry metadata construction, and transcription completion/failure branches inside `commands/audio.rs` with service helpers so the command layer remains an orchestrator.
- Moved retry WAV loading, mono conversion, resampling, and transcription request execution into `services/retry_transcription.rs`, leaving `retry_transcription_internal` to coordinate state emission, polish, and final result handling.
- Added focused retry service tests for missing audio files, cleanup behavior, stereo-to-mono WAV loading, and empty-audio rejection.
- Verified the first TDD slice with:
  - `cargo test --lib recording_start_guard_`
- Verified the first Unit 2 extraction slice with:
  - `cargo test --lib services::recording_lifecycle::tests::`
  - `cargo test --lib commands::audio::tests::`
- Verified the first Unit 4 extraction slice with:
  - `cargo test --lib events::tests::`
  - `cargo test --lib commands::audio::tests::`
- Verified the retry/finalization extraction slice with:
  - `cargo test --lib services::retry_transcription::tests`
  - `cargo test --lib services::transcription_finalize::tests::`
  - `cargo test --lib commands::audio::tests::`
  - `cargo fmt -- --check`
- Completed the final command-layer split so `apps/desktop/src-tauri/src/commands/audio.rs` now serves only as a module barrel and public export surface.
- Added focused command submodules under `apps/desktop/src-tauri/src/commands/audio/` for `start`, `stop`, `cancel`, `capture`, `retry`, `polish`, `query`, `level_monitor`, and shared helpers/tests.
- Removed the pseudo-refactor shape where `audio.rs` only forwarded to a monolithic `audio/internal.rs`; the command logic now lives in responsibility-aligned modules instead of a hidden single-file sink.
- Verified the modular command split with:
  - `cargo fmt`
  - `cargo test --lib`
  - `cargo clippy --all-features -- -D warnings`
