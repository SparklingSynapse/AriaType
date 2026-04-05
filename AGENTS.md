# Agent Operating Contract

**Mission**: Convert user intent into complete, verified software changes with minimal back-and-forth.

**Full documentation**: [`docs/README.md`](docs/README.md) — progressive disclosure, not encyclopedia.

---

## Non-Negotiable Rules

| Rule | Description |
|------|-------------|
| **Spec-first** | Never code from vague intuition. Find the spec at `docs/feat/[name]/[ver]/prd/erd.md` first. |
| **TDD/BDD** | `spec → failing test → implement → refactor → verify`. Regressions ship with test first. |
| **No fabrication** | Never invent APIs, files, commands, test results, or behavior. |
| **No fake completion** | Never claim done without running verification. Never present mocks as finished. |
| **English-only** | All code, identifiers, comments, test names, commit messages in English. |
| **No Git modification** | Do not use git to modify history (add/commit/etc.) without explicit user request. |

**Product priority**: STT accuracy > STT stability > user experience > speed.

**Volcengine interface**: Always `bigmodel_nostream`. Bidirectional interfaces have lower accuracy — never use unless user explicitly acknowledges the tradeoff.

---

## Invariants

- Feature delivery follows `docs/feat/[name]/[semver]/prd/erd.md` as source of truth.
- Coverage gates: E2E = 100% for affected workflow, unit = 100% for critical modules.
- Logging follows [`docs/spec/logs.md`](docs/spec/logs.md) — structured fields, lowercase messages, no `println!`.
- Engine testing follows [`docs/spec/engine-api-contract.md`](docs/spec/engine-api-contract.md) — auth errors prove correctness.
- Recovery: 3 consecutive failures → STOP, REVERT, DOCUMENT, ESCALATE.
- Forbidden: `as any`, `@ts-ignore`, empty catch blocks, deleting failing tests, `background_cancel(all=true)`.
- Autonomous decisions: Choose the smallest complete solution. Do not ask for routine confirmation.
- Evidence-backed reporting: Facts as facts, unknowns as unknowns, no speculative filler.

---

## Quick Reference

### Workspace Layout

```
apps/desktop/          # Tauri v2 app (React 19 + Rust)
├── src/               # Frontend (main.tsx, pill.tsx, toast.tsx)
├── src-tauri/src/     # Backend (audio/, stt_engine/, polish_engine/, commands/, state/, text_injector/)
└── src/lib/tauri.ts   # Typed IPC boundary — extend this, not raw invoke()
packages/shared/       # Shared TypeScript types/constants
packages/website/      # Next.js marketing site (static export)
```

### Boundaries

- `src-tauri/capabilities/` — never modify without asking
- `lib.rs` — all commands registered here
- `src/lib/tauri.ts` — all new IPC calls go here

### Verification Commands

```bash
# Rust
cd apps/desktop/src-tauri && cargo test && cargo clippy --all-features -- -D warnings && cargo fmt -- --check

# Frontend
pnpm --filter @ariatype/desktop build && pnpm --filter @ariatype/shared typecheck && pnpm check:i18n

# Website
pnpm --filter @ariatype/website build && pnpm --filter @ariatype/website lint
```

---

## Where to Find Things

| Need | Document |
|------|----------|
| Why the system works this way | [`docs/beliefs.md`](docs/beliefs.md) |
| System architecture and layers | [`docs/architecture/`](docs/architecture/README.md) |
| Data flow and state machines | [`docs/architecture/data-flow.md`](docs/architecture/data-flow.md) |
| Test pyramid and coverage gates | [`docs/spec/testing.md`](docs/spec/testing.md) |
| Engine API contract testing | [`docs/spec/engine-api-contract.md`](docs/spec/engine-api-contract.md) |
| Logging standard | [`docs/spec/logs.md`](docs/spec/logs.md) |
| Debugging and log investigation | [`docs/guides/debugging.md`](docs/guides/debugging.md) |
| Adding a new STT provider | [`docs/guides/adding-stt-provider.md`](docs/guides/adding-stt-provider.md) |
| Design system and UI patterns | [`docs/conventions/design-system.md`](docs/conventions/design-system.md) |
| Quality grades by domain | [`docs/quality/README.md`](docs/quality/README.md) |
| Architecture decisions (ADRs) | [`docs/decisions/README.md`](docs/decisions/README.md) |
| Feature specifications | `docs/feat/<name>/<version>/prd/erd.md` |
| Package-specific guides | `apps/desktop/CONTRIBUTING.md`, `packages/*/CONTRIBUTING.md` |
