# ADR-004: Engine Trait Separation (Batch vs Streaming)

**Date**: 2025-10
**Status**: Accepted

## Decision

Separate `SttEngine` (batch) from `StreamingSttEngine` (streaming) as two independent traits.

## Rationale

Local engines (Whisper, SenseVoice) process complete audio files in one call. Cloud engines (Volcengine, OpenAI, Deepgram) stream audio chunks in real-time via WebSocket. These fundamentally different lifecycles require different interfaces.

## Alternatives Considered

- Single trait with optional streaming methods — violates interface segregation principle, makes it unclear which methods are required
- Enum-based dispatch — adds unnecessary indirection

## Consequences

- Each engine implements exactly the trait it supports
- `SttEngine` trait: `transcribe(audio: AudioData) -> Result<String>`
- `StreamingSttEngine` trait: `start_streaming()`, `send_chunk()`, `stop_streaming()` lifecycle
- Cloud engines must implement `StreamingSttEngine`
- Local engines must implement `SttEngine`
- Factory pattern selects the correct trait based on configuration
