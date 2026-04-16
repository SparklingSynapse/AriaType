# E2E Harness Implementation Roadmap

## Executive Summary

This document provides the complete implementation roadmap for E2E testing and Harness Engineering in the AriaType Tauri application.

### Key Decisions

| Decision | Rationale |
|----------|-----------|
| Use multi-layer harness approach | No single solution covers all verification needs |
| Contract harness as primary backend verification | Works on all platforms, fast feedback |
| Mock harness for frontend unit testing | Isolated, deterministic, fast |
| Playwright MCP for macOS E2E | WebDriver doesn't support macOS |
| WebDriver for Linux/Windows CI | Mature, well-supported |
| Structured JSON output | Agent-readable results |

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        E2E HARNESS ARCHITECTURE                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│                         ┌─────────────────────┐                              │
│                         │   Verification CLI   │                              │
│                         │  (pnpm run verify)   │                              │
│                         └──────────┬──────────┘                              │
│                                    │                                         │
│         ┌──────────────────────────┼──────────────────────────┐             │
│         │                          │                          │             │
│         ▼                          ▼                          ▼             │
│  ┌─────────────┐           ┌─────────────┐           ┌─────────────┐        │
│  │   Contract  │           │    Mock     │           │     E2E     │        │
│  │   Harness   │           │   Harness   │           │   Harness   │        │
│  │  (Rust)     │           │ (TypeScript)│           │ (Playwright/│        │
│  │             │           │             │           │  WebDriver) │        │
│  └─────────────┘           └─────────────┘           └─────────────┘        │
│         │                          │                          │             │
│         │                          │                          │             │
│         ▼                          ▼                          ▼             │
│  ┌─────────────┐           ┌─────────────┐           ┌─────────────┐        │
│  │   Backend   │           │   Frontend  │           │  Full Stack │        │
│  │   Logic     │           │   State     │           │  Integration│        │
│  │   Only      │           │   Only      │           │             │        │
│  └─────────────┘           └─────────────┘           └─────────────┘        │
│         │                          │                          │             │
│         └──────────────────────────┼──────────────────────────┘             │
│                                    │                                         │
│                                    ▼                                         │
│                         ┌─────────────────────┐                              │
│                         │   Decision Engine   │                              │
│                         │   + Evidence Merge  │                              │
│                         └──────────┬──────────┘                              │
│                                    │                                         │
│                                    ▼                                         │
│                         ┌─────────────────────┐                              │
│                         │ VerificationResult  │                              │
│                         │     (JSON)          │                              │
│                         │                     │                              │
│                         │ • passed: bool      │                              │
│                         │ • confidence: level │                              │
│                         │ • reasoning: string │                              │
│                         │ • agentSummary      │                              │
│                         └─────────────────────┘                              │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Implementation Phases

### Phase 1: Foundation (Week 1)

**Goals**: Establish harness infrastructure

**Tasks**:

| Task | Files | Priority |
|------|-------|----------|
| Create harness directory structure | `apps/desktop/src-tauri/tests/ipc_harness/` | P0 |
| Define HarnessResult schema | `tests/ipc_harness/result.rs` | P0 |
| Implement ContractHarness framework | `tests/ipc_harness/mod.rs` | P0 |
| Create command handlers | `tests/ipc_harness/commands.rs` | P0 |
| Create event tracker | `tests/ipc_harness/events.rs` | P1 |

**Deliverables**:
- Working ContractHarness that executes commands and tracks events
- JSON output format defined
- Basic test case for recording flow

**Commands**:

```bash
# Create directory structure
mkdir -p apps/desktop/src-tauri/tests/ipc_harness

# Add to Cargo.toml
# [dependencies]
# serde_json = "1.0"
# uuid = { version = "1.0", features = ["v4", "serde"] }

# Run first test
cargo test --test ipc_contract_test -- --nocapture
```

---

### Phase 2: Backend Coverage (Week 2)

**Goals**: Complete backend verification harness

**Tasks**:

| Task | Files | Priority |
|------|-------|----------|
| Implement all command handlers | `commands.rs` | P0 |
| Add state machine verification | `mod.rs` | P0 |
| Create assertion system | `result.rs` | P0 |
| Write contract tests for core flows | `tests/ipc_contract_test.rs` | P0 |
| Add to CI workflow | `.github/workflows/test.yml` | P1 |

**Test Coverage**:

| Flow | Commands | Events | State Transitions |
|------|----------|--------|-------------------|
| Recording cycle | `start_recording`, `stop_recording` | `recording-state-changed`, `transcription-complete` | Idle→Recording→Processing→Idle |
| Settings update | `update_settings` | `settings-changed` | - |
| History retrieval | `get_history` | - | - |
| Model download | `download_model` | `model-download-progress` | - |

**Deliverables**:
- Full backend contract test suite
- CI integration for contract tests
- Structured JSON output working

---

### Phase 3: Frontend Mock Harness (Week 3)

**Goals**: Complete frontend verification harness

**Tasks**:

| Task | Files | Priority |
|------|-------|----------|
| Create MockHarness class | `src/test/harness/mock-backend.ts` | P0 |
| Implement IPC tracer | `src/test/harness/ipc-tracer.ts` | P0 |
| Implement state snapshot | `src/test/harness/state-snapshot.ts` | P0 |
| Implement visual collector | `src/test/harness/visual-collector.ts` | P1 |
| Write mock tests | `src/test/contract/*.test.ts` | P0 |

**Deliverables**:
- Frontend mock harness with IPC tracing
- State snapshot system
- Tests for all frontend components that trigger IPC

---

### Phase 4: Decision Engine (Week 4)

**Goals**: Complete agent-readable verification system

**Tasks**:

| Task | Files | Priority |
|------|-------|----------|
| Implement DecisionEngine | `src/test/harness/decision-engine.ts` | P0 |
| Create verification CLI | `scripts/verify.ts` | P0 |
| Define verification request schema | `src/test/harness/types.ts` | P0 |
| Create verification patterns library | `e2e-tests/patterns/*.yaml` | P1 |
| Update AGENTS.md | `AGENTS.md` | P1 |

**Deliverables**:
- CLI that produces VerificationResult JSON
- Decision engine with reasoning
- Agent integration documentation

---

### Phase 5: E2E Integration (Week 5-6)

**Goals**: Complete full-stack verification

**Tasks**:

| Task | Files | Priority |
|------|-------|----------|
| Install Playwright MCP | `package.json` | P0 |
| Create E2E session templates | `e2e-tests/playwright/templates/` | P0 |
| Implement E2E runner | `e2e-tests/playwright/runner.ts` | P0 |
| Setup WebDriver for Linux | `e2e-tests/webdriver/` | P1 |
| Add macOS E2E CI workflow | `.github/workflows/e2e-macos.yml` | P1 |
| Add Linux WebDriver CI workflow | `.github/workflows/e2e-linux.yml` | P1 |

**Deliverables**:
- Playwright MCP E2E tests for macOS
- WebDriver tests for Linux CI
- Full E2E CI pipeline

---

## File Structure

```
apps/desktop/
│
├── src-tauri/
│   ├── tests/
│   │   ├── ipc_harness/                 # NEW
│   │   │   ├── mod.rs                   # ContractHarness
│   │   │   ├── commands.rs              # Command simulators
│   │   │   ├── events.rs                # Event tracker
│   │   │   ├── result.rs                # HarnessResult schema
│   │   │   └─────────────────────────
│   │   │   └── assertions.rs            # Assertion helpers
│   │   │
│   │   ├── ipc_contract_test.rs         # NEW
│   │   │
│   │   ├── common/
│   │   │   ├── mock_stt.rs              # Existing
│   │   │   ├── mock_polish.rs           # Existing
│   │   │   ├── audio_fixtures.rs        # Existing
│   │   │   └─────────────────────────
│   │   │   ├── harness_helpers.rs       # NEW
│   │   │
│   │   └── *_test.rs                    # Existing tests
│   │
│   └── Cargo.toml                       # Add harness deps
│
├── src/
│   ├── test/
│   │   ├── harness/                     # NEW
│   │   │   ├── mock-backend.ts          # MockHarness
│   │   │   ├── ipc-tracer.ts            # IPC trace collector
│   │   │   ├── state-snapshot.ts        # State collector
│   │   │   ├── visual-collector.ts      # Screenshot/DOM collector
│   │   │   ├── decision-engine.ts       # Verification logic
│   │   │   ├── types.ts                 # All interfaces
│   │   │   └─────────────────────────
│   │   │   └ index.ts                   # Public exports
│   │   │
│   │   ├── contract/                    # NEW
│   │   │   ├── recording.test.ts
│   │   │   ├── settings.test.ts
│   │   │   └─────────────────────────
│   │   │   └ history.test.ts
│   │   │
│   │   └───── Existing test files ──────
│   │
│   └─────────────────────────
│   └── vitest.config.ts                 # Update for harness
│
├── e2e-tests/                           # NEW
│   ├── playwright/
│   │   ├── templates/
│   │   │   ├── recording-flow.yml
│   │   │   ├── settings-change.yml
│   │   │   └─────────────────────────
│   │   │   └ history-retry.yml
│   │   │
│   │   ├── runner.ts                    # Session runner
│   │   ├── config.ts                    # Playwright MCP config
│   │   └─────────────────────────
│   │   └ helpers.ts
│   │
│   ├── webdriver/                       # NEW (Linux/Windows)
│   │   ├── wdio.conf.ts
│   │   ├── test/
│   │   │   ├── recording.spec.ts
│   │   │   └─────────────────────────
│   │   │   └ ipc-tracker.spec.ts
│   │   │
│   │   └─────────────────────────
│   │   └── ipc-inject.ts                # IPC tracking injection
│   │
│   └─────────────────────────
│   └── patterns/                        # NEW
│       ├── frontend-to-backend.yaml     # Pattern library
│       ├── backend-to-frontend.yaml
│       └─────────────────────────
│       └ state-transitions.yaml
│
├── scripts/
│   └─────────────────────────
│   └── verify.ts                        # NEW: CLI
│
├── .verification-results/               # NEW (gitignored)
│   └───────────────────────────────────────
│   └── v-*.json                         # Output files
│
├── package.json                         # Add scripts
│
└─────────────────────────
└── AGENTS.md                            # Update with verification
```

---

## Dependencies

### Rust (Backend)

```toml
# apps/desktop/src-tauri/Cargo.toml

[dev-dependencies]
# Harness dependencies
serde_json = "1.0"
uuid = { version = "1.0", features = ["v4", "serde"] }
tokio-test = "0.4"
```

### TypeScript (Frontend/E2E)

```json
// apps/desktop/package.json

{
  "scripts": {
    "test:harness:contract": "cargo test --test ipc_contract_test -- --format json",
    "test:harness:mock": "vitest run src/test/contract",
    "test:harness:e2e": "node e2e-tests/playwright/runner.ts",
    "test:harness:webdriver": "wdio run e2e-tests/webdriver/wdio.conf.ts",
    "verify": "node scripts/verify.ts"
  },
  
  "devDependencies": {
    // Playwright MCP
    "@anthropic/mcp-client": "^0.1.0",
    
    // WebDriver
    "@wdio/cli": "^9.0.0",
    "@wdio/local-runner": "^9.0.0",
    "@wdio/mocha-framework": "^9.0.0",
    "@wdio/spec-reporter": "^9.0.0"
  }
}
```

---

## CI Workflows

### Contract Tests (All Platforms)

```yaml
# .github/workflows/ipc-contract.yml
name: IPC Contract Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    
    steps:
      - uses: actions/checkout@v4
      
      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable
      
      - name: Run contract tests
        run: |
          cd apps/desktop/src-tauri
          cargo test --test ipc_contract_test -- --format json > results.json
          
      - name: Upload results
        uses: actions/upload-artifact@v4
        with:
          name: contract-results-${{ matrix.os }}
          path: apps/desktop/src-tauri/results.json
```

### E2E macOS (Playwright MCP)

```yaml
# .github/workflows/e2e-macos.yml
name: E2E Tests (macOS)

on: [push, pull_request]

jobs:
  playwright:
    runs-on: macos-latest
    
    steps:
      - uses: actions/checkout@v4
      
      - name: Setup Node
        uses: actions/setup-node@v4
        with:
          node-version: 22
      
      - name: Install dependencies
        run: pnpm install
      
      - name: Build app
        run: pnpm tauri build --debug
      
      - name: Run E2E tests
        run: pnpm run test:harness:e2e
      
      - name: Upload results
        uses: actions/upload-artifact@v4
        with:
          name: e2e-macos-results
          path: .verification-results/
```

### E2E Linux (WebDriver)

```yaml
# .github/workflows/e2e-linux.yml
name: E2E Tests (Linux)

on: [push, pull_request]

jobs:
  webdriver:
    runs-on: ubuntu-latest
    
    steps:
      - uses: actions/checkout@v4
      
      - name: Install system dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y \
            libwebkit2gtk-4.1-dev \
            webkit2gtk-driver \
            xvfb
      
      - name: Install tauri-driver
        run: cargo install tauri-driver --locked
      
      - name: Setup Node
        uses: actions/setup-node@v4
        with:
          node-version: 22
      
      - name: Install dependencies
        run: pnpm install
      
      - name: Build app
        run: pnpm tauri build --debug
      
      - name: Run WebDriver tests
        run: xvfb-run pnpm run test:harness:webdriver
      
      - name: Upload results
        uses: actions/upload-artifact@v4
        with:
          name: e2e-linux-results
          path: .verification-results/
```

---

## Success Criteria

### Phase 1 Complete

- [ ] ContractHarness executes commands and produces JSON output
- [ ] Basic recording flow test passes
- [ ] Schema defined and documented

### Phase 2 Complete

- [ ] All backend commands have contract tests
- [ ] CI runs contract tests on push
- [ ] Evidence recording works (IPC calls, events, state)

### Phase 3 Complete

- [ ] MockHarness works in Vitest
- [ ] IPC tracer captures frontend→backend calls
- [ ] State snapshot captures UI state

### Phase 4 Complete

- [ ] DecisionEngine produces structured conclusions
- [ ] CLI produces VerificationResult JSON
- [ ] Agent can run `pnpm run verify --request '...'`

### Phase 5 Complete

- [ ] Playwright MCP E2E tests pass on macOS
- [ ] WebDriver tests pass on Linux CI
- [ ] Full E2E pipeline in CI

---

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Playwright MCP availability | Keep WebDriver as fallback for non-macOS |
| macOS E2E complexity | Start with mock harness, add E2E incrementally |
| Agent interpretation errors | Use strict schema validation, clear reasoning |
| CI timeout | Keep contract tests fast (<1s), E2E optional |
| Schema drift | Use TypeScript types for all schemas |

---

## Maintenance

### Adding New Verification

1. Define expected behavior in YAML pattern
2. Add command handler to ContractHarness (if backend)
3. Add mock response to MockHarness (if frontend)
4. Create E2E session template (if full stack)
5. Update documentation

### Updating Schema

1. Modify `HarnessResult` interface
2. Update Rust struct in `result.rs`
3. Update TypeScript type in `types.ts`
4. Add migration notes to CHANGELOG

---

## Summary

This roadmap provides a complete implementation plan for:

1. **Contract Harness** — Backend-only verification (all platforms)
2. **Mock Harness** — Frontend-only verification (fast, deterministic)
3. **E2E Harness** — Full stack verification (macOS via Playwright MCP, Linux via WebDriver)
4. **Decision Engine** — Agent-readable structured conclusions
5. **Verification CLI** — `pnpm run verify` for agent use

The multi-layer approach ensures coverage at every verification need, with structured JSON output enabling AI agents to understand and verify their own work.