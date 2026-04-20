# E2E Testing Design: Tauri + Playwright

## Overview

Add end-to-end testing for the AriaType desktop app using Playwright connected to Tauri via tauri-driver. Tests verify all 11 pages render correctly with backend data and pass pixel-level visual regression checks.

## Goals

1. Verify each page loads and displays data from backend IPC calls
2. Detect unintended UI changes via screenshot comparison
3. Establish baseline screenshots for visual regression testing
4. Keep tests lightweight and runnable locally

## Non-Goals

- Testing complex workflows (recording, STT, polish pipeline)
- CI integration (local development only for now)
- Cross-platform testing (macOS only)

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│  Test Runner (Playwright)                                    │
│  ├── tests/e2e/pages/*.spec.ts                               │
│  ├── playwright.config.ts                                    │
│  └── baseline screenshots in tests/e2e/baseline/             │
└─────────────────────────────────────────────────────────────┘
                              │
                              │ tauri-driver (WebDriver protocol)
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  Tauri Desktop App (built)                                   │
│  ├── src-tauri/ (Rust backend)                               │
│  └── src/ (React frontend)                                   │
└─────────────────────────────────────────────────────────────┘
```

**Dependencies**:
- `@playwright/test` - test framework
- `tauri-driver` - official Tauri WebDriver proxy (`cargo install tauri-driver`)
- Tauri build artifact (required before running tests)

## Test Coverage

**11 page render tests**:

| Page | Route | Backend IPC Calls |
|------|-------|-------------------|
| Dashboard | `/` | `get_dashboard_stats`, `get_daily_usage` |
| History | `/history` | `get_transcription_history` |
| GeneralSettings | `/settings` | `get_settings` |
| HotkeySettings | `/hotkey` | `get_settings` (shortcut_profiles) |
| ModelSettings | `/private-ai` | `get_models`, `is_model_downloaded` |
| CloudService | `/cloud` | `get_settings`, `get_cloud_provider_schemas` |
| PolishTemplates | `/polish-templates` | `get_polish_templates`, `get_polish_custom_templates` |
| PermissionSettings | `/permission` | `check_permission` |
| LogViewer | `/logs` | `get_log_content` |
| Changelog | `/changelog` | static content |
| About | `/about` | `get_platform` |

## Test Flow (Per Page)

```typescript
test('Dashboard renders with backend data', async ({ page }) => {
  // 1. Navigate and wait for load
  await page.goto('/');
  await page.waitForLoadState('networkidle');
  
  // 2. Wait for backend data to render
  await page.waitForSelector('[data-testid="dashboard-stats"]');
  
  // 3. Verify data presence
  const statsText = await page.textContent('[data-testid="total-count"]');
  expect(statsText).toBeTruthy();
  
  // 4. Screenshot comparison
  await expect(page).toHaveScreenshot('dashboard.png', {
    threshold: 0.1,
  });
});
```

## Directory Structure

```
apps/desktop/tests/e2e/
├── playwright.config.ts        # Playwright config
├── global-setup.ts             # Start tauri-driver + Tauri app
├── global-teardown.ts          # Cleanup processes
├── baseline/                   # Baseline screenshots (gitignored or tracked)
│   ├── dashboard.png
│   ├── history.png
│   └── ...
├── pages/                      # Page tests
│   ├── dashboard.spec.ts
│   ├── history.spec.ts
│   ├── settings.spec.ts
│   ├── hotkey.spec.ts
│   ├── model.spec.ts
│   ├── cloud.spec.ts
│   ├── polish-templates.spec.ts
│   ├── permission.spec.ts
│   ├── logs.spec.ts
│   ├── changelog.spec.ts
│   └── about.spec.ts
└── utils/
    └── navigation.ts           # Navigation helpers
```

## Playwright Configuration

```typescript
// playwright.config.ts
import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './pages',
  globalSetup: '../global-setup.ts',
  globalTeardown: '../global-teardown.ts',
  
  use: {
    connectOptions: {
      webdriverUrl: 'http://127.0.0.1:4444',
    },
    screenshot: 'only-on-failure',
    trace: 'retain-on-failure',
  },
  
  expect: {
    toHaveScreenshot: {
      threshold: 0.1, // 10% tolerance for font rendering differences
    },
  },
  
  timeout: 30000,
});
```

## Global Setup/Teardown

**global-setup.ts**:
1. Start `tauri-driver` on port 4444
2. Launch built Tauri app
3. Wait for app window to be ready
4. Return connection details

**global-teardown.ts**:
1. Kill Tauri app process
2. Kill tauri-driver process

## Run Commands

```bash
# Install dependencies
pnpm --filter @ariatype/desktop add -D @playwright/test

# Install tauri-driver
cargo install tauri-driver

# Build Tauri app (required before tests)
cd apps/desktop && pnpm tauri build

# Run E2E tests
pnpm test:e2e

# Update baselines (after intentional UI changes)
pnpm test:e2e -- --update-snapshots
```

Add to `apps/desktop/package.json`:
```json
{
  "scripts": {
    "test:e2e": "playwright test --config=tests/e2e/playwright.config.ts",
    "test:e2e:update": "playwright test --config=tests/e2e/playwright.config.ts --update-snapshots"
  }
}
```

## Key Decisions

| Decision | Rationale |
|----------|-----------|
| tauri-driver + built app | Unified flow for local development, matches CI pattern if added later |
| 10% screenshot threshold | Allows minor font/anti-aliasing differences, avoids false positives |
| data-testid selectors | Stable selection for waiting on backend data rendering |
| global-setup automation | Reduces manual steps, ensures clean test environment |

## Baseline Management

- First run generates baseline screenshots in `tests/e2e/baseline/`
- Subsequent runs compare against baseline
- When UI intentionally changes, run with `--update-snapshots`
- **Baseline files tracked in git** (enables team-wide visual consistency)
- **Failure artifacts gitignored** (`test-results/`, `playwright-report/` - already in .gitignore)

Note: Baseline filenames use pattern `{page}.png` (e.g., `dashboard.png`) which differs from gitignore pattern `**/*-screenshot.png`, ensuring baselines are tracked while failure diffs are ignored.

## Error Handling

- Test fails if page doesn't render within timeout
- Test fails if screenshot diff exceeds threshold
- On failure: Playwright saves diff images to `test-results/`
- Developer reviews diff to determine if intentional or bug

## Future Extensions

- Add workflow tests (recording → transcription) if needed
- Add CI integration with macOS runner
- Add accessibility tree verification