---
title: Recording Chunking and Short-Utterance Truncation
type: fix
status: active
date: 2026-04-13
---

# Recording Chunking and Short-Utterance Truncation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reduce recording chunk size from 500ms to 200ms and document the first-principles causes of short cloud-STT utterance truncation.

**Architecture:** The current recorder callback accumulates device PCM into a fixed runtime chunk buffer in `commands/audio.rs`, then sends processed chunks through the unified `send_chunk()` / `finish()` pipeline. We will change only the runtime chunk threshold in this iteration, keep behavior under test, and separately record the dominant truncation causes without bundling a larger flush/preroll refactor.

**Tech Stack:** Rust, Tauri, CPAL audio capture, unified STT consumer pipeline, cloud STT streaming clients.

---

## Overview

Current runtime chunking was hardcoded to 500ms even though cloud streaming providers recommend 100–200ms chunks. That made short hold-to-talk recordings lose a large tail remainder on stop and delayed the first chunk. The implementation now covers both the 200ms runtime chunk threshold and the stop-time tail flush needed to prevent the final buffered remainder from being dropped by the normal streaming VAD gate.

## Problem Frame

- **Current state:** Recorder callback emitted processed audio once 500ms of PCM accumulated, and the stop flush reused normal `process_chunk()` logic. Short recordings could therefore both delay the first send and lose the final partial chunk when VAD rejected the trailing remainder.
- **Desired state:** Runtime chunking uses 200ms, and stop-time tail flushing keeps the resample/denoise path while bypassing VAD drop decisions for the final buffered remainder. Tests prove both behaviors. Root-cause evidence still explains why immediate keydown clipping remains a separate follow-up.

## Scope Boundaries

### In scope
- Change runtime recording chunk threshold from 500ms to 200ms.
- Add/adjust targeted tests for the chunk threshold.
- Flush the trailing partial buffer on stop without letting the normal streaming VAD gate drop it.
- Document first-principles truncation causes from the current code.

### Out of scope
- Adding preroll/ring-buffer capture.
- Reworking cloud provider finish protocols.

## Implementation Units

### Unit 1: Add failing chunk-threshold test

**Files**
- Modify: `apps/desktop/src-tauri/src/commands/audio.rs`

**Approach**
- Extract the runtime chunk-size calculation into a tiny helper.
- Add a unit test that expects 200ms-equivalent chunk sizing from sample rate and channel count.
- Run the targeted test first and confirm it fails before changing production logic.

**Verification**
- `cargo test recording_chunk_size_`

### Unit 2: Implement 200ms runtime chunking

**Files**
- Modify: `apps/desktop/src-tauri/src/commands/audio.rs`

**Approach**
- Replace the hardcoded `0.5` second threshold with a helper-backed 200ms threshold.
- Keep the rest of the recording pipeline unchanged.

**Verification**
- `cargo test recording_chunk_size_`

### Unit 3: Capture truncation analysis evidence

**Files**
- No code change required unless brief inline comment/documentation becomes necessary.

**Approach**
- Summarize evidence from `commands/audio.rs`, `audio/recorder.rs`, `audio/stream_processor.rs`, and cloud streaming client finish paths.
- Separate what 200ms chunking mitigates from what still requires a follow-up fix.

**Verification**
- Evidence references included in handoff.

### Unit 4: Preserve stop-time tail audio

**Files**
- Modify: `apps/desktop/src-tauri/src/audio/stream_processor.rs`
- Modify: `apps/desktop/src-tauri/src/commands/audio.rs`

**Approach**
- Split normal streaming chunk processing from stop-time flush processing.
- Reuse the same resample/denoise pipeline for stop flushes, but bypass the normal VAD drop decision for the final buffered remainder.
- Add a targeted unit test that proves a VAD-rejected trailing chunk is still sent during stop flush.

**Verification**
- `cargo test --lib flush_pending_chunk_for_stop_ -- --nocapture`
- `cargo test --lib commands::audio::tests:: -- --nocapture`

## System-Wide Impact

- Cloud STT requests receive smaller, more frequent processed chunks.
- Very short utterances between 200ms and 500ms should improve.
- Explicit stop now sends the final buffered remainder even if normal streaming VAD would have skipped it.
- Start clipping before capture begins still remains possible until preroll/ring-buffer work is added.

## Risks & Dependencies

- Smaller chunks may slightly increase send frequency and CPU overhead, though still aligned with provider guidance.
- Stop-flush bypass sends the final remainder even when it is weak or short, which slightly increases the chance of sending low-value tail audio but matches the product priority of STT completeness over token savings.

## Verification Evidence

- `apps/desktop/src-tauri/src/commands/audio.rs` now flushes pending stop-time PCM through `process_chunk_for_stop_flush()` instead of the normal streaming `process_chunk()` path.
- `apps/desktop/src-tauri/src/audio/stream_processor.rs` now centralizes chunk processing in `process_chunk_inner()` and uses `force_send` only for explicit stop flushes.
- Added `flush_pending_chunk_for_stop_does_not_drop_tail_when_vad_rejects_it` to prove a VAD-rejected tail chunk is still delivered on stop.
- Verified with:
  - `cargo test --lib flush_pending_chunk_for_stop_ -- --nocapture`
  - `cargo test --lib commands::audio::tests:: -- --nocapture`
