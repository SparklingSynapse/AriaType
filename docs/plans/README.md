# Execution Plans

Plans are first-class artifacts. They capture intent, approach, and progress for non-trivial work.

## When to Read This

- Read [`../README.md`](../README.md) for document routing and canonical sources
- Read [`../../AGENTS.md`](../../AGENTS.md) for the planning threshold and default iteration strategy
- Read [`../feat/README.md`](../feat/README.md) for feature intent and acceptance criteria
- Read this directory when work needs execution structure, progress tracking, or handoff continuity beyond a tight single iteration

## Plan Types

| Type | Purpose | Lifecycle |
|------|---------|-----------|
| **Fix plans** | Structured bug fixes with root cause analysis | Draft → Active → Completed |
| **Feature plans** | Implementation plans for feature specs | Draft → Active → Completed |
| **Refactor plans** | Code reorganization with safety checks | Draft → Active → Completed |

## Plan Lifecycle

| State | Meaning | Source of Truth |
|------|---------|-----------------|
| **Draft** | Plan exists but is not yet being executed | File frontmatter `status: draft` |
| **Active** | Work is in progress | File lives in `./active/` and frontmatter `status: active` |
| **Completed** | Scope is finished and verification evidence is recorded | File moves to `./completed/` and frontmatter becomes `status: completed` |

A plan is only completed when all implementation units are closed, verification evidence is captured in the plan, and the file is moved from `./active/` to `./completed/`.

## Active Plans

| Plan | Type | Date | Status |
|------|------|------|--------|
| [Logging Standardization](./active/2026-04-03-001-fix-logging-standardization-plan.md) | fix | 2026-04-03 | Active |

## Completed Plans

| Plan | Type | Date |
|------|------|------|
| [sherpa-onnx STT Engine Refactor](./completed/2026-04-08-001-refactor-sherpa-onnx-stt-engine.md) | refactor | 2026-04-08 |
| [Startup Permission Logging Architecture](./completed/2026-04-14-006-startup-permission-logging-architecture-plan.md) | fix | 2026-04-14 |

## Provider API Reference

Provider API docs have moved to [`reference/providers/`](../reference/README.md):
- [STT Provider APIs](../reference/providers/stt.md) — Speech-to-Text cloud providers
- [Polish Provider APIs](../reference/providers/polish.md) — Text polishing cloud providers

## Completion Procedure

When a plan moves from `active/` to `completed/`:
1. Update frontmatter `status` from `active` to `completed`
2. Add or finalize verification evidence inside the plan
3. Move the file into `./completed/`
4. Update this index and `docs/README.md` if the active/completed listings changed

## Plan Format

Every plan MUST include:
1. **Frontmatter**: title, type, status, date
2. **Overview**: What and why
3. **Problem Frame**: Current state vs desired state
4. **Scope Boundaries**: What's in scope, what's out
5. **Implementation Units**: Atomic tasks with files, approach, verification
6. **System-Wide Impact**: What else is affected
7. **Risks & Dependencies**: Known unknowns
8. **Verification Evidence**: Commands run, results observed, or linked proof when completed
