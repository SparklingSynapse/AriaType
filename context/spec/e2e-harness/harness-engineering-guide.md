# Harness Engineering Implementation Guide

## What is Harness Engineering?

Harness Engineering is a structured approach to creating test frameworks that:
1. **Agents can execute autonomously** — no manual interpretation needed
2. **Produce structured, interpretable results** — JSON/YAML output, not just console logs
3. **Verify multi-layer interactions** — frontend → backend → frontend flow
4. **Provide evidence for assertions** — screenshots, state snapshots, event logs

This enables AI agents to verify their own work and understand the consequences of code changes.

---

## Harness Types for Tauri Applications

### Type 1: Contract Harness (Backend-Focused)

**Purpose**: Verify IPC command handlers and state transitions without UI.

**When to use**:
- Verifying backend logic changes
- Testing state machine correctness
- Validating event emission sequence

**Implementation**:

```rust
// tests/ipc_harness/mod.rs
pub struct ContractHarness {
    app_state: AppState,
    command_log: Vec<CommandRecord>,
    event_log: Vec<EventRecord>,
    assertions: Vec<Assertion>,
}

pub struct CommandRecord {
    command: String,
    args: serde_json::Value,
    response: serde_json::Value,
    duration_ms: u64,
}

pub struct EventRecord {
    event: EventName,
    payload: serde_json::Value,
    emitted_at: std::time::Instant,
}

pub struct Assertion {
    description: String,
    expected: serde_json::Value,
    actual: serde_json::Value,
    passed: bool,
}

impl ContractHarness {
    /// Execute a command as if it came from frontend
    pub fn execute(&mut self, command: &str, args: Value) -> Value {
        let start = std::time::Instant::now();
        
        // Delegate to actual command handler
        let response = match command {
            "start_recording" => {
                self.app_state.recording_state.transition_to(RecordingState::Recording);
                self.emit_event(EventName::RecordingStateChanged, json!({
                    "state": "Recording"
                }));
                json!({ "success": true })
            }
            // ... other commands
            _ => json!({ "error": "unknown command" })
        };
        
        self.command_log.push(CommandRecord {
            command: command.to_string(),
            args,
            response: response.clone(),
            duration_ms: start.elapsed().as_millis() as u64,
        });
        
        response
    }
    
    /// Emit an event as if it came from backend
    pub fn emit_event(&mut self, event: EventName, payload: Value) {
        self.event_log.push(EventRecord {
            event,
            payload,
            emitted_at: std::time::Instant::now(),
        });
    }
    
    /// Assert state matches expectation
    pub fn assert_state(&mut self, expected: RecordingState) -> &mut Self {
        let actual = self.app_state.recording_state.current();
        let passed = actual == expected;
        
        self.assertions.push(Assertion {
            description: format!("Recording state should be {}", expected),
            expected: json!(expected.to_string()),
            actual: json!(actual.to_string()),
            passed,
        });
        
        self
    }
    
    /// Assert event was emitted in sequence
    pub fn assert_event_sequence(&mut self, expected: &[EventName]) -> &mut Self {
        let actual_sequence: Vec<EventName> = self.event_log.iter()
            .map(|r| r.event.clone())
            .collect();
        
        let passed = actual_sequence.len() == expected.len()
            && actual_sequence.iter().zip(expected.iter()).all(|(a, e)| a == *e);
        
        self.assertions.push(Assertion {
            description: "Event sequence matches expected",
            expected: json!(expected.iter().map(|e| e.to_string()).collect::<Vec<_>>()),
            actual: json!(actual_sequence.iter().map(|e| e.to_string()).collect::<Vec<_>>()),
            passed,
        });
        
        self
    }
    
    /// Export result for agent consumption
    pub fn finalize(&self) -> HarnessResult {
        HarnessResult {
            session_id: uuid::Uuid::new_v4().to_string(),
            platform: std::env::consts::OS,
            commands_executed: self.command_log.clone(),
            events_emitted: self.event_log.clone(),
            assertions: self.assertions.clone(),
            passed: self.assertions.iter().all(|a| a.passed),
            final_state: self.app_state.snapshot(),
        }
    }
}

/// Output format for agent interpretation
#[derive(Debug, Clone, Serialize)]
pub struct HarnessResult {
    session_id: String,
    platform: String,
    commands_executed: Vec<CommandRecord>,
    events_emitted: Vec<EventRecord>,
    assertions: Vec<Assertion>,
    passed: bool,
    final_state: serde_json::Value,
}
```

### Type 2: Mock Harness (Frontend-Focused)

**Purpose**: Verify frontend behavior without real backend.

**When to use**:
- Testing frontend state management
- Validating UI component behavior
- Simulating backend responses

**Implementation**:

```typescript
// src/test/harness/mock-backend.ts
import { mockIPC, mockWindows, clearMocks } from '@tauri-apps/api/mocks';
import type { UnlistenFn } from '@tauri-apps/api/event';

interface MockBackendState {
  recordingState: 'Idle' | 'Recording' | 'Processing' | 'Error';
  transcription: string | null;
  settings: Record<string, unknown>;
}

interface HarnessResult {
  sessionId: string;
  commandsInvoked: Array<{ cmd: string; args: unknown; response: unknown }>;
  eventsReceived: Array<{ event: string; payload: unknown }>;
  assertions: Array<{ description: string; passed: boolean }>;
  passed: boolean;
  finalState: MockBackendState;
}

export class MockHarness {
  private state: MockBackendState = {
    recordingState: 'Idle',
    transcription: null,
    settings: {},
  };
  
  private commandsLog: Array<{ cmd: string; args: unknown; response: unknown }> = [];
  private eventsLog: Array<{ event: string; payload: unknown }> = [];
  private listeners: Map<string, Set<(payload: unknown) => void>> = new Map();
  
  constructor() {
    this.setupMockIPC();
    mockWindows('main');
  }
  
  private setupMockIPC() {
    mockIPC((cmd, args) => {
      this.commandsLog.push({ cmd, args, response: null });
      
      const result = this.handleCommand(cmd, args);
      this.commandsLog[this.commandsLog.length - 1].response = result;
      
      return result;
    }, { shouldMockEvents: true });
  }
  
  private handleCommand(cmd: string, args: unknown): unknown {
    switch (cmd) {
      case 'start_recording':
        this.transitionTo('Recording');
        return { success: true };
        
      case 'stop_recording':
        this.transitionTo('Processing');
        // Simulate async transcription
        setTimeout(() => {
          this.state.transcription = 'mock transcription text';
          this.emit('transcription-complete', { text: this.state.transcription });
          this.transitionTo('Idle');
        }, 100);
        return { success: true };
        
      case 'get_recording_state':
        return { state: this.state.recordingState };
        
      default:
        return { error: 'unknown command' };
    }
  }
  
  private transitionTo(newState: MockBackendState['recordingState']) {
    this.state.recordingState = newState;
    this.emit('recording-state-changed', { state: newState });
  }
  
  private emit(event: string, payload: unknown) {
    this.eventsLog.push({ event, payload });
    const handlers = this.listeners.get(event) || new Set();
    handlers.forEach(h => h(payload));
  }
  
  // Public API for test use
  
  subscribe(event: string, handler: (payload: unknown) => void): UnlistenFn {
    if (!this.listeners.has(event)) {
      this.listeners.set(event, new Set());
    }
    this.listeners.get(event)!.add(handler);
    
    return () => {
      this.listeners.get(event)?.delete(handler);
    };
  }
  
  async invoke(cmd: string, args?: unknown): Promise<unknown> {
    return handleCommand(cmd, args || {});
  }
  
  assertState(expected: MockBackendState['recordingState']): this {
    const actual = this.state.recordingState;
    // Record assertion for final report
    return this;
  }
  
  assertEventReceived(event: string): this {
    const received = this.eventsLog.some(e => e.event === event);
    return this;
  }
  
  finalize(): HarnessResult {
    return {
      sessionId: crypto.randomUUID(),
      commandsInvoked: this.commandsLog,
      eventsReceived: this.eventsLog,
      assertions: [], // populated during test
      passed: true,   // based on assertions
      finalState: this.state,
    };
  }
  
  cleanup() {
    clearMocks();
    this.listeners.clear();
  }
}
```

### Type 3: E2E Harness (Full Integration)

**Purpose**: Verify complete frontend-backend flow with real application.

**When to use**:
- Final verification before release
- Testing real user workflows
- Validating IPC in production context

**Implementation (Playwright MCP for macOS)**:

```yaml
# e2e-tests/playwright/session-template.yml
session:
  name: "${session_name}"
  platform: "${platform}"
  app_path: "${app_binary_path}"

setup:
  - action: launch_app
    timeout: 30000

workflow:
  - step: 1
    action: click_element
    selector: "[data-testid='record-btn']"
    capture:
      screenshot: true
      console: true
    verify:
      - type: element_visible
        selector: "[data-testid='recording-indicator']"
      - type: console_contains
        pattern: "Recording state changed: Recording"

  - step: 2
    action: wait
    duration: 2000

  - step: 3
    action: click_element
    selector: "[data-testid='stop-btn']"
    capture:
      screenshot: true
      console: true
    verify:
      - type: console_contains
        pattern: "Recording state changed: Processing"

  - step: 4
    action: wait_for_event
    event: transcription-complete
    timeout: 10000
    capture:
      screenshot: true

  - step: 5
    action: verify_element
    selector: "[data-testid='transcription-output']"
    expected:
      text_contains: "."

teardown:
  - action: close_app
```

```typescript
// e2e-tests/playwright/runner.ts
import { MCPClient } from '@anthropic/mcp-client';

interface PlaywrightSessionResult {
  sessionId: string;
  platform: string;
  steps: StepResult[];
  screenshots: string[];
  consoleLogs: ConsoleLog[];
  ipcCalls: IpcCall[];
  passed: boolean;
}

export async function runSession(config: SessionConfig): Promise<PlaywrightSessionResult> {
  const mcp = new MCPClient();
  
  // Launch app via MCP
  await mcp.invoke('playwright.launch', { path: config.appPath });
  
  const results: StepResult[] = [];
  const screenshots: string[] = [];
  const consoleLogs: ConsoleLog[] = [];
  
  for (const step of config.workflow) {
    // Execute action
    await mcp.invoke(`playwright.${step.action}`, step.params);
    
    // Capture state
    if (step.capture?.screenshot) {
      const screenshot = await mcp.invoke('playwright.screenshot');
      screenshots.push(screenshot.path);
    }
    
    if (step.capture?.console) {
      const logs = await mcp.invoke('playwright.console');
      consoleLogs.push(...logs);
    }
    
    // Verify expectations
    const stepPassed = await verifyStep(step.verify || [], mcp);
    results.push({
      stepId: step.step,
      action: step.action,
      passed: stepPassed,
      screenshot: screenshots[screenshots.length - 1],
    });
  }
  
  // Close app
  await mcp.invoke('playwright.close');
  
  return {
    sessionId: config.sessionId,
    platform: config.platform,
    steps: results,
    screenshots,
    consoleLogs,
    ipcCalls: extractIpcCalls(consoleLogs),
    passed: results.every(r => r.passed),
  };
}

function extractIpcCalls(consoleLogs: ConsoleLog[]): IpcCall[] {
  // Parse console logs for IPC invoke patterns
  return consoleLogs
    .filter(log => log.message.includes('[IPC]'))
    .map(log => {
      const match = log.message.match(/\[IPC\] (\w+)\((.+)?\)/);
      return {
        command: match?.[1] || '',
        args: JSON.parse(match?.[2] || '{}'),
        timestamp: log.timestamp,
      };
    });
}
```

---

## Agent Verification Protocol

### Protocol Steps

```
┌─────────────────────────────────────────────────────────────────┐
│                    AGENT VERIFICATION PROTOCOL                    │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  1. FRAME: Define what needs verification                        │
│     ↓                                                            │
│  2. SELECT: Choose appropriate harness type                       │
│     ↓                                                            │
│  3. EXECUTE: Run harness with structured output                   │
│     ↓                                                            │
│  4. INTERPRET: Parse HarnessResult JSON/YAML                     │
│     ↓                                                            │
│  5. DECIDE: Based on evidence, what next?                        │
│     ↓                                                            │
│  6. REPORT: Document findings with evidence                      │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Decision Matrix

| Evidence Type | What It Proves | Confidence Level |
|--------------|----------------|------------------|
| `commands_executed` matches expected | Frontend correctly invoked backend | High |
| `events_emitted` sequence matches | Backend correctly responded to frontend | High |
| `state_transitions` match expected | State machine behaving correctly | High |
| `screenshots` show expected UI | Visual verification of rendering | Medium |
| `console_logs` contain patterns | Backend logic executed correctly | Medium |
| `assertions_passed: N, assertions_failed: 0` | All expectations met | High |

### Example Agent Verification Session

```markdown
## Verification Task

Verify that clicking the "Record" button:
1. Triggers `start_recording` IPC call
2. Changes state to "Recording"
3. Emits `recording-state-changed` event
4. Shows recording indicator UI

### Harness Selection

Frontend + backend interaction → Contract Harness

### Execution

```bash
cargo test --test recording_flow_contract -- --format json
```

### Result Interpretation

```json
{
  "session_id": "rec-001",
  "commands_executed": [
    {"command": "start_recording", "args": {}, "response": {"success": true}}
  ],
  "events_emitted": [
    {"event": "recording-state-changed", "payload": {"state": "Recording"}}
  ],
  "state_transitions": ["Idle", "Recording"],
  "assertions": [
    {"description": "State should be Recording", "expected": "Recording", "actual": "Recording", "passed": true},
    {"description": "Event should be emitted", "expected": ["recording-state-changed"], "actual": ["recording-state-changed"], "passed": true}
  ],
  "passed": true
}
```

### Conclusion

✅ All 4 verification points confirmed:
- IPC call made: `start_recording` ✓
- State changed: Idle → Recording ✓
- Event emitted: `recording-state-changed` ✓
- (UI verification requires E2E harness with screenshots)
```

---

## Harness Engineering Principles

### 1. Structured Output First

All harnesses MUST produce machine-readable output (JSON/YAML), not just human-readable logs.

```rust
// BAD: Console output only
println!("Test passed!");

// GOOD: Structured output
println!("{}", serde_json::to_string(&result)?);
```

### 2. Evidence Recording

Every assertion must be backed by captured evidence:

- State snapshots before/after
- Event timestamps
- IPC call traces
- Screenshots (E2E)

### 3. Agent-Readable Schema

Use consistent output schema across all harness types:

```typescript
interface UniversalHarnessResult {
  sessionId: string;
  harnessType: 'contract' | 'mock' | 'e2e';
  platform: string;
  passed: boolean;
  
  // Core evidence
  commands: CommandRecord[];
  events: EventRecord[];
  stateTransitions: StateTransition[];
  assertions: Assertion[];
  
  // Optional per-type
  screenshots?: string[];
  consoleLogs?: ConsoleLog[];
}
```

### 4. Deterministic Behavior

Harness should produce identical results for identical inputs:

- No random timing delays (use fixed intervals)
- No external API calls (mock everything)
- No system-dependent behavior (isolate platform specifics)

### 5. Quick Feedback Loop

Harness must complete in reasonable time:

- Contract harness: < 1 second
- Mock harness: < 5 seconds
- E2E harness: < 30 seconds

---

## Integration with Existing Architecture

### Where Harness Fits in Layer Model

```
                    ┌───────────────────┐
                    │     E2E Harness    │  ← Full stack
                    │   (Playwright MCP) │
                    └─────────┬─────────┘
                              │
          ┌───────────────────┼───────────────────┐
          │                   │                   │
    ┌─────┴─────┐       ┌─────┴─────┐       ┌─────┴─────┐
    │   Mock    │       │ Contract  │       │ WebDriver │
    │  Harness  │       │  Harness  │       │   Harness │
    │(Frontend) │       │ (Backend) │       │(Linux/Win)│
    └───────────┘       └───────────┘       └───────────┘
          │                   │                   │
    ┌─────┴─────┐       ┌─────┴─────┐       ┌─────┴─────┐
    │ Frontend  │       │ Backend   │       │ IPC + UI  │
    │   Tests   │       │   Tests   │       │   Tests   │
    └───────────┘       └───────────┘       └───────────┘
```

### Harness Entry Points

| Harness Type | Entry Point | Command |
|--------------|-------------|---------|
| Contract | `tests/ipc_contract_test.rs` | `cargo test --test ipc_contract` |
| Mock | `src/test/harness/mock-backend.ts` | `pnpm test:harness:mock` |
| E2E Playwright | `e2e-tests/playwright/` | `pnpm test:harness:e2e` |
| WebDriver | `e2e-tests/webdriver/` | `pnpm test:harness:webdriver` |

---

## Maintenance Guidelines

### Adding New Commands to Harness

1. Add to `ContractHarness::execute()` switch
2. Update `MockHarness::handleCommand()`
3. Create new session template for E2E
4. Document in ` HarnessResult` schema

### Adding New Events to Harness

1. Add to `EventName` enum
2. Update `ContractHarness::emit_event()` logging
3. Update `MockHarness::emit()` dispatch
4. Add console pattern for E2E detection

### Updating Schema

1. Modify `UniversalHarnessResult` interface
2. Update Rust `HarnessResult` struct
3. Add JSON serialization
4. Document change in this file