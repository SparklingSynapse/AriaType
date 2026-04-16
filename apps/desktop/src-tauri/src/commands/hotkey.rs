//! IPC commands for hotkey recording.
//!
//! Allows the frontend to record a new hotkey by capturing keyboard input
//! via the ShortcutManager capture runtime. This enables capturing hardware-level keys
//! like FN/Globe that browser keyboard events cannot detect.

use tauri::{AppHandle, Manager};

use crate::shortcut::ShortcutManager;

/// Starts the hotkey recording listener.
///
/// This spawns a background thread that captures the next hotkey combination
/// pressed by the user. When a hotkey is captured, emits `hotkey-captured` event.
///
/// IMPORTANT: Does NOT unregister the current hotkey before capture.
/// The current hotkey remains active during capture to preserve existing functionality.
/// Unregistration happens only when a new valid hotkey is successfully captured.
///
/// The listener can capture:
/// - Regular keys (A-Z, F1-F12, etc.)
/// - Modifiers (Cmd, Ctrl, Shift, Alt)
/// - FN/Globe key on macOS (hardware-level modifier)
///
/// # Errors
/// Returns an error if recording is already in progress.
#[tauri::command]
pub fn start_hotkey_recording(app: AppHandle) -> Result<(), String> {
    app.try_state::<ShortcutManager>()
        .ok_or_else(|| "shortcut manager not available".to_string())?
        .start_recording_capture()
}

/// Stops the hotkey recording listener and registers the captured hotkey.
///
/// After capture ends successfully:
/// 1. Unregister the old hotkey (from settings)
/// 2. Register the new hotkey
/// 3. Save to settings
///
/// If no valid hotkey was captured, does NOT change the current hotkey.
///
/// # Returns
/// * `Some(String)` - The captured hotkey combination (e.g., "cmd+shift+s", "fn")
/// * `None` - No valid hotkey was captured, current hotkey unchanged
#[tauri::command]
pub fn stop_hotkey_recording(app: AppHandle) -> Option<String> {
    let Some(shortcut_manager) = app.try_state::<ShortcutManager>() else {
        tracing::error!("shortcut_manager_not_available");
        return None;
    };

    match shortcut_manager.stop_recording_capture() {
        Ok(result) => result,
        Err(error) => {
            tracing::error!(error = %error, "stop_hotkey_recording_failed");
            None
        }
    }
}

/// Cancels an active hotkey recording without saving.
///
/// Current hotkey remains unchanged - no unregister/register happens.
#[tauri::command]
pub fn cancel_hotkey_recording(app: AppHandle) {
    if let Some(shortcut_manager) = app.try_state::<ShortcutManager>() {
        shortcut_manager.cancel_recording_capture();
    } else {
        tracing::error!("shortcut_manager_not_available");
    }
}

/// Peeks at the currently captured hotkey without stopping the listener.
///
/// Use this for previewing the hotkey in real-time while recording.
/// Returns None if no hotkey has been captured yet.
#[tauri::command]
pub fn peek_hotkey_recording(app: AppHandle) -> Option<String> {
    app.try_state::<ShortcutManager>()
        .and_then(|shortcut_manager| shortcut_manager.peek_recording_capture())
}
