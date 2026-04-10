# Architectural Layers

## Layer Model

Within each domain, code depends "forward" through layers. Cross-cutting concerns enter through a single explicit interface.

```
Types → Config → Repo/State → Service → Runtime → UI
```

## Backend Layers (Rust)

```
types/          # Data structures, no logic (request/response types, enums)
    ↓
config/         # Settings, defaults, configuration helpers
    ↓
state/          # unified_state.rs — single source of runtime truth
    ↓
engines/        # stt_engine/*, polish_engine/* — implement traits
    ↓
commands/       # Tauri IPC handlers — thin layer, delegates to services
    ↓
events/         # backend → frontend event emission
```

### Layer Responsibilities

| Layer | Responsibility |
|-------|----------------|
| `types/` | Pure data structures, serialization/deserialization |
| `config/` | Settings validation, default values, config file handling |
| `state/` | Runtime state container (StreamingSttState, settings) |
| `engines/` | Business logic, trait implementations for STT/Polish |
| `commands/` | IPC handler, minimal logic, delegates to services |
| `events/` | Async event emission to frontend |

## Frontend Layers (TypeScript/React)

```
types/          # From @ariatype/shared + local type definitions
    ↓
lib/            # tauri.ts IPC boundary, logger.ts
    ↓
contexts/       # SettingsContext — app-wide state
    ↓
hooks/          # Custom React hooks
    ↓
components/     # UI components
```

## Cross-Cutting Concerns

| Concern | Entry Point |
|---------|-------------|
| Authentication | Cloud config in settings, passed to engine constructors |
| Telemetry | Logging via `tracing` (backend) and `logger.ts` (frontend) |
| i18n | `src/i18n/locales/` with 10 languages |
| Feature Flags | Settings flags (cloud_stt_enabled, local_stt_enabled, polish_enabled) |

## Boundary Rules

| Rule | Enforcement |
|------|-------------|
| `src-tauri/capabilities/` — never modify without explicit request | Manual review |
| `lib.rs` — commands registered here | Manual review |
| `src/lib/tauri.ts` — all IPC calls go here | TypeScript strict mode |
| `audio/` → `stt_engine/` — recorder must be agnostic to engine type; engine-specific logic stays in engine implementations | Code review |
| Frontend never calls raw `invoke()` | Lint rule in `tsconfig.json` |
| Backend never imports frontend code | Rust module system |
| `packages/shared/` has zero runtime dependencies | No imports from apps/ or packages/ |

## Dependency Rules

1. **No backward dependencies** — A layer cannot depend on layers "behind" it
2. **No cross-domain dependencies** — Audio cannot depend on UI, STT engine cannot depend on text_injector
3. **Boundaries are enforced by the module system** — Rust crate boundaries, TypeScript module boundaries

## Enforcement Mechanisms

| Mechanism | What it catches |
|-----------|-----------------|
| `cargo clippy --all-features -- -D warnings` | Rust layer violations, incorrect error handling |
| `cargo fmt -- --check` | Rust formatting, discourages large files |
| TypeScript strict mode (`strict: true`) | Frontend layer violations, any-type avoidance |
| `tsc --noEmit` | Type errors, missing imports from tauri.ts |
| `oxlint` | TypeScript/React best practices |
| Manual architecture review | Cross-domain dependencies, trait violations |

## Key Architectural Files

| File | Layer | Purpose |
|------|-------|---------|
| `stt_engine/traits.rs` | engine | Defines unified SttEngine trait (send_chunk + finish) |
| `stt_engine/unified_manager.rs` | engine | Engine lifecycle, selection logic |
| `polish_engine/unified_manager.rs` | engine | Polish engine lifecycle |
| `state/unified_state.rs` | state | Single source of runtime truth |
| `commands/settings/mod.rs` | command | Settings persistence |
| `lib.rs` | command | ALL command registrations |
| `src/lib/tauri.ts` | lib | Typed IPC boundary |
| `src/contexts/SettingsContext.tsx` | context | App-wide settings state |
