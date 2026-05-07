# @ariatype/e2e-harness

Reusable E2E harness for real Tauri app verification.

It gives you three things:

- a shared Tauri Playwright fixture
- an ordered runner that boots your dev server and one reusable Tauri runtime
- snapshot helpers that stabilize the app before comparing screenshots

## 30-Second Mental Model

If you only remember one thing, remember this:

- `e2e.config.mjs` is the only app-specific contract
- `fixtures.ts` turns that config into a shared Tauri Playwright test fixture
- `playwright.config.ts` turns that config into a Playwright runtime
- `tauri-e2e-runner` is the normal entrypoint for local runs and CI

In practice, adopting the harness means copying 3 small files, filling in your app-specific commands, then writing specs.

## Before You Start

This harness assumes your app already has these pieces:

- a Tauri app that can boot locally from `pnpm ...`
- a frontend dev server that can boot locally from `pnpm ...`
- `@srsholmes/tauri-playwright` wired into the Tauri app for E2E mode
- a dedicated E2E feature flag or equivalent app-side switch such as `e2e-testing`

For example, the desktop app enables the plugin only behind the Rust feature gate:

```rust
#[cfg(feature = "e2e-testing")]
let builder = builder.plugin(tauri_plugin_playwright::init_with_config(
    tauri_plugin_playwright::PluginConfig::new()
        .socket_path(playwright_socket),
));
```

If your Tauri app is not already exposing the Playwright socket in E2E mode, this package cannot bootstrap that for you.

## Quick Start

If you want the shortest path, create these files:

- `tests/e2e/e2e.config.mjs`
- `tests/e2e/fixtures.ts`
- `tests/e2e/playwright.config.ts`
- `tests/e2e/pages/*.spec.ts`

Then add one script that runs the harness runner.

### 1. Install dependencies

```sh
pnpm add -D @ariatype/e2e-harness @playwright/test @srsholmes/tauri-playwright
```

Your app also needs a working Tauri dev command, for example `pnpm tauri dev`.

Important command shape:

- `tauriCommand` is an argument array appended after `pnpm`
- `devServerCommand` is an argument array appended after `pnpm`

So this:

```js
devServerCommand: ['exec', 'vite', '--port', '1423', '--strictPort']
```

becomes this at runtime:

```sh
pnpm exec vite --port 1423 --strictPort
```

### 2. Create `tests/e2e/e2e.config.mjs`

This is the single source of truth for your E2E runtime.

```js
import { join } from 'node:path';
import {
  createRunnerConfig,
  resolveHarnessDir,
} from '@ariatype/e2e-harness/runner';

const e2eDir = resolveHarnessDir(import.meta.url);
const projectRoot = join(e2eDir, '..', '..');
const runtimeKey = 'ordered-shared';

export default createRunnerConfig({
  projectRoot,
  pagesDir: 'tests/e2e/pages',
  specsPrefix: 'tests/e2e/pages',
  playwrightConfig: 'tests/e2e/playwright.config.ts',
  specOrder: ['journey.spec.ts'],
  runtimeRoot: join(projectRoot, `tests/e2e/.runtime/${runtimeKey}`),
  socketPath: `/tmp/tauri-e2e-${runtimeKey}.sock`,
  killCommand: 'pkill -f "target/debug/your-app-name"',
  tauriCommand: [
    'tauri',
    'dev',
    '--config',
    'src-tauri/tauri.dev.conf.json',
    '--config',
    'src-tauri/tauri.e2e.conf.json',
  ],
  tauriFeatures: ['e2e-testing'],
  startTimeoutSeconds: 180,
  socketWaitMs: 5000,
  snapshotStabilizationMs: 1000,
  devServerCommand: ['exec', 'vite', '--port', '1423', '--strictPort'],
  devServerUrl: 'http://localhost:1423',
  devServerReadyTimeoutMs: 30000,
});
```

The easiest safe defaults are:

- `runtimeRoot`: `tests/e2e/.runtime/<runtime-key>` under your app root
- `socketPath`: `/tmp/tauri-e2e-<runtime-key>.sock`
- `specOrder`: put your longest stateful journey first

### 3. Create `tests/e2e/fixtures.ts`

```ts
import { createTauriFixturesFromConfigModule } from '@ariatype/e2e-harness/playwright';

const { test, expect } = await createTauriFixturesFromConfigModule(
  './e2e.config.mjs',
  { sharedRuntimeKey: 'shared' },
  new URL('.', import.meta.url).pathname,
);

export { test, expect };
```

This fixture automatically manages one shared Tauri runtime and exposes `tauriPage` to each test.

### 4. Create `tests/e2e/playwright.config.ts`

```ts
import { createTauriPlaywrightConfig } from '@ariatype/e2e-harness/playwright';
import config from './e2e.config.mjs';

const snapshotDir = new URL('./snapshots/', import.meta.url).pathname;

export default createTauriPlaywrightConfig(config, { snapshotDir });
```

Because this config file lives under `tests/e2e/`, the harness default `testDir: './pages'` resolves to `tests/e2e/pages`.

### 5. Write your first spec

```ts
import { waitForContentLoaded } from '@ariatype/e2e-harness/helpers';
import { test } from '../fixtures';

test('app boots', async ({ tauriPage }) => {
  await waitForContentLoaded(tauriPage, 'body');
});
```

By default, the fixture also captures a final end-of-test snapshot for passing tests. If a test should not assert a final screenshot, use `disableAutoSnapshot(testInfo)`.

### 6. Add package scripts

If `@ariatype/e2e-harness` is installed as a normal dependency, use the published bin:

```json
{
  "scripts": {
    "test:e2e": "pnpm exec tauri-e2e-runner tests/e2e/e2e.config.mjs",
    "test:e2e:update": "pnpm exec tauri-e2e-runner tests/e2e/e2e.config.mjs --update-snapshots",
    "test:e2e:debug": "playwright test --config=tests/e2e/playwright.config.ts --debug"
  }
}
```

If you are developing the harness itself inside the same monorepo and the bin is not linked yet, call the source entrypoint directly:

```json
{
  "scripts": {
    "test:e2e": "node ../../packages/e2e-harness/src/runner-cli.mjs tests/e2e/e2e.config.mjs",
    "test:e2e:update": "node ../../packages/e2e-harness/src/runner-cli.mjs tests/e2e/e2e.config.mjs --update-snapshots"
  }
}
```

## Mental Model

The easiest way to use the harness is to treat `tests/e2e/e2e.config.mjs` as the only app-specific contract.

- the runner reads it to start your dev server and shared Tauri runtime
- the Playwright config reads it to configure screenshots and startup behavior
- the fixture reads it to connect tests to the same runtime shape

That keeps app-specific details in one place:

- your Tauri dev command
- your socket path
- your runtime directory
- your app cleanup command
- your desired spec order

## Which Command To Use

Use the ordered runner for normal CI and local E2E runs.

```sh
pnpm run test:e2e
```

Use the ordered runner when updating snapshots too.

```sh
pnpm run test:e2e:update
```

Use raw Playwright only when you want interactive debugging.

```sh
pnpm run test:e2e:debug
```

Why: the runner starts one shared external runtime, then executes spec batches in order. That avoids repeated cold starts and keeps stateful desktop journeys more stable.

## Required Config Fields

These fields are the minimum you should understand:

| Field | Why it exists |
| --- | --- |
| `projectRoot` | Working directory for Playwright, Vite, and Tauri commands |
| `pagesDir` | Directory used to discover spec files |
| `specsPrefix` | Prefix used when passing spec paths to Playwright |
| `playwrightConfig` | Playwright config path used by the runner |
| `specOrder` | Explicit first batches to run before the rest |
| `runtimeRoot` | Isolated XDG runtime directory for the app under test |
| `socketPath` | Tauri Playwright plugin socket path |
| `tauriCommand` | Command array passed after `pnpm` to boot Tauri |
| `devServerCommand` | Command array passed after `pnpm` to boot the frontend dev server |
| `devServerUrl` | URL the runner probes until the dev server is ready |

Useful optional fields:

| Field | When to use it |
| --- | --- |
| `killCommand` | Your app leaves behind a previous process or single-instance lock |
| `systemDataPaths` | You need to delete app-owned local state between cold starts |
| `tauriFeatures` | Your Tauri E2E build needs feature flags |
| `capabilityFiles` | You need to inject feature-gated capability files before the Tauri build |
| `seedFiles` | You need to seed files into the XDG data directory before the app starts |
| `seedDataFiles` | You need to seed files into the app's real data directory before the app starts |
| `startTimeoutSeconds` | Your Rust app takes longer than the default to boot |
| `socketWaitMs` | Socket creation is slower on your machine or CI |
| `snapshotStabilizationMs` | Your UI animations need a bit more settle time before screenshots |
| `devServerPrepareCommand` | You need to prewarm or clean Vite before starting |
| `devServerResetPaths` | You want to delete dev-server caches before startup |
| `devServerReadyTimeoutMs` | Your frontend server takes longer to come up |

## How To Choose The App-Specific Values

### `killCommand`

Use a command that kills the dev app binary from previous runs.

Example:

```sh
pkill -f "target/debug/your-app-name"
```

Use it when your app keeps a single-instance lock, lingering socket, or background process.

### `systemDataPaths`

Put only app-owned state that must be reset for deterministic first-run behavior.

Typical candidates:

- app settings files
- local history databases
- app-owned WebKit persistence
- temporary recordings created by the app

Do not delete heavyweight caches that you intentionally want to reuse between runs, such as downloaded models.

### `capabilityFiles`

Use this when your Tauri app has optional plugins gated behind Cargo features, and those plugins define permissions that must exist in `src-tauri/capabilities/` at build time.

The problem: Tauri validates all capability files at build time. If a capability references a permission from a plugin that isn't compiled, the build fails. This means you can't keep feature-gated permissions in the shared `capabilities/` directory — they break regular dev builds.

The solution: store the E2E capability file outside the capabilities directory (e.g., `tests/e2e/capabilities/e2e.json`) and let the runner copy it in before the Tauri build starts, then clean it up after.

```js
// In e2e.config.mjs
export const capabilityFiles = [
  {
    src: join(e2eDir, 'capabilities', 'e2e.json'),
    dest: join(projectRoot, 'src-tauri', 'capabilities', 'e2e.json'),
  },
];
```

The source file (`tests/e2e/capabilities/e2e.json`) should contain only the permissions needed for E2E:

```json
{
  "$schema": "https://schema.tauri.app/capabilities/2",
  "identifier": "e2e",
  "description": "E2E testing capability (playwright plugin)",
  "local": true,
  "windows": ["main", "pill", "toast"],
  "permissions": [
    "playwright:default"
  ]
}
```

Add `capabilityFiles` to your runner config alongside `tauriFeatures`:

```js
export default createRunnerConfig({
  // ... other fields
  tauriFeatures: ['e2e-testing'],
  capabilityFiles: [
    {
      src: join(e2eDir, 'capabilities', 'e2e.json'),
      dest: join(projectRoot, 'src-tauri', 'capabilities', 'e2e.json'),
    },
  ],
});
```

The runner copies each file before `TauriProcessManager.start()` and removes them in cleanup, so the capabilities directory stays clean for regular dev builds.

### `seedFiles`

Use this to seed files into the XDG data directory (`$XDG_DATA_HOME/`) before the app starts. The runner copies each file to `runtimeRoot/xdg-data/{dest}`.

```js
export default createRunnerConfig({
  // ... other fields
  seedFiles: [
    {
      src: join(e2eDir, 'fixtures', 'settings.json'),
      dest: 'myapp/settings.json',  // relative to $XDG_DATA_HOME
    },
  ],
});
```

This is useful when your app reads config from a known subdirectory under the XDG data path. The `dest` is relative to `$XDG_DATA_HOME` (which the runner sets to `runtimeRoot/xdg-data`).

### `seedDataFiles`

Use this to seed files into the app's **real** data directory (e.g., `~/Library/Application Support/{app}/` on macOS) before the app starts. Unlike `seedFiles`, the `dest` here is an absolute path.

```js
const userHome = process.env.HOME ?? '/Users/you';

export default createRunnerConfig({
  // ... other fields
  seedDataFiles: [
    {
      src: join(e2eDir, 'fixtures', 'settings-cloud-enabled.json'),
      dest: join(userHome, 'Library', 'Application Support', 'myapp', 'settings.json'),
    },
  ],
});
```

Use `seedDataFiles` when your app uses platform-native data directories (via `dirs::data_dir()` or similar) instead of XDG paths. Pair it with `systemDataPaths` to clean up the seeded files after each run.

| Scenario | Use |
|----------|-----|
| App reads from `$XDG_DATA_HOME/...` | `seedFiles` |
| App reads from `~/Library/Application Support/...` or `~/.local/share/...` | `seedDataFiles` |

### `runtimeRoot`

Use a test-only directory inside the app repo, for example:

```js
runtimeRoot: join(projectRoot, 'tests/e2e/.runtime/ordered-shared')
```

The harness uses it as isolated XDG storage for the runtime under test.

### `socketPath`

Use a unique socket per app and runtime key.

Good:

```js
socketPath: `/tmp/tauri-e2e-${runtimeKey}.sock`
```

Avoid reusing a generic path across different apps.

### `specOrder`

Put the most stateful, broadest black-box journey first. Let smaller specs run after it.

## App-Specific Code vs Shared Harness Code

Keep these in your app:

- `tests/e2e/e2e.config.mjs`
- app-specific fixture helpers that call your IPC layer
- specs and snapshots

Keep these in the shared harness:

- Tauri runtime bootstrapping
- Playwright fixture creation
- runner CLI
- snapshot stabilization
- generic navigation and wait helpers

If a helper mentions your product, settings schema, onboarding state, or custom IPC commands, it probably belongs in the app, not in `@ariatype/e2e-harness`.

## Waiting Strategy

Do not add hard sleeps for business readiness.

Prefer assertions or condition waits tied to real state, for example:

```ts
await expect(modalReadyIcon).toBeVisible({ timeout: 10000 });
```

The harness only intentionally waits during screenshot stabilization, where a short settle period is useful to avoid flaky image diffs.

## Common Helpers

From `@ariatype/e2e-harness/helpers`:

- `waitForAppReady(page)`
- `waitForContentLoaded(page, selector)`
- `openRoute(page, route)`
- `remountRoute(page, route)`
- `navigateViaSidebar(page, label)`
- `dismissOnboardingIfPresent(page)`
- `invokeTauri(page, command, args)`
- `expectNativeScreenshot(page, name)`
- `disableAutoSnapshot(testInfo)`

## Local Verification

To verify the harness package itself:

```sh
pnpm --filter @ariatype/e2e-harness test
```

To verify a consumer app using the harness:

```sh
pnpm run test:e2e
pnpm run test:e2e:update
pnpm run test:e2e:debug
```

## Troubleshooting

### `Command "tauri-e2e-runner" not found`

Use one of these fixes:

- run it through `pnpm exec tauri-e2e-runner ...`
- make sure `@ariatype/e2e-harness` is installed in the current package
- if you are editing the harness inside the same workspace, call `node ../../packages/e2e-harness/src/runner-cli.mjs ...`

### The app starts, but Playwright cannot connect

Check these first:

- `socketPath` matches across the runner and fixture
- your Tauri app is built with the expected E2E feature flag
- `killCommand` is strong enough to clear old single-instance processes

### The first run is flaky

Usually the fix is not a longer sleep. Prefer:

- waiting for a concrete UI-ready signal
- cleaning the correct `systemDataPaths`
- keeping heavyweight assets outside reset paths if you want warm-cache runs

## API Surface

Main exports:

- `@ariatype/e2e-harness`
- `@ariatype/e2e-harness/helpers`
- `@ariatype/e2e-harness/playwright`
- `@ariatype/e2e-harness/playwright-hooks`
- `@ariatype/e2e-harness/runner`
- `@ariatype/e2e-harness/snapshot`
