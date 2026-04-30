---
name: tauri-e2e-harness-adoption
description: Use when adopting, refactoring, or debugging packages/e2e-harness in a Tauri app, especially when deciding app-specific E2E config values, runner wiring, readiness waits, or cleanup boundaries.
---

# Tauri E2E Harness Adoption

## Overview

Use this skill when wiring `packages/e2e-harness` into a Tauri app or when fixing a broken harness integration.

Core principle: keep the shared harness generic, and keep all app-specific decisions concentrated in `tests/e2e/e2e.config.mjs`.

## When to Use

- You need to add reusable Tauri E2E support to a new app.
- You are moving app-local Playwright harness code into `packages/e2e-harness`.
- `tauri-e2e-runner` works in one app but not another.
- You are unsure how to choose `killCommand`, `systemDataPaths`, `runtimeRoot`, `socketPath`, or `specOrder`.
- E2E tests are flaky because the app is not really ready when the test proceeds.

Do not use this skill for product-specific test flows or app IPC details. Those stay in the consumer app.

## Preflight

Before writing any harness files, verify the app already has these prerequisites:

- the frontend dev server can start from `pnpm ...`
- the Tauri app can start from `pnpm ...`
- the Tauri app enables `tauri_plugin_playwright` in an E2E-only mode or feature flag
- the Playwright socket path is read from `TAURI_PLAYWRIGHT_SOCKET` or equivalent config

If these prerequisites are missing, fix the app-side E2E wiring first.

## Fast Path

Create exactly these files first:

- `tests/e2e/e2e.config.mjs`
- `tests/e2e/fixtures.ts`
- `tests/e2e/playwright.config.ts`
- `tests/e2e/pages/*.spec.ts`

Use the package README for the exact starter snippets.

## The One Contract Rule

Treat `tests/e2e/e2e.config.mjs` as the only app-specific contract.

It should own:

- project root
- Tauri command
- frontend dev-server command
- runtime root
- socket path
- cleanup command
- system data reset paths
- spec order

Do not duplicate those values in random scripts or helpers.

## Choosing App-Specific Values

### `killCommand`

Choose a best-effort command that kills stale dev binaries from previous runs.

Example:

```sh
pkill -f "target/debug/your-app-name"
```

Use it when the app holds a single-instance lock, socket, or tray/background process.

### `systemDataPaths`

Include only app-owned state that must be reset for deterministic first-run behavior.

Good candidates:

- settings files
- local databases
- app-specific WebKit persistence
- app-created recordings or temp files

Do not put expensive reusable assets here if warm-cache runs matter, such as downloaded speech models.

### `runtimeRoot`

Use a test-only directory inside the app repo, typically under `tests/e2e/.runtime/`.

### `socketPath`

Use a unique path per app and runtime key, usually under `/tmp/tauri-e2e-<runtime-key>.sock`.

### `specOrder`

Run the broadest, stateful journey first. Keep smaller specs after it.

### `capabilityFiles`

Use when your Tauri app has optional plugins gated behind Cargo features (e.g., `tauri-plugin-playwright` behind `e2e-testing`). Tauri validates all capability files at build time, so feature-gated permissions can't live in the shared `capabilities/` directory — they break regular dev builds.

Store the E2E capability file outside the capabilities directory (e.g., `tests/e2e/capabilities/e2e.json`) and let the runner copy it in before the Tauri build:

```js
export const capabilityFiles = [
  {
    src: join(e2eDir, 'capabilities', 'e2e.json'),
    dest: join(projectRoot, 'src-tauri', 'capabilities', 'e2e.json'),
  },
];
```

### `seedDataFiles`

Use when tests need a specific app state (e.g., cloud services enabled, mock API keys). Copies a fixture file to the app's real data directory before the app starts. The `dest` is an absolute path.

```js
export const seedDataFiles = [
  {
    src: join(e2eDir, 'fixtures', 'settings-cloud-enabled.json'),
    dest: join(userHome, 'Library', 'Application Support', 'myapp', 'settings.json'),
  },
];
```

Pair with `systemDataPaths` to clean up seeded files after each run. Do not modify app code for test isolation — seed files to the existing data directory the app already reads from.

## Runner Decision

Use the ordered runner for normal execution:

```sh
pnpm exec tauri-e2e-runner tests/e2e/e2e.config.mjs
```

Use direct Playwright only for interactive debugging:

```sh
playwright test --config=tests/e2e/playwright.config.ts --debug
```

Why: the ordered runner reuses one external Tauri runtime and runs spec batches in a deterministic order.

## Readiness Rules

Never use guessed sleeps for business readiness.

Prefer explicit readiness signals such as:

- visible granted icons for permission checks
- visible ready icons for model download steps
- concrete route changes
- concrete enabled/disabled button states

Good:

```ts
await expect(modalReadyIcon).toBeVisible({ timeout: 10000 });
```

Fixed waits are acceptable only for pre-snapshot stabilization after the business state is already ready.

## Boundary Rules

Keep in the shared harness:

- runtime lifecycle
- fixture creation
- runner CLI
- snapshot stabilization
- generic route/wait helpers

Keep in the app:

- product-specific IPC helpers
- onboarding shortcuts
- test data assumptions
- app-specific cleanup paths
- product-specific assertions

If a helper mentions your product name or app settings schema, it probably belongs in the app.

## Common Failure Checklist

### App boots but Playwright cannot connect

Check:

- app-side `tauri_plugin_playwright` is enabled in E2E mode
- `TAURI_PLAYWRIGHT_SOCKET` and configured `socketPath` align
- stale previous app process is not holding the socket
- `playwright:default` permission is present in capabilities at build time (use `capabilityFiles` if the permission is feature-gated)

### Ordered runner fails but debug mode works

Check:

- `devServerCommand` and `tauriCommand` are argument arrays for `pnpm`, not full shell strings
- `playwrightConfig` points at the app-local config file
- `specOrder` references files under the configured `specsPrefix`

### First-run flow is nondeterministic

Check:

- `systemDataPaths` is resetting the right app-owned persistence
- you are not deleting heavyweight caches you expect to reuse
- tests wait on explicit ready signals instead of sleeps

## Quick Review Before Shipping

- README quick start matches the actual file layout
- scripts use `pnpm exec tauri-e2e-runner ...` for installed-package usage
- workspace-local development uses the source runner path only when the bin is not linked
- app-local spec files import `test` from `fixtures.ts`
- readiness assertions are explicit
- if the app uses feature-gated plugins, `capabilityFiles` injects their permissions before the Tauri build

## References

- `packages/e2e-harness/README.md`
- `packages/e2e-harness/src/playwright.ts`
- `packages/e2e-harness/src/ordered-tauri-runner.mjs`
- `apps/desktop/tests/e2e/e2e.config.mjs`
