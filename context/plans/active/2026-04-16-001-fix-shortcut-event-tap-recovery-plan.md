---
title: "fix: Recover shortcut runtime from macOS event-tap permission loss"
type: fix
status: completed
date: 2026-04-16
---

# fix: Recover Shortcut Runtime From macOS Event-Tap Permission Loss

## Overview

Apply the smallest backend-only fix that prevents the app from leaving a stale macOS event tap active when accessibility permission is missing or revoked. The shortcut controller should probe event-tap health before mounting, unmount on permission abnormality, and remount automatically after recovery.

## Problem Frame

Current behavior creates a blocking `handy-keys` event tap and a companion `FnEmojiBlocker` tap, then leaves them running for the life of the app. When macOS accessibility permission is revoked while the app is running, the tap can enter an invalid state and the system may stop delivering input correctly. The current startup check only logs missing accessibility and still attempts to start shortcut runtime.

Desired behavior:
- A fresh event-tap probe gates shortcut runtime mount.
- If the probe or permission health turns bad, the runtime is proactively torn down.
- If health becomes good again, the runtime is mounted again and re-registers the saved hotkey.
- Application code keeps `ShortcutManager` as the only lifecycle entry point and does not rely on a local `handy-keys` patch.

## Scope Boundaries

- **In scope**: `apps/desktop/src-tauri/src/shortcut/manager.rs`, `apps/desktop/src-tauri/src/shortcut/macos.rs`, `apps/desktop/src-tauri/src/lib.rs` startup behavior as needed, dependency wiring in `apps/desktop/src-tauri/Cargo.toml`
- **Out of scope**: frontend permission UX, capability changes, hotkey validation semantics, large shortcut architecture refactors

## Implementation Units

- [x] **Unit 1: Add macOS fresh event-tap probe and runtime transition logic**

**Goal:** Gate mount/unmount decisions on a fresh probe result instead of startup-only permission logging.

**Files:**
- Modify: `apps/desktop/src-tauri/src/shortcut/macos.rs`
- Modify: `apps/desktop/src-tauri/src/shortcut/manager.rs`

**Approach:**
- Add a fresh event-tap probe helper in `shortcut/macos.rs` that attempts to create and immediately tear down a keyboard-only session tap.
- Keep `ShortcutManager` as a long-lived controller, but let its internal runtime mount/unmount independently inside the background loop.
- Persist the desired hotkey string in manager state so runtime remount can re-register it after recovery.

**Verification:**
- Unit tests cover runtime action selection for healthy/unhealthy probe transitions.
- `cargo test shortcut::manager`

---

- [x] **Unit 2: Tear down shortcut runtime on permission abnormality and remount on recovery**

**Goal:** Ensure stale taps do not stay alive after permission loss.

**Files:**
- Modify: `apps/desktop/src-tauri/src/shortcut/manager.rs`

**Approach:**
- Poll probe health at a small interval from the existing shortcut thread.
- On healthy -> unhealthy transition, drop the live `HotkeyManager` runtime and stop the `FnEmojiBlocker`.
- On unhealthy -> healthy transition, recreate runtime and re-register desired hotkey and cancel hotkeys if present.

**Verification:**
- Manual code-path verification via logs.
- `cargo clippy --all-features -- -D warnings`

---

- [x] **Unit 3: Remove local handy-keys override and keep lifecycle logic in ShortcutManager**

**Goal:** Keep dependency sourcing standard and keep runtime mount/unmount decisions inside app-owned manager logic.

**Files:**
- Modify: `apps/desktop/src-tauri/Cargo.toml`
- Remove: local vendored `handy-keys` copy

**Approach:**
- Remove the local `[patch.crates-io]` override for `handy-keys`.
- Delete the vendored crate copy so the project resolves the registry dependency directly.
- Keep permission-loss teardown and recovery logic in `ShortcutManager` instead of carrying library-specific app patches.

**Verification:**
- `cargo check`
- Confirm application code still only starts the shortcut system via `ShortcutManager::start()`

## System-Wide Impact

- Startup still manages one `ShortcutManager` Tauri state, but actual macOS tap lifetime becomes dynamic.
- Existing command callers (`settings`, `audio`, `hotkey`) keep using the same manager API.
- Recording listener behavior continues to use upstream `handy-keys`; runtime lifecycle stays owned by `ShortcutManager`.

## Risks & Dependencies

| Risk | Mitigation |
|------|------------|
| Probe-only tests cannot emulate real macOS permission revocation in CI | Cover the transition state machine with unit tests and run strongest local verification available |
| Repeated remount attempts may spam logs while permission is denied | Gate remount by probe transitions and a fixed polling interval |
| Depending on app-owned remount logic without a crate patch may miss library-level edge cases | Keep permission and probe transitions covered by manager unit tests and targeted runtime verification |

## Verification Evidence

- `cargo test shortcut::manager --lib`
- `cargo clippy --all-features -- -D warnings`
- `cargo fmt -- --check` reports a pre-existing import-order diff in `apps/desktop/src-tauri/src/lib.rs`, not in the shortcut files changed by this fix
