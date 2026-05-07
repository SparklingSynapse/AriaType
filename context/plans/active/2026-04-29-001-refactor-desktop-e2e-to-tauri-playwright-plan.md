---
title: "refactor: migrate desktop e2e suite to tauri-playwright"
type: refactor
status: active
date: 2026-04-29
---

# refactor: Migrate Desktop E2E Suite to tauri-playwright

## Overview

Replace the current browser-only mock-IPC Playwright setup in `apps/desktop/tests/e2e` with a `tauri-playwright`-based test harness that can drive the real Tauri desktop webview. The goal is for desktop E2E tests to validate actual frontend-backend wiring instead of only rendering mocked browser pages.

## Problem Frame

Current state:
- `apps/desktop/tests/e2e` runs against `pnpm dev` in a browser tab.
- IPC is faked via `page.addInitScript(...)` and a handwritten `window.__TAURI_INTERNALS__.invoke` shim.
- Screenshots and selectors prove UI rendering, but they do not prove real Tauri IPC, native webview execution, or Tauri plugin integration.

Desired state:
- Playwright tests use `@srsholmes/tauri-playwright` fixtures and run against the real Tauri webview.
- Rust is compiled with an opt-in E2E feature that enables `tauri-plugin-playwright`.
- Existing page specs are migrated to the new fixture surface with minimal behavioral drift.
- The package scripts and docs make the new E2E workflow explicit.

## Scope Boundaries

- **In scope**: `apps/desktop/tests/e2e/**`, `apps/desktop/package.json`, `apps/desktop/src-tauri/Cargo.toml`, `apps/desktop/src-tauri/src/lib.rs`, desktop testing docs
- **Out of scope**: `apps/desktop/src-tauri/tests/*.rs` Rust integration tests, CI workflow redesign beyond the local runnable path, new product behavior changes for testability

## Implementation Units

- [ ] **Unit 0: Reframe desktop E2E around a black-box first-run journey**

**Goal:** Make the first ordered desktop E2E case behave like a real user journey instead of a bundle of state-reset-driven page probes.

**Files:**
- Modify: `apps/desktop/tests/e2e/e2e.config.mjs`
- Create or modify: `apps/desktop/tests/e2e/pages/journey.spec.ts`
- Modify: `apps/desktop/tests/e2e/pages/onboarding.spec.ts`
- Modify: `apps/desktop/tests/e2e/utils/helpers.ts`
- Modify: `context/spec/e2e-harness/README.md`

**Approach:**
- Add one first-run journey spec that starts from the app's natural initial state and walks the visible user flow in order.
- Keep the journey black-box: prefer visible UI interactions and route navigation over localStorage inspection or backend state forcing.
- Limit long waits to startup and snapshot stabilization. If a branch is inherently slow because of real downloads or system prompts, cover that branch in a smaller dedicated case instead of blocking the main journey.
- Reduce or remove onboarding checks that only exist because tests repeatedly re-open and re-seed the modal.

**Verification:**
- `pnpm --filter @ariatype/desktop run test:e2e -- --grep "Desktop first-run journey"`
- `pnpm --filter @ariatype/desktop run test:e2e -- --list`

---

- [ ] **Unit 1: Replace the E2E harness foundation**

**Goal:** Switch the desktop package from handwritten Playwright mocking to `tauri-playwright`.

**Files:**
- Modify: `apps/desktop/package.json`
- Modify: `apps/desktop/tests/e2e/playwright.config.ts`
- Modify: `apps/desktop/tests/e2e/fixtures.ts`
- Modify: `apps/desktop/tests/e2e/global-setup.ts`
- Modify: `apps/desktop/tests/e2e/global-teardown.ts`
- Delete or simplify: `apps/desktop/tests/e2e/utils/mock-ipc.ts`

**Approach:**
- Add `@srsholmes/tauri-playwright` as a dev dependency.
- Rebuild the Playwright fixture around `createTauriTest(...)`.
- Configure a single Tauri-mode project for the fully replaced suite.
- Keep test expectations compatible with current selectors and screenshots where possible.

**Verification:**
- `pnpm --filter @ariatype/desktop install`
- `pnpm --filter @ariatype/desktop exec playwright test --config=tests/e2e/playwright.config.ts --list`

---

- [ ] **Unit 2: Add the Rust-side Tauri plugin hook**

**Goal:** Make the desktop app expose the plugin bridge required by `tauri-playwright`.

**Files:**
- Modify: `apps/desktop/src-tauri/Cargo.toml`
- Modify: `apps/desktop/src-tauri/src/lib.rs`

**Approach:**
- Add an opt-in `e2e-testing` feature.
- Add optional `tauri-plugin-playwright` dependency gated behind the feature.
- Register the plugin in the Tauri builder only when the feature is enabled so production behavior is unchanged.

**Verification:**
- `cd apps/desktop/src-tauri && cargo test --features e2e-testing`
- `cd apps/desktop/src-tauri && cargo clippy --all-features -- -D warnings`

---

- [ ] **Unit 3: Migrate existing page specs to the tauri-playwright fixture**

**Goal:** Convert current E2E specs to the new fixture API without silently dropping coverage.

**Files:**
- Modify: `apps/desktop/tests/e2e/pages/*.spec.ts`
- Modify: `apps/desktop/tests/e2e/utils/helpers.ts`

**Approach:**
- Replace raw Playwright `page` usage with the `tauriPage` fixture where required.
- Centralize navigation and app-ready helpers so Tauri startup latency does not duplicate waits across tests.
- Keep the same assertions first; only tighten them when the real Tauri environment requires different synchronization.

**Verification:**
- `pnpm --filter @ariatype/desktop test:e2e -- --list`
- `pnpm --filter @ariatype/desktop test:e2e -- tests/e2e/pages/dashboard.spec.ts`

---

- [ ] **Unit 4: Update docs and package workflow**

**Goal:** Make the new E2E model discoverable and consistent with the repo testing guidance.

**Files:**
- Modify: `apps/desktop/CONTRIBUTING.md`
- Modify: `context/guides/testing.md`
- Modify: `context/spec/e2e-harness/README.md`

**Approach:**
- Document that `apps/desktop/tests/e2e` is now real Tauri E2E, not browser mock rendering.
- Add the commands needed to start Tauri in E2E mode and run Playwright against it.
- Remove or reframe outdated references to the previous mock-IPC browser harness where they would mislead contributors.

**Verification:**
- Manual doc review against actual commands and file paths

## System-Wide Impact

- Desktop E2E tests move closer to true end-to-end validation of the backend-driven architecture.
- Tauri startup for E2E becomes slower than the previous browser-only mock flow, but test evidence quality materially improves.
- Existing Vitest frontend tests remain the right place for browser-only mocked IPC coverage.

## Risks & Dependencies

| Risk | Mitigation |
|------|------------|
| Real Tauri startup makes flaky waits more visible | Introduce shared readiness helpers and run a small representative subset first |
| Some existing specs rely on browser-local storage assumptions | Migrate those through explicit app-state setup in shared helpers |
| Plugin/version drift between npm package and Rust crate | Pin explicit versions and verify with Cargo + Playwright listing before broad test migration |

## Working Agreements

- Keep E2E screenshots real and raw. Do not add masking, cropping, or image post-processing just to make snapshots pass.
- Wait for UI stabilization before snapshot capture. The current baseline is at least 1000ms, and longer waits are allowed for animation-heavy steps when evidence shows they are still settling.
- Keep per-test timeout at 2 minutes by default. App startup is handled separately via the worker-scoped Tauri runtime timeout so slow launch does not consume the per-test budget.
- Do not blanket-delete the entire snapshot directory on every `e2e:update` run. Update only the snapshots exercised by the run, then explicitly clean up stale files from renamed or removed tests.

## Verification Evidence

- Pending implementation
