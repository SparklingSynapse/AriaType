# Architecture Decision Records

Lightweight decision log for significant architectural choices. When an agent or human makes a decision that affects multiple files, modules, or future work, record it here.

## Format

```markdown
# ADR-NNN: Title

**Date**: YYYY-MM-DD
**Status**: Proposed | Accepted | Deprecated | Superseded

## Decision
[1-2 sentences]

## Rationale
[Why this choice]

## Alternatives Considered
[What else was tried or considered]

## Consequences
[What changes as a result]
```

## Records

| ADR | Title | Date | Status |
|-----|-------|------|--------|
| ADR-001 | Unified State Layer | 2025-09 | Accepted |
| ADR-002 | NoStream Interface for Volcengine | 2025-09 | Accepted |
| ADR-003 | Dual-Layer Text Injection | 2025-10 | Accepted |
| ADR-004 | Engine Trait Separation (Batch vs Streaming) | 2025-10 | Accepted |

### ADR-001: Unified State Layer
**Decision**: Use `unified_state.rs` as single source of truth for all runtime state.
**Rationale**: Prevents state fragmentation across modules, enables consistent logging and state transitions.
**Alternatives**: Per-module state with synchronization; global statics.

### ADR-002: NoStream Interface for Volcengine
**Decision**: Use `bigmodel_nostream` URL for Volcengine STT.
**Rationale**: Bidirectional interfaces have lower accuracy. Product priority order mandates accuracy over latency.
**Alternatives**: `bigmodel_async` (lower accuracy), `bigmodel` (lower accuracy).

### ADR-003: Dual-Layer Text Injection
**Decision**: Layer 0 (keyboard simulation) for short text ≤200 chars, Layer 2 (clipboard paste) for long/multiline text.
**Rationale**: Keyboard simulation loses characters on long input; clipboard paste is reliable but modifies clipboard.
**Alternatives**: Always clipboard (breaks clipboard state); always keyboard (corrupts long text); chunked keyboard with delays (complex, fragile).

### ADR-004: Engine Trait Separation
**Decision**: Separate `SttEngine` (batch) from `StreamingSttEngine` (streaming) traits.
**Rationale**: Local engines process complete files; cloud engines stream chunks in real-time. Different lifecycles require different interfaces.
**Alternatives**: Single trait with optional streaming methods (violates interface segregation).
