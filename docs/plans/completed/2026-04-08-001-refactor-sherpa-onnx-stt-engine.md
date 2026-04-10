---
title: STT Engine Polish — Post sherpa-onnx Migration Cleanup
type: refactor
status: active
date: 2026-04-08
---

# Overview

The sherpa-onnx migration replaced `whisper-rs` (ggml) and custom SenseVoice (gguf) engines with a unified `SherpaOnnxEngine` backed by sherpa-onnx ONNX runtime. The migration is staged but not yet committed. Before committing, several issues need resolution based on code review.

The core architectural problem is that the current code has two separate recording paths (`start_streaming_recording` for cloud, `start_chunked_recording` for local) that duplicate identical audio processing logic. The fix is to unify them using the same pattern that `start_streaming_recording` already uses:

```
recorder callback → mpsc channel → spawned consumer task
```

All engines — local and cloud — implement the same `send_chunk()` + `finish()` interface. The consumer task code is identical for all engines:

```rust
while let Some(chunk) = rx.recv().await {
    engine.send_chunk(chunk).await?;
}
let text = engine.finish().await?;  // channel closed = recording ended
```

The only difference is what each engine does inside `send_chunk()` and `finish()`. The recorder does not know which engine it is talking to.

## Issues Found During Review

1. **Duplicated recording paths** — `start_streaming_recording` (cloud) and `start_chunked_recording` (local) duplicate identical audio pipeline code (i16→f32, stereo→mono, process_chunk, VAD gate). Only the output sink differs.
2. **Denoise semantics diverge** — Recording paths use `denoise_mode == "on"` only; `SherpaOnnxEngine::transcribe` handles "on"/"auto"/"off". Not unified.
3. **Model recommendation excludes English from SenseVoice** — `recommend_by_language("en")` returns only Whisper models, but SenseVoice lists `"en"` in `prefer_lang`. The function name `is_cjk_language` is misleading since the actual intent is "SenseVoice-preferred languages" (zh, yue, ja, ko, en).
4. **Onboarding model step still shows multiple options** — Should show only the single recommended model for the user's language, not a selection grid.
5. **No startup model auto-download** — Models are only downloaded during onboarding. Should auto-ensure the default model is available at app startup.
6. **Old model files not cleaned up** — Legacy flat files (`ggml-*.bin`, `*.gguf`) remain on disk after migration.
7. **Model step shown for all languages in onboarding** — Should be simplified: one recommended model per language, no manual selection.

# Current State (Post-Migration, Pre-Cleanup)

- **Local STT**: `SherpaOnnxEngine` wraps sherpa-onnx `OfflineRecognizer` for both SenseVoice and Whisper
- **Model definitions**: 3 models in `models.rs` — SENSE_VOICE_SMALL, WHISPER_BASE, WHISPER_SMALL
- **Offline VAD**: Silero VAD via `sherpa_onnx/vad.rs` `filter_silence()` — working
- **Streaming VAD**: Silero VAD via `ThreadSafeVad` in `stream_processor.rs` — working
- **Recording paths**: Two duplicated functions (`start_streaming_recording`, `start_chunked_recording`) with identical audio processing
- **AudioStorage enum**: `Local { f32_chunks, pcm_chunks }` for memory accumulation, `Streaming` for cloud WebSocket
- **Onboarding**: 6 steps — model step conditionally filtered for CJK users
- **Settings migration**: `validate_model_name()` resets unknown model names to `whisper-base`

# Scope Boundaries

## In Scope

1. **Unify recording paths** — All engines use `recorder → mpsc → consumer task` with unified `send_chunk()` / `finish()` trait; removes `start_chunked_recording`
2. **Unify denoise logic** — Same `denoise_mode` handling for streaming and batch paths
3. **Rename `is_cjk_language` → `is_sensevoice_preferred`** — Reflect actual intent: zh, yue, ja, ko, en → SenseVoice
4. **Simplify onboarding model step** — Show single recommended model, not selection grid
5. **Startup model auto-ensure** — Auto-download default model at app startup (in `.setup()`)
6. **Cleanup legacy model files** — Delete old flat files on settings migration
7. **Update docs/specs** — data-flow.md, engine-api-contract.md reflect unified architecture

## Out of Scope

- Cloud STT engine implementations (Volcengine, Qwen, ElevenLabs) — unchanged
- Offline VAD (already Silero, unchanged)
- Model definitions (already correct in models.rs)
- Polish/text processing pipeline
- SherpaOnnxEngine internals (already working)

# Implementation Units

## Unit 1: Unified SttEngine Trait & Recording Path

**Files**:
- `apps/desktop/src-tauri/src/stt_engine/traits.rs`
- `apps/desktop/src-tauri/src/commands/audio.rs`
- `apps/desktop/src-tauri/src/state/unified_state.rs`

### Current Problem

Two functions with nearly identical audio processing pipelines:

- `start_streaming_recording` — creates `StreamingSttClient`, feeds chunks via mpsc channel, spawned task forwards to WebSocket, awaits `finish()`
- `start_chunked_recording` — accumulates `f32_chunks` and `pcm_chunks` in `Arc<Mutex<Vec<...>>>`, no mpsc, no spawned task

Both contain: device selection, settings read, `StreamAudioProcessor::new()`, audio callback with i16→f32, stereo→mono, `process_chunk()`, VAD gate. The recorder callback and audio processing are identical — the only difference is where the processed chunk goes.

### Key Insight

The existing `start_streaming_recording` pattern is already correct for **all** engines:

```
recorder callback → mpsc::Sender<Vec<i16>> → spawned task (mpsc::Receiver loop) → engine.finish()
```

We just need to make local engines speak the same `send_chunk()` + `finish()` API that cloud streaming engines already use.

### Target Architecture

Replace `SttEngine` (batch) and `StreamingSttEngine` (streaming) with a single unified `SttEngine` trait:

```rust
/// Unified STT engine — all engines implement this.
///
/// Every recording follows the same pattern:
///   1. Create engine (cloud: connect WebSocket; local: no-op)
///   2. Loop: send_chunk(pcm_16khz_mono) for each VAD-filtered chunk
///   3. finish() → await final transcription
///
/// Engine implementations decide internally what send_chunk does:
/// - Local (SherpaOnnxEngine): buffer Vec<i16> in memory
/// - Cloud streaming (Volcengine/Qwen/ElevenLabs): forward to WebSocket
/// - Cloud non-streaming: buffer in memory
#[async_trait]
pub trait SttEngine: Send + Sync {
    /// Engine type identifier.
    fn engine_type(&self) -> EngineType;

    /// Feed a processed audio chunk to the engine.
    /// `pcm_data`: 16-bit PCM, 16kHz mono, VAD-filtered.
    async fn send_chunk(&self, pcm_data: Vec<i16>) -> Result<(), String>;

    /// Signal end of audio and await final transcription.
    /// Called after the mpsc channel is closed (recording stopped).
    async fn finish(&self) -> Result<String, String>;

    /// Set callback for partial transcription results (streaming engines only).
    fn set_partial_callback(&mut self, callback: PartialResultCallback);
}
```

Note: `start()` is removed — initialization happens in the constructor. `get_audio_sender()` is removed — the mpsc sender is managed by the recording code, not the engine.

### Unified Recording Path

The existing `start_streaming_recording` recorder callback stays **unchanged**. The only change is what the spawned consumer task does:

```rust
// Before (cloud only): start_streaming_recording creates StreamingSttClient
// After (all engines):  start_streaming_recording creates any SttEngine

fn start_streaming_recording(app: &AppHandle, task_id: u64) -> Result<(), String> {
    let state = app.state::<AppState>();

    // Read settings (unchanged)
    let (cloud_enabled, cloud_config, language, denoise_mode, vad_enabled, stt_context) = { ... };

    // Create engine based on settings
    let mut engine: Box<dyn SttEngine> = if cloud_enabled {
        Box::new(StreamingSttClient::new(cloud_config, Some(&language), stt_context)?)
    } else {
        Box::new(SherpaOnnxBufferingEngine::new(/* model, language from settings */))
    };

    // Set partial callback (cloud uses it, local ignores it)
    let app_clone = app.clone();
    engine.set_partial_callback(Arc::new(move |result| {
        // emit TRANSCRIPTION_PARTIAL (unchanged)
    }));

    // Create mpsc channel — SAME as current cloud streaming code
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<i16>>(100);

    // Store tx in state for recorder callback (unchanged)
    *state.streaming_stt.lock() = Some(StreamingSttState {
        audio_tx: tx.clone(),
        accumulated_text: String::new(),
        task_id,
        streaming_task: Arc::new(ParkingMutex::new(None)),
    });
    *state.audio_storage.lock() = None; // No more AudioStorage

    // Recorder callback — IDENTICAL to current start_streaming_recording
    // (i16→f32, stereo→mono, process_chunk, VAD gate, tx.try_send(pcm))
    let (sr, ch) = {
        let recorder = state.recorder.lock();
        recorder.start_streaming(device_name, move |pcm, sr, ch| {
            // ... identical audio processing ...
            if result.has_speech {
                let _ = app_tx_clone.try_send(result.pcm_16khz_mono);
            }
        })
    }?;

    // Spawned consumer task — SAME code for ALL engines
    let handle = tauri::async_runtime::spawn(async move {
        let mut chunks_sent = 0;
        while let Some(chunk) = rx.recv().await {
            if let Err(e) = engine.send_chunk(chunk).await {
                error!(task_id, error = %e, "chunk_send_failed");
                break;
            }
            chunks_sent += 1;
        }
        // Channel closed = recording stopped
        let text = engine.finish().await;
        // ... polish, inject, history (unchanged) ...
    });

    // Store handle (unchanged)
    if let Some(stt) = state.streaming_stt.lock().as_mut() {
        stt.streaming_task.lock().replace(handle);
    }
    Ok(())
}
```

### SherpaOnnxBufferingEngine (Local)

Local engine buffers chunks internally, transcribes at `finish()`:

```rust
/// Local STT engine that buffers chunks and batch-transcribes at finish().
/// Implements SttEngine so it uses the same consumer task as cloud engines.
pub struct SherpaOnnxBufferingEngine {
    model_name: String,
    language: String,
    /// Accumulated PCM chunks, consumed at finish()
    pcm_chunks: Mutex<Vec<Vec<i16>>>,
    engine_manager: Arc<UnifiedEngineManager>,
}

#[async_trait]
impl SttEngine for SherpaOnnxBufferingEngine {
    fn engine_type(&self) -> EngineType {
        self.engine_manager.get_engine_by_model_name(&self.model_name)
            .unwrap_or(EngineType::Whisper)
    }

    async fn send_chunk(&self, pcm_data: Vec<i16>) -> Result<(), String> {
        // Just buffer — no processing yet
        self.pcm_chunks.lock().push(pcm_data);
        Ok(())
    }

    async fn finish(&self) -> Result<String, String> {
        let chunks: Vec<Vec<i16>> = {
            let mut guard = self.pcm_chunks.lock();
            std::mem::take(&mut *guard)
        };

        // Flatten i16 → f32 for sherpa-onnx
        let samples: Vec<f32> = chunks
            .into_iter()
            .flatten()
            .map(|s| s as f32 / 32768.0)
            .collect();

        if samples.is_empty() {
            return Ok(String::new());
        }

        let request = TranscriptionRequest::new_memory(samples)
            .with_model(&self.model_name)
            .with_language(&self.language);

        let result = self.engine_manager.transcribe(self.engine_type(), request).await?;
        Ok(result.text)
    }

    fn set_partial_callback(&mut self, _callback: PartialResultCallback) {
        // Local engine doesn't produce partial results — no-op
    }
}
```

### Cloud Streaming Engine (Unchanged)

The existing `StreamingSttClient` already implements `send_chunk` + `finish` semantics — just rename the trait impl:

```rust
// StreamingSttClient currently implements StreamingSttEngine with:
//   send_chunk() → forward to internal mpsc → WebSocket
//   finish()     → drop internal sender, await WebSocket final result
//
// Change: impl SttEngine instead of StreamingSttEngine
// The send_chunk/finish methods map directly.
```

### Implementation Steps

1. Rename `StreamingSttEngine` trait to `SttEngine` — remove `start()`, `get_audio_sender()`, add `engine_type()`
2. Remove old `SttEngine` trait (batch `transcribe()`) — kept only as `transcribe_file()` on `UnifiedEngineManager` for drag-drop
3. Create `SherpaOnnxBufferingEngine` — implements `SttEngine` with `send_chunk` = buffer, `finish` = batch transcribe
4. Update `StreamingSttClient` to impl new `SttEngine` instead of `StreamingSttEngine`
5. Delete `start_chunked_recording` entirely
6. Update `start_streaming_recording` to create either `SherpaOnnxBufferingEngine` or `StreamingSttClient` based on settings
7. Update `stop_recording_sync` — remove `AudioStorage::Local` branch; all paths go through the `StreamingSttState.streaming_task` handle
8. Remove `AudioStorage` enum — no longer needed

**Verification**: `cargo test` passes; `cargo clippy --all-features -- -D warnings` clean

## Unit 2: Unify Denoise Logic

**Files**: `apps/desktop/src-tauri/src/commands/audio.rs`

**Current**: Both recording paths use `denoise_enabled = denoise_mode == "on"` — the "auto" mode is silently ignored in the recording pipeline. Meanwhile, `SherpaOnnxEngine::transcribe` handles "auto" with `should_denoise()` heuristic.

**Target**: Recording pipeline denoise should match the engine's behavior — or at minimum, "auto" should be a valid option.

**Approach**: Pass `denoise_mode: &str` to `StreamAudioProcessor::new()` instead of `denoise_enabled: bool`. The processor decides internally:
- `"on"` → always denoise
- `"off"` → never denoise
- `"auto"` → apply RNNoise denoise (current behavior for `"on"`)

This keeps denoise consistent across all paths. The `"auto"` heuristic (`should_denoise`) stays in `SherpaOnnxEngine` for file-based transcription only, since it analyzes the full audio buffer after recording.

**Verification**: `cargo check` passes

## Unit 3: Rename is_cjk_language → is_sensevoice_preferred

**Files**:
- `apps/desktop/src-tauri/src/stt_engine/models.rs`
- `apps/desktop/src-tauri/src/components/Home/OnboardingGuide.tsx` (TypeScript equivalent)
- All callers

**Current**: `is_cjk_language()` checks `CJK_BASE_CODES = &["zh", "yue", "ja", "ko"]`. Name says "CJK" but English is also SenseVoice's strength. The recommendation logic splits on CJK vs non-CJK, sending English users to Whisper only.

**Target**: Rename to `is_sensevoice_preferred()`. Include `"en"` in the check:

```rust
const SENSEVOICE_PREFERRED_CODES: &[&str] = &["zh", "yue", "ja", "ko", "en"];

pub fn is_sensevoice_preferred(lang: &str) -> bool {
    let base_lang = lang.split('-').next().unwrap_or(lang);
    SENSEVOICE_PREFERRED_CODES.contains(&base_lang)
}
```

Update `recommend_by_language` and `default_for_language`:
- `SenseVoice-preferred languages` → recommend SenseVoice Small
- Other languages → recommend Whisper Base

Update TypeScript `isCjkLanguage` → `isSenseVoicePreferred` in `OnboardingGuide.tsx`.

**Verification**: `cargo test` passes (update test names and assertions); `pnpm build` passes

## Unit 4: Simplify Onboarding Model Step

**Files**: `apps/desktop/src/components/Home/OnboardingGuide.tsx`

**Current**: `ModelStep` renders `models.map(model => ...)` showing all recommended models as a selection grid. CJK users skip the step entirely. Non-CJK users see 2 Whisper models.

**Target**: Model step shows exactly one recommended model (the default for the user's language). No selection — just a download progress indicator. The step label changes from "Select Model" to "Downloading Model".

**Approach**:
1. `ModelStep` always shows single model: `default_for_language(language)`
2. Remove model selection grid UI
3. Show model name, size, and download progress
4. Auto-start download on mount (current behavior, but for single model only)
5. Remove the CJK skip logic — all languages go through this simplified step (it's just a download, not a choice)
6. Step count stays at 6 for all languages, but model step is now trivial (auto-download only)

**Verification**: `pnpm --filter @ariatype/desktop build` passes

## Unit 5: Startup Model Auto-Ensure

**Files**:
- `apps/desktop/src-tauri/src/lib.rs` (`.setup()` closure)
- `apps/desktop/src-tauri/src/stt_engine/unified_manager.rs` (add `ensure_default_model`)

**Current**: Models are downloaded only during onboarding (`ModelStep` or CJK auto-config). If a user skips onboarding or the model isn't ready, recording fails.

**Target**: At app startup, check if the default model for the current language is downloaded. If not, start a background download.

**Approach**:
1. Add `UnifiedEngineManager::ensure_default_model(language: &str) -> JoinHandle<()>` that:
   - Resolves `default_for_language(language)`
   - Checks `is_model_downloaded()`
   - If missing, spawns `download_model()` with a dummy progress callback
   - Also calls `ensure_vad_model()` if VAD model missing
2. In `lib.rs` `.setup()`, after history cleanup:
   ```rust
   let state = app.state::<AppState>();
   let settings = state.settings.lock();
   let language = settings.stt_engine_language.clone();
   drop(settings);
   let mgr = state.engine_manager.clone();
   tokio::spawn(async move {
      if let Err(e) = mgr.ensure_default_model(&language).await {
          warn!(error = %e, "startup_model_ensure_failed");
      }
   });
   ```
3. Remove the onboarding-only model download logic — it becomes redundant

**Verification**: App starts without crash; background download works; no duplicate downloads if model exists

## Unit 6: Cleanup Legacy Model Files

**Files**:
- `apps/desktop/src-tauri/src/commands/settings/mod.rs`

**Current**: `validate_model_name()` resets unknown model names to `whisper-base` in settings, but old model files remain on disk:
- `ggml-tiny.bin`, `ggml-base.bin`, `ggml-small-q8_0.bin`, `ggml-medium-q5_0.bin`, `ggml-large-v3-turbo-q8_0.bin`
- `sense-voice-small-q4_k.gguf`, `sense-voice-small-q8_0.gguf`

**Target**: Delete these files during settings migration (one-time cleanup).

**Approach**:
1. Define legacy file list as constant:
   ```rust
   const LEGACY_MODEL_FILES: &[&str] = &[
       "ggml-tiny.bin", "ggml-base.bin", "ggml-small-q8_0.bin",
       "ggml-medium-q5_0.bin", "ggml-large-v3-turbo-q8_0.bin",
       "sense-voice-small-q4_k.gguf", "sense-voice-small-q8_0.gguf",
   ];
   ```
2. Add `cleanup_legacy_models()` function called during `load_settings_from_disk()`:
   ```rust
   fn cleanup_legacy_models() {
       let models_dir = AppPaths::models_dir();
       for filename in LEGACY_MODEL_FILES {
           let path = models_dir.join(filename);
           if path.exists() {
               match std::fs::remove_file(&path) {
                   Ok(()) => info!(file = filename, "legacy_model_removed"),
                   Err(e) => warn!(file = filename, error = %e, "legacy_model_removal_failed"),
               }
           }
       }
   }
   ```
3. Call once during settings load (idempotent — safe to run every startup)

**Verification**: App starts clean; old files deleted if present; no errors if files already gone

## Unit 7: Update Documentation

**Files**:
- `docs/architecture/data-flow.md` — update recording pipeline (unified send_chunk/finish consumer task)
- `docs/spec/engine-api-contract.md` — unified `SttEngine` replaces `SttEngine` + `StreamingSttEngine`
- `docs/plans/completed/2026-04-08-001-refactor-sherpa-onnx-stt-engine.md` — this plan

**Approach**:
- Document the unified `SttEngine` lifecycle: `send_chunk()` × N → `finish()`
- Document the consumer task pattern: `recorder → mpsc → while let Some(chunk) = rx.recv() { engine.send_chunk() }; engine.finish()`
- Remove references to `AudioStorage::Local/Streaming`, `start_chunked_recording`, `ProcessedChunkSink`
- Add `SherpaOnnxBufferingEngine` to the engine API contract

**Verification**: Docs are consistent with code

# System-Wide Impact

| Area | Impact |
|------|--------|
| Recording pipeline | Single `start_streaming_recording` for all engines; `start_chunked_recording` deleted |
| STT trait architecture | Unified `SttEngine` (send_chunk + finish) replaces `SttEngine` (batch) + `StreamingSttEngine` (streaming) |
| AudioStorage | Removed — replaced by mpsc channel + engine internal buffer |
| Consumer task | Identical code for all engines: `while let Some(chunk) = rx.recv() { engine.send_chunk() }; engine.finish()` |
| Local STT | `SherpaOnnxBufferingEngine`: send_chunk = Vec::push, finish = batch transcribe |
| Cloud STT | Unchanged internally — `StreamingSttClient` impls same trait with WebSocket forwarding |
| Denoise behavior | Consistent across streaming and batch paths |
| Model recommendation | SenseVoice recommended for zh/yue/ja/ko/en; Whisper for all others |
| Onboarding | Model step simplified to single auto-download for all languages |
| App startup | Auto-ensures default model is available |
| Settings migration | Cleans up legacy model files from disk |

# Risks

| Risk | Mitigation |
|------|-----------|
| Unified trait forces local engines through async send_chunk | `send_chunk` for local is just `Vec::push` — negligible overhead vs `Arc<Mutex<Vec>>::push` it replaces |
| Channel capacity (100) insufficient for long recordings | Chunks are ~0.5s each; 100 = 50s buffer. Consumer task drains continuously. Same as current cloud behavior. |
| Denoise "auto" in streaming may increase CPU | Monitor; "auto" heuristic only available post-recording for engine, streaming defaults to "on" behavior |
| SenseVoice quality for English | SenseVoice explicitly lists "en" in prefer_lang; acceptable per product decision |
| Startup model download on slow networks | Background task, non-blocking; user can start recording once download completes |
| Legacy cleanup removes files user might want | Files are for old engines that no longer exist in code — zero utility |
| Model step in onboarding feels redundant with startup download | The step confirms download progress to the user; still valuable for first-run experience |

# Verification Evidence

_To be filled during implementation:_
- [ ] `cargo test` passes
- [ ] `cargo clippy --all-features -- -D warnings` clean
- [ ] `pnpm --filter @ariatype/desktop build` passes
- [ ] Manual test: streaming recording with unified session works (cloud STT)
- [ ] Manual test: local recording with unified session works (local STT)
- [ ] Manual test: onboarding model step shows single model
- [ ] Manual test: app startup auto-downloads default model
- [ ] Manual test: legacy model files cleaned up
- [ ] Manual test: English language recommends SenseVoice

(End of file - total 527 lines)
