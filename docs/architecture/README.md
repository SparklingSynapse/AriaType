# Architecture Overview

This document describes the high-level architecture of AriaType, a Tauri v2 desktop voice keyboard application.

## Domain Map

| Domain | Packages | Responsibility |
|--------|----------|----------------|
| **Audio Pipeline** | `audio/recorder.rs`, `audio/resampler.rs`, `audio/vad.rs`, `audio/beep.rs`, `audio/level_meter.rs` | Capture, process, and manage audio data |
| **STT Engine** | `stt_engine/whisper.rs`, `stt_engine/sense_voice.rs`, `stt_engine/volcengine.rs`, `stt_engine/qwen_omni.rs`, `stt_engine/elevenlabs.rs`, `stt_engine/deepgram.rs` | Convert speech to text using local or cloud models |
| **Polish Engine** | `polish_engine/lfm.rs`, `polish_engine/qwen.rs`, `polish_engine/anthropic.rs`, `polish_engine/openai.rs` | Refine transcribed text for accuracy and formatting |
| **Text Injection** | `text_injector/macos.rs`, `text_injector/windows.rs` | Insert text at cursor position across platforms |
| **Settings** | `commands/settings/`, `state/unified_state.rs` | Persist and manage user preferences and hotkeys |
| **History** | `history/` | Store and retrieve transcription sessions |
| **UI** | `src/main.tsx`, `src/pill.tsx`, `src/toast.tsx`, `src/contexts/SettingsContext.tsx` | Multi-window interface and notifications |
| **Infrastructure** | `events/`, `utils/`, `tray.rs`, `lib.rs` | Logging, state management, system integration, CI/CD |

## Package Layering

```
packages/shared/          # Zero runtime dependencies, types/constants only
       ↑
apps/desktop/src/         # React 19 frontend (TypeScript strict mode)
       ↑
apps/desktop/src-tauri/  # Rust backend (clippy + fmt enforced)
```

Cross-cutting concerns (auth, telemetry, i18n, feature flags) enter through a single explicit interface per domain.

## Directory Structure

```
apps/desktop/src/
├── main.tsx              # Settings window entry
├── pill.tsx              # Floating recording indicator
├── toast.tsx             # Transient notifications
├── lib/tauri.ts          # Typed IPC boundary (invoke calls)
├── contexts/             # React contexts (SettingsContext)
├── components/           # UI components
├── hooks/                # Custom React hooks
└── i18n/locales/         # 10 language translations

apps/desktop/src-tauri/src/
├── audio/                # Recording, resampling, VAD, beep, level meter
├── stt_engine/           # Local + cloud STT engines
├── polish_engine/        # Local + cloud polishing
├── commands/             # Tauri IPC handlers
├── state/                # Unified runtime state
├── text_injector/        # Platform text injection
├── events/               # Backend-to-frontend events
├── utils/                # Downloader, paths, configuration
├── history/              # Transcription storage
├── tray.rs               # System tray
├── lib.rs                # Command registration
└── main.rs               # App entry point
```

## Key Files Reference

| File | Role |
|------|------|
| `lib.rs` | All Tauri commands must be registered here |
| `state/unified_state.rs` | Single source of runtime truth |
| `stt_engine/traits.rs` | SttEngine + StreamingSttEngine trait definitions |
| `stt_engine/unified_manager.rs` | Engine lifecycle management |
| `polish_engine/unified_manager.rs` | Polish engine lifecycle |
| `text_injector/macos.rs` | macOS text injection (keyboard simulation + clipboard) |
| `src/lib/tauri.ts` | Frontend IPC boundary, all invoke calls go through here |

## Detailed Architecture

- [Architectural Layers](layers.md) — Layer model, dependency rules, boundaries
- [Data Flow](data-flow.md) — Primary workflow, state machine, IPC communication

## Boundaries

| Boundary | Rule |
|----------|------|
| `src-tauri/capabilities/` | Never modify without explicit request |
| `lib.rs` | Commands registered here, not just in modules |
| `src/lib/tauri.ts` | All IPC calls through this file |
| `packages/shared/` | Zero runtime dependencies |
