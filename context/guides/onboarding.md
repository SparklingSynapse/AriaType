# Onboarding Guide

Welcome to AriaType. This guide helps you (human contributor or AI agent) quickly understand the project and become productive.

---

## What is AriaType?

AriaType is a **local-first voice keyboard for macOS and Windows**. Hold a hotkey, speak naturally, and release—the app transcribes your speech and types it into any active application. Powered by optimized local AI models for STT (Whisper, SenseVoice) and text polish.

**Tech stack**: Tauri v2 (Rust backend + React 19 frontend), strict TypeScript, Tailwind CSS, zero runtime dependencies in shared package.

**Core workflow**: Audio capture → Speech-to-text → Text polish → Cursor injection

---

## Architecture at a Glance

```
┌─────────────────────────────────────────────────────────────────┐
│                    User Interface Layer                         │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐                        │
│  │  main.tsx│ │ pill.tsx │ │toast.tsx │  (React 19)            │
│  └──────────┘ └──────────┘ └──────────┘                        │
│         └──────────────────┬───────────────────────────────────│
│                            │ src/lib/tauri.ts (IPC boundary)    │
├────────────────────────────┼────────────────────────────────────┤
│                    Command Layer                                │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │ commands/ (Tauri IPC handlers)                              ││
│  └─────────────────────────────────────────────────────────────┘│
├────────────────────────────┼────────────────────────────────────┤
│                    State Layer                                  │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │ state/unified_state.rs (single source of runtime truth)     ││
│  └─────────────────────────────────────────────────────────────┘│
├────────────────────────────┼────────────────────────────────────┤
│                    Engine Layer                                 │
│  ┌────────────┐ ┌─────────────┐ ┌─────────────────────────────┐│
│  │   audio/   │ │ stt_engine/ │ │      polish_engine/         ││
│  │ recorder   │ │ whisper     │ │      lfm, qwen, cloud       ││
│  │ resampler  │ │ sense_voice │ │                             ││
│  │ vad, beep  │ │ cloud STT   │ │                             ││
│  └────────────┘ └─────────────┘ └─────────────────────────────┘│
├────────────────────────────┼────────────────────────────────────┤
│                    Injection Layer                              │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │ text_injector/macos.rs (keyboard simulation + clipboard)    ││
│  └─────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────┘

Package dependency: packages/shared/ → apps/desktop/src/ → apps/desktop/src-tauri/
```

---

## First-Time Setup

**Prerequisites**: Node.js 18+, Rust 1.70+, pnpm 8+, macOS with Apple Silicon (M1/M2/M3/M4).

```bash
# 1. Clone repository
git clone https://github.com/joe223/AriaType.git
cd AriaType

# 2. Install dependencies
pnpm install

# 3. Start development server
pnpm --filter @ariatype/desktop tauri:dev

# 4. Verify toolchain
cd apps/desktop/src-tauri
cargo test && cargo clippy --all-features -- -D warnings
cd ../..
pnpm --filter @ariatype/desktop build
```

**Detailed setup**: See [`apps/desktop/CONTRIBUTING.md`](../../apps/desktop/CONTRIBUTING.md) for full development environment configuration.

### Desktop Dev Variant Notes

- `pnpm --filter @ariatype/desktop tauri:dev` launches the inhouse/dev app variant, not the production-branded bundle.
- The dev entrypoint regenerates inhouse icon assets automatically before launching Tauri.
- Generated inhouse app icons live in `apps/desktop/assets/icons/inhouse/`; the tray variant lives at `apps/desktop/src-tauri/assets/tray-icon-inhouse.png`.
- If the Dock or packaged dev app loses the green corner marker, check `scripts/generate-inhouse-icons.sh`, `apps/desktop/package.json`, and `apps/desktop/src-tauri/tauri.dev.conf.json` together.
- Hotkey UI intentionally distinguishes left/right modifier keys. Expect labels such as `L⌘`, `R⌘`, `LCtrl`, and `R⌥`.

---

## Key Directories

| Directory | What It Contains | When You Need It |
|-----------|------------------|------------------|
| `apps/desktop/src/` | React frontend (UI components, hooks, contexts) | Adding UI features, fixing display bugs |
| `apps/desktop/src-tauri/src/` | Rust backend (engines, commands, state) | Core logic, STT/Polish engines, IPC |
| `packages/shared/` | TypeScript types/constants (zero deps) | Shared data structures across packages |
| `packages/website/` | Next.js marketing site (static export) | Website updates, marketing content |
| `context/` | All documentation (progressive disclosure) | Understanding architecture, conventions, specs |

### Backend Module Map

| Module | Purpose |
|--------|---------|
| `audio/` | Recording, resampling, VAD, beep, level meter |
| `stt_engine/` | Whisper, SenseVoice, cloud STT (Volcengine, Deepgram, etc.) |
| `polish_engine/` | Local LFM, Qwen polish; cloud polish (Anthropic, OpenAI) |
| `services/` | Backend use cases and orchestration; may depend on state/history/engines but never on Tauri commands |
| `commands/` | Tauri IPC handlers and adapters—thin layer, delegates to services and performs side effects |
| `state/` | `unified_state.rs`—single source of runtime truth |
| `text_injector/` | Platform-specific text insertion (macOS keyboard simulation) |
| `events/` | Backend → frontend event emission |

### Frontend Module Map

| Module | Purpose |
|--------|---------|
| `main.tsx` | Settings window entry point |
| `pill.tsx` | Floating recording indicator |
| `toast.tsx` | Transient notifications |
| `lib/tauri.ts` | Typed IPC boundary—**all invoke calls go here** |
| `contexts/` | React contexts (SettingsContext) |
| `components/` | UI components organized by window (Home, Pill, Toast) |
| `hooks/` | Custom React hooks |
| `i18n/locales/` | 10 language translations |

---

## Critical Files

The 6-8 files that matter most:

| File | Why It Matters |
|------|----------------|
| `apps/desktop/src-tauri/src/lib.rs` | **All commands registered here**—missing registration = broken IPC |
| `apps/desktop/src/lib/tauri.ts` | Frontend IPC boundary—extend this, never use raw `invoke()` |
| `apps/desktop/src-tauri/src/state/unified_state.rs` | Runtime state container—single source of truth |
| `apps/desktop/src-tauri/src/stt_engine/traits.rs` | Unified `SttEngine` trait definition (send_chunk + finish) |
| `AGENTS.md` | Agent operating contract—rules, verification, coverage gates |
| `context/README.md` | Documentation map—entry points, document roles, canonical indexes |
| `context/architecture/data-flow.md` | Primary workflow, state machine, IPC contracts |

---

## How to Navigate Documentation

Documentation follows **progressive disclosure**: start shallow, go deep only when needed.

```
AGENTS.md                    # Execution constraints and verification rules
    ↓
context/README.md               # Documentation map and canonical source routing
    ↓
context/architecture/decisions/ # Architectural rationale and major decisions
context/architecture/           # System architecture, layers, data flow
context/spec/                   # Contracts, testing, logging, API boundaries
context/conventions/            # Coding conventions, design system
context/guides/                 # Debugging, adding providers
context/feat/<name>/<ver>/      # Feature specifications (source of truth)
```

**Reading order for new contributors**:

1. [`AGENTS.md`](../../AGENTS.md) — execution constraints, verification commands, and default iteration strategy
2. [`context/README.md`](../README.md) — document roles and canonical source routing
3. [`context/architecture/decisions/README.md`](../architecture/decisions/README.md) — architectural rationale and major decisions
4. [`context/spec/`](../spec/) and [`context/architecture/data-flow.md`](../architecture/data-flow.md) — contracts, invariants, and the primary workflow
5. [`context/architecture/README.md`](../architecture/README.md) — system map and package boundaries
6. [`apps/desktop/CONTRIBUTING.md`](../../apps/desktop/CONTRIBUTING.md) — dev setup details

---

## Development Workflow

1. **Find the spec** — Features driven by `context/feat/<name>/<version>/prd/erd.md`
2. **Branch** — Create feature branch from main
3. **Write failing test** — TDD/BDD: spec → failing test → implement → verify
4. **Implement** — Follow layer dependencies (Types → Config → State → Engines → Commands)
5. **Verify** — Run tests, linters, type checks (see Verification Commands below)
6. **PR** — Reference spec in PR description, include test evidence

**Coverage gates**:
- E2E: 100% for affected user workflow
- Unit: 100% for affected critical core modules

---

## Verification Commands

```bash
# Rust backend
cd apps/desktop/src-tauri
cargo test && cargo clippy --all-features -- -D warnings && cargo fmt -- --check

# Frontend
pnpm --filter @ariatype/desktop build
pnpm --filter @ariatype/shared typecheck
pnpm check:i18n

# Website
pnpm --filter @ariatype/website build && pnpm --filter @ariatype/website lint
```

---

## Common Tasks

| Task | Where to Start |
|------|----------------|
| Add a new STT provider | [`apps/desktop/src-tauri/CONTRIBUTING.md`](../../apps/desktop/src-tauri/CONTRIBUTING.md) + [`context/guides/adding-stt-provider.md`](./adding-stt-provider.md) |
| Add a new IPC command | `lib.rs` (register) + `commands/` (implement) + `src/lib/tauri.ts` (frontend wrapper) |
| Add a new UI component | `src/components/` + check `src/contexts/SettingsContext.tsx` for state |
| Fix a bug in STT engine | `stt_engine/traits.rs` (trait) + `stt_engine/unified_manager.rs` (lifecycle) |
| Add user-facing text | Update all 10 locale files in `src/i18n/locales/` |
| Debug production issue | [`context/guides/debugging.md`](./debugging.md) — log locations, crash reports |
| Understand logging requirements | [`context/spec/logs.md`](../spec/logs.md) — structured fields, lowercase messages |

---

## Mental Model: The Dependency Graph

When tracing code, follow direct dependencies—not the file tree. This reduces relevant files from hundreds to 6-8.

```
Audio → STT → Polish → Injection

audio/recorder.rs
    → audio/resampler.rs (format conversion)
    → audio/stream_processor.rs (VAD + denoise)
    → mpsc channel (audio chunk transport)

stt_engine/traits.rs::SttEngine
    → stt_engine/unified_manager.rs (engine selection)
    → stt_engine/whisper.rs OR stt_engine/cloud/ (implementation)

polish_engine/unified_manager.rs
    → polish_engine/lfm.rs OR polish_engine/cloud/ (implementation)

text_injector/macos.rs
    → keyboard simulation + clipboard fallback
```

**Key insight**: Each domain (audio, STT, polish, injection) has:
1. Trait/interface definition
2. Unified manager (lifecycle, selection)
3. Concrete implementations

---

## Boundaries You Must Respect

| Boundary | Rule | Consequence of Violation |
|----------|------|--------------------------|
| `src-tauri/capabilities/` | Never modify without explicit request | Security vulnerability, app breakage |
| `lib.rs` | Commands must be registered here | IPC calls fail silently |
| `src/lib/tauri.ts` | All IPC calls through this file | Type safety lost, debugging harder |
| `packages/shared/` | Zero runtime dependencies | Circular dependency, build failure |

---

## Where to Get Help

| Need | Document |
|------|----------|
| Agent rules and verification | [`AGENTS.md`](../../AGENTS.md) |
| Documentation entry points and canonical indexes | [`context/README.md`](../README.md) |
| Architecture layers | [`context/architecture/layers.md`](../architecture/layers.md) |
| Data flow and state machines | [`context/architecture/data-flow.md`](../architecture/data-flow.md) |
| Testing and coverage | [`context/spec/testing.md`](../spec/testing.md) |
| Logging standard | [`context/spec/logs.md`](../spec/logs.md) |
| STT engine architecture | [`apps/desktop/src-tauri/CONTRIBUTING.md`](../../apps/desktop/src-tauri/CONTRIBUTING.md) |
| Debug guide | [`context/guides/debugging.md`](./debugging.md) |

---

## Product Priority

**Always remember**: `STT accuracy > STT stability > user experience > speed`

- Never accept latency gains that reduce accuracy or stability
- Speed optimizations only after accuracy, stability, and UX are protected
- When in doubt: prefer reliability, validation, clearer state, safer fallback

---

## Next Steps

1. Run the app locally (`pnpm --filter @ariatype/desktop tauri:dev`)
2. Try the core workflow (hold hotkey, speak, release)
3. Read [`AGENTS.md`](../../AGENTS.md) for agent rules
4. Pick a task from "Common Tasks" above and trace the dependency graph
5. Run verification commands to ensure your environment is working

---

*This guide follows Harness Engineering: give agents a map, not a 1000-page manual. For depth, follow the links to specific documentation.*
