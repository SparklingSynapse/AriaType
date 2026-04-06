# ADR-003: Dual-Layer Text Injection

**Date**: 2025-10
**Status**: Accepted

## Decision

Use two injection strategies based on text length:
- **Layer 0**: Keyboard simulation (CGEvent) for short text ≤ 200 chars
- **Layer 2**: Clipboard paste (NSPasteboard + Cmd+V) for long/multiline text > 200 chars

## Rationale

Keyboard simulation loses characters on long input due to event queue limitations. Clipboard paste is reliable but modifies clipboard state. The dual approach balances reliability with user experience.

## Alternatives Considered

- Always clipboard — breaks user clipboard state
- Always keyboard — corrupts long text
- Chunked keyboard with delays — complex, fragile, still unreliable for very long text

## Consequences

- Short text injected without clipboard modification
- Long text uses clipboard (user's previous clipboard content is lost)
- Text length threshold is configurable at compile time
- macOS-specific implementation (Windows uses different approach)
