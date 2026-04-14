# Contributing to @ariatype/desktop

## Overview

**Package**: Tauri v2 desktop application for voice-to-text transcription.

**Stack**:
- Frontend: React 19 + TypeScript + Vite + Tailwind CSS
- Backend: Rust (65+ source files)

**Key Features**: Local STT (Whisper/SenseVoice), cloud STT (Volcengine/Deepgram), AI text polish, global hotkey, multi-window UI.

---

## Prerequisites

- Node.js 18+
- Rust 1.70+ (for backend compilation)
- pnpm 8+
- macOS: Apple Silicon (M1/M2/M3/M4) recommended
- Windows: Development support in progress

---

## Development Setup

```bash
# From repository root
pnpm install

# Start development server
pnpm --filter @ariatype/desktop tauri:dev

# Build frontend only
pnpm --filter @ariatype/desktop build

# Build full application (macOS)
pnpm --filter @ariatype/desktop tauri:build:mac

# Build full application (Windows)
pnpm --filter @ariatype/desktop tauri:build:win
```

### Dev / Inhouse Build Conventions

- `pnpm --filter @ariatype/desktop tauri:dev` starts the inhouse/dev desktop variant.
- The `tauri:dev` script regenerates inhouse icon assets before launching Tauri. Do not hand-edit generated files in `apps/desktop/assets/icons/inhouse/`.
- The canonical generator is `scripts/generate-inhouse-icons.sh`. Change the corner marker style there, then regenerate assets.
- Dev bundle icons are wired through `src-tauri/tauri.dev.conf.json`.
- macOS tray icon selection is runtime-driven in `src-tauri/src/tray.rs`: app identifiers ending with `.inhouse` load `assets/tray-icon-inhouse.png`.

---

## Architecture

### Frontend (`src/`)

| Entry Point | Purpose |
|-------------|---------|
| `main.tsx` | Settings window |
| `pill.tsx` | Floating recording indicator |
| `toast.tsx` | Transient notifications |

**Key Files**:
- `src/lib/tauri.ts` — Typed IPC boundary (**extend this, not raw invoke()**)
- `src/lib/events.ts` — Event definitions for frontend-backend communication
- `src/contexts/SettingsContext.tsx` — App-wide settings state
- `src/i18n/locales/` — 10 language translations (`de`, `en`, `es`, `fr`, `it`, `ja`, `ko`, `pt`, `ru`, `zh`)

**Path Alias**: `@/` maps to `src/`

### Backend (`src-tauri/src/`)

| Module | Purpose |
|--------|---------|
| `audio/` | Recording, resampling, beep generation, VAD, level meter |
| `stt_engine/` | Whisper, SenseVoice, cloud STT engines |
| `polish_engine/` | Local and cloud text polishing |
| `commands/` | Tauri IPC command handlers |
| `state/` | Unified runtime state management |
| `text_injector/` | Platform-specific text insertion |
| `events/` | Backend-to-frontend event emission |
| `utils/` | Downloader, paths, configuration helpers |

**Critical Files**:
- `lib.rs` — **Commands must be registered here** (not just in modules)
- `state/unified_state.rs` — Runtime state container
- `commands/settings/mod.rs` — Settings persistence

---

## Code Style

### Frontend (TypeScript/React)

- TypeScript strict mode
- React 19 functional components with hooks
- Tailwind CSS for styling
- `@/` path alias for imports
- All user-facing text → i18n keys (no hardcoded strings)
- Prefer stable UI state over aggressive fast updates
- No premature optimistic UI for STT status

**Example IPC Pattern**:
```typescript
// Always use src/lib/tauri.ts
import { startRecording } from '@/lib/tauri';

// Never use raw invoke()
import { invoke } from '@tauri-apps/api/core'; // ❌ Avoid
```

**Hotkey Display Convention**:
- Side-specific modifier keys stay visible in UI text.
- Use `L⌘` / `R⌘`, `LCtrl` / `RCtrl`, `L⌥` / `R⌥`, and `L⇧` / `R⇧` when the backend captures left/right variants.

### Backend (Rust)

- Rust 2021 edition
- All identifiers, comments, and doc strings in **English**
- `clippy --all-features -- -D warnings` (warnings are errors)
- `cargo fmt -- --check` (format check)
- Prefer deterministic logs over silent fallbacks
- For audio/transcription: **correctness > throughput**
- No silent degradation without justification

**Example Command Registration**:
```rust
// In lib.rs - REQUIRED for all commands
.invoke_handler(tauri::generate_handler![
    commands::start_recording,
    commands::stop_recording,
    commands::get_settings,
    // ... all other commands
])
```

---

## Testing

### Frontend Tests (Vitest)

```bash
# Run tests
pnpm --filter @ariatype/desktop test

# Run with coverage
pnpm --filter @ariatype/desktop test:coverage

# Watch mode
pnpm --filter @ariatype/desktop test:watch
```

**Test Location**: `src/**/*.{test,spec}.{ts,tsx}`
**Setup File**: `src/test/setup.ts`
**Environment**: jsdom

### Backend Tests (Cargo)

```bash
# Run unit tests
pnpm --filter @ariatype/desktop test:rust

# Run with coverage
pnpm --filter @ariatype/desktop test:rust:coverage

# Coverage report (HTML)
cd apps/desktop/src-tauri && cargo llvm-cov --html
```

**Test Categories**:

| Type | Location | Examples |
|------|----------|----------|
| Unit | `src/` alongside code | `settings_test.rs` |
| Integration | `src-tauri/tests/` | `whisper_engine_test.rs`, `audio_processor_test.rs` |
| E2E/Pipeline | `src-tauri/tests/` | `pipeline_integration_test.rs`, `e2e_test.rs` |

**Coverage Gate** (from root AGENTS.md):
- End-to-end: 100% for affected user workflow
- Unit: 100% for affected critical core modules

---

## Key Boundaries

| Boundary | Rule |
|----------|------|
| `src-tauri/capabilities/` | **Never modify without explicit request** |
| `lib.rs` | Commands **must be registered here** |
| `src/lib/tauri.ts` | All new IPC calls go through this layer |
| `src/i18n/locales/` | Update all 10 locales for user-facing changes |

---

## Build & Release

```bash
# macOS (signed)
pnpm --filter @ariatype/desktop tauri:build:mac

# macOS (unsigned, for testing)
pnpm --filter @ariatype/desktop tauri:build:mac:unsigned

# macOS ARM-only
pnpm --filter @ariatype/desktop tauri:build:mac-arm

# macOS Intel-only
pnpm --filter @ariatype/desktop tauri:build:mac-intel

# macOS Universal
pnpm --filter @ariatype/desktop tauri:build:mac-universal

# Windows
pnpm --filter @ariatype/desktop tauri:build:win
```

---

## Internationalization (i18n)

**10 Supported Locales**: `de`, `en`, `es`, `fr`, `it`, `ja`, `ko`, `pt`, `ru`, `zh`

**Location**: `src/i18n/locales/*.json`

**Rules**:
- Add new keys to **all 10 locale files**
- Use i18next + react-i18next
- Run `pnpm check:i18n` from root to validate

---

## Debugging

**Log Locations** (from root AGENTS.md):

| Platform | Path |
|----------|------|
| macOS | `~/Library/Logs/ariatype/` |
| Windows | `%LOCALAPPDATA%\ariatype\logs\` |

**Quick Access**:
```bash
# macOS - open log folder
open ~/Library/Logs/ariatype/

# macOS - tail latest log
tail -f ~/Library/Logs/ariatype/ariatype.log.*
```

---

## See Also

- **Root AGENTS.md** — Agent guidelines, TDD/BDD workflow, coverage gates, error handling
- **packages/shared/CONTRIBUTING.md** — Shared types and constants
- **packages/website/CONTRIBUTING.md** — Marketing website

---

## Product Priority Order

From root AGENTS.md Section 1.2:

```
STT accuracy > STT stability > user experience > speed
```

- Do NOT accept latency gains that reduce accuracy or stability
- Speed is optimized **only after** accuracy, stability, and UX are protected
