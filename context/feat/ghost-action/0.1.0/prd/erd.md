# Ghost-Action Feature Specification

## Version

- Feature: `ghost-action`
- Version: `0.1.0`
- Status: Draft (deferred)

## Overview

Ghost-Action is one of two Ghost modules. It provides macOS computer-use capability via Ghost OS MCP server integration, enabling users to delegate UI operations (click, type, scroll, execute workflows) after transcription.

See also:
- **Ghost-Language** (`context/feat/ghost-language/0.1.0/prd/erd.md`) — Language habit learning, personalization for STT and Polish

## Problem

After transcription completes, users want to act on the transcribed text—send an email, post a message, fill a form, or navigate to a URL. Currently, AriaType only injects text at the cursor position. The user must manually switch apps, find the right field, and paste.

This manual handoff breaks the voice-first workflow. Users speak, see the text, then perform a series of mouse/keyboard operations themselves. For frequent workflows like "send email to X about Y" or "search arXiv for Z", this friction accumulates.

## Goal

Integrate Ghost OS (a macOS computer-use MCP server) into AriaType so that after transcription, users can delegate follow-up UI operations to Ghost via voice commands or UI triggers. AriaType becomes an orchestrator: capture speech → transcribe → route to Ghost for execution.

## First-Principles Model

The integration must answer three questions:

1. **How does AriaType talk to Ghost?** — Process-level integration via MCP protocol (stdio), not FFI or library embedding.
2. **When does Ghost get involved?** — After transcription completes, when user explicitly requests action execution.
3. **What does Ghost do?** — Execute recipes (learned workflows) or direct tool calls (click, type, scroll) on macOS apps.

Key insight: Ghost OS is a Swift program that runs as an independent MCP server. AriaType (Rust) cannot embed Swift code directly. The only clean integration path is process orchestration via MCP's stdio transport.

## Information Architecture

### System Components

```
┌─────────────────────────────────────────────────────────────────────┐
│                     AriaType App (Tauri v2)                         │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                     Frontend (React)                         │   │
│  │   - Ghost status indicator (installed, running, permissions) │   │
│  │   - Transcription complete → "Execute with Ghost" button     │   │
│  │   - Recipe selector UI                                       │   │
│  │   - Ghost task progress toast                                │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                              │ IPC                                  │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                     Backend (Rust)                           │   │
│  │  ┌───────────────────────────────────────────────────────┐  │   │
│  │  │ ghost/ (new module)                                    │  │   │
│  │  │   ├── manager.rs    — GhostManager lifecycle           │  │   │
│  │  │   ├── client.rs     — MCP client via rmcp              │  │   │
│  │  │   ├── recipe.rs     — Recipe definitions               │  │   │
│  │  │   ├── intent.rs     — Intent router (text → action)    │  │   │
│  │  │   └── types.rs      — Ghost data types                 │  │   │
│  │  └───────────────────────────────────────────────────────┘  │   │
│  │                              │ stdio (MCP JSON-RPC)          │   │
│  │  ┌───────────────────────────────────────────────────────┐  │   │
│  │  │ Ghost OS subprocess (Swift)                            │  │   │
│  │  │   - 29 tools: ghost_context, ghost_click, ghost_type   │  │   │
│  │  │   - Recipes: gmail-send, arxiv-download, slack-message │  │   │
│  │  │   - Vision fallback: ShowUI-2B (local)                 │  │   │
│  │  └───────────────────────────────────────────────────────┘  │   │
│  └─────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
```

### User Flow

```
User holds hotkey → Recording → Transcription → Polish (optional)
                                                      │
                                                      ▼
                                              TranscriptionResult
                                                      │
                      ┌───────────────────────────────┼───────────────────────────────┐
                      │                               │                               │
                      ▼                               ▼                               ▼
              Auto-inject at cursor           Save to history            "Execute with Ghost"
              (existing behavior)             (existing behavior)                 │
                                                                              │
                                                                              ▼
                                                                      Intent Router
                                                                      (parse intent from text
                                                                       or explicit user request)
                                                                              │
                                                                              ▼
                                                                      Recipe or Tool Selection
                                                                              │
                                                                              ▼
                                                                      GhostManager.execute()
                                                                              │
                                                                              ▼
                                                                      Ghost OS subprocess
                                                                      executes UI actions
                                                                              │
                                                                              ▼
                                                                      Result → Toast notification
```

### Ghost-Action Module Structure

| Layer | File | Responsibility |
|-------|------|----------------|
| `types/` | `ghost/action/types.rs` | GhostTool, GhostRecipe, GhostResult, GhostStatus data structures |
| `client/` | `ghost/action/client.rs` | MCP client wrapper using rmcp SDK, connection lifecycle |
| `recipe/` | `ghost/action/recipe.rs` | Recipe registry, parameter extraction, validation |
| `intent/` | `ghost/action/intent.rs` | Intent router: parse text/user input → recipe/tool call |
| `manager/` | `ghost/action/manager.rs` | Process spawning, lifecycle, health checks, state machine |
| `command/` | `commands/ghost_action.rs` | Tauri IPC handlers (ghost_action_status, ghost_action_execute, ghost_action_list_recipes) |

### Layer Compliance

The Ghost-Action module follows the backend layer model from `architecture/layers.md`:

```
ghost/action/types.rs     — pure data structures
        ↓
ghost/action/client.rs    — MCP client, rmcp integration
        ↓
ghost/action/recipe.rs    — recipe parsing, parameter extraction
        ↓
ghost/action/intent.rs    — intent routing logic
        ↓
ghost/action/manager.rs   — process orchestration, state machine
↓
commands/ghost_action.rs  — Tauri IPC adapters
        ↓
events/ghost_action.rs    — backend → frontend events (ghost-action-task-progress, ghost-action-task-complete)
```

Boundary rule enforcement:
- `ghost/action/manager.rs` may spawn processes but NOT emit Tauri events directly
- `commands/ghost_action.rs` receives manager results and emits events via Tauri handle
- No `ghost/action/` → `commands/` reverse dependency

## Data Contract

### GhostActionStatus

```typescript
interface GhostActionStatus {
  installed: boolean;          // ghost CLI available on system
  running: boolean;            // subprocess active
  healthy: boolean;            // MCP connection established
  permissions: {
    accessibility: boolean;    // macOS Accessibility permission
    screen_recording: boolean; // macOS Screen Recording permission
    input_monitoring: boolean; // macOS Input Monitoring permission (for learning)
  };
  recipes_count: number;       // Number of installed recipes
  version: string;             // Ghost OS version (e.g., "2.2.1")
}
```

### GhostActionRecipe

```typescript
interface GhostActionRecipe {
  name: string;                // Recipe identifier (e.g., "gmail-send")
  description: string;         // Human-readable description
  parameters: GhostRecipeParam[];
  steps: number;               // Number of steps in the recipe
}

interface GhostRecipeParam {
  name: string;                // Parameter name (e.g., "recipient")
  type: "string" | "number" | "boolean";
  required: boolean;
  description: string;
}
```

### GhostActionExecuteRequest

```typescript
interface GhostActionExecuteRequest {
  mode: "recipe" | "tool" | "intent";
  // When mode = "recipe"
  recipe?: string;
  params?: Record<string, string | number | boolean>;
  // When mode = "tool"
  tool?: string;
  tool_args?: Record<string, unknown>;
  // When mode = "intent"
  text?: string;               // Transcribed text to route
  context?: {
    source_app?: string;       // Where user was before recording
    timestamp?: number;
  };
}
```

### GhostActionExecuteResult

```typescript
interface GhostActionExecuteResult {
  success: boolean;
  mode: "recipe" | "tool" | "intent";
  recipe?: string;
  tool?: string;
  steps_executed: number;
  duration_ms: number;
  error?: string;
  output?: string;             // Final output from Ghost
}
```

### GhostTool (from Ghost OS)

The 29 Ghost OS tools available via MCP:

| Category | Tools |
|----------|-------|
| **Perception** | `ghost_context`, `ghost_state`, `ghost_find`, `ghost_read`, `ghost_inspect`, `ghost_element_at` |
| **Vision** | `ghost_screenshot`, `ghost_annotate`, `ghost_ground`, `ghost_parse_screen` |
| **Action** | `ghost_click`, `ghost_hover`, `ghost_long_press`, `ghost_drag`, `ghost_type`, `ghost_press`, `ghost_hotkey`, `ghost_scroll` |
| **Window** | `ghost_focus`, `ghost_window` |
| **Wait** | `ghost_wait` |
| **Recipe** | `ghost_recipes`, `ghost_run`, `ghost_recipe_show`, `ghost_recipe_save`, `ghost_recipe_delete` |
| **Learning** | `ghost_learn_start`, `ghost_learn_stop`, `ghost_learn_status` |

### Backend Events

| Event | Payload | Trigger |
|-------|---------|---------|
| `ghost-action-status-changed` | `GhostActionStatus` | Ghost subprocess lifecycle changes |
| `ghost-action-task-started` | `{ task_id, mode, recipe/tool }` | Task execution begins |
| `ghost-action-task-progress` | `{ task_id, step, total_steps, message }` | Progress updates |
| `ghost-action-task-complete` | `GhostActionExecuteResult` | Task finishes |
| `ghost-action-task-error` | `{ task_id, error }` | Task fails |

## Acceptance Criteria

1. **Ghost OS Detection**: On app startup, backend detects if Ghost OS CLI (`ghost`) is installed and reports status to frontend.
2. **Permission Check**: Backend runs `ghost doctor` to verify macOS Accessibility, Screen Recording, and Input Monitoring permissions.
3. **Subprocess Lifecycle**: Ghost OS subprocess starts on first Ghost-Action request or on app startup if enabled in settings. Process terminates on app shutdown.
4. **MCP Connection**: Backend establishes MCP client connection to Ghost OS subprocess via stdio using rmcp SDK.
5. **Recipe Discovery**: Backend exposes `ghost_action_list_recipes` command that returns all installed Ghost OS recipes.
6. **Recipe Execution**: Backend exposes `ghost_action_execute` command that calls `ghost_run` tool with parameter substitution.
7. **Direct Tool Call**: Backend exposes `ghost_action_call_tool` command for direct tool invocation without recipe.
8. **Intent Router**: Backend parses user intent from transcribed text and routes to appropriate recipe or tool.
9. **Progress Events**: Backend emits `ghost-action-task-progress` events during execution.
10. **Error Handling**: Backend handles Ghost OS subprocess crashes, MCP connection failures, and tool execution errors with clear error messages.
11. **Headless Architecture**: All Ghost-Action logic lives in backend; frontend only displays status and triggers commands.
12. **Settings Integration**: New settings section for Ghost: enable/disable, auto-start, preferred recipes.
13. **i18n**: All user-facing Ghost-Action UI copy is internationalized.

## BDD Scenarios

### Scenario: Ghost OS not installed

- Given Ghost OS CLI is not installed on the system
- When AriaType starts
- Then `ghost_action_status` returns `{ installed: false, running: false, healthy: false }`
- And frontend shows "Ghost not installed" with installation instructions link

### Scenario: Ghost OS installed but missing permissions

- Given Ghost OS CLI is installed
- And macOS Accessibility permission is not granted
- When AriaType starts
- Then `ghost_action_status` returns `{ installed: true, permissions: { accessibility: false } }`
- And frontend shows permission prompt with guidance

### Scenario: Start Ghost OS subprocess on demand

- Given Ghost OS is installed and all permissions granted
- And Ghost-Action is disabled in settings (auto-start = false)
- When user clicks "Execute with Ghost" after transcription
- Then backend starts Ghost OS subprocess
- And establishes MCP connection
- And executes the requested task
- And subprocess terminates after task completion (or stays alive if configured)

### Scenario: Execute recipe with transcribed text

- Given Ghost OS subprocess is running
- And recipe "gmail-send" is installed
- When user transcribes "Send an email to john@example.com about the Q4 report"
- And triggers Ghost-Action execution with mode = "intent"
- Then Intent Router extracts `{ recipient: "john@example.com", subject: "Q4 report", body: [transcribed text] }`
- And calls `ghost_run` with recipe = "gmail-send" and params
- And Gmail compose window opens, fields fill, email sends
- And frontend receives `ghost-action-task-complete` with result

### Scenario: Direct tool call for simple action

- Given Ghost OS subprocess is running
- When user wants to click a specific button
- And frontend sends `ghost_action_call_tool({ tool: "ghost_click", args: { name: "Submit" } })`
- Then backend invokes `ghost_click` tool via MCP
- And returns result to frontend

### Scenario: Ghost OS subprocess crash recovery

- Given Ghost OS subprocess is running
- When subprocess crashes unexpectedly
- Then backend detects crash via MCP connection error
- And emits `ghost-action-status-changed` with `{ running: false, healthy: false }`
- And frontend shows "Ghost disconnected" notification
- And backend optionally restarts subprocess if auto-restart is enabled

### Scenario: Learn new workflow

- Given Ghost OS subprocess is running
- And Input Monitoring permission is granted
- When user triggers `ghost_learn_start` with task description
- Then Ghost OS starts recording user actions
- And user performs the workflow manually
- And user triggers `ghost_learn_stop`
- Then Ghost OS returns learned action sequence
- And backend saves as new recipe via `ghost_recipe_save`

## Verification

### Backend Tests

```bash
# Rust unit tests for ghost-action module
cargo test ghost::action:: --no-fail-fast

# Integration test with mock Ghost OS subprocess
cargo test ghost_action_integration::

# MCP client connection test
cargo test mcp_client::
```

### Frontend Tests

```bash
# Component tests for Ghost-Action UI
pnpm --filter @ariatype/desktop test ghost-action

# Build verification
pnpm --filter @ariatype/desktop build
```

### Manual Verification

1. Install Ghost OS: `brew install ghostwright/ghost-os/ghost-os && ghost setup`
2. Run AriaType, verify Ghost-Action status shows installed + permissions granted
3. Transcribe text, click "Execute with Ghost", select recipe
4. Verify Ghost OS executes the workflow and UI updates
5. Kill Ghost OS subprocess, verify AriaType handles gracefully

## Implementation Notes

### rmcp SDK Integration

```rust
// Cargo.toml addition
rmcp = { version = "0.16.0", features = ["client"] }

// ghost/action/client.rs
use rmcp::{ServiceExt, transport::TokioChildProcess};
use tokio::process::Command;

pub struct GhostActionClient {
    client: rmcp::service::Service<RoleClient>,
}

impl GhostActionClient {
    pub async fn connect() -> Result<Self, GhostActionError> {
        let client = ().serve(TokioChildProcess::new(
            Command::new("ghost")  // Ghost OS CLI
        ))?).await?;
        Ok(Self { client })
    }

    pub async fn call_tool(&self, name: &str, args: serde_json::Value) -> Result<ToolResult, GhostActionError> {
        let result = self.client.call_tool(CallToolRequestParams {
            name: name.into(),
            arguments: Some(args),
        }).await?;
        Ok(result)
    }

    pub async fn list_tools(&self) -> Result<Vec<ToolInfo>, GhostActionError> {
        let result = self.client.list_all_tools().await?;
        Ok(result.tools)
    }
}
```

### Ghost-Action Manager State Machine

```
GhostActionManagerState

NotInstalled
  │
  │ Ghost OS CLI detected
  ▼
Installed
  │
  │ start() called
  ▼
Starting
  │
  │ subprocess spawned
  ▼
Connecting
  │
  │ MCP handshake complete
  ▼
Ready
  │
  │ execute() called
  ▼
Executing
  │
  │ task complete OR task failed
  ▼
Ready

Error States:
  PermissionDenied  — macOS permissions not granted
  StartFailed       — subprocess spawn error
  ConnectionFailed  — MCP handshake timeout
  Crash             — subprocess terminated unexpectedly
```

### Intent Router Design

The Intent Router maps user intent to Ghost-Action calls:

```
Input: transcribed text + optional explicit command
Output: { mode, recipe/tool, params }

Examples:
"Send email to X about Y" → recipe: gmail-send, params: { recipient: X, subject: Y }
"Search arXiv for Z" → recipe: arxiv-download, params: { query: Z }
"Click the Submit button" → tool: ghost_click, args: { name: "Submit" }
"Type hello in the search field" → tool: ghost_type, args: { field: "search", text: "hello" }
```

Implementation approach (v0.1.0):
- Rule-based intent detection (regex/pattern matching)
- Future: LLM-based intent parsing (use existing Polish engine infrastructure)

### Settings Schema Addition

```typescript
// Settings type extension
interface Settings {
  // ... existing fields
  ghost_action: {
    enabled: boolean;           // Ghost-Action feature on/off
    auto_start: boolean;        // Start Ghost OS subprocess on app launch
    auto_restart: boolean;      // Restart on crash
    keep_alive: boolean;        // Keep subprocess running after task
    preferred_recipes: string[]; // User's favorite recipes
  };
}
```

### File Structure

```
apps/desktop/src-tauri/src/
├── ghost/
│   ├── mod.rs                 # Exports action and language submodules
│   ├── action/
│   │   ├── mod.rs
│   │   ├── types.rs
│   │   ├── client.rs
│   │   ├── recipe.rs
│   │   ├── intent.rs
│   │   └── manager.rs
│   └── language/              # Ghost-Language module (see ghost-language spec)
│       ├── mod.rs
│       ├── types.rs
│       ├── learner.rs
│       ├── adapter.rs
│       └── storage.rs
├── commands/
│   ├── ghost_action.rs        # Ghost-Action IPC handlers
│   ├── ghost_language.rs      # Ghost-Language IPC handlers
│   └── mod.rs                 # Register ghost modules
├── events/
│   ├── ghost_action.rs        # Ghost-Action event types
│   └── ghost_language.rs      # Ghost-Language event types
├── lib.rs                     # Register ghost commands
└─────────────────────────────────────────────────

apps/desktop/src/
├── components/
│   ├── Ghost/
│   │   ├── GhostStatus.tsx         # Unified Ghost status (action + language)
│   │   ├── GhostActionRecipeSelector.tsx
│   │   ├── GhostActionExecuteButton.tsx
│   │   └── GhostLanguageProgress.tsx
├── lib/
│   └── tauri.ts               # Add ghost_action_* and ghost_language_* invoke calls
├── i18n/locales/
│   ├── en.json                # Add ghost.action.* and ghost.language.* keys
│   └── ...                    # All 10 locales
```

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| Ghost OS subprocess crash | Auto-restart option; graceful error events; crash detection via MCP timeout |
| MCP protocol version mismatch | Use official rmcp SDK; pin rmcp version; protocol version negotiation |
| macOS permission friction | Clear UI guidance; link to `ghost setup` instructions |
| Intent routing accuracy | Start with rule-based; fallback to manual recipe selection; future LLM integration |
| Cross-process race conditions | Single GhostActionManager instance; mutex on subprocess handle; task queue |
| Ghost OS updates breaking integration | Version check; deprecation warning if incompatible; CI test against Ghost releases |

## Dependencies

### External

- Ghost OS: `brew install ghostwright/ghost-os/ghost-os` (macOS only)
- rmcp: `rmcp = { version = "0.16.0", features = ["client"] }` (Rust MCP SDK)

### Internal

- `state/unified_state.rs` — GhostActionManager state integration
- `events/` — Ghost-Action event emission
- `commands/` — IPC handler registration
- `services/transcription_finalize.rs` — Hook for post-transcription Ghost-Action trigger

## Platform Limitation

Ghost OS is macOS-only. This feature is unavailable on Windows and Linux. The backend must handle this gracefully:

```rust
#[cfg(target_os = "macos")]
mod action;

#[cfg(not(target_os = "macos"))]
mod action {
    pub fn status() -> GhostActionStatus {
        GhostActionStatus {
            installed: false,
            running: false,
            healthy: false,
            error: Some("Ghost OS is only available on macOS"),
            ..Default::default()
        }
    }
}
```

## Out of Scope (v0.1.0)

- LLM-based intent routing (use rule-based first)
- Custom recipe creation UI (use `ghost learn` via CLI)
- Ghost UI window overlay (future: show Ghost OS actions in real-time)
- Multi-recipe chaining (single recipe execution per request)
- Ghost result history (transcription history only)

## Future Enhancements

1. **LLM Intent Router** — Use Polish engine infrastructure for smarter intent detection
2. **Ghost Overlay UI** — Show Ghost OS actions in a small overlay window
3. **Recipe Recommendation** — Suggest recipes based on user context
4. **Ghost-Action History** — Record Ghost OS execution history separately
5. **Workflow Chaining** — Execute multiple recipes in sequence
6. **Ghost Learning UI** — In-app workflow learning (no CLI required)