# Testing Specification

This document defines the testing strategy, coverage requirements, and verification workflows for the AriaType monorepo.

## Test Pyramid

| Layer | Validates | Location |
|-------|-----------|----------|
| Unit | Pure logic, boundaries, parsing, error branches | `src/` alongside code, `tests/` for Rust |
| Integration | Module boundaries, state transitions, IPC, persistence | `src-tauri/tests/` |
| E2E | Real workflows: user input to observable output | `src-tauri/tests/` pipeline tests |

## Coverage Gates

Coverage requirements are mandatory and enforced per-task:

- **End-to-end**: 100% coverage for the affected user workflow before handoff
- **Unit**: 100% coverage for affected critical core modules
- **Other coverage**: Extend honestly where risk justifies it

Must cover: Happy path, error paths, edge cases, regression scenarios.

## Coverage Verification Commands

```bash
# Rust
cd apps/desktop/src-tauri && cargo llvm-cov --html

# Frontend
pnpm --filter @ariatype/desktop test -- --coverage
```

## TDD Execution Order

Follow this sequence for every change:

1. Write failing test (capture behavior or reproduce bug)
2. Confirm test fails without implementation
3. Implement smallest correct change
4. Refactor while keeping tests green
5. Run full verification set

## BDD Scenario Model

```
Given: initial state, permissions, config, files, models
When:  user action, command, event, or pipeline input
Then:  observable UI state, events, persisted settings, output
```

## No-Fabrication Test Policy

- Do NOT use mocks to simulate product completion
- Do NOT replace missing behavior with fake success
- Use test doubles ONLY for external dependencies in automated tests
- Report ONLY coverage from executed tooling, never estimates

## E2E UI Verification

Must inspect what user sees, not just backend events:

- Screenshots + DOM content
- Rendered text snapshots
- Accessibility tree inspection

All screenshots stored in `.playwright-mcp/` (gitignored).

## Verification Commands by Package

```bash
# Rust Backend
cd apps/desktop/src-tauri
cargo test
cargo test --test pipeline_integration_test
cargo clippy --all-features -- -D warnings
cargo fmt -- --check

# Frontend
pnpm --filter @ariatype/desktop build
pnpm --filter @ariatype/desktop test

# Website
pnpm --filter @ariatype/website build
pnpm --filter @ariatype/website lint

# Shared types
pnpm --filter @ariatype/shared typecheck

# i18n
pnpm check:i18n
```
