# Agent Operating Contract

**Mission**: Convert user intent into complete, verified software changes with minimal back-and-forth.

**Documentation map**: [`docs/README.md`](docs/README.md) — progressive disclosure, not encyclopedia. Start there, follow links for depth.

---

## Default Execution Strategy

Use a superpowers-style iteration loop for all non-trivial work. The default mode is not "big-bang implementation"; it is rapid, evidence-driven convergence.

### Core Loop

1. **Frame** — Restate the user goal as an observable outcome, not an implementation guess
2. **Reduce** — Shrink scope to the smallest vertical slice that proves progress
3. **Locate** — Find the spec, affected boundaries, existing contracts, and canonical docs
4. **Test First** — Add or identify the failing check that proves the gap
5. **Implement** — Change the minimum code needed to make the check pass
6. **Verify** — Run the strongest relevant verification, not the cheapest plausible command
7. **Extract** — Update docs, invariants, and follow-up risks before handoff

### Iteration Rules

- **Prefer vertical slices** — Ship one end-to-end behavior at a time instead of editing many layers speculatively
- **Prefer evidence over intuition** — Logs, tests, contracts, and code paths outrank guesses
- **Prefer existing seams** — Reuse current traits, managers, commands, and document entry points before introducing new structure
- **Prefer small batches** — One user-visible outcome or one invariant per iteration
- **Checkpoint frequently** — After each slice, reassess whether the remaining plan is still the right plan
- **Surface uncertainty early** — State assumptions, missing specs, and external risks before they compound
- **Escalate before drift** — If the task starts widening, stop and reframe instead of silently expanding scope

### Planning Threshold

Create or update a plan before coding when work is multi-file, cross-layer, ambiguous, risky, or likely to take more than one tight iteration.

Use plans to:

- define the target behavior and proof of completion
- split work into atomic implementation units
- record risks, dependencies, and deferred follow-ups
- preserve momentum across agent handoffs

See [`docs/plans/README.md`](docs/plans/README.md) for lifecycle and format.

### Definition of Progress

Progress is only real when it leaves behind at least one of:

- a passing failing-first test
- a verified behavioral fix
- a narrowed root cause with concrete evidence
- a cleaned-up invariant or canonical document

Code churn without stronger evidence is not progress.

---

## Non-Negotiable Rules

| # | Rule | Enforcement |
|---|------|-------------|
| 1 | **Spec-first** — Never code from vague intuition. Find the spec at `docs/feat/[name]/[ver]/prd/erd.md` first. | Blocker |
| 2 | **TDD/BDD** — `spec → failing test → implement → refactor → verify`. Regressions ship with test first. | Blocker |
| 3 | **No fabrication** — Never invent APIs, files, commands, test results, or behavior. | Blocker |
| 4 | **No fake completion** — Never claim done without running verification. Never present mocks as finished. | Blocker |
| 5 | **English-only** — All code, identifiers, comments, test names, commit messages in English. | CI |
| 6 | **No Git operations** — Do not perform any git operations (commit, push, checkout, modify history) without explicit user request. | Manual |
| 7 | **No type suppression** — `as any`, `@ts-ignore`, `@ts-expect-error`, empty catch blocks forbidden. | CI |
| 8 | **Continuous Documentation** — Post-task: extract workflow, compare with docs, and optimize/correct unreasonable parts based on first principles. | Manual |

**Product priority**: STT accuracy > STT stability > user experience > speed.

**Volcengine interface**: Always `bigmodel_nostream`. Bidirectional interfaces have lower accuracy — never use unless user explicitly acknowledges the tradeoff.

**Architecture boundary rules**: See [`docs/architecture/layers.md`](docs/architecture/layers.md#boundary-rules) for layer dependency and module boundary constraints (including recording pipeline separation).

---

## Recovery Protocol

3 consecutive failures → **STOP**, REVERT, DOCUMENT, ESCALATE.

Forbidden: `as any`, `@ts-ignore`, empty catch blocks, deleting failing tests, `background_cancel(all=true)`.

When recovery is triggered:

1. Capture the failed hypothesis, commands, and observed evidence
2. Revert only the unverified change set that caused the regression
3. Narrow the problem frame before attempting another iteration
4. Ask for help or direction instead of continuing blind

---

## Verification Commands

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
| **Project principles and documentation entry points** | [`docs/README.md`](docs/README.md) |
| **New contributor/agent onboarding** | [`docs/guides/onboarding.md`](docs/guides/onboarding.md) |
| System architecture and layers | [`docs/architecture/README.md`](docs/architecture/README.md) |
| Data flow and state machines | [`docs/architecture/data-flow.md`](docs/architecture/data-flow.md) |
| Architecture decisions (ADRs) | [`docs/architecture/decisions/README.md`](docs/architecture/decisions/README.md) |
| Test pyramid and coverage gates | [`docs/spec/testing.md`](docs/spec/testing.md) |
| How to write and run tests | [`docs/guides/testing.md`](docs/guides/testing.md) |
| Engine API contract testing | [`docs/spec/engine-api-contract.md`](docs/spec/engine-api-contract.md) |
| Logging standard | [`docs/spec/logs.md`](docs/spec/logs.md) |
| Debugging and log investigation | [`docs/guides/debugging.md`](docs/guides/debugging.md) |
| Adding a new STT provider | [`docs/guides/adding-stt-provider.md`](docs/guides/adding-stt-provider.md) |
| Adding a new Polish provider | [`docs/guides/adding-polish-provider.md`](docs/guides/adding-polish-provider.md) |
| Rust coding style | [`docs/conventions/rust-style.md`](docs/conventions/rust-style.md) |
| TypeScript/React coding style | [`docs/conventions/typescript-style.md`](docs/conventions/typescript-style.md) |
| Design system and UI patterns | [`docs/conventions/design-system.md`](docs/conventions/design-system.md) |
| Quality grades by domain | [`docs/quality/README.md`](docs/quality/README.md) |
| Doc gardening process | [`docs/quality/gardening.md`](docs/quality/gardening.md) |
| STT provider API reference | [`docs/reference/providers/stt.md`](docs/reference/providers/stt.md) |
| Polish provider API reference | [`docs/reference/providers/polish.md`](docs/reference/providers/polish.md) |
| Boundary rules and module constraints | [`docs/architecture/layers.md`](docs/architecture/layers.md) |
| Feature specifications | `docs/feat/<name>/<version>/prd/erd.md` |
| Execution plans | [`docs/plans/README.md`](docs/plans/README.md) |
| Package-specific guides | `apps/desktop/CONTRIBUTING.md`, `packages/*/CONTRIBUTING.md` |

---

## Boundaries

- `src-tauri/capabilities/` — never modify without asking
- `lib.rs` — all commands registered here
- `src/lib/tauri.ts` — all new IPC calls go here (typed wrappers only, never raw `invoke()`)
