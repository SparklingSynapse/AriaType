# ADR-001: Unified State Layer

**Date**: 2025-09
**Status**: Accepted

## Decision

Use `unified_state.rs` as single source of truth for all runtime state.

## Rationale

Prevents state fragmentation across modules, enables consistent logging and state transitions.

## Alternatives Considered

- Per-module state with synchronization
- Global statics

## Consequences

- All state reads/writes go through a single module
- State transitions are logged and traceable
- Adding new state requires updating the unified state module
- Enables consistent error handling across state boundaries
