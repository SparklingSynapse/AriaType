# Tauri E2E Testing & Harness Engineering Design

## Research Summary

### Current Tauri Testing Ecosystem

Tauri provides three testing approaches:

| Approach | Description | Limitations |
|----------|-------------|-------------|
| **Mock Runtime** | Unit/integration tests with `mock_ipc()` from `@tauri-apps/api/mocks` | Only tests frontend logic, no real IPC |
| **WebDriver** | E2E via `tauri-driver` + Selenium/WebdriverIO | No macOS support, requires GUI |
| **Pipeline Integration** | Rust `tests/` with mock engines | Backend only, no UI interaction |

#### WebDriver Architecture

```
Test Runner (WebdriverIO/Selenium)
    ↓ HTTP (127.0.0.1:4444)
tauri-driver (Rust binary)
    ↓ Platform-specific
Native WebDriver Server:
  - Linux: WebKitWebDriver
  - Windows: Microsoft Edge Driver
    ↓
Tauri WebView (real application)
```

**Critical limitation**: macOS has NO WKWebView driver. Cannot run WebDriver E2E on macOS.

#### Existing Project Testing Coverage

| Layer | Coverage | Gaps |
|-------|----------|------|
| Backend Unit | `tests/*.rs` with mock STT/Polish | No IPC command testing |
| Backend Integration | `pipeline_integration_test.rs` | No Tauri runtime context |
| Frontend Unit | Vitest + mockIPC | No real backend calls |
| E2E UI | `.playwright-mcp/` traces exist | No formal E2E framework |

### Playwright MCP

Playwright MCP (Model Context Protocol) provides:
- Browser automation via MCP tools
- Screenshot capture
- Console log extraction
- DOM state inspection

Current traces in `.playwright-mcp/` show page snapshots and console logs, indicating prior exploration.

---

## Problem Statement

### What We Need

1. **Agent Verification**: Model needs to know if frontend interactions correctly trigger backend logic
2. **Cross-Platform E2E**: Must work on macOS (primary dev platform)
3. **Harness Engineering**: Structured test harness that agents can execute and interpret
4. **IPC Contract Testing**: Verify frontend→backend→frontend event flow

### What Existing Solutions Lack

| Gap | Why It Matters |
|-----|----------------|
| macOS WebDriver | Cannot test on primary development platform |
| IPC verification | WebDriver only sees UI, not backend state transitions |
| Agent interpretation | Test results need structured output for model consumption |
| Headless execution | CI requires non-GUI testing mode |

---

## Proposed Architecture

### Layer 1: Tauri Mock Runtime Enhancement

Extend existing mock system for full IPC simulation:

```typescript
// Enhanced mockIPC with backend state tracking
interface MockBackendState {
  recordingState: RecordingState;
  lastTranscription: TranscriptionResult | null;
  settings: AppSettings;
  historyEntries: HistoryEntry[];
}

mockIPC((cmd, args) => {
  const state = getMockBackendState();
  
  switch (cmd) {
    case 'start_recording':
      state.recordingState = 'Recording';
      emitMockEvent('recording-state-changed', state.recordingState);
      return { success: true };
    
    case 'stop_recording':
      state.recordingState = 'Processing';
      emitMockEvent('recording-state-changed', state.recordingState);
      // Simulate transcription
      setTimeout(() => {
        state.lastTranscription = { text: 'mock transcription' };
        emitMockEvent('transcription-complete', state.lastTranscription);
        state.recordingState = 'Idle';
        emitMockEvent('recording-state-changed', state.recordingState);
      }, 100);
      return { success: true };
  }
}, { 
  shouldMockEvents: true,
  trackStateChanges: true  // NEW: record all state transitions
});
```

**Benefits**:
- Works on all platforms including macOS
- Agent can query mock backend state
- Full IPC contract testing without real backend

### Layer 2: IPC Contract Test Harness

Rust-based contract testing that verifies IPC without UI:

```rust
// tests/ipc_contract_test.rs
#[cfg(test)]
mod ipc_contract {
    use ariatype_lib::commands::*;
    use ariatype_lib::events::*;
    
    struct IpcHarness {
        state: UnifiedRecordingState,
        event_log: Vec<(EventName, serde_json::Value)>,
    }
    
    impl IpcHarness {
        fn new() -> Self { ... }
        
        // Simulate frontend command, verify backend response
        fn execute_command(&mut self, cmd: &str, args: Value) -> Value {
            match cmd {
                "start_recording" => self.handle_start_recording(args),
                "stop_recording" => self.handle_stop_recording(args),
                ...
            }
        }
        
        // Verify expected event sequence
        fn verify_event_sequence(&self, expected: &[EventName]) -> bool {
            self.event_log.iter()
                .map(|(name, _)| name)
                .collect::<Vec<_>>() == expected
        }
        
        // Export structured result for agent consumption
        fn to_harness_result(&self) -> HarnessResult {
            HarnessResult {
                commands_executed: self.command_log,
                events_emitted: self.event_log,
                state_transitions: self.state.transitions(),
                assertions_passed: self.assertions.iter().filter(|a| a.passed).count(),
            }
        }
    }
    
    #[test]
    fn test_recording_flow_contract() {
        let harness = IpcHarness::new();
        
        // Simulate user workflow
        harness.execute_command("start_recording", json!({}));
        assert!(harness.state.current() == RecordingState::Recording);
        harness.verify_event_sequence(&["recording-state-changed"]);
        
        harness.execute_command("stop_recording", json!({}));
        assert!(harness.state.current() == RecordingState::Processing);
        
        // Wait for mock transcription
        harness.wait_for_event("transcription-complete", 1000);
        harness.verify_event_sequence(&[
            "recording-state-changed",
            "recording-state-changed",  // Processing
            "transcription-complete",
            "recording-state-changed",  // Idle
        ]);
        
        // Export result
        let result = harness.to_harness_result();
        assert!(result.assertions_passed == 4);
    }
}
```

### Layer 3: Playwright MCP Integration for macOS

Use Playwright MCP for macOS E2E where WebDriver is unavailable:

```yaml
# .playwright-mcp/test-session.yml
test_session:
  name: "recording_flow_e2e"
  platform: "darwin"
  
steps:
  - action: "launch_app"
    expected: { window_visible: true, state: "Idle" }
    
  - action: "click_record_button"
    expected: { state: "Recording", pill_visible: true }
    capture: { screenshot: true, console: true }
    
  - action: "wait_duration"
    duration: 2000
    
  - action: "click_stop_button"
    expected: { state: "Processing" }
    capture: { screenshot: true, console: true }
    
  - action: "wait_for_event"
    event: "transcription-complete"
    timeout: 5000
    
  - action: "verify_output"
    expected: { text_field_has_content: true }

result_format:
  - screenshots: [path/to/screenshot.png]
  - console_logs: [timestamp, level, message]
  - state_transitions: [Idle → Recording → Processing → Idle]
  - assertions: [{ step: N, expected: X, actual: Y, passed: bool }]
```

**Agent Interpretation Interface**:

```typescript
// HarnessResult format for model consumption
interface HarnessResult {
  session_id: string;
  platform: string;
  steps: StepResult[];
  assertions_passed: number;
  assertions_failed: number;
  screenshots: string[];  // Paths to captured screenshots
  console_logs: ConsoleLog[];
  ipc_calls: IpcCall[];
  events_emitted: Event[];
  final_state: AppState;
}

interface StepResult {
  step_id: number;
  action: string;
  expected: Record<string, unknown>;
  actual: Record<string, unknown>;
  passed: boolean;
  duration_ms: number;
  screenshot?: string;
}
```

### Layer 4: Hybrid WebDriver + IPC Verification

For Linux/Windows CI, combine WebDriver UI automation with IPC verification:

```javascript
// e2e-tests/test/recording.spec.js
import { Builder, By } from 'selenium-webdriver';

describe('Recording Flow', () => {
  it('verifies IPC sequence on recording workflow', async () => {
    const driver = await buildTauriDriver();
    
    // Track IPC calls via injected listener
    const ipcTracker = await driver.executeScript(`
      window.__IPC_TRACKER = [];
      const originalInvoke = window.__TAURI_INTERNALS__.invoke;
      window.__TAURI_INTERNALS__.invoke = async (cmd, args) => {
        window.__IPC_TRACKER.push({ cmd, args, timestamp: Date.now() });
        return originalInvoke(cmd, args);
      };
      return window.__IPC_TRACKER;
    `);
    
    // UI interaction
    await driver.findElement(By.css('[data-testid="record-btn"]')).click();
    
    // Verify IPC call was made
    const ipcCalls = await driver.executeScript('return window.__IPC_TRACKER');
    expect(ipcCalls).toContainEqual({ cmd: 'start_recording', args: {} });
    
    // Verify backend event via frontend state
    const state = await driver.findElement(By.css('[data-testid="state-indicator"]')).getText();
    expect(state).toBe('Recording');
    
    // Stop recording
    await driver.findElement(By.css('[data-testid="stop-btn"]')).click();
    
    // Wait for transcription
    await driver.wait(async () => {
      const ipcCalls = await driver.executeScript('return window.__IPC_TRACKER');
      return ipcCalls.some(c => c.cmd === 'transcription-complete');
    }, 5000);
    
    // Export structured result
    const result = {
      ipc_calls: await driver.executeScript('return window.__IPC_TRACKER'),
      screenshots: [await driver.takeScreenshot()],
      passed: true,
    };
    
    await driver.quit();
    return result;
  });
});
```

---

## Implementation Plan

### Phase 1: IPC Contract Harness (2-3 days)

1. Create `tests/ipc_harness/mod.rs` with harness framework
2. Implement command handlers that simulate frontend calls
3. Add event emission tracking
4. Create `HarnessResult` JSON output format
5. Add `cargo test --test ipc_contract_test` to CI

### Phase 2: Enhanced Mock Runtime (1-2 days)

1. Extend `@tauri-apps/api/mocks` with state tracking
2. Create `MockBackendState` interface
3. Add `trackStateChanges` option to `mockIPC`
4. Write frontend tests using enhanced mocks
5. Document in `context/guides/testing.md`

### Phase 3: Playwright MCP Integration (2-3 days)

1. Install Playwright MCP tools
2. Create E2E test templates in `.playwright-mcp/templates/`
3. Implement screenshot + console capture helpers
4. Create agent-readable result format
5. Add macOS E2E CI workflow

### Phase 4: WebDriver IPC Tracker (1-2 days)

1. Create IPC tracking injection script
2. Integrate with WebdriverIO test suite
3. Add structured result export
4. Update CI for Linux/Windows WebDriver tests

### Phase 5: Agent Integration (1 day)

1. Define `HarnessResult` schema
2. Create harness execution CLI command
3. Add result interpretation guide for agents
4. Update `AGENTS.md` with harness usage

---

## Directory Structure

```
apps/desktop/
├── src-tauri/
│   └── tests/
│       ├── ipc_harness/
│       │   ├── mod.rs           # Harness framework
│       │   ├── commands.rs      # Command simulators
│       │   ├── events.rs        # Event trackers
│       │   └── result.rs        # HarnessResult format
│       └── ipc_contract_test.rs # Contract tests
│
├── src/
│   └── test/
│       ├── harness/
│       │   ├── mock-backend.ts  # Enhanced mock state
│       │   ├── ipc-tracker.ts   # IPC call tracker
│       │   └── result-types.ts  # TypeScript interfaces
│       └── contract/
│           └── recording.test.ts
│
├── e2e-tests/
│   ├── webdriver/
│   │   ├── wdio.conf.js
│   │   └── test/
│   │       └── recording.spec.js
│   └── playwright/
│       ├── test-session.yml
│       └── templates/
│
└── .playwright-mcp/
    ├── sessions/
    ├── screenshots/
    └── results/
```

---

## Agent Verification Protocol

### How Agents Use Harness

```markdown
# Agent E2E Verification Workflow

1. **Identify scope**: Which feature/flow needs verification
2. **Choose harness layer**:
   - Frontend-only → Enhanced Mock Runtime
   - Backend-only → IPC Contract Harness
   - Full flow → Playwright MCP / WebDriver
3. **Execute harness**: Run test with structured output
4. **Interpret result**: Parse `HarnessResult` JSON
5. **Report findings**: State transitions, IPC calls, assertions

### Example Verification

```bash
# Run IPC contract test
cargo test --test ipc_contract_test -- --format json

# Parse result
{
  "session_id": "recording-flow-001",
  "assertions_passed": 5,
  "assertions_failed": 0,
  "ipc_calls": [
    {"cmd": "start_recording", "args": {}, "response": {"success": true}},
    {"cmd": "stop_recording", "args": {}, "response": {"success": true}}
  ],
  "events_emitted": [
    {"event": "recording-state-changed", "payload": {"state": "Recording"}},
    {"event": "transcription-complete", "payload": {"text": "mock"}}
  ],
  "state_transitions": ["Idle", "Recording", "Processing", "Idle"],
  "passed": true
}
```

Agent can now verify:
- Did frontend trigger correct backend commands?
- Did backend emit expected events?
- Did state transition correctly?
- Are all assertions passed?

---

## CI Integration

```yaml
# .github/workflows/e2e-test.yml
name: E2E Testing

on: [push, pull_request]

jobs:
  ipc-contract:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo test --test ipc_contract_test
      
  webdriver-linux:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: |
          sudo apt-get install -y webkit2gtk-driver xvfb
          cargo install tauri-driver
          cd e2e-tests/webdriver && xvfb-run npm test
      
  playwright-macos:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      - run: npm run e2e:playwright
```

---

## Conclusion

This design provides:

1. **Multi-layer testing**: Mock, Contract, WebDriver, Playwright MCP
2. **macOS coverage**: Playwright MCP fills WebDriver gap
3. **Agent interpretation**: Structured `HarnessResult` for model consumption
4. **IPC verification**: Track frontend→backend→frontend flow
5. **CI integration**: Cross-platform automated testing

The harness enables agents to verify that frontend interactions correctly trigger backend logic, with clear evidence via screenshots, console logs, IPC call tracking, and event sequence verification.