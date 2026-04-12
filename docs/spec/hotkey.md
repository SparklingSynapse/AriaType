# Global Hotkey Specification

Global keyboard shortcuts trigger recording from anywhere in the system. This spec defines valid hotkey combinations, capture behavior, and validation rules.

## Design Principle

**Frontend only handles UI display. Backend handles all logic and validation.**

| Layer | Responsibility |
|-------|----------------|
| Frontend | Display hotkey input, show capture state, format hotkey for display |
| Backend | Record key presses, validate combinations, emit results, register hotkeys |

Frontend does NOT validate hotkeys. Backend emits `hotkey-captured` event only when validation passes.

## Valid Hotkey Combinations

| Pattern | Valid | Reason |
|---------|-------|--------|
| `Fn` alone | ✅ | Hardware-level key, not prone to accidental triggers |
| `F1-F20` alone | ✅ | Function keys, not prone to accidental triggers |
| Modifier + single key | ✅ | Standard combination (e.g., `Cmd+A`, `Shift+Space`) |
| Multiple modifiers + single key | ✅ | Complex combination (e.g., `Cmd+Shift+A`) |
| `Fn` as modifier + key | ✅ | `Fn` with another key (e.g., `Fn+Space`) |
| Single non-F key alone | ❌ | Requires modifier to prevent accidental triggers |
| Modifier-only | ❌ | System limitation — global shortcuts need a key |
| Multiple keys | ❌ | System limitation — only one non-modifier key supported |

### Side-Specific Modifiers

Side-specific modifiers (left/right) are supported:

| Hotkey | Example |
|--------|---------|
| `CmdLeft+Slash` | Left Command key + Slash |
| `CmdRight+A` | Right Command key + A |
| `CtrlLeft+Space` | Left Control + Space |
| `ShiftRight+B` | Right Shift + B |

When only one side is pressed, the side-specific name is used. When both sides or compound modifier is detected, the unified name (`Cmd`, `Ctrl`, `Shift`) is used.

## Modifiers

Supported modifier keys:

| Modifier | Aliases | Platform |
|----------|---------|----------|
| `Cmd` | `command`, `meta`, `super`, `win` | All |
| `Ctrl` | `control` | All |
| `Opt` | `option`, `alt` | All |
| `Shift` | — | All |
| `Fn` | `function` | macOS only |

Side-specific variants: `CmdLeft`, `CmdRight`, `CtrlLeft`, `CtrlRight`, `OptLeft`, `OptRight`, `ShiftLeft`, `ShiftRight`.

## Capture Flow

### Core Design Rules

| Rule | Description |
|------|-------------|
| **No unregister on enter** | Entering capture mode does NOT unregister the current hotkey. The hotkey remains active. |
| **Pause recording during capture** | If user presses the current hotkey during capture, it does NOT trigger recording. |
| **Unregister only on success** | Old hotkey is unregistered only when a new valid hotkey is captured. |
| **No change on cancel** | Canceling capture does NOT change any hotkey registration. |

### Step-by-step Flow

| Step | Action | Layer | Notes |
|------|--------|-------|-------|
| 1 | User clicks hotkey input | Frontend | |
| 2 | Call `start_hotkey_recording` | Frontend → Backend | |
| 3 | Backend starts keyboard listener | Backend | **Current hotkey remains registered** |
| 4 | Backend sets `capture_mode_active` | Backend | Prevents hotkey from triggering recording |
| 5 | User presses keys (modifiers + key) | User | |
| 6 | Backend records all key presses | Backend | |
| 7 | User releases any key | User | |
| 8 | Backend analyzes and validates | Backend | |
| 9 | Backend emits `hotkey-captured` event | Backend | Only if valid |
| 10 | Frontend receives event | Frontend | |
| 11 | Frontend calls `stop_hotkey_recording` | Frontend → Backend | |
| 12 | Backend unregisters old hotkey | Backend | **Only after successful capture** |
| 13 | Backend registers new hotkey | Backend | |
| 14 | Backend saves to settings | Backend | |
| 15 | Frontend displays hotkey | Frontend | |

### Failure/Cancellation Flow

| Step | Action | Result |
|------|--------|--------|
| User presses invalid combination (e.g., `Cmd` alone) | Backend clears `pressed_keys`, continues waiting | Current hotkey unchanged |
| User cancels (ESC, blur) | Frontend calls `cancel_hotkey_recording` | Current hotkey unchanged |
| New hotkey registration fails | Backend restores old hotkey | Old hotkey restored |

### Why Current Hotkey Stays Active During Capture

1. **User can re-register same hotkey**: If user wants to keep their current hotkey, they can press it during capture to re-confirm it.
2. **No disruption**: Existing functionality continues working while user decides on new hotkey.
3. **Recovery from failed registration**: If new hotkey fails to register, old hotkey can be restored.

### How Recording is Paused During Capture

When capture mode is active, `handle_recording_trigger` checks the `hotkey_recording_listener.is_active()` flag:

```rust
if listener.is_active() {
    tracing::info!("capture_mode_active_hotkey_trigger_ignored");
    return; // Don't trigger recording
}
```

This means pressing the registered hotkey during capture:
- Does NOT start/stop recording
- IS captured by the listener as a potential new hotkey
- Allows user to re-register their current hotkey

## Backend Analysis Logic

### `listener.rs:analyze_sequence`

```rust
// 1. Separate modifiers and actual keys
// 2. Validate:
//    - FN alone → valid
//    - F1-F20 alone → valid
//    - Single non-F key without modifier → invalid
//    - Modifier-only → invalid
//    - Multiple non-modifier keys → invalid
// 3. Build hotkey string: modifiers (standard order) + key
// 4. Handle side-specific modifiers (CmdLeft, CmdRight, etc.)
```

### `manager.rs:handle_recording_trigger`

```rust
// 1. Check if capture mode is active
// 2. If active → ignore hotkey trigger (don't start/stop recording)
// 3. If inactive → proceed with normal recording trigger logic
```

### `commands/hotkey.rs:start_hotkey_recording`

```rust
// 1. Check if already recording
// 2. Create new RecordingListener
// 3. Start listener (NO unregister of current hotkey)
```

### `commands/hotkey.rs:stop_hotkey_recording`

```rust
// 1. Get captured hotkey from listener
// 2. If captured:
//    a. Unregister old hotkey
//    b. Register new hotkey
//    c. If registration fails → restore old hotkey
//    d. Save to settings
// 3. If not captured → current hotkey unchanged
```

### `commands/hotkey.rs:cancel_hotkey_recording`

```rust
// 1. Stop listener (ignore result)
// 2. Current hotkey remains unchanged
```

## System Event Blocking

Registered hotkeys are blocked from reaching other applications. This prevents:

- `Fn` from triggering input method switching
- `Cmd+Space` from triggering Spotlight
- Custom hotkeys from interfering with system shortcuts

Implementation: `handy_keys::HotkeyManager` intercepts at kernel level on macOS, hooks on Windows/Linux.

## Platform Notes

### macOS

- Requires accessibility permissions
- `Fn` key is hardware-level (Globe key on newer keyboards)
- F13-F20 may not exist on all keyboards

### Windows/Linux

- `Fn` key may not be available (laptop-specific)
- F1-F12 universally supported
- Wayland compositor may limit blocking capability

## Implementation References

| Component | File |
|-----------|------|
| Backend capture listener | `apps/desktop/src-tauri/src/shortcut/listener.rs` |
| Backend registration manager | `apps/desktop/src-tauri/src/shortcut/manager.rs` |
| Backend IPC commands | `apps/desktop/src-tauri/src/commands/hotkey.rs` |
| Frontend UI | `apps/desktop/src/components/ui/hotkey-input.tsx` |
| Backend tests | `apps/desktop/src-tauri/src/shortcut/__test__/` |
| Frontend tests | `apps/desktop/src/components/ui/__test__/hotkey-input.test.ts` |

## Library

Uses `handy-keys` v0.2.4 for cross-platform hotkey support.

| Feature | Support |
|---------|---------|
| F-keys | F1-F20 only |
| Side-specific modifiers | CmdLeft, CmdRight, etc. |
| Modifier-only hotkeys | Supported by library, rejected by our validation |
| Key blocking | macOS (kernel), Windows (hooks), Linux (rdev) |

### Frontend Integration

Frontend **must NOT** call `updateSetting("hotkey", value)` after hotkey capture. The backend handles everything:

| Event | Frontend Action |
|-------|------------------|
| `hotkey-captured` | Call `stop_hotkey_recording` to complete registration |
| `SETTINGS_CHANGED` | Auto-refresh settings UI via `useSettings` hook |
| `onChange` callback | Only track analytics, **no API calls** |

**Why no `updateSetting` for hotkey?**

Backend `stop_hotkey_recording` already:
1. Unregisters old hotkey
2. Registers new hotkey
3. Saves settings to disk
4. Emits `SETTINGS_CHANGED`

If frontend calls `updateSetting("hotkey", value)`:
- Backend `update_settings` triggers hotkey handler
- Handler tries to unregister + register again
- Results in `HotkeyAlreadyRegistered` error

### Code Pattern

**Wrong** (causes duplicate registration):
```tsx
const saveHotkey = async (value: string) => {
  await updateSetting("hotkey", value); // ❌ Duplicate!
};
```

**Correct**:
```tsx
const handleHotkeyChange = (value: string) => {
  analytics.track(AnalyticsEvents.SETTING_CHANGED, { setting: "hotkey", value });
  // Backend already handled everything - no API call needed
};
```

## Error Messages

Errors are emitted via `SHORTCUT_REGISTRATION_FAILED` event when registration fails after capture.

| Error | Trigger | Message |
|-------|---------|---------|
| Registration failed | New hotkey conflicts with existing system hotkey | `HotkeyAlreadyRegistered("...")` |
| Invalid hotkey | Backend validation failed during capture | Backend clears and continues waiting (no error shown) |

## Test Coverage

Backend tests: 19 tests in `listener.rs::tests`

- Fn alone valid
- F-key alone valid (F1, F12, F20)
- Side-specific modifier + key valid (CmdRight+Slash, CmdLeft+A)
- Single key without modifier invalid
- Space without modifier invalid
- Modifier-only invalid
- Multiple modifiers-only invalid
- Modifier + key valid
- Multiple modifiers + key valid
- Fn + key valid
- Multiple keys rejected
- Modifier + multiple keys rejected

Frontend tests: 25 tests in `hotkey-input.test.ts`

- Format hotkey for display (all combinations)
- Side-specific modifier formatting
- Case insensitive handling