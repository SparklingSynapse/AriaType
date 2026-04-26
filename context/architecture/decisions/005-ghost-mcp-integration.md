# ADR-005: Ghost Concept Architecture

**Date**: 2025-04
**Status**: Proposed

## Decision

Introduce a unified "Ghost" concept with two independent modules:

1. **Ghost-Action** — macOS computer-use via Ghost OS MCP server (Swift subprocess, process-level integration)
2. **Ghost-Language** — Language habit learning for STT/Polish personalization (fully local, no external dependencies)

Both modules share a unified product brand ("your Ghost learns and helps") but are technically independent with separate code paths, data models, and lifecycles.

## Rationale

### Why unify under Ghost concept

| Dimension | Analysis |
|-----------|----------|
| **Product simplicity** | One concept: "Ghost follows you, learns your habits, helps you" |
| **User mental model** | Ghost-Language = learns your words; Ghost-Action = learns your workflows |
| **Brand coherence** | Ghost (幽灵) implies invisible companion, continuous learning, silent help |
| **Technical independence** | Two modules don't share state; can be enabled/disabled separately |

### Ghost-Action: Why MCP process integration

| Approach | Feasibility | Tradeoffs |
|----------|-------------|-----------|
| **FFI (Swift → Rust)** | Not feasible | Swift cannot compile into Rust; ABI instability |
| **MCP stdio (chosen)** | Clean | Standard protocol, rmcp SDK, process isolation, independent updates |

Ghost OS already exposes MCP server via stdio. rmcp provides Rust client. This is the natural path.

### Ghost-Language: Why fully local

| Approach | Tradeoffs |
|----------|-----------|
| **Local learning (chosen)** | Privacy-first, no cloud dependency, works offline |
| **Cloud learning** | Latency, privacy concerns, requires sync infrastructure |

Language learning is sensitive (vocabulary, style patterns). Local storage aligns with AriaType's privacy-first principle.

## Alternatives Considered

1. **Separate features (Forge + Ghost)**: Two concepts confuse users; no unified brand
2. **Cloud-based language learning**: Privacy concerns, latency, offline unavailable
3. **Browser-only computer-use (Browser-Use)**: Cannot operate native macOS apps
4. **Anthropic Computer Use API**: Cloud dependency, screenshot approach, API costs

## Consequences

### Architecture changes

- New `ghost/` module with two submodules:
  - `ghost/action/` — Ghost OS MCP client (6 files: types, client, recipe, intent, manager, stats)
  - `ghost/language/` — Language learning (6 files: types, learner, adapter, storage, stats, decay)
- New IPC handlers: `commands/ghost_action.rs`, `commands/ghost_language.rs`
- New events: `events/ghost_action.rs`, `events/ghost_language.rs`
- rmcp dependency for Ghost-Action
- SQLite/JSON storage for Ghost-Language profile

### Platform differences

| Module | Platform Support |
|--------|------------------|
| **Ghost-Action** | macOS only (Ghost OS is Swift/macOS-only) |
| **Ghost-Language** | All platforms (fully local, no external dependencies) |

### Product integration

- Unified UI: "Ghost" settings section, one status indicator
- Separate toggles: ghost_action.enabled, ghost_language.enabled
- Learning progress UI: shows both correction count and workflow count
- "Ghost learns your words" + "Ghost learns your workflows"

### Data flow extension

```
STT → [Ghost-Language Adapter] → Corrected STT → Polish → [Personalized Prompt] → Output
                                                            ↓
                                                 History Edit → [Ghost-Language Learner]
                                                            ↓
                                                 User workflow → [Ghost-Action Intent Router]
                                                            ↓
                                                 Ghost OS → macOS UI actions
```

### Future considerations

- Ghost-Language may add cloud sync for cross-device profiles
- Ghost-Action may add overlay UI for real-time action visualization
- Both modules may share a unified "Ghost Memory" storage layer

## References

- Ghost OS: https://github.com/ghostwright/ghost-os
- rmcp (Rust MCP SDK): https://github.com/modelcontextprotocol/rust-sdk
- MCP Specification: https://modelcontextprotocol.io/specification/latest
- Ghost-Action Spec: [`feat/ghost-action/0.1.0/prd/erd.md`](../../feat/ghost-action/0.1.0/prd/erd.md)
- Ghost-Language Spec: [`feat/ghost-language/0.1.0/prd/erd.md`](../../feat/ghost-language/0.1.0/prd/erd.md)