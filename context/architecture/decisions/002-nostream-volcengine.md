# ADR-002: NoStream Interface for Volcengine

**Date**: 2025-09
**Status**: Accepted

## Decision

Use `bigmodel_nostream` URL (`wss://openspeech.bytedance.com/api/v3/sauc/bigmodel_nostream`) for Volcengine STT.

## Rationale

Bidirectional interfaces (`bigmodel_async`, `bigmodel`) have lower accuracy. Product priority order mandates accuracy over latency.

## Alternatives Considered

- `bigmodel_async` — lower accuracy
- `bigmodel` — lower accuracy
- Dual interface with fallback — adds complexity without accuracy benefit

## Consequences

- Volcengine STT uses non-streaming (request-response) pattern
- Higher accuracy at the cost of slightly higher latency for final result
- Exception only with explicit user request + measured accuracy impact + documented fallback
- This decision is enforced in `AGENTS.md` as a non-negotiable product constraint
