---
title: "fix: Recover fn emoji blocker after macOS permission mutations"
type: fix
status: active
date: 2026-04-23
---

# fix: Recover Fn Emoji Blocker After macOS Permission Mutations

## Overview

Apply the smallest backend-only fix that keeps the macOS shortcut runtime fully recoverable when a System Settings privacy change invalidates active event taps while the app is running.

## Problem Frame

The desktop shortcut runtime is a composite of two active macOS event taps:
- the primary shortcut tap in `shortcut/platform/macos.rs`
- the companion `FnEmojiBlocker` tap in `shortcut/fn_emoji_blocker.rs`

The primary tap already requests a runtime restart when macOS disables it. The blocker tap only logs `tap disabled` and stays silent. If a privacy mutation such as microphone permission removal invalidates the blocker tap, the manager does not restart the composite runtime, which can leave system input handling in a broken state.

Desired behavior:
- Any blocker tap disable event requests the same runtime restart path as the main shortcut tap.
- Restarting the runtime tears down both taps and mounts them together again.
- Coverage proves blocker-disable signals are translated into restart requests.

## Scope Boundaries

- **In scope**: `apps/desktop/src-tauri/src/shortcut/fn_emoji_blocker.rs`, `apps/desktop/src-tauri/src/shortcut/platform/macos.rs`, `apps/desktop/src-tauri/src/shortcut/platform/mod.rs`, targeted shortcut tests
- **Out of scope**: new frontend permission UX, broader shortcut architecture refactors, changing microphone permission request behavior

## Implementation Units

- [ ] **Unit 1: Add a failing regression check for blocker disable handling**

**Goal:** Prove the blocker currently lacks a runtime restart signal for macOS tap-disable events.

**Files:**
- Modify: `apps/desktop/src-tauri/src/shortcut/fn_emoji_blocker.rs`

**Approach:**
- Extract the blocker's tap-disable decision into a small pure helper that can be unit tested.
- Add a test asserting tap-disable events map to a runtime restart request.
- Run the targeted test first and confirm it fails before implementation.

**Verification:**
- `cargo test shortcut::fn_emoji_blocker --lib`

---

- [ ] **Unit 2: Wire blocker disable events into the existing runtime restart path**

**Goal:** Ensure the composite macOS runner restarts when the blocker tap is disabled.

**Files:**
- Modify: `apps/desktop/src-tauri/src/shortcut/fn_emoji_blocker.rs`
- Modify: `apps/desktop/src-tauri/src/shortcut/platform/macos.rs`
- Modify: `apps/desktop/src-tauri/src/shortcut/platform/mod.rs`

**Approach:**
- Pass the runtime event sender into `FnEmojiBlocker` startup.
- When the blocker receives tap-disabled events, emit the existing restart signal instead of only logging.
- Reuse the manager's current `MainRunnerNeedsRestart` handling so teardown/remount remains centralized.

**Verification:**
- `cargo test shortcut::manager shortcut::fn_emoji_blocker --lib`

---

- [ ] **Unit 3: Make runner recovery symmetric for unexpected capture-runtime exit**

**Goal:** Ensure `CaptureOnly` can recover from asynchronous runtime exit and failed immediate remount, not just tap-disabled callbacks.

**Files:**
- Modify: `apps/desktop/src-tauri/src/shortcut/platform/macos.rs`
- Modify: `apps/desktop/src-tauri/src/shortcut/manager.rs`
- Modify: `apps/desktop/src-tauri/src/shortcut/platform/mod.rs`

**Approach:**
- Treat platform-runner teardown as a mode-aware lifecycle event owned by the manager.
- Emit a restart request when a macOS runner exits unexpectedly during startup or runtime.
- Add a manager-side reconciliation path for `CaptureOnly` so an active capture session remounts its owning runner until the runtime is healthy again.

**Verification:**
- `cargo test shortcut:: --lib`
- `cargo clippy --all-features -- -D warnings`

## System-Wide Impact

- macOS shortcut runtime recovery becomes symmetric across both event taps.
- Existing manager lifecycle remains the single backend-owned recovery entry point.
- No frontend or IPC contracts change.

## Risks & Dependencies

| Risk | Mitigation |
|------|------------|
| macOS permission mutation may affect more than tap-disabled events | Keep fix minimal, reuse existing manager restart path, preserve room for a later composite probe if needed |
| Unit tests cannot reproduce real TCC mutation | Cover the event translation logic directly and run targeted Rust verification |

## Verification Evidence

- Pending implementation
