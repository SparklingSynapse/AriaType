# Architecture Decision Records

Lightweight decision log for significant architectural choices. When an agent or human makes a decision that affects multiple files, modules, or future work, record it here.

## When to Read This

- Read [`../../README.md`](../../README.md) for document routing and canonical sources
- Read [`../README.md`](../README.md) for the current system structure and domain map
- Read this directory when the question is "why was this architectural direction chosen?"
- Do not use ADRs as implementation checklists or formal API contracts; use guides and specs for that

## Format

Every ADR is a separate file: `NNN-kebab-case-title.md`

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
| [ADR-001](./001-unified-state-layer.md) | Unified State Layer | 2025-09 | Accepted |
| [ADR-002](./002-nostream-volcengine.md) | NoStream Interface for Volcengine | 2025-09 | Accepted |
| [ADR-003](./003-dual-layer-text-injection.md) | Dual-Layer Text Injection | 2025-10 | Accepted |
| [ADR-004](./004-engine-trait-separation.md) | Engine Trait Separation (Batch vs Streaming) | 2025-10 | Accepted |
