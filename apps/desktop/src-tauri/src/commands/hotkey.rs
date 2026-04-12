//! IPC commands for hotkey recording.
//!
//! Allows the frontend to record a new hotkey by capturing keyboard input
//! via the RecordingListener. This enables capturing hardware-level keys
//! like FN/Globe that browser keyboard events cannot detect.

use tauri::{AppHandle, Emitter, Manager, State};
use tracing::{error, info, warn};

use crate::shortcut::{RecordingListener, ShortcutManager};
use crate::state::app_state::AppState;

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
pub fn start_hotkey_recording(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    // Do NOT unregister current hotkey here!
    // It remains active during capture. Unregister only happens after successful capture.

    let mut listener_guard = state.hotkey_recording_listener.lock();

    // Check if already recording
    if let Some(ref listener) = *listener_guard {
        if listener.is_active() {
            return Err("hotkey recording already in progress".to_string());
        }
    }

    // Create new listener and start it with app handle for event emission
    let mut listener = RecordingListener::new()
        .map_err(|e| format!("failed to create recording listener: {}", e))?;

    listener
        .start(app)
        .map_err(|e| format!("failed to start recording listener: {}", e))?;

    *listener_guard = Some(listener);
    info!("hotkey_recording_started");
    Ok(())
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
pub fn stop_hotkey_recording(app: AppHandle, state: State<'_, AppState>) -> Option<String> {
    let mut listener_guard = state.hotkey_recording_listener.lock();

    if let Some(mut listener) = listener_guard.take() {
        let result = listener.stop();
        if let Some(ref new_hotkey) = result {
            info!(hotkey = %new_hotkey, "hotkey_recording_stopped_with_capture");

            // Only unregister old and register new if we have a valid captured hotkey
            if let Some(shortcut_manager) = app.try_state::<ShortcutManager>() {
                // Get old hotkey from settings
                let old_hotkey = state.settings.lock().hotkey.clone();

                // Unregister old hotkey first
                if let Err(e) = shortcut_manager.unregister() {
                    warn!(error = %e, hotkey = %old_hotkey, "old_hotkey_unregister_failed");
                } else {
                    info!(hotkey = %old_hotkey, "old_hotkey_unregistered");
                }

                // Register new hotkey
                if let Err(e) = shortcut_manager.register(new_hotkey) {
                    error!(error = %e, hotkey = %new_hotkey, "new_hotkey_registration_failed");

                    // Registration failed - try to restore old hotkey
                    if let Err(restore_err) = shortcut_manager.register(&old_hotkey) {
                        warn!(error = %restore_err, hotkey = %old_hotkey, "old_hotkey_restore_failed");
                    }

                    // Emit registration failure event
                    if let Err(emit_err) =
                        app.emit(crate::events::EventName::SHORTCUT_REGISTRATION_FAILED, e)
                    {
                        warn!(error = %emit_err, "event_emit_failed");
                    }
                    return None;
                } else {
                    info!(hotkey = %new_hotkey, "new_hotkey_registered");

                    // Save to settings
                    state.settings.lock().hotkey = new_hotkey.clone();

                    // Persist settings to disk
                    if let Err(e) = crate::commands::settings::save_settings_internal(&app) {
                        warn!(error = %e, "failed_to_save_settings");
                    }

                    // Emit SETTINGS_CHANGED to refresh frontend UI
                    // Frontend should NOT call updateSetting for hotkey - backend handled it
                    let settings = state.settings.lock().clone();
                    if let Err(e) = app.emit(crate::events::EventName::SETTINGS_CHANGED, settings) {
                        warn!(error = %e, "failed_to_emit_settings_changed");
                    }
                }
            } else {
                error!("shortcut_manager_not_available");
            }
        } else {
            info!("hotkey_recording_stopped_no_capture");
            // No hotkey captured - current hotkey remains unchanged
        }
        result
    } else {
        error!("stop_hotkey_recording_called_without_active_listener");
        None
    }
}

/// Cancels an active hotkey recording without saving.
///
/// Current hotkey remains unchanged - no unregister/register happens.
#[tauri::command]
pub fn cancel_hotkey_recording(state: State<'_, AppState>) {
    let mut listener_guard = state.hotkey_recording_listener.lock();

    if let Some(mut listener) = listener_guard.take() {
        // Stop the listener but ignore the result
        listener.stop();
        info!("hotkey_recording_cancelled_current_hotkey_unchanged");
    }
}

/// Peeks at the currently captured hotkey without stopping the listener.
///
/// Use this for previewing the hotkey in real-time while recording.
/// Returns None if no hotkey has been captured yet.
#[tauri::command]
pub fn peek_hotkey_recording(state: State<'_, AppState>) -> Option<String> {
    let listener_guard = state.hotkey_recording_listener.lock();

    if let Some(ref listener) = *listener_guard {
        listener.peek_captured()
    } else {
        None
    }
}
