# Execution Plans

Plans are first-class artifacts. They capture intent, approach, and progress for non-trivial work.

## Plan Types

| Type | Purpose | Lifecycle |
|------|---------|-----------|
| **Fix plans** | Structured bug fixes with root cause analysis | Active → Completed |
| **Feature plans** | Implementation plans for feature specs | Active → Completed |
| **Refactor plans** | Code reorganization with safety checks | Active → Completed |

## Active Plans

| Plan | Type | Date | Status |
|------|------|------|--------|
| [Logging Standardization](./2026-04-03-001-fix-logging-standardization-plan.md) | fix | 2026-04-03 | Active |

## Completed Plans

(None yet — completed plans are moved to `./completed/`)

## Provider API Documentation

API reference docs for cloud providers:
- [STT Provider APIs](./feat/stt-provider/api.md) — Speech-to-Text cloud providers
- [LLM Provider APIs](./feat/llm-provider/api.md) — Text polishing cloud providers

## Plan Format

Every plan MUST include:
1. **Frontmatter**: title, type, status, date
2. **Overview**: What and why
3. **Problem Frame**: Current state vs desired state
4. **Scope Boundaries**: What's in scope, what's out
5. **Implementation Units**: Atomic tasks with files, approach, verification
6. **System-Wide Impact**: What else is affected
7. **Risks & Dependencies**: Known unknowns
