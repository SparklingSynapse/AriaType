# ADR-004: Unified SttEngine Trait

**Date**: 2025-10 (revised 2026-04)
**Status**: Accepted (supersedes original Engine Trait Separation)

## Decision

Replace separate `SttEngine` (batch) and `StreamingSttEngine` (streaming) traits with a single unified `SttEngine` trait. All engines — local and cloud — implement the same `send_chunk()` + `finish()` interface.

```rust
#[async_trait]
pub trait SttEngine: Send + Sync {
    fn engine_type(&self) -> EngineType;
    async fn send_chunk(&self, pcm_data: Vec<i16>) -> Result<(), String>;
    async fn finish(&self) -> Result<String, String>;
    fn set_partial_callback(&mut self, callback: PartialResultCallback);
}
```

Every recording follows the same consumer task pattern:
```rust
while let Some(chunk) = rx.recv().await {
    engine.send_chunk(chunk).await?;
}
let text = engine.finish().await?;
```

## Rationale

The original separation assumed local (batch) and cloud (streaming) engines had fundamentally different lifecycles. In practice:

1. The recording pipeline is identical for both — `recorder callback → mpsc channel → spawned consumer task`
2. The consumer task code is the same for all engines: receive chunks, forward to engine, await finish
3. The only difference is what the engine does inside `send_chunk()` — buffer or forward
4. The existing `start_streaming_recording` pattern already works correctly; `start_chunked_recording` is unnecessary duplication

This approach reuses the existing streaming architecture unchanged — local engines just buffer instead of forwarding to WebSocket.

## Previous Decision (2025-10)

Separate `SttEngine` (batch) from `StreamingSttEngine` (streaming) as two independent traits.

**Why this changed**: Code review revealed `start_chunked_recording` duplicates all audio processing from `start_streaming_recording`. Instead of adding abstraction layers (SttSession, ProcessedChunkSink), the simplest fix is to make local engines speak the same `send_chunk`/`finish` API that cloud engines already use.

## Alternatives Considered

- **SttSession trait** (accept_chunk + finish consuming self) — adds new trait + factory + AudioChunk struct. More complex than needed.
- **ProcessedChunkSink enum** — unifies recording callback but doesn't unify the engine interface.
- **Single trait with optional streaming methods** — violates interface segregation. Rejected in original ADR, still valid.

## Consequences

- Single recording path: `start_streaming_recording` creates `Box<dyn SttEngine>` (local or cloud)
- `start_chunked_recording` deleted entirely
- `AudioStorage` enum removed — mpsc channel replaces in-memory accumulation
- `StreamingSttEngine` trait merged into `SttEngine`
- Old `SttEngine` trait (batch `transcribe()`) removed — file transcription stays on `UnifiedEngineManager`
- Local engine: `send_chunk` = `Vec::push`, `finish` = batch transcribe
- Cloud engine: `send_chunk` = WebSocket forward, `finish` = await final result
