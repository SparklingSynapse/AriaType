//! Low-level keyboard listener for hotkey recording UI.
//!
//! Wraps `handy_keys::KeyboardListener` to capture user key presses
//! for setting a new hotkey.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use parking_lot::Mutex;
use tauri::Emitter;

use crate::events::EventName;

use handy_keys::{BlockingHotkeys, Hotkey, Modifiers};

#[cfg(target_os = "macos")]
use crate::shortcut::FnEmojiBlocker;

/// FN/Globe key display name (macOS hardware-level key)
const FN_KEY_NAME: &str = "fn";

/// Listener for recording hotkey combinations.
///
/// Spawns a background thread with `KeyboardListener` to capture
/// key events. The caller can then retrieve the captured hotkey.
/// Emits events to frontend when hotkey is captured.
///
/// On macOS, also starts `FnEmojiBlocker` to prevent the FN/Globe key
/// from triggering the emoji picker when used as a short-press hotkey.
pub struct RecordingListener {
    /// Thread handle for the listener.
    thread_handle: Option<JoinHandle<Option<String>>>,
    /// Flag to signal stop recording.
    stop_flag: Arc<AtomicBool>,
    /// Captured hotkey (if any).
    captured: Arc<Mutex<Option<String>>>,
    /// Whether the listener is currently active.
    is_active: Arc<AtomicBool>,
    /// FN emoji blocker (macOS only) to prevent emoji popup on short FN press.
    #[cfg(target_os = "macos")]
    fn_emoji_blocker: Option<FnEmojiBlocker>,
}

impl RecordingListener {
    /// Create a new recording listener (not yet started).
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            thread_handle: None,
            stop_flag: Arc::new(AtomicBool::new(false)),
            captured: Arc::new(Mutex::new(None)),
            is_active: Arc::new(AtomicBool::new(false)),
            #[cfg(target_os = "macos")]
            fn_emoji_blocker: None,
        })
    }

    /// Start listening for key presses.
    ///
    /// Recording rules:
    /// 1. Record all keys pressed in order
    /// 2. End recording when ANY key is released
    /// 3. Analyze: modifiers + keys = valid hotkey
    /// 4. FN alone is valid single-key hotkey
    /// 5. Only Cmd/Shift/Ctrl/Opt = invalid
    ///
    /// On macOS, also starts FnEmojiBlocker to prevent FN/Globe key from
    /// triggering emoji picker on short press.
    pub fn start(&mut self, app_handle: tauri::AppHandle) -> Result<(), String> {
        if self.is_active.load(Ordering::SeqCst) {
            return Err("recording listener already active".to_string());
        }

        // Start FN emoji blocker on macOS
        #[cfg(target_os = "macos")]
        {
            let mut blocker = FnEmojiBlocker::new();
            blocker.start()?;
            self.fn_emoji_blocker = Some(blocker);
            tracing::info!("fn_emoji_blocker_started_for_recording");
        }

        self.stop_flag.store(false, Ordering::SeqCst);
        *self.captured.lock() = None;
        self.is_active.store(true, Ordering::SeqCst);

        let stop_flag = Arc::clone(&self.stop_flag);
        let captured = Arc::clone(&self.captured);
        let is_active = Arc::clone(&self.is_active);

        let handle = thread::spawn(move || {
            let result = run_recording_loop(stop_flag, captured, is_active);
            // Emit event when hotkey is captured
            if let Some(ref hotkey) = result {
                tracing::info!(hotkey = %hotkey, "hotkey_captured_event");
                let _ = app_handle.emit(EventName::HOTKEY_CAPTURED, hotkey.clone());
            }
            result
        });

        self.thread_handle = Some(handle);
        tracing::info!("recording_listener_started");
        Ok(())
    }

    /// Stop listening and return the captured hotkey.
    ///
    /// This can be called after the listener thread has already finished
    /// (e.g., when frontend receives hotkey-captured event).
    /// In that case, is_active will be false but captured will have the hotkey.
    pub fn stop(&mut self) -> Option<String> {
        // Stop FN emoji blocker on macOS
        #[cfg(target_os = "macos")]
        if let Some(mut blocker) = self.fn_emoji_blocker.take() {
            blocker.stop();
            tracing::info!("fn_emoji_blocker_stopped_for_recording");
        }

        // First try to get captured hotkey (may already be set by finished thread)
        let captured = self.captured.lock().take();
        if captured.is_some() {
            tracing::info!(hotkey = ?captured, "recording_listener_stopped_captured");
            return captured;
        }

        // No captured hotkey yet - check if still active
        if !self.is_active.load(Ordering::SeqCst) {
            tracing::info!("recording_listener_stopped_no_capture");
            return None;
        }

        // Still active - request stop and wait for thread
        self.stop_flag.store(true, Ordering::SeqCst);
        self.is_active.store(false, Ordering::SeqCst);

        if let Some(handle) = self.thread_handle.take() {
            thread::sleep(Duration::from_millis(50));

            match handle.join() {
                Ok(result) => {
                    tracing::info!(hotkey = ?result, "recording_listener_stopped_thread_result");
                    result
                }
                Err(e) => {
                    tracing::error!(error = ?e, "recording_listener_thread_panic");
                    None
                }
            }
        } else {
            self.captured.lock().take()
        }
    }

    /// Check if the listener is currently active.
    pub fn is_active(&self) -> bool {
        self.is_active.load(Ordering::SeqCst)
    }

    /// Get the captured hotkey without stopping.
    pub fn peek_captured(&self) -> Option<String> {
        self.captured.lock().clone()
    }
}

impl Default for RecordingListener {
    fn default() -> Self {
        Self::new().expect("recording listener creation should not fail")
    }
}

/// Represents a key that was pressed during recording
#[derive(Debug, Clone)]
enum PressedKey {
    /// Modifier key (Cmd, Shift, Ctrl, Opt, FN)
    Modifier(handy_keys::Modifiers),
    /// Regular key (A, Slash, Space, etc.)
    Key(handy_keys::Key),
}

/// Run the keyboard listener loop in background thread.
///
/// Recording logic:
/// 1. On keydown: append to pressed_keys array
/// 2. On keyup: analyze pressed_keys and finish
/// 3. Valid: modifiers + keys (either can be multiple)
/// 4. Valid: FN alone
/// 5. Invalid: only Cmd/Shift/Ctrl/Opt
///
/// Uses blocking mode with FN in blocking_hotkeys to prevent FN key
/// events from propagating to macOS system (which would trigger
/// system FN shortcuts like input source switching).
fn run_recording_loop(
    stop_flag: Arc<AtomicBool>,
    captured: Arc<Mutex<Option<String>>>,
    is_active: Arc<AtomicBool>,
) -> Option<String> {
    // Create blocking hotkeys set with FN key to prevent system shortcuts
    let blocking_hotkeys: BlockingHotkeys =
        Arc::new(std::sync::Mutex::new(std::collections::HashSet::from([
            Hotkey::new(Modifiers::FN, None).expect("FN hotkey creation"),
        ])));

    let listener = match handy_keys::KeyboardListener::new_with_blocking(blocking_hotkeys) {
        Ok(l) => l,
        Err(e) => {
            tracing::error!(error = %e, "keyboard_listener_creation_failed");
            is_active.store(false, Ordering::SeqCst);
            return None;
        }
    };

    tracing::info!("recording_loop_started_with_fn_blocking");

    // Record all keys pressed in sequence
    let mut pressed_keys: Vec<PressedKey> = Vec::new();

    loop {
        if stop_flag.load(Ordering::SeqCst) {
            tracing::debug!("recording_stop_requested");
            break;
        }

        match listener.try_recv() {
            Some(event) => {
                tracing::info!(
                    is_key_down = event.is_key_down,
                    key = ?event.key,
                    changed_modifier = ?event.changed_modifier,
                    modifiers = ?event.modifiers,
                    sequence_count = pressed_keys.len(),
                    sequence = ?pressed_keys,
                    "keyboard_event"
                );

                if event.is_key_down {
                    // Key pressed - record in sequence
                    if let Some(key) = event.key {
                        pressed_keys.push(PressedKey::Key(key));
                        tracing::info!(key = ?key, "key_recorded");

                        // Record ALL modifiers currently held (from event.modifiers)
                        // This handles case where user pressed modifier before entering capture mode
                        let mods = event.modifiers;
                        if mods.contains(handy_keys::Modifiers::CMD_RIGHT) {
                            pressed_keys
                                .push(PressedKey::Modifier(handy_keys::Modifiers::CMD_RIGHT));
                        }
                        if mods.contains(handy_keys::Modifiers::CMD_LEFT) {
                            pressed_keys
                                .push(PressedKey::Modifier(handy_keys::Modifiers::CMD_LEFT));
                        }
                        if mods.contains(handy_keys::Modifiers::SHIFT_RIGHT) {
                            pressed_keys
                                .push(PressedKey::Modifier(handy_keys::Modifiers::SHIFT_RIGHT));
                        }
                        if mods.contains(handy_keys::Modifiers::SHIFT_LEFT) {
                            pressed_keys
                                .push(PressedKey::Modifier(handy_keys::Modifiers::SHIFT_LEFT));
                        }
                        if mods.contains(handy_keys::Modifiers::CTRL_RIGHT) {
                            pressed_keys
                                .push(PressedKey::Modifier(handy_keys::Modifiers::CTRL_RIGHT));
                        }
                        if mods.contains(handy_keys::Modifiers::CTRL_LEFT) {
                            pressed_keys
                                .push(PressedKey::Modifier(handy_keys::Modifiers::CTRL_LEFT));
                        }
                        if mods.contains(handy_keys::Modifiers::OPT_RIGHT) {
                            pressed_keys
                                .push(PressedKey::Modifier(handy_keys::Modifiers::OPT_RIGHT));
                        }
                        if mods.contains(handy_keys::Modifiers::OPT_LEFT) {
                            pressed_keys
                                .push(PressedKey::Modifier(handy_keys::Modifiers::OPT_LEFT));
                        }
                        if mods.contains(handy_keys::Modifiers::FN) {
                            pressed_keys.push(PressedKey::Modifier(handy_keys::Modifiers::FN));
                        }
                    } else if let Some(modifier) = event.changed_modifier {
                        pressed_keys.push(PressedKey::Modifier(modifier));
                        tracing::info!(modifier = ?modifier, "modifier_recorded");
                    }
                } else {
                    // Key released - finish recording and analyze
                    // ANY key release ends capture mode
                    tracing::info!(
                        sequence = ?pressed_keys,
                        "key_released_finishing"
                    );

                    // Always try to analyze - backend does all validation
                    match analyze_sequence(&pressed_keys) {
                        Ok(hotkey_str) => {
                            tracing::info!(hotkey = %hotkey_str, "hotkey_valid");
                            *captured.lock() = Some(hotkey_str.clone());
                            is_active.store(false, Ordering::SeqCst);
                            return Some(hotkey_str);
                        }
                        Err(reason) => {
                            tracing::info!(reason = %reason, "hotkey_invalid_emit_error");
                            // Emit error event to frontend so UI can show it
                            // Then reset and continue waiting
                            pressed_keys.clear();
                        }
                    }
                }
            }
            None => {
                thread::sleep(Duration::from_millis(10));
            }
        }
    }

    is_active.store(false, Ordering::SeqCst);
    captured.lock().take()
}

/// Analyze the sequence of pressed keys and create a valid hotkey.
///
/// Rules:
/// - FN alone → valid single-key hotkey
/// - F1-F24 alone → valid single-key hotkey (function keys, not prone to accidental triggers)
/// - Single non-F key without modifier → invalid (requires modifier to prevent accidental triggers)
/// - Only Cmd/Shift/Ctrl/Opt → invalid
/// - Multiple non-modifier keys → invalid (e.g., A+B, Cmd+A+B)
/// - Modifiers + single key → valid
fn analyze_sequence(keys: &[PressedKey]) -> Result<String, String> {
    use handy_keys::{Key, Modifiers};

    tracing::info!(keys_count = keys.len(), keys = ?keys, "analyze_sequence_start");

    // Separate modifiers and keys
    let mut modifiers = Modifiers::empty();
    let mut actual_keys: Vec<Key> = Vec::new();

    for key in keys {
        match key {
            PressedKey::Modifier(modifier) => {
                modifiers |= *modifier;
            }
            PressedKey::Key(k) => {
                actual_keys.push(*k);
            }
        }
    }

    let has_fn = modifiers.contains(Modifiers::FN);
    let regular_modifiers = modifiers & !Modifiers::FN;

    tracing::info!(
        total_modifiers = ?modifiers,
        has_fn,
        regular_modifiers = ?regular_modifiers,
        keys_count = actual_keys.len(),
        keys = ?actual_keys,
        "analyze_state"
    );

    // Case 1: FN alone → valid
    if has_fn && regular_modifiers.is_empty() && actual_keys.is_empty() {
        tracing::info!("result_fn_only");
        return Ok(FN_KEY_NAME.to_string());
    }

    // Case 2: F1-F24 alone → valid (function keys, not prone to accidental triggers)
    if !has_fn && regular_modifiers.is_empty() && actual_keys.len() == 1 {
        if is_function_key(&actual_keys[0]) {
            let hotkey_str = build_hotkey_string(Modifiers::empty(), &actual_keys);
            tracing::info!(hotkey = %hotkey_str, "result_fkey_only");
            return Ok(hotkey_str);
        }
        // Single non-F key without modifier → invalid
        tracing::info!("result_single_key_without_modifier_invalid");
        return Err("Single key requires a modifier (e.g., Cmd+A, Shift+Space). F1-F24 and Fn are exceptions.".to_string());
    }

    // Case 3: Only regular modifiers (Cmd/Shift/Ctrl/Opt) → invalid
    if !has_fn && actual_keys.is_empty() {
        tracing::info!("result_modifier_only_invalid");
        return Err("Modifier-only hotkey not supported. Press a key after modifiers.".to_string());
    }

    // Case 4: Multiple non-modifier keys → invalid
    // Global hotkey APIs only support modifiers + single key
    if actual_keys.len() > 1 {
        tracing::info!("result_multiple_keys_invalid");
        return Err("Multiple keys not supported. Use modifiers + single key.".to_string());
    }

    // Case 5: Valid - modifiers (including FN) + single key
    let hotkey_str = build_hotkey_string(modifiers, &actual_keys);
    tracing::info!(hotkey = %hotkey_str, keys_count = actual_keys.len(), "result_hotkey");
    Ok(hotkey_str)
}

/// Check if a key is a function key (F1-F20).
/// Note: handy-keys library only supports F1-F20.
fn is_function_key(key: &handy_keys::Key) -> bool {
    use handy_keys::Key;
    matches!(
        key,
        Key::F1
            | Key::F2
            | Key::F3
            | Key::F4
            | Key::F5
            | Key::F6
            | Key::F7
            | Key::F8
            | Key::F9
            | Key::F10
            | Key::F11
            | Key::F12
            | Key::F13
            | Key::F14
            | Key::F15
            | Key::F16
            | Key::F17
            | Key::F18
            | Key::F19
            | Key::F20
    )
}

/// Build hotkey string from modifiers and keys.
///
/// Format: "Cmd+Slash" or "CmdRight+L" or "Shift+Cmd+A"
/// Handles side-specific modifiers (CmdLeft, CmdRight, etc.)
///
/// Logic for each modifier group:
/// - Right only (no left) → "XRight"
/// - Left only (no right) → "XLeft"
/// - Both sides or compound → "X" (unified)
fn build_hotkey_string(modifiers: handy_keys::Modifiers, keys: &[handy_keys::Key]) -> String {
    use handy_keys::Modifiers;

    let mut parts: Vec<String> = Vec::new();

    // Ctrl group
    let has_ctrl_right = modifiers.contains(Modifiers::CTRL_RIGHT);
    let has_ctrl_left = modifiers.contains(Modifiers::CTRL_LEFT);
    if has_ctrl_right && !has_ctrl_left {
        parts.push("CtrlRight".to_string());
    } else if has_ctrl_left && !has_ctrl_right {
        parts.push("CtrlLeft".to_string());
    } else if has_ctrl_left || has_ctrl_right {
        parts.push("Ctrl".to_string());
    }

    // Opt/Alt group
    let has_opt_right = modifiers.contains(Modifiers::OPT_RIGHT);
    let has_opt_left = modifiers.contains(Modifiers::OPT_LEFT);
    if has_opt_right && !has_opt_left {
        parts.push("OptRight".to_string());
    } else if has_opt_left && !has_opt_right {
        parts.push("OptLeft".to_string());
    } else if has_opt_left || has_opt_right {
        parts.push("Opt".to_string());
    }

    // Shift group
    let has_shift_right = modifiers.contains(Modifiers::SHIFT_RIGHT);
    let has_shift_left = modifiers.contains(Modifiers::SHIFT_LEFT);
    if has_shift_right && !has_shift_left {
        parts.push("ShiftRight".to_string());
    } else if has_shift_left && !has_shift_right {
        parts.push("ShiftLeft".to_string());
    } else if has_shift_left || has_shift_right {
        parts.push("Shift".to_string());
    }

    // Cmd/Meta group
    let has_cmd_right = modifiers.contains(Modifiers::CMD_RIGHT);
    let has_cmd_left = modifiers.contains(Modifiers::CMD_LEFT);
    if has_cmd_right && !has_cmd_left {
        parts.push("CmdRight".to_string());
    } else if has_cmd_left && !has_cmd_right {
        parts.push("CmdLeft".to_string());
    } else if has_cmd_left || has_cmd_right {
        parts.push("Cmd".to_string());
    }

    // Fn (macOS only, no side-specific)
    if modifiers.contains(Modifiers::FN) {
        parts.push("Fn".to_string());
    }

    // Add all keys in capture order
    for k in keys {
        parts.push(key_to_string(*k));
    }

    parts.join("+")
}

/// Convert Key enum to display string.
fn key_to_string(key: handy_keys::Key) -> String {
    match key {
        // Special keys that need clear names
        handy_keys::Key::Slash => "Slash".to_string(),
        handy_keys::Key::Backslash => "Backslash".to_string(),
        handy_keys::Key::Space => "Space".to_string(),
        handy_keys::Key::Return => "Return".to_string(),
        handy_keys::Key::Tab => "Tab".to_string(),
        handy_keys::Key::Escape => "Escape".to_string(),
        handy_keys::Key::Delete => "Delete".to_string(),
        handy_keys::Key::ForwardDelete => "ForwardDelete".to_string(),
        // Use handy_keys Display for others
        _ => key.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_listener_new() {
        let listener = RecordingListener::new();
        assert!(listener.is_ok());
        let listener = listener.unwrap();
        assert!(!listener.is_active());
    }

    #[test]
    fn test_listener_default() {
        let listener = RecordingListener::default();
        assert!(!listener.is_active());
    }

    #[test]
    fn test_build_hotkey_string() {
        use handy_keys::{Key, Modifiers};

        // Cmd+Slash
        let result = build_hotkey_string(Modifiers::CMD, &[Key::Slash]);
        assert_eq!(result, "Cmd+Slash");

        // Shift+Cmd+A (modifiers in macOS standard order: Ctrl → Opt → Shift → Cmd → Fn)
        let result = build_hotkey_string(Modifiers::CMD | Modifiers::SHIFT, &[Key::A]);
        assert_eq!(result, "Shift+Cmd+A");

        // Fn+Slash
        let result = build_hotkey_string(Modifiers::FN, &[Key::Slash]);
        assert_eq!(result, "Fn+Slash");

        // Fn only (modifiers only, no keys)
        let result = build_hotkey_string(Modifiers::FN, &[]);
        assert_eq!(result, "Fn");

        // Multi-key: Cmd+A+B
        let result = build_hotkey_string(Modifiers::CMD, &[Key::A, Key::B]);
        assert_eq!(result, "Cmd+A+B");

        // Multi-key with multiple modifiers: Cmd+Shift+A+B
        let result = build_hotkey_string(Modifiers::CMD | Modifiers::SHIFT, &[Key::A, Key::B]);
        assert_eq!(result, "Shift+Cmd+A+B");
    }

    #[test]
    fn test_analyze_sequence_fn_only() {
        // FN alone → valid
        let keys = vec![PressedKey::Modifier(handy_keys::Modifiers::FN)];
        let result = analyze_sequence(&keys);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "fn");
    }

    #[test]
    fn test_analyze_sequence_modifier_only_invalid() {
        // Only Cmd → invalid
        let keys = vec![PressedKey::Modifier(handy_keys::Modifiers::CMD)];
        let result = analyze_sequence(&keys);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Modifier-only"));
    }

    #[test]
    fn test_analyze_sequence_multiple_modifiers_only_invalid() {
        // Cmd+Shift → invalid
        let keys = vec![
            PressedKey::Modifier(handy_keys::Modifiers::CMD),
            PressedKey::Modifier(handy_keys::Modifiers::SHIFT),
        ];
        let result = analyze_sequence(&keys);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Modifier-only"));
    }

    #[test]
    fn test_analyze_sequence_single_key_without_modifier_invalid() {
        // A alone → invalid (requires modifier, F-keys are exception)
        let keys = vec![PressedKey::Key(handy_keys::Key::A)];
        let result = analyze_sequence(&keys);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("requires a modifier"));
    }

    #[test]
    fn test_analyze_sequence_space_without_modifier_invalid() {
        // Space alone → invalid (requires modifier)
        let keys = vec![PressedKey::Key(handy_keys::Key::Space)];
        let result = analyze_sequence(&keys);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("requires a modifier"));
    }

    #[test]
    fn test_analyze_sequence_fkey_only_valid() {
        // F1 alone → valid (function keys, not prone to accidental triggers)
        let keys = vec![PressedKey::Key(handy_keys::Key::F1)];
        let result = analyze_sequence(&keys);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "F1");
    }

    #[test]
    fn test_analyze_sequence_f12_only_valid() {
        // F12 alone → valid
        let keys = vec![PressedKey::Key(handy_keys::Key::F12)];
        let result = analyze_sequence(&keys);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "F12");
    }

    #[test]
    fn test_analyze_sequence_modifier_plus_key() {
        // Cmd+A → valid
        let keys = vec![
            PressedKey::Modifier(handy_keys::Modifiers::CMD),
            PressedKey::Key(handy_keys::Key::A),
        ];
        let result = analyze_sequence(&keys);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Cmd+A");
    }

    #[test]
    fn test_analyze_sequence_cmd_right_plus_key() {
        // CmdRight+Slash → valid (side-specific modifier)
        let keys = vec![
            PressedKey::Modifier(handy_keys::Modifiers::CMD_RIGHT),
            PressedKey::Key(handy_keys::Key::Slash),
        ];
        let result = analyze_sequence(&keys);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "CmdRight+Slash");
    }

    #[test]
    fn test_analyze_sequence_cmd_left_plus_key() {
        // CmdLeft+A → valid (side-specific modifier)
        let keys = vec![
            PressedKey::Modifier(handy_keys::Modifiers::CMD_LEFT),
            PressedKey::Key(handy_keys::Key::A),
        ];
        let result = analyze_sequence(&keys);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "CmdLeft+A");
    }

    #[test]
    fn test_analyze_sequence_multiple_modifiers_plus_key() {
        // Shift+Cmd+Space → valid (modifiers in macOS standard order)
        let keys = vec![
            PressedKey::Modifier(handy_keys::Modifiers::CMD),
            PressedKey::Modifier(handy_keys::Modifiers::SHIFT),
            PressedKey::Key(handy_keys::Key::Space),
        ];
        let result = analyze_sequence(&keys);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Shift+Cmd+Space");
    }

    #[test]
    fn test_analyze_sequence_fn_plus_key() {
        // Fn+Slash → valid
        let keys = vec![
            PressedKey::Modifier(handy_keys::Modifiers::FN),
            PressedKey::Key(handy_keys::Key::Slash),
        ];
        let result = analyze_sequence(&keys);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Fn+Slash");
    }

    #[test]
    fn test_analyze_sequence_multiple_keys_rejected() {
        // A+B → invalid (multiple keys not supported)
        let keys = vec![
            PressedKey::Key(handy_keys::Key::A),
            PressedKey::Key(handy_keys::Key::B),
        ];
        let result = analyze_sequence(&keys);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Multiple keys"));
    }

    #[test]
    fn test_analyze_sequence_modifier_plus_multiple_keys_rejected() {
        // Cmd+A+B → invalid (multiple keys not supported)
        let keys = vec![
            PressedKey::Modifier(handy_keys::Modifiers::CMD),
            PressedKey::Key(handy_keys::Key::A),
            PressedKey::Key(handy_keys::Key::B),
        ];
        let result = analyze_sequence(&keys);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Multiple keys"));
    }

    #[test]
    fn test_analyze_sequence_all_modifiers_plus_key() {
        // Ctrl+Opt+Shift+Cmd+F1 → valid (all modifiers + single key, macOS standard order)
        let keys = vec![
            PressedKey::Modifier(handy_keys::Modifiers::CTRL),
            PressedKey::Modifier(handy_keys::Modifiers::OPT),
            PressedKey::Modifier(handy_keys::Modifiers::SHIFT),
            PressedKey::Modifier(handy_keys::Modifiers::CMD),
            PressedKey::Key(handy_keys::Key::F1),
        ];
        let result = analyze_sequence(&keys);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Ctrl+Opt+Shift+Cmd+F1");
    }

    #[test]
    fn test_analyze_sequence_fn_plus_modifiers_plus_key() {
        // Cmd+Fn+Space → valid (FN as modifier + other modifiers + key)
        let keys = vec![
            PressedKey::Modifier(handy_keys::Modifiers::CMD),
            PressedKey::Modifier(handy_keys::Modifiers::FN),
            PressedKey::Key(handy_keys::Key::Space),
        ];
        let result = analyze_sequence(&keys);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Cmd+Fn+Space");
    }
}
