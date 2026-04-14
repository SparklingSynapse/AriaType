---
title: Refactor Shortcut System to handy-keys
type: refactor
status: active
date: 2026-04-11
---

# Refactor Shortcut System to handy-keys

## Overview

Replace `tauri-plugin-global-shortcut` with `handy-keys` (v0.2.4) and create a dedicated shortcut module following architectural layer rules. The refactor separates shortcut logic from settings module, removes duplicate parsing implementation, and simplifies hotkey recording flow.

## Problem Frame

Current shortcut system has architectural issues:
- **Layer violation**: Shortcut registration logic lives in `commands/settings/mod.rs` (command layer), but contains business logic that should be in a service/engine layer
- **Duplicate implementation**: `parse_hotkey()` function reimplements string parsing that `handy-keys` provides built-in via `Hotkey::parse()`
- **State fragmentation**: `hotkey_capture_mode` AtomicBool in `unified_state.rs` is a workaround for the current plugin's inability to record without unregistering
- **Plugin dependency**: `tauri-plugin-global-shortcut` ties the app to Tauri-specific plugin ecosystem; `handy-keys` is a pure Rust library with better API design

## Requirements Trace

- R1. Shortcut registration and triggering must work identically to current behavior (hold/release recording mode)
- R2. All code must pass `cargo clippy --all-features -- -D warnings` and `cargo fmt -- --check`
- R3. Shortcut logic must be in a dedicated module following `context/architecture/layers.md` layer rules
- R4. Remove `parse_hotkey()` duplicate implementation — use `handy-keys` built-in parsing
- R5. Frontend behavior unchanged — `HotkeyInput` component works without modification
- R6. Tests must cover new module with happy path, edge cases, error paths

## Scope Boundaries

**In Scope:**
- Create new `shortcut/` module with `ShortcutManager` and recording logic
- Remove `tauri-plugin-global-shortcut` dependency
- Remove `parse_hotkey()` and `hotkey_capture_mode` from settings/state
- Update `lib.rs` to initialize `ShortcutManager` instead of plugin
- Preserve existing event emissions (`SHORTCUT_REGISTRATION_FAILED`, `SettingsChangedEvent.hotkey`)

**Out of Scope:**
- No UI changes to `HotkeyInput` component
- No new features (multi-shortcut support, new recording modes)
- No changes to frontend IPC bindings in `src/lib/tauri.ts`
- No changes to recording workflow behavior (hold/release remains same)

## Context & Research

### Relevant Code and Patterns

- **Module pattern**: `text_injector/mod.rs` shows flat dir + platform-specific submodules with `#[cfg(target_os)]`
- **Thread pattern**: `lib.rs:253` uses `std::thread::spawn` + `mpsc::channel` for blocking background work
- **Event pattern**: `events/mod.rs` defines event names + payload structs; emit via `app.emit()`
- **State pattern**: `unified_state.rs` uses `parking_lot::Mutex` and `AtomicBool`
- **ADR-004**: Standard pipeline pattern: `callback → channel → spawned consumer`

### External References

- handy-keys docs: https://docs.rs/handy-keys/latest/handy_keys/
- handy-keys repo: https://github.com/handy-computer/handy-keys
- API: `HotkeyManager::new_with_blocking()`, `manager.register(hotkey)`, `"Ctrl+Space".parse()`, `KeyboardListener` for recording

### Institutional Learnings

- Layer rules: types → config → state → engines → commands → events
- Commands layer must be thin, delegate to services
- Background threads use `std::thread::spawn` for blocking event loops

## Key Technical Decisions

- **Decision**: Create standalone `shortcut/` module instead of keeping in settings
  - **Rationale**: Follows layer architecture; shortcut is a cross-cutting concern with its own lifecycle
  
- **Decision**: Use `std::thread::spawn` + `mpsc::channel` for `HotkeyManager` event loop
  - **Rationale**: `manager.recv()` is blocking; Tauri requires non-blocking main thread; matches existing audio level monitor pattern
  
- **Decision**: Use `KeyboardListener` for recording instead of unregister/register workaround
  - **Rationale**: handy-keys provides built-in low-level listener; eliminates `hotkey_capture_mode` state complexity
  
- **Decision**: Keep frontend IPC bindings unchanged
  - **Rationale**: Frontend already calls `updateSettings("hotkey", value)`; backend handles registration internally

## Open Questions

### Resolved During Planning

- **How to handle macOS accessibility?**: handy-keys provides `check_accessibility()` and `open_accessibility_settings()`. Add check at startup in `ShortcutManager::init()`.
- **How to preserve hold/release behavior?**: `HotkeyEvent.state` (Pressed/Released) provides the info needed for hold mode recording.

### Deferred to Implementation

- **Exact channel buffer size**: Implementation will choose based on event frequency (likely 1-4)
- **Error handling for registration failure**: Follow existing pattern — emit `SHORTCUT_REGISTRATION_FAILED` event
- **Thread cleanup on app shutdown**: Implementation will handle graceful shutdown

## High-Level Technical Design

> *This illustrates the intended approach and is directional guidance for review, not implementation specification. The implementing agent should treat it as context, not code to reproduce.*

### Module Structure

```
apps/desktop/src-tauri/src/shortcut/
├── mod.rs          # Public API: init(), register_hotkey(), get_listener()
├── manager.rs      # ShortcutManager: spawns thread, owns HotkeyManager
├── listener.rs     # RecordingListener: wrapper around KeyboardListener
├── types.rs        # HotkeyConfig, ShortcutEvent (internal types)
└── macos.rs        # #[cfg(target_os = "macos")] accessibility helpers
```

### Thread/Channel Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Main Thread (Tauri)                       │
├─────────────────────────────────────────────────────────────────┤
│  lib.rs: setup()                                                 │
│    - ShortcutManager::init(app) → spawns background thread      │
│    - Returns (command_tx, event_rx)                             │
│                                                                  │
│  commands/settings/mod.rs                                        │
│    - update_settings("hotkey") → send RegisterCmd to command_tx │
└──────────────────────────┬──────────────────────────────────────┘
                           │ mpsc::channel<ShortcutCommand>
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Background Thread                             │
├─────────────────────────────────────────────────────────────────┤
│  ShortcutManager thread                                          │
│    - HotkeyManager::new_with_blocking() + recv() loop           │
│    - On ShortcutCommand::Register: manager.register(hotkey)     │
│    - On HotkeyEvent: send to event_tx                           │
│                                                                  │
│  RecordingListener (separate thread when active)                │
│    - KeyboardListener::new() + recv() loop                      │
│    - On KeyEvent: parse as Hotkey, return to caller             │
└──────────────────────────┬──────────────────────────────────────┘
                           │ mpsc::channel<ShortcutEvent>
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│                        Main Thread                               │
├─────────────────────────────────────────────────────────────────┤
│  Event handler ( spawned task )                                  │
│    - On ShortcutEvent::Triggered: emit to frontend              │
│    - On ShortcutEvent::Error: emit SHORTCUT_REGISTRATION_FAILED │
└─────────────────────────────────────────────────────────────────┘
```

## Implementation Units

- [ ] **Unit 1: Create shortcut/types.rs — Core Types**

**Goal:** Define internal types for shortcut module

**Requirements:** R3

**Dependencies:** None

**Files:**
- Create: `apps/desktop/src-tauri/src/shortcut/types.rs`

**Approach:**
- Define `HotkeyConfig` struct wrapping string representation
- Define `ShortcutCommand` enum (Register, Unregister, Shutdown)
- Define `ShortcutEvent` enum (Triggered, RegistrationFailed)
- Follow existing `types/` pattern for pure data structures

**Patterns to follow:**
- `stt_engine/models.rs` for type definitions

**Test scenarios:**
- Test expectation: none — pure type definitions, no behavior

**Verification:**
- File compiles with `cargo check`
- Types are accessible from other shortcut modules

---

- [ ] **Unit 2: Create shortcut/macos.rs — Platform Helpers**

**Goal:** macOS-specific accessibility permission handling

**Requirements:** R3

**Dependencies:** Unit 1

**Files:**
- Create: `apps/desktop/src-tauri/src/shortcut/macos.rs`

**Approach:**
- Gate with `#[cfg(target_os = "macos")]`
- Wrap `handy_keys::check_accessibility()` and `open_accessibility_settings()`
- Return Result with clear error messages
- Provide fallback for non-macOS platforms in `mod.rs`

**Patterns to follow:**
- `text_injector/macos.rs` for platform-specific module pattern

**Test scenarios:**
- Happy path: check_accessibility returns true on macOS with permissions granted
- Error path: check_accessibility returns false when permissions missing
- Edge case: Non-macOS build compiles without this module

**Verification:**
- `cargo clippy` passes on macOS target
- Cross-platform build succeeds

---

- [ ] **Unit 3: Create shortcut/manager.rs — ShortcutManager**

**Goal:** Core manager with background thread and event loop

**Requirements:** R1, R3

**Dependencies:** Unit 1, Unit 2

**Files:**
- Create: `apps/desktop/src-tauri/src/shortcut/manager.rs`
- Test: `apps/desktop/src-tauri/src/shortcut/__test__/manager_test.rs`

**Approach:**
- `ShortcutManager::init()` spawns thread with `HotkeyManager::new_with_blocking()`
- Thread runs `manager.recv()` loop, maps `HotkeyEvent` to `ShortcutEvent`
- Accept `ShortcutCommand` via channel for register/unregister
- Store current `HotkeyId` for unregister before re-register
- Emit events back to main thread via separate channel

**Execution note:** Start with failing test for register → trigger → emit event flow

**Technical design:**
```rust
// Directional guidance, not implementation
pub struct ShortcutManager {
    command_tx: mpsc::Sender<ShortcutCommand>,
    event_rx: mpsc::Receiver<ShortcutEvent>,
    current_id: Option<HotkeyId>,  // tracked in background thread
}

// Thread loop pseudocode:
loop {
    select! {
        recv(manager) => event => send ShortcutEvent::Triggered(state)
        recv(command_rx) => cmd => handle Register/Unregister
    }
}
```

**Patterns to follow:**
- `lib.rs:253` audio level monitor for thread spawn pattern
- `stt_engine/unified_manager.rs` for manager lifecycle

**Test scenarios:**
- Happy path: Register hotkey → trigger event emitted with correct state (Pressed/Released)
- Happy path: Unregister old → register new works without conflict
- Error path: Invalid hotkey string returns RegistrationFailed event
- Edge case: Empty hotkey string handled gracefully
- Integration: Manager thread starts and stops cleanly on init/shutdown

**Verification:**
- `cargo test` passes for manager module
- Thread spawns and receives events in test harness
- Clippy passes with no warnings

---

- [ ] **Unit 4: Create shortcut/listener.rs — RecordingListener**

**Goal:** Low-level keyboard listener for hotkey recording UI

**Requirements:** R1, R4

**Dependencies:** Unit 1

**Files:**
- Create: `apps/desktop/src-tauri/src/shortcut/listener.rs`
- Test: `apps/desktop/src-tauri/src/shortcut/__test__/listener_test.rs`

**Approach:**
- Wrap `handy_keys::KeyboardListener`
- `RecordingListener::start()` spawns thread with listener
- `RecordingListener::stop()` shuts down thread and returns captured hotkey
- Use `event.as_hotkey()` to parse pressed combination
- Return `Option<String>` (None if cancelled/invalid)

**Patterns to follow:**
- `ShortcutManager` thread pattern from Unit 3

**Test scenarios:**
- Happy path: Start → simulate key press → stop returns hotkey string
- Happy path: Modifier-only hotkey (e.g., "Cmd+Shift") captured correctly
- Error path: Invalid key combination returns None
- Edge case: Multiple key presses during recording (first valid wins)
- Edge case: Listener thread cleanup on stop

**Verification:**
- `cargo test` passes
- Listener returns hotkey string matching handy-keys format

---

- [ ] **Unit 5: Create shortcut/mod.rs — Public API**

**Goal:** Module entry point with public interface

**Requirements:** R3

**Dependencies:** Unit 1, Unit 2, Unit 3, Unit 4

**Files:**
- Create: `apps/desktop/src-tauri/src/shortcut/mod.rs`

**Approach:**
- Re-export `ShortcutManager` and `RecordingListener`
- Provide `init_shortcut_manager(app)` function for `lib.rs` setup
- Platform-specific imports via `#[cfg(target_os)]`
- Document public API with rustdoc comments

**Patterns to follow:**
- `text_injector/mod.rs` for module entry pattern
- `stt_engine/mod.rs` for re-export pattern

**Test scenarios:**
- Test expectation: none — module orchestration, behavior tested in submodules

**Verification:**
- `cargo doc` generates documentation
- Public API accessible from `lib.rs`

---

- [ ] **Unit 6: Update unified_state.rs — Remove Capture Mode**

**Goal:** Remove `hotkey_capture_mode` AtomicBool

**Requirements:** R4

**Dependencies:** None (cleanup)

**Files:**
- Modify: `apps/desktop/src-tauri/src/state/unified_state.rs`

**Approach:**
- Remove `hotkey_capture_mode: AtomicBool` field
- Remove any accessor methods for this field
- Ensure no compilation errors in dependent code

**Patterns to follow:**
- Existing state cleanup in ADR history

**Test scenarios:**
- Test expectation: none — removal, no behavior change
- Verify: Existing tests still pass after removal

**Verification:**
- `cargo clippy` passes
- No references to `hotkey_capture_mode` in codebase

---

- [ ] **Unit 7: Refactor commands/settings/mod.rs — Remove Shortcut Logic**

**Goal:** Remove shortcut registration functions from settings module

**Requirements:** R3, R4

**Dependencies:** Unit 3 (ShortcutManager exists)

**Files:**
- Modify: `apps/desktop/src-tauri/src/commands/settings/mod.rs`
- Modify: `apps/desktop/src-tauri/src/commands/settings/__test__/settings_test.rs`

**Approach:**
- Remove `register_global_shortcut()` function
- Remove `parse_hotkey()` function (use handy-keys parsing)
- Remove `set_hotkey_capture_mode()` command
- In `update_settings()`, delegate to `ShortcutManager` for hotkey changes
- Keep `AppSettings.hotkey` field (settings persistence unchanged)

**Patterns to follow:**
- Command layer should be thin per `layers.md`
- Delegate to service layer (`ShortcutManager`)

**Test scenarios:**
- Happy path: `update_settings("hotkey", new_value)` triggers registration via ShortcutManager
- Error path: Invalid hotkey emits `SHORTCUT_REGISTRATION_FAILED`
- Remove tests for `parse_hotkey()` (function removed)
- Verify existing settings update tests still pass

**Verification:**
- `cargo test` passes in settings module
- No references to removed functions
- `update_settings` correctly delegates

---

- [ ] **Unit 8: Update lib.rs — Integration**

**Goal:** Initialize ShortcutManager and remove plugin

**Requirements:** R1, R3

**Dependencies:** Unit 3, Unit 5, Unit 7

**Files:**
- Modify: `apps/desktop/src-tauri/src/lib.rs`
- Modify: `apps/desktop/src-tauri/src/events/mod.rs`

**Approach:**
- Remove `tauri_plugin_global_shortcut::Builder::new().build()` plugin registration
- In `setup()`, call `init_shortcut_manager(app)` to start manager thread
- Handle `ShortcutEvent::Triggered` → emit to frontend for recording trigger
- Handle `ShortcutEvent::RegistrationFailed` → emit existing event
- Remove `hotkey_to_register` workaround from setup

**Patterns to follow:**
- `lib.rs:253` audio level monitor initialization pattern
- Existing event emission in setup

**Test scenarios:**
- Integration: App starts without shortcut plugin
- Integration: Hotkey trigger triggers recording workflow (existing flow)
- Integration: Registration failure emits correct event
- Edge case: Manager thread cleanup on app shutdown

**Verification:**
- App builds and runs
- Hotkey triggers recording as before
- No plugin-related code in `lib.rs`

---

- [ ] **Unit 9: Update Cargo.toml — Dependencies**

**Goal:** Replace plugin dependency with handy-keys

**Requirements:** R1

**Dependencies:** None

**Files:**
- Modify: `apps/desktop/src-tauri/Cargo.toml`

**Approach:**
- Remove `tauri-plugin-global-shortcut = "2"`
- Add `handy-keys = "0.2.4"`
- Run `cargo update` to fetch new dependency
- Check `cargo deny check` for advisories

**Test scenarios:**
- Test expectation: none — dependency change
- Verify: `cargo build` succeeds with new deps

**Verification:**
- `cargo clippy` passes
- `cargo deny check` reports no advisories for handy-keys

---

- [ ] **Unit 10: Add Integration Tests**

**Goal:** End-to-end verification of shortcut flow

**Requirements:** R6

**Dependencies:** All previous units

**Files:**
- Create: `apps/desktop/src-tauri/src/shortcut/__test__/integration_test.rs`

**Approach:**
- Test full flow: init manager → register hotkey → simulate trigger → receive event
- Test recording flow: start listener → capture → stop → return hotkey
- Test settings integration: update_settings("hotkey") → manager receives command
- Use mock channels instead of real keyboard for testing

**Test scenarios:**
- Integration: Full registration → trigger → emit pipeline works
- Integration: Recording captures valid hotkey string
- Integration: Settings change propagates to manager
- Integration: Graceful shutdown cleans up threads

**Verification:**
- `cargo test --all` passes
- All integration tests pass

---

- [ ] **Unit 11: Cleanup and Documentation**

**Goal:** Remove dead code, add module documentation

**Requirements:** R2, R3

**Dependencies:** All previous units

**Files:**
- Modify: `apps/desktop/src-tauri/src/shortcut/mod.rs`
- Modify: `apps/desktop/src-tauri/src/shortcut/manager.rs`
- Modify: `apps/desktop/src-tauri/src/shortcut/listener.rs`

**Approach:**
- Add rustdoc comments to public API
- Remove any unused imports or dead code
- Run `cargo clippy --all-features -- -D warnings`
- Run `cargo fmt -- --check`

**Test scenarios:**
- Test expectation: none — cleanup phase

**Verification:**
- `cargo clippy` passes with zero warnings
- `cargo fmt -- --check` passes
- `cargo doc` generates clean documentation

## System-Wide Impact

- **Interaction graph:** Shortcut trigger → recording workflow (audio recorder, STT engine) — unchanged
- **Error propagation:** Registration failure → `SHORTCUT_REGISTRATION_FAILED` event → frontend notification — unchanged
- **State lifecycle risks:** Thread cleanup on shutdown must be graceful; `ShortcutManager` owns background thread
- **API surface parity:** Frontend `setHotkeyCaptureMode()` command removed — no longer needed with `KeyboardListener`
- **Integration coverage:** Integration tests cover manager → event → frontend pipeline
- **Unchanged invariants:** Recording workflow (hold/release), hotkey persistence, frontend UI behavior all preserved

## Risks & Dependencies

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| handy-keys has undiscovered bugs | Medium | Medium | Keep error handling robust; emit events for failures; test thoroughly |
| Thread cleanup race on shutdown | Low | High | Use `select!` with shutdown channel; graceful stop pattern |
| macOS accessibility not granted | Medium | Medium | Check at startup; prompt user to enable; emit error event |
| Parsing format mismatch with frontend | Low | Medium | handy-keys uses standard `"Mod+Key"` format; validate in tests |

## Documentation / Operational Notes

- Update `context/reference/providers/stt.md` if shortcut affects recording trigger docs
- No changes to frontend docs — IPC interface unchanged
- Add inline rustdoc for `ShortcutManager` public API

## Sources & References

- handy-keys documentation: https://docs.rs/handy-keys/latest/handy_keys/
- handy-keys repository: https://github.com/handy-computer/handy-keys
- Layer architecture: `context/architecture/layers.md`
- ADR-004 recording pipeline pattern: `context/architecture/decisions/004-*.md`
- Current shortcut code: `apps/desktop/src-tauri/src/commands/settings/mod.rs:862-929`
