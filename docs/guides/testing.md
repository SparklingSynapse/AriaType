# Testing Guide

Guide for writing and running tests in AriaType (Tauri v2 + React 19 + Rust).

## When to Read This

- Read [`../../AGENTS.md`](../../AGENTS.md) for execution constraints and the default iteration loop
- Read [`../spec/testing.md`](../spec/testing.md) for mandatory testing policy and coverage gates
- Read this guide for concrete commands, test placement, and day-to-day testing workflow
- Read [`../architecture/data-flow.md`](../architecture/data-flow.md) when test scope depends on pipeline contracts or the primary user workflow

---

## Table of Contents

1. [Running Tests](#running-tests)
2. [Test Architecture](#test-architecture)
3. [Rust Backend Testing](#rust-backend-testing)
4. [React Frontend Testing](#react-frontend-testing)
5. [TDD Workflow](#tdd-workflow)
6. [Coverage Requirements](#coverage-requirements)
7. [Contract Testing](#contract-testing)
8. [Adding New Tests](#adding-new-tests)

---

## Running Tests

### Quick Commands

```bash
# === Rust Backend ===
cd apps/desktop/src-tauri
cargo test                                    # Run all unit tests
cargo test --test pipeline_integration_test   # Run specific test file
cargo test -- --ignored                       # Run ignored tests (requires models)
cargo llvm-cov --html                         # Generate HTML coverage report

# === Frontend ===
pnpm --filter @ariatype/desktop test          # Run all tests
pnpm --filter @ariatype/desktop test:watch    # Watch mode
pnpm --filter @ariatype/desktop test:coverage # With coverage

# === Linting ===
cargo clippy --all-features -- -D warnings    # Lint (warnings are errors)
cargo fmt -- --check                          # Format check
```

### Full Verification

```bash
cd apps/desktop/src-tauri && cargo test && cargo clippy --all-features -- -D warnings && cargo fmt -- --check
pnpm --filter @ariatype/desktop build && pnpm --filter @ariatype/shared typecheck && pnpm check:i18n
```

---

## Test Architecture

| Layer | Purpose | Location |
|-------|---------|----------|
| **Unit** | Pure logic, parsing, error branches | `src/**/__test__/*.rs` (Rust), `src/**/*.{test,spec}.{ts,tsx}` (Frontend) |
| **Integration** | Module boundaries, IPC, state transitions | `apps/desktop/src-tauri/tests/` |
| **E2E/Pipeline** | Real workflows: audio → STT → polish → output | `tests/pipeline_integration_test.rs` |

### Test Locations

| Purpose | Location |
|---------|----------|
| Cloud STT API contract | `tests/cloud_provider_api_test.rs` |
| Cloud STT integration | `tests/cloud_stt_test.rs` |
| Streaming client tests | `tests/volcengine_streaming_test.rs` |
| Pipeline integration | `tests/pipeline_integration_test.rs` |
| Shared utilities | `tests/common/mod.rs` (mocks, fixtures) |

---

## Rust Backend Testing

### Unit Tests

Place in `__test__/` directories alongside modules:

```rust
// src/commands/settings/__test__/settings_test.rs
use crate::commands::settings::parse_hotkey;

#[test]
fn test_parse_hotkey_valid_shift_space() {
    let (modifiers, _code) = parse_hotkey("shift+space").unwrap();
    assert!(modifiers.unwrap().contains(Modifiers::SHIFT));
}

#[test]
fn test_parse_hotkey_rejects_modifier_only_keys() {
    for hotkey in ["cmd", "ctrl", "shift", "alt"] {
        assert!(parse_hotkey(hotkey).unwrap_err().contains("not supported"));
    }
}
```

### Integration Tests

Use the library crate in `tests/`:

```rust
// tests/cloud_provider_api_test.rs
use ariatype_lib::stt_engine::traits::{SttEngine, TranscriptionRequest};

#[tokio::test]
async fn test_cloud_stt_batch_mode_rejected() {
    let engine = CloudSttEngine::new().unwrap();
    let result = engine.transcribe(request).await;
    assert!(result.unwrap_err().contains("streaming lifecycle"));
}
```

### Test Utilities

Import from `tests/common/`:

```rust
use common::{create_test_wav, write_temp_wav, cleanup_temp_files};

#[test]
fn test_audio() {
    let wav_data = create_test_wav(16000, 1, 1.0);  // 16kHz mono, 1s
    let temp_path = write_temp_wav(&wav_data);
    // ... test logic ...
    cleanup_temp_files(&[temp_path]);
}
```

### Ignored Tests

Mark tests requiring external resources:

```rust
#[test]
#[ignore = "Requires Whisper model to be downloaded"]
fn test_whisper_real_transcription() { /* ... */ }

// Run with: cargo test -- --ignored
```

---

## React Frontend Testing

### Configuration

```typescript
// vitest.config.ts
export default defineConfig({
  test: {
    environment: 'jsdom',
    globals: true,
    setupFiles: ['./src/test/setup.ts'],
    include: ['src/**/*.{test,spec}.{ts,tsx}'],
  },
});
```

### Component Tests

```typescript
// src/components/Home/__tests__/Dashboard.test.tsx
import { render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { historyCommands } from "@/lib/tauri";

vi.mock("@/lib/tauri", async () => ({
  historyCommands: {
    getDashboardStats: vi.fn(),
  },
}));

describe("Dashboard", () => {
  beforeEach(() => vi.clearAllMocks());

  it("renders dashboard with data", async () => {
    vi.mocked(historyCommands.getDashboardStats).mockResolvedValue({ total_count: 14 });
    render(<Dashboard />);
    await waitFor(() => expect(screen.getByText("14")).toBeInTheDocument());
  });
});
```

### Utility Tests

```typescript
// src/components/ui/__test__/hotkey-input.test.ts
import { describe, expect, it } from "vitest";
import { validateHotkeyString } from "../hotkey-input";

describe("validateHotkeyString", () => {
  it.each(["a", "ctrl+a", "ctrl+shift+a"])("accepts %s", (hotkey) => {
    expect(validateHotkeyString(hotkey).valid).toBe(true);
  });
});
```

---

## TDD Workflow

```
1. Write failing test (capture behavior or reproduce bug)
2. Confirm test fails without implementation
3. Implement smallest correct change
4. Refactor while keeping tests green
5. Run full verification set
```

### BDD Model

```
Given: initial state, permissions, config
When:  user action, command, event
Then:  observable UI state, events, output
```

---

## Coverage Requirements

| Type | Requirement |
|------|-------------|
| **E2E** | 100% for affected user workflow |
| **Unit** | 100% for affected critical core modules |

**Must cover**: Happy path, error paths, edge cases, regression scenarios.

```bash
# Rust coverage
cd apps/desktop/src-tauri && cargo llvm-cov --html

# Frontend coverage
pnpm --filter @ariatype/desktop test -- --coverage
```

### No-Fabrication Policy

- Do NOT use mocks to simulate product completion
- Use test doubles ONLY for external dependencies
- Report ONLY coverage from executed tooling

---

## Contract Testing

For cloud STT/Polish engines, verify API contracts using auth errors with mock credentials.

### Pattern

```rust
mod mock_credentials {
    pub const API_KEY: &str = "mock_api_key";
}

#[tokio::test]
async fn test_cloud_stt_schema() {
    let config = CloudSttConfig {
        api_key: mock_credentials::API_KEY.to_string(),
        // ...
    };

    let result = engine.transcribe(request).await;
    assert!(result.is_err());
    let err = result.unwrap_err();

    // Auth error proves endpoint/headers/body are correct
    assert!(err.contains("403") || err.contains("Forbidden"));
    // NOT a parameter error
    assert!(!err.contains("400"));
}
```

### Error Interpretation

| Error | Proves |
|-------|--------|
| 401/403 | Endpoint, headers, body correctly formed |
| 400 | Request body/headers malformed |
| 404 | Endpoint URL incorrect |

---

## Adding New Tests

1. **Identify type**: unit, integration, or contract
2. **Choose location**:
   - Rust unit: `src/**/__test__/*.rs`
   - Rust integration: `tests/*.rs`
   - Frontend: `src/**/__tests__/*.test.tsx`
3. **Write test**: Cover happy path, errors, edges
4. **Run test**: Verify it fails first
5. **Implement**: Smallest correct change
6. **Verify**: Run full suite

### Naming Conventions

| Type | Pattern |
|------|---------|
| Rust unit | `<module>_test.rs` in `__test__/` |
| Rust integration | `<feature>_test.rs` in `tests/` |
| React component | `<Component>.test.tsx` in `__tests__/` |
| Frontend utility | `<module>.test.ts` in `__test__/` |

---

## CI Integration

Tests run on push to `main`/`develop` and on PRs.

```yaml
# .github/workflows/test.yml
cargo llvm-cov --all-features --workspace --lcov --output-path lcov.info
cargo clippy --all-features -- -D warnings
cargo fmt -- --check
```

Coverage uploads to Codecov on macOS CI.

---

## See Also

- [Testing Specification](../spec/testing.md) — Test pyramid and coverage gates
- [Engine API Contract](../spec/engine-api-contract.md) — STT/Polish contract testing
- [Debugging Guide](debugging.md) — Log investigation
- [Desktop CONTRIBUTING](../../apps/desktop/CONTRIBUTING.md) — Development setup
