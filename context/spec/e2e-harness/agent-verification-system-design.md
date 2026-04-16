# Agent Verification System Design

## Core Problem

When an AI agent modifies code, it needs to verify that:

1. **Frontend changes** correctly trigger backend logic
2. **Backend changes** correctly respond to frontend calls
3. **IPC contract** is maintained (no breaking changes)
4. **State transitions** follow expected patterns
5. **Events** are emitted in correct sequence

Traditional testing gives "pass/fail" but no structured evidence. Agents need **explainable verification**.

---

## Verification System Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         AGENT VERIFICATION SYSTEM                            │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌───────────────┐    ┌───────────────┐    ┌───────────────┐                 │
│  │  Verification │    │   Evidence    │    │   Decision    │                 │
│  │    Request    │───▶│   Collector   │───▶│    Engine     │                 │
│  └───────────────┘    └───────────────┘    └───────────────┘                 │
│        │                      │                     │                        │
│        │                      │                     │                        │
│        ▼                      ▼                     ▼                        │
│  ┌───────────────┐    ┌───────────────┐    ┌───────────────┐                 │
│  │   Scope       │    │   IPC Trace   │    │   Structured  │                 │
│  │   Definition  │    │   Events      │    │   Report      │                 │
│  │               │    │   State       │    │               │                 │
│  │               │    │   Screenshots │    │               │                 │
│  └───────────────┘    └───────────────┘    └───────────────┘                 │
│                                                                              │
│                              Output for Agent                                │
│                                                                              │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │                        VerificationResult                              │  │
│  │  ┌─────────────────────────────────────────────────────────────────┐  │  │
│  │  │ {                                                                │  │  │
│  │  │   "verification_id": "v-001",                                    │  │  │
│  │  │   "scope": "frontend_backend_interaction",                       │  │  │
│  │  │   "what_changed": "frontend click handler",                      │  │  │
│  │  │   "what_should_happen": "start_recording IPC call",              │  │  │
│  │  │   "evidence": {                                                  │  │  │
│  │  │     "ipc_calls": [...],                                          │  │  │
│  │  │     "events_emitted": [...],                                     │  │  │
│  │  │     "state_before": "Idle",                                      │  │  │
│  │  │     "state_after": "Recording",                                  │  │  │
│  │  │     "screenshots": ["before.png", "after.png"]                   │  │  │
│  │  │   },                                                             │  │  │
│  │  │   "conclusion": {                                                │  │  │
│  │  │     "frontend_triggered_backend": true,                          │  │  │
│  │  │     "backend_responded_correctly": true,                         │  │  │
│  │  │     "state_transition_correct": true,                            │  │  │
│  │  │     "confidence": "high",                                        │  │  │
│  │  │     "reasoning": "All 4 assertions passed with matching evidence" │  │  │
│  │  │   }                                                              │  │  │
│  │  │ }                                                                │  │  │
│  │  └─────────────────────────────────────────────────────────────────┘  │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Verification Request Schema

```typescript
interface VerificationRequest {
  verificationId: string;
  
  // What changed (from agent)
  changeContext: {
    filesModified: string[];
    changeType: 'frontend' | 'backend' | 'ipc' | 'state' | 'event';
    changeDescription: string;
  };
  
  // What should happen (expected behavior)
  expectedBehavior: {
    frontendAction?: string;         // e.g., "click record button"
    backendCommand?: string;         // e.g., "start_recording"
    stateTransition?: [string, string]; // e.g., ["Idle", "Recording"]
    eventsExpected?: string[];       // e.g., ["recording-state-changed"]
    uiChange?: string;               // e.g., "recording indicator visible"
  };
  
  // Verification strategy
  strategy: 'contract' | 'mock' | 'e2e' | 'hybrid';
  
  // Confidence requirement
  confidenceThreshold: 'low' | 'medium' | 'high';
}
```

---

## Evidence Collector

### IPC Trace Collector

```typescript
// src/test/harness/ipc-tracer.ts
export class IpcTracer {
  private traces: IpcTrace[] = [];
  
  startTracing(): void {
    // Inject tracing into Tauri invoke
    const originalInvoke = window.__TAURI_INTERNALS__?.invoke;
    if (originalInvoke) {
      window.__TAURI_INTERNALS__.invoke = async (cmd: string, args: unknown) => {
        const trace: IpcTrace = {
          timestamp: Date.now(),
          direction: 'frontend_to_backend',
          command: cmd,
          args: args,
          response: null,
          durationMs: 0,
        };
        
        const start = performance.now();
        try {
          const response = await originalInvoke(cmd, args);
          trace.response = response;
          trace.durationMs = performance.now() - start;
          return response;
        } catch (error) {
          trace.response = { error: String(error) };
          trace.durationMs = performance.now() - start;
          throw error;
        } finally {
          this.traces.push(trace);
        }
      };
    }
  }
  
  // Capture backend → frontend events
  startEventTracing(): void {
    const eventTypes = [
      'recording-state-changed',
      'transcription-complete',
      'transcription-error',
      'audio-level',
      'retry-state-changed',
      'model-download-progress',
      'polish-progress',
    ];
    
    for (const eventType of eventTypes) {
      listen(eventType, (event) => {
        this.traces.push({
          timestamp: Date.now(),
          direction: 'backend_to_frontend',
          event: eventType,
          payload: event.payload,
        });
      });
    }
  }
  
  getTraces(): IpcTrace[] {
    return this.traces;
  }
  
  exportForAgent(): IpcEvidence {
    return {
      frontendCalls: this.traces.filter(t => t.direction === 'frontend_to_backend'),
      backendEvents: this.traces.filter(t => t.direction === 'backend_to_frontend'),
      summary: {
        totalCalls: this.traces.filter(t => t.direction === 'frontend_to_backend').length,
        totalEvents: this.traces.filter(t => t.direction === 'backend_to_frontend').length,
        commandsUsed: [...new Set(this.traces.map(t => t.command).filter(Boolean))],
        eventsEmitted: [...new Set(this.traces.map(t => t.event).filter(Boolean))],
      },
    };
  }
}

interface IpcTrace {
  timestamp: number;
  direction: 'frontend_to_backend' | 'backend_to_frontend';
  command?: string;
  args?: unknown;
  response?: unknown;
  event?: string;
  payload?: unknown;
  durationMs?: number;
}

interface IpcEvidence {
  frontendCalls: IpcTrace[];
  backendEvents: IpcTrace[];
  summary: {
    totalCalls: number;
    totalEvents: number;
    commandsUsed: string[];
    eventsEmitted: string[];
  };
}
```

### State Snapshot Collector

```typescript
// src/test/harness/state-snapshot.ts
export class StateSnapshotCollector {
  private snapshots: StateSnapshot[] = [];
  
  capture(label: string): StateSnapshot {
    const snapshot: StateSnapshot = {
      timestamp: Date.now(),
      label,
      recordingState: this.getRecordingState(),
      settings: this.getSettings(),
      historyCount: this.getHistoryCount(),
      uiState: this.captureUiState(),
    };
    
    this.snapshots.push(snapshot);
    return snapshot;
  }
  
  private getRecordingState(): string {
    // Read from state indicator element
    const indicator = document.querySelector('[data-testid="state-indicator"]');
    return indicator?.textContent || 'Unknown';
  }
  
  private captureUiState(): UiState {
    return {
      recordButtonState: document.querySelector('[data-testid="record-btn"]')?.className,
      pillVisible: !!document.querySelector('[data-testid="recording-pill"]'),
      transcriptionOutput: document.querySelector('[data-testid="transcription-output"]')?.textContent,
      errorVisible: !!document.querySelector('[data-testid="error-message"]'),
    };
  }
  
  getTransitions(): StateTransition[] {
    return this.snapshots
      .slice(1)
      .map((snap, i) => ({
        from: this.snapshots[i].recordingState,
        to: snap.recordingState,
        durationMs: snap.timestamp - this.snapshots[i].timestamp,
        trigger: snap.label,
      }));
  }
  
  exportForAgent(): StateEvidence {
    return {
      snapshots: this.snapshots,
      transitions: this.getTransitions(),
      finalState: this.snapshots[this.snapshots.length - 1],
    };
  }
}

interface StateSnapshot {
  timestamp: number;
  label: string;
  recordingState: string;
  settings: Record<string, unknown>;
  historyCount: number;
  uiState: UiState;
}

interface StateTransition {
  from: string;
  to: string;
  durationMs: number;
  trigger: string;
}
```

### Visual Evidence Collector

```typescript
// src/test/harness/visual-collector.ts
export class VisualEvidenceCollector {
  private screenshots: Screenshot[] = [];
  private domSnapshots: DomSnapshot[] = [];
  
  async captureScreenshot(label: string): Promise<Screenshot> {
    // For E2E harness, use Playwright MCP
    // For mock harness, use browser API
    const canvas = await this.renderToCanvas();
    const dataUrl = canvas.toDataURL('image/png');
    
    const screenshot: Screenshot = {
      timestamp: Date.now(),
      label,
      dataUrl,
      elementsVisible: this.detectVisibleElements(),
    };
    
    this.screenshots.push(screenshot);
    return screenshot;
  }
  
  captureDom(label: string): DomSnapshot {
    const dom: DomSnapshot = {
      timestamp: Date.now(),
      label,
      structure: this.serializeDom(document.body),
      accessibilityTree: this.getAccessibilityTree(),
    };
    
    this.domSnapshots.push(dom);
    return dom;
  }
  
  exportForAgent(): VisualEvidence {
    return {
      screenshots: this.screenshots.map(s => ({
        timestamp: s.timestamp,
        label: s.label,
        elementsVisible: s.elementsVisible,
      })),
      domChanges: this.detectChanges(),
    };
  }
  
  private detectVisibleElements(): string[] {
    const testIds = [
      'record-btn', 'stop-btn', 'state-indicator',
      'recording-pill', 'transcription-output',
      'error-message', 'settings-panel',
    ];
    
    return testIds.filter(id => 
      document.querySelector(`[data-testid="${id}"]`)
    );
  }
}
```

---

## Decision Engine

### Verification Logic

```typescript
// src/test/harness/decision-engine.ts
export class DecisionEngine {
  
  evaluate(request: VerificationRequest, evidence: Evidence): VerificationResult {
    const conclusions: Conclusions = {
      frontend_triggered_backend: this.evaluateFrontendTrigger(request, evidence),
      backend_responded_correctly: this.evaluateBackendResponse(request, evidence),
      state_transition_correct: this.evaluateStateTransition(request, evidence),
      events_correct: this.evaluateEventSequence(request, evidence),
      ui_updated_correctly: this.evaluateUiChange(request, evidence),
    };
    
    const passed = Object.values(conclusions).every(c => c.passed);
    const confidence = this.calculateConfidence(evidence, request.confidenceThreshold);
    
    return {
      verificationId: request.verificationId,
      scope: request.changeContext.changeType,
      whatChanged: request.changeContext.changeDescription,
      whatShouldHappen: this.summarizeExpected(request.expectedBehavior),
      evidence: evidence,
      conclusions: conclusions,
      passed,
      confidence,
      reasoning: this.generateReasoning(conclusions, evidence),
    };
  }
  
  private evaluateFrontendTrigger(request: VerificationRequest, evidence: Evidence): Conclusion {
    const expectedCommand = request.expectedBehavior.backendCommand;
    const actualCalls = evidence.ipc.frontendCalls;
    
    if (!expectedCommand) {
      return { passed: true, reasoning: 'No backend command expected' };
    }
    
    const matchingCall = actualCalls.find(c => c.command === expectedCommand);
    
    if (matchingCall) {
      return {
        passed: true,
        reasoning: `Frontend correctly invoked '${expectedCommand}' with args ${JSON.stringify(matchingCall.args)}`,
        evidence: { call: matchingCall },
      };
    } else {
      return {
        passed: false,
        reasoning: `Expected frontend to invoke '${expectedCommand}', but no such call found. Actual calls: ${actualCalls.map(c => c.command).join(', ') || 'none'}`,
        evidence: { actualCalls },
      };
    }
  }
  
  private evaluateBackendResponse(request: VerificationRequest, evidence: Evidence): Conclusion {
    const expectedCommand = request.expectedBehavior.backendCommand;
    const call = evidence.ipc.frontendCalls.find(c => c.command === expectedCommand);
    
    if (!call) {
      return { passed: true, reasoning: 'No command to verify response' };
    }
    
    // Check response is valid (not error)
    if (call.response && typeof call.response === 'object' && 'error' in call.response) {
      return {
        passed: false,
        reasoning: `Backend returned error: ${call.response.error}`,
        evidence: { response: call.response },
      };
    }
    
    return {
      passed: true,
      reasoning: `Backend responded successfully with ${JSON.stringify(call.response)}`,
      evidence: { response: call.response },
    };
  }
  
  private evaluateStateTransition(request: VerificationRequest, evidence: Evidence): Conclusion {
    const expectedTransition = request.expectedBehavior.stateTransition;
    const actualTransitions = evidence.state.transitions;
    
    if (!expectedTransition) {
      return { passed: true, reasoning: 'No state transition expected' };
    }
    
    const [expectedFrom, expectedTo] = expectedTransition;
    const matchingTransition = actualTransitions.find(t => 
      t.from === expectedFrom && t.to === expectedTo
    );
    
    if (matchingTransition) {
      return {
        passed: true,
        reasoning: `State correctly transitioned from '${expectedFrom}' to '${expectedTo}'`,
        evidence: { transition: matchingTransition },
      };
    } else {
      const lastState = evidence.state.finalState?.recordingState || 'Unknown';
      return {
        passed: false,
        reasoning: `Expected transition '${expectedFrom}' → '${expectedTo}', but final state is '${lastState}'`,
        evidence: { transitions: actualTransitions, finalState: evidence.state.finalState },
      };
    }
  }
  
  private evaluateEventSequence(request: VerificationRequest, evidence: Evidence): Conclusion {
    const expectedEvents = request.expectedBehavior.eventsExpected;
    const actualEvents = evidence.ipc.backendEvents;
    
    if (!expectedEvents || expectedEvents.length === 0) {
      return { passed: true, reasoning: 'No events expected' };
    }
    
    const actualEventTypes = actualEvents.map(e => e.event);
    const allExpectedFound = expectedEvents.every(expected => 
      actualEventTypes.includes(expected)
    );
    
    if (allExpectedFound) {
      return {
        passed: true,
        reasoning: `All expected events emitted: ${expectedEvents.join(', ')}`,
        evidence: { events: actualEvents },
      };
    } else {
      const missing = expectedEvents.filter(e => !actualEventTypes.includes(e));
      return {
        passed: false,
        reasoning: `Missing events: ${missing.join(', ')}. Actual: ${actualEventTypes.join(', ') || 'none'}`,
        evidence: { expected, actual: actualEventTypes },
      };
    }
  }
  
  private calculateConfidence(evidence: Evidence, threshold: string): ConfidenceLevel {
    const evidenceCounts = {
      ipcCalls: evidence.ipc.frontendCalls.length,
      events: evidence.ipc.backendEvents.length,
      stateSnapshots: evidence.state.snapshots.length,
      screenshots: evidence.visual.screenshots.length,
    };
    
    // High confidence requires multiple evidence types
    if (evidenceCounts.ipcCalls > 0 && evidenceCounts.events > 0 && evidenceCounts.stateSnapshots > 2) {
      return 'high';
    }
    
    if (evidenceCounts.ipcCalls > 0 && evidenceCounts.stateSnapshots > 1) {
      return 'medium';
    }
    
    return 'low';
  }
  
  private generateReasoning(conclusions: Conclusions, evidence: Evidence): string {
    const passedCount = Object.values(conclusions).filter(c => c.passed).length;
    const totalCount = Object.values(conclusions).length;
    
    const parts = Object.entries(conclusions)
      .map(([key, conclusion]) => `${key}: ${conclusion.reasoning}`)
      .join('\n');
    
    return `${passedCount}/${totalCount} checks passed.\n${parts}`;
  }
}

interface Conclusion {
  passed: boolean;
  reasoning: string;
  evidence?: Record<string, unknown>;
}

interface Conclusions {
  frontend_triggered_backend: Conclusion;
  backend_responded_correctly: Conclusion;
  state_transition_correct: Conclusion;
  events_correct: Conclusion;
  ui_updated_correctly: Conclusion;
}

type ConfidenceLevel = 'low' | 'medium' | 'high';
```

---

## Verification Result Schema

```typescript
interface VerificationResult {
  verificationId: string;
  scope: string;
  whatChanged: string;
  whatShouldHappen: string;
  
  evidence: {
    ipc: IpcEvidence;
    state: StateEvidence;
    visual: VisualEvidence;
    console?: ConsoleLog[];
  };
  
  conclusions: Conclusions;
  passed: boolean;
  confidence: ConfidenceLevel;
  reasoning: string;
  
  // Agent actionable output
  agentSummary: {
    shouldProceed: boolean;
    issues: string[];
    recommendations: string[];
  };
}
```

---

## Agent Integration

### How Agent Uses Verification

```markdown
# Agent Workflow with Verification

## Before Making Changes

1. Identify what needs to change
2. Define expected behavior
3. Prepare verification request

## After Making Changes

1. Run verification harness
2. Parse structured result
3. Evaluate conclusions

## Decision Making

Based on `VerificationResult.agentSummary.shouldProceed`:

- `true` → Continue with next change or commit
- `false` → Review `issues`, address `recommendations`

## Example Session

```bash
# Agent executes verification
pnpm run verify --request '{"expectedBehavior": {"backendCommand": "start_recording"}}'

# Output (structured JSON)
{
  "verificationId": "v-recording-001",
  "passed": true,
  "confidence": "high",
  "conclusions": {
    "frontend_triggered_backend": {
      "passed": true,
      "reasoning": "Frontend correctly invoked 'start_recording'"
    },
    "backend_responded_correctly": {
      "passed": true,
      "reasoning": "Backend responded with {success: true}"
    },
    "state_transition_correct": {
      "passed": true,
      "reasoning": "State transitioned 'Idle' → 'Recording'"
    },
    "events_correct": {
      "passed": true,
      "reasoning": "Event 'recording-state-changed' emitted"
    },
    "ui_updated_correctly": {
      "passed": true,
      "reasoning": "Recording indicator visible"
    }
  },
  "agentSummary": {
    "shouldProceed": true,
    "issues": [],
    "recommendations": []
  }
}
```

Agent interprets: All 5 conclusions passed. Confidence is high. `shouldProceed: true`. Proceed with commit.
```

### Verification CLI

```typescript
// scripts/verify.ts
import { program } from 'commander';

program
  .command('verify')
  .option('--request <json>', 'Verification request JSON')
  .option('--strategy <type>', 'Verification strategy: contract, mock, e2e')
  .option('--output <format>', 'Output format: json, yaml, markdown')
  .action(async (options) => {
    const request: VerificationRequest = JSON.parse(options.request);
    const harness = createHarness(options.strategy);
    
    const evidence = await harness.execute(request);
    const decision = new DecisionEngine().evaluate(request, evidence);
    
    const output = formatOutput(decision, options.output);
    console.log(output);
    
    // Write to file for agent to read
    writeFileSync(
      `.verification-results/${request.verificationId}.json`,
      JSON.stringify(decision, null, 2)
    );
    
    // Exit code indicates pass/fail
    process.exit(decision.passed ? 0 : 1);
  });

function createHarness(strategy: string): Harness {
  switch (strategy) {
    case 'contract':
      return new ContractHarness();
    case 'mock':
      return new MockHarness();
    case 'e2e':
      return new E2eHarness();
    default:
      throw new Error(`Unknown strategy: ${strategy}`);
  }
}
```

---

## Verification Patterns

### Pattern 1: Frontend Button Click → Backend Command

```yaml
verification_request:
  verification_id: "v-click-to-command-001"
  change_context:
    change_type: "frontend"
    change_description: "Updated record button click handler"
  expected_behavior:
    frontend_action: "click record button"
    backend_command: "start_recording"
    state_transition: ["Idle", "Recording"]
    events_expected: ["recording-state-changed"]
    ui_change: "recording indicator visible"
  strategy: "mock"
  confidence_threshold: "high"
```

### Pattern 2: Backend State Change → Frontend Event

```yaml
verification_request:
  verification_id: "v-state-to-event-001"
  change_context:
    change_type: "backend"
    change_description: "Modified recording state transition logic"
  expected_behavior:
    state_transition: ["Recording", "Processing"]
    events_expected: ["recording-state-changed"]
  strategy: "contract"
  confidence_threshold: "high"
```

### Pattern 3: IPC Response → UI Update

```yaml
verification_request:
  verification_id: "v-response-to-ui-001"
  change_context:
    change_type: "ipc"
    change_description: "Changed transcription response format"
  expected_behavior:
    backend_command: "stop_recording"
    events_expected: ["transcription-complete"]
    ui_change: "transcription output populated"
  strategy: "e2e"
  confidence_threshold: "high"
```

---

## Integration with AGENTS.md

Add verification protocol to agent rules:

```markdown
## Verification Before Commit

Before committing changes that affect:

1. IPC commands (frontend → backend)
2. Event emission (backend → frontend)
3. State transitions
4. UI state updates

Run verification:

```bash
pnpm run verify --request '{"expectedBehavior": {...}}'
```

If `VerificationResult.passed === false`:
- DO NOT commit
- Review `issues` and `recommendations`
- Fix and re-run verification

If `VerificationResult.passed === true`:
- Review `reasoning` for each conclusion
- Ensure `confidence` meets threshold
- Proceed with commit
```

---

## Summary

The Agent Verification System provides:

1. **Structured Requests**: Define what to verify in machine-readable format
2. **Evidence Collection**: IPC traces, state snapshots, visual evidence
3. **Decision Engine**: Automated conclusion generation with reasoning
4. **Agent Summary**: Direct `shouldProceed` boolean with actionable recommendations
5. **Confidence Levels**: Transparent evidence quality assessment

Agents can now verify that frontend interactions correctly trigger backend logic with **explainable, evidence-backed conclusions** rather than opaque pass/fail results.