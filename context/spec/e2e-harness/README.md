# E2E Testing & Harness Engineering Specification

## Overview

This directory contains the complete design specification for E2E testing and Harness Engineering for AriaType (Tauri application).

## Problem Statement

When AI agents modify code, they need to verify:
- Did frontend interactions correctly trigger backend logic?
- Did backend respond correctly to frontend calls?
- Is the IPC contract maintained?
- Are state transitions correct?
- Are events emitted in correct sequence?

Traditional testing gives "pass/fail" but no structured evidence. **Agents need explainable verification**.

## Documents

| Document | Description | Reading Order |
|----------|-------------|---------------|
| [`tauri-e2e-testing-harness-design.md`](./tauri-e2e-testing-harness-design.md) | Comprehensive research and architecture design | 1 (Start here) |
| [`harness-engineering-guide.md`](./harness-engineering-guide.md) | Implementation patterns and principles | 2 |
| [`agent-verification-system-design.md`](./agent-verification-system-design.md) | How agents use verification system | 3 |
| [`implementation-roadmap.md`](./implementation-roadmap.md) | Phased implementation plan | 4 |

## Key Concepts

### Harness Engineering

Structured approach to creating test frameworks that:
1. **Agents can execute autonomously**
2. **Produce structured, interpretable results** (JSON)
3. **Verify multi-layer interactions** (frontend → backend → frontend)
4. **Provide evidence for assertions** (screenshots, state snapshots, event logs)

### Multi-Layer Harness

| Layer | Purpose | Platform |
|-------|---------|----------|
| Contract Harness | Backend-only verification | All (Rust) |
| Mock Harness | Frontend-only verification | All (TypeScript) |
| E2E Playwright MCP | Full stack verification | macOS |
| WebDriver Harness | Full stack verification | Linux/Windows |

### Agent Verification Protocol

```
1. FRAME: Define what needs verification
2. SELECT: Choose appropriate harness type
3. EXECUTE: Run harness with structured output
4. INTERPRET: Parse VerificationResult JSON
5. DECIDE: Based on evidence, what next?
6. REPORT: Document findings
```

## VerificationResult Schema

```typescript
interface VerificationResult {
  verificationId: string;
  passed: boolean;
  confidence: 'low' | 'medium' | 'high';
  
  evidence: {
    ipcCalls: IpcTrace[];
    eventsEmitted: EventTrace[];
    stateTransitions: StateTransition[];
    screenshots?: string[];
  };
  
  conclusions: {
    frontend_triggered_backend: Conclusion;
    backend_responded_correctly: Conclusion;
    state_transition_correct: Conclusion;
    events_correct: Conclusion;
    ui_updated_correctly: Conclusion;
  };
  
  reasoning: string;
  
  agentSummary: {
    shouldProceed: boolean;
    issues: string[];
    recommendations: string[];
  };
}
```

## Quick Start

### Contract Harness (Backend)

```bash
cd apps/desktop/src-tauri
cargo test --test ipc_contract_test -- --format json
```

### Mock Harness (Frontend)

```bash
pnpm run test:harness:mock
```

### Full Verification

```bash
pnpm run verify --request '{"expectedBehavior": {"backendCommand": "start_recording"}}'
```

## Implementation Status

| Phase | Status | ETA |
|-------|--------|-----|
| Phase 1: Foundation | Not started | Week 1 |
| Phase 2: Backend Coverage | Not started | Week 2 |
| Phase 3: Frontend Mock | Not started | Week 3 |
| Phase 4: Decision Engine | Not started | Week 4 |
| Phase 5: E2E Integration | Not started | Week 5-6 |

## Related Documentation

- [Testing Specification](../testing.md) — Existing test policy
- [Testing Guide](../../guides/testing.md) — How to write tests
- [Data Flow](../../architecture/data-flow.md) — IPC communication patterns
- [Architecture Layers](../../architecture/layers.md) — Backend/frontend structure

## Research Sources

- [Tauri WebDriver Documentation](https://tauri.app/develop/tests/webdriver/)
- [Tauri Mock API Documentation](https://tauri.app/develop/tests/mocking/)
- [WebDriver Example Repository](https://github.com/tauri-apps/webdriver-example)
- Project existing tests: `apps/desktop/src-tauri/tests/e2e_test.rs`, `pipeline_integration_test.rs`