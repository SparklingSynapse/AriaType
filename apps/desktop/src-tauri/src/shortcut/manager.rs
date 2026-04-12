//! Shortcut manager running in a background thread.
//!
//! Handles hotkey registration, triggering, and event emission.
//! Uses `handy_keys::HotkeyManager` for cross-platform support.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use parking_lot::Mutex;
use tauri::{Emitter, Manager};

use crate::events::EventName;

use super::types::{ShortcutEvent, ShortcutState};

#[cfg(target_os = "macos")]
use super::FnEmojiBlocker;

/// Command sent to the background thread.
#[derive(Clone, Debug)]
enum ShortcutCommand {
    Register(String),
    Unregister,
}

/// Internal state shared between main thread and background thread.
struct ManagerState {
    /// Command to execute on next cycle.
    pending_command: Mutex<Option<ShortcutCommand>>,
    /// Current registered hotkey ID (stored directly as HotkeyId which is Copy).
    current_id: Mutex<Option<handy_keys::HotkeyId>>,
    /// Signal to shut down the background thread.
    shutdown: AtomicBool,
}

/// Manager for global keyboard shortcuts.
///
/// Spawns a background thread that runs the `HotkeyManager` event loop.
/// Commands are sent via shared state; events are emitted back to main thread.
///
/// On macOS, also runs `FnEmojiBlocker` to prevent system FN shortcuts
/// (emoji picker, input source switching) when FN is used as hotkey.
pub struct ShortcutManager {
    /// Channel to send events from background thread to main thread.
    event_tx: Sender<ShortcutEvent>,
    /// Channel to receive events from background thread (protected for Sync).
    event_rx: Mutex<Receiver<ShortcutEvent>>,
    /// Background thread handle.
    thread_handle: Option<JoinHandle<()>>,
    /// Shared state with background thread.
    state: Arc<ManagerState>,
    /// FN emoji blocker (macOS only) to prevent system FN shortcuts during normal operation.
    #[cfg(target_os = "macos")]
    fn_emoji_blocker: Option<FnEmojiBlocker>,
}

impl ShortcutManager {
    /// Create a new shortcut manager without starting the thread.
    ///
    /// Use `start()` to begin the event loop.
    pub fn new() -> Result<Self, String> {
        let (event_tx, event_rx) = std::sync::mpsc::channel();

        let state = Arc::new(ManagerState {
            pending_command: Mutex::new(None),
            current_id: Mutex::new(None),
            shutdown: AtomicBool::new(false),
        });

        Ok(Self {
            event_tx,
            event_rx: Mutex::new(event_rx),
            thread_handle: None,
            state,
            #[cfg(target_os = "macos")]
            fn_emoji_blocker: None,
        })
    }

    /// Start the background thread with the event loop.
    ///
    /// The thread runs `HotkeyManager::recv()` and handles commands.
    ///
    /// On macOS, also starts `FnEmojiBlocker` to prevent system FN shortcuts
    /// (emoji picker, input source switching) when FN is used as hotkey.
    pub fn start(&mut self, app_handle: tauri::AppHandle) -> Result<(), String> {
        if self.thread_handle.is_some() {
            return Err("shortcut manager already started".to_string());
        }

        // Check accessibility on macOS
        #[cfg(target_os = "macos")]
        {
            if !super::macos::check_accessibility() {
                tracing::warn!("accessibility_permissions_not_granted");
                // Continue anyway; hotkeys may not work but won't crash
            }
        }

        // Start FN emoji blocker on macOS
        #[cfg(target_os = "macos")]
        {
            let mut blocker = FnEmojiBlocker::new();
            blocker.start()?;
            self.fn_emoji_blocker = Some(blocker);
            tracing::info!("fn_emoji_blocker_started_for_shortcut_manager");
        }

        let state = Arc::clone(&self.state);
        let event_tx = self.event_tx.clone();

        // Spawn the hotkey manager thread
        let handle = thread::spawn(move || {
            run_hotkey_loop(state, event_tx, app_handle);
        });

        self.thread_handle = Some(handle);
        tracing::info!("shortcut_manager_started");
        Ok(())
    }

    /// Register a new hotkey, replacing any existing one.
    ///
    /// Stores in shared state; background thread will pick it up.
    pub fn register(&self, hotkey: &str) -> Result<(), String> {
        // Store in pending state; background thread will pick it up
        let mut pending = self.state.pending_command.lock();
        *pending = Some(ShortcutCommand::Register(hotkey.to_string()));

        tracing::info!(hotkey = %hotkey, "shortcut_register_requested");
        Ok(())
    }

    /// Unregister the current hotkey.
    pub fn unregister(&self) -> Result<(), String> {
        let mut pending = self.state.pending_command.lock();
        *pending = Some(ShortcutCommand::Unregister);

        tracing::info!("shortcut_unregister_requested");
        Ok(())
    }

    /// Stop the background thread and cleanup.
    pub fn stop(&mut self) -> Result<(), String> {
        if self.thread_handle.is_none() {
            return Ok(());
        }

        // Stop FN emoji blocker on macOS
        #[cfg(target_os = "macos")]
        if let Some(mut blocker) = self.fn_emoji_blocker.take() {
            blocker.stop();
            tracing::info!("fn_emoji_blocker_stopped_for_shortcut_manager");
        }

        self.state.shutdown.store(true, Ordering::SeqCst);

        // Wait for thread to finish (with timeout)
        if let Some(handle) = self.thread_handle.take() {
            // Give it a moment to shutdown gracefully
            thread::sleep(Duration::from_millis(10));
            // Drop handle without blocking indefinitely
            std::mem::drop(handle);
        }

        tracing::info!("shortcut_manager_stopped");
        Ok(())
    }

    /// Get the event receiver for handling shortcut triggers.
    pub fn event_receiver(&self) -> parking_lot::MutexGuard<'_, Receiver<ShortcutEvent>> {
        self.event_rx.lock()
    }
}

impl Default for ShortcutManager {
    fn default() -> Self {
        Self::new().expect("shortcut manager creation should not fail")
    }
}

/// Run the hotkey event loop in a background thread.
///
/// This function:
/// 1. Creates a `HotkeyManager`
/// 2. Registers pending hotkeys from shared state
/// 3. Receives hotkey events and emits to main thread
/// 4. Handles shutdown signal
fn run_hotkey_loop(
    state: Arc<ManagerState>,
    event_tx: Sender<ShortcutEvent>,
    app_handle: tauri::AppHandle,
) {
    // Create the hotkey manager
    let mut manager = match handy_keys::HotkeyManager::new() {
        Ok(m) => m,
        Err(e) => {
            tracing::error!(error = %e, "hotkey_manager_creation_failed");
            let _ = event_tx.send(ShortcutEvent::RegistrationFailed {
                error: e.to_string(),
            });
            return;
        }
    };

    tracing::info!("hotkey_manager_created");

    // Main event loop
    loop {
        // Check shutdown signal
        if state.shutdown.load(Ordering::SeqCst) {
            tracing::info!("hotkey_manager_shutdown_requested");
            break;
        }

        // Check for pending command
        {
            let mut pending = state.pending_command.lock();
            if let Some(command) = pending.take() {
                match command {
                    ShortcutCommand::Register(hotkey_str) => {
                        // Unregister old hotkey if exists (using stored HotkeyId)
                        {
                            let mut current = state.current_id.lock();
                            if let Some(old_id) = current.take() {
                                tracing::info!(old_id = old_id.as_u32(), "unregistering_old_hotkey");
                                if let Err(e) = manager.unregister(old_id) {
                                    tracing::warn!(error = ?e, "old_hotkey_unregister_failed");
                                } else {
                                    tracing::info!("old_hotkey_unregistered");
                                }
                            }
                        }

                        // Recreate the manager to reset any stuck modifier states
                        // This fixes an issue where the recording listener's tap blocks the key-down
                        // but allows the key-up, causing the manager's internal modifier state to invert.
                        tracing::info!("recreating_hotkey_manager_to_reset_state");
                        manager = match handy_keys::HotkeyManager::new() {
                            Ok(m) => m,
                            Err(e) => {
                                tracing::error!(error = %e, "hotkey_manager_recreation_failed");
                                let _ = event_tx.send(ShortcutEvent::RegistrationFailed { error: e.to_string() });
                                return;
                            }
                        };

                        // Parse and register new hotkey
                        match register_hotkey(&manager, &hotkey_str) {
                            Ok(id) => {
                                tracing::info!(hotkey = %hotkey_str, id = id.as_u32(), "hotkey_registered");
                                let mut current = state.current_id.lock();
                                *current = Some(id);
                            }
                            Err(e) => {
                                tracing::error!(hotkey = %hotkey_str, error = %e, "hotkey_registration_failed");
                                // Emit failure event
                                let _ =
                                    event_tx.send(ShortcutEvent::RegistrationFailed { error: e.clone() });
                                // Also emit to frontend via Tauri
                                let _ = app_handle.emit(EventName::SHORTCUT_REGISTRATION_FAILED, e);
                            }
                        }
                    }
                    ShortcutCommand::Unregister => {
                        let mut current = state.current_id.lock();
                        if let Some(old_id) = current.take() {
                            tracing::info!(old_id = old_id.as_u32(), "unregistering_old_hotkey_explicit");
                            if let Err(e) = manager.unregister(old_id) {
                                tracing::warn!(error = ?e, "old_hotkey_unregister_failed_explicit");
                            } else {
                                tracing::info!("old_hotkey_unregistered_explicit");
                            }
                        }
                    }
                }
            }
        }

        // Receive hotkey event (non-blocking)
        // Use try_recv() to allow checking shutdown signal
        match manager.try_recv() {
            Some(event) => {
                // Convert handy_keys state to our state
                let state_enum = match event.state {
                    handy_keys::HotkeyState::Pressed => ShortcutState::Pressed,
                    handy_keys::HotkeyState::Released => ShortcutState::Released,
                };

                tracing::info!(state = %state_enum.as_str(), "hotkey_triggered");

                // Emit event to main thread
                let _ = event_tx.send(ShortcutEvent::Triggered { state: state_enum });

                // Emit to frontend for recording trigger
                let _ = app_handle.emit(EventName::SHORTCUT_TRIGGERED, state_enum.as_str());

                // Handle recording trigger based on recording mode
                handle_recording_trigger(&app_handle, state_enum);
            }
            None => {
                // No event available, check shutdown and sleep briefly
                if state.shutdown.load(Ordering::SeqCst) {
                    tracing::info!("hotkey_manager_shutdown_detected");
                    break;
                }
                // Sleep briefly to avoid busy-waiting
                thread::sleep(Duration::from_millis(50));
            }
        }
    }

    tracing::info!("hotkey_manager_loop_exited");
}

/// Handle recording trigger based on hotkey state and recording mode.
///
/// This function replicates the logic from the old register_global_shortcut:
/// - Hold mode: Press to start, Release to stop
/// - Toggle mode: Press to toggle recording
///
/// IMPORTANT: If capture mode is active, do NOT trigger recording.
/// This allows users to press their current hotkey during capture to re-register it.
fn handle_recording_trigger(app_handle: &tauri::AppHandle, state: ShortcutState) {
    use std::sync::atomic::Ordering;

    tracing::debug!(state = %state.as_str(), "handle_recording_trigger_entered");

    // Get app state
    let state_result = app_handle.try_state::<crate::state::app_state::AppState>();
    match state_result {
        Some(app_state) => {
            // Check if capture mode is active - if so, don't trigger recording
            let listener_guard = app_state.hotkey_recording_listener.lock();
            if let Some(ref listener) = *listener_guard {
                if listener.is_active() {
                    tracing::info!("capture_mode_active_hotkey_trigger_ignored");
                    return;
                }
            }
            // Drop the lock before proceeding
            drop(listener_guard);

            tracing::debug!("app_state_acquired");
            let is_recording = app_state.is_recording.load(Ordering::SeqCst);
            let recording_mode = app_state.settings.lock().recording_mode.clone();

            tracing::debug!(
                is_recording = is_recording,
                recording_mode = %recording_mode,
                "handle_recording_trigger_state"
            );

            match recording_mode.as_str() {
                "hold" => {
                    // Hold mode: Press to start, Release to stop
                    if state == ShortcutState::Pressed && !is_recording {
                        tracing::info!("hold_mode_start_recording_requested");
                        match crate::commands::audio::start_recording_sync(app_handle.clone()) {
                            Ok(_) => tracing::info!("hold_mode_recording_started"),
                            Err(e) => tracing::error!(error = %e, "hold_mode_start_failed"),
                        }
                    } else if state == ShortcutState::Released && is_recording {
                        tracing::info!("hold_mode_stop_recording_requested");
                        match crate::commands::audio::stop_recording_sync(app_handle.clone()) {
                            Ok(_) => tracing::info!("hold_mode_recording_stopped"),
                            Err(e) => tracing::error!(error = %e, "hold_mode_stop_failed"),
                        }
                    }
                }
                _ => {
                    // Toggle mode (default): Press to toggle
                    if state == ShortcutState::Pressed {
                        if is_recording {
                            tracing::info!("toggle_mode_stop_recording_requested");
                            match crate::commands::audio::stop_recording_sync(app_handle.clone()) {
                                Ok(_) => tracing::info!("toggle_mode_recording_stopped"),
                                Err(e) => tracing::error!(error = %e, "toggle_mode_stop_failed"),
                            }
                        } else {
                            tracing::info!("toggle_mode_start_recording_requested");
                            match crate::commands::audio::start_recording_sync(app_handle.clone()) {
                                Ok(_) => tracing::info!("toggle_mode_recording_started"),
                                Err(e) => tracing::error!(error = %e, "toggle_mode_start_failed"),
                            }
                        }
                    }
                }
            }
        }
        None => {
            tracing::error!("app_state_unavailable_for_recording_trigger");
        }
    }
}

/// Register a hotkey with the manager.
///
/// Parses the string and registers, returning the HotkeyId for later unregister.
fn register_hotkey(
    manager: &handy_keys::HotkeyManager,
    hotkey_str: &str,
) -> Result<handy_keys::HotkeyId, String> {
    // Handle FN key specially (macOS Globe/FN key)
    // FN is a hardware-level modifier that may be parsed differently
    if hotkey_str == FN_KEY_NAME || hotkey_str == "globe" {
        // Create FN-only hotkey
        let hotkey = handy_keys::Hotkey::new(handy_keys::Modifiers::FN, None)
            .map_err(|e| format!("failed to create FN hotkey: {:?}", e))?;

        // Register with manager (returns HotkeyId)
        let id = manager
            .register(hotkey)
            .map_err(|e| format!("FN registration failed: {:?}", e))?;

        tracing::info!(id = id.as_u32(), "fn_hotkey_registered");
        return Ok(id);
    }

    // Parse hotkey string using handy-keys built-in parser
    let hotkey: handy_keys::Hotkey = hotkey_str
        .parse()
        .map_err(|e| format!("invalid hotkey '{}': {:?}", hotkey_str, e))?;

    // Register with manager (returns HotkeyId)
    let id = manager
        .register(hotkey)
        .map_err(|e| format!("registration failed: {:?}", e))?;

    tracing::info!(hotkey = %hotkey_str, id = id.as_u32(), "hotkey_registered");
    Ok(id)
}

/// FN/Globe key name constant
const FN_KEY_NAME: &str = "fn";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manager_new() {
        let manager = ShortcutManager::new();
        assert!(manager.is_ok());
    }

    #[test]
    fn test_manager_default() {
        let manager = ShortcutManager::default();
        assert!(manager.event_receiver().try_recv().is_err()); // Empty channel
    }

    #[test]
    fn test_manager_register_updates_state() {
        let manager = ShortcutManager::new().unwrap();
        let result = manager.register("Shift+Space");
        assert!(result.is_ok());

        let pending = manager.state.pending_command.lock();
        if let Some(ShortcutCommand::Register(ref h)) = *pending {
            assert_eq!(h, "Shift+Space");
        } else {
            panic!("Expected ShortcutCommand::Register");
        }
    }

    #[test]
    fn test_manager_stop_without_start() {
        let mut manager = ShortcutManager::new().unwrap();
        let result = manager.stop();
        assert!(result.is_ok()); // Should handle gracefully
    }
}
