//! Shortcut manager running in a background thread.
//!
//! Handles hotkey registration, triggering, and event emission.
//! Uses `handy_keys::HotkeyManager` for cross-platform support.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;
#[cfg(target_os = "macos")]
use std::time::Instant;

use parking_lot::Mutex;
use tauri::{Emitter, Manager};

use crate::commands::settings::save_settings_internal;
use crate::events::EventName;
use crate::services::shortcut::{
    cancel_hotkey_release_unregister_owner, capture_cancel_hotkey_release_owner,
    primary_shortcut_action, primary_shortcut_context, should_unregister_cancel_hotkeys,
    PrimaryShortcutAction,
};

use super::listener::RecordingListener;
use super::types::{ShortcutEvent, ShortcutState};

#[cfg(target_os = "macos")]
use super::FnEmojiBlocker;

/// Command sent to the background thread.
#[derive(Clone, Debug)]
enum ShortcutCommand {
    Register(String),
    Unregister,
    RegisterCancel { owner_task_id: u64 },
    UnregisterCancel { owner_task_id: Option<u64> },
}

/// Internal state shared between main thread and background thread.
struct ManagerState {
    /// App handle stored after startup for manager-owned side effects.
    app_handle: Mutex<Option<tauri::AppHandle>>,
    /// Commands to execute on next cycle.
    pending_commands: Mutex<Vec<ShortcutCommand>>,
    /// Desired hotkey string that should be registered whenever runtime is available.
    desired_hotkey: Mutex<Option<String>>,
    /// Current registered hotkey ID (stored directly as HotkeyId which is Copy).
    current_id: Mutex<Option<handy_keys::HotkeyId>>,
    /// Cancel hotkey IDs. Hold mode may need modifier-aware Escape variants.
    cancel_ids: Mutex<Vec<handy_keys::HotkeyId>>,
    /// Task that currently owns the cancel hotkey registration.
    cancel_owner_task_id: Mutex<Option<u64>>,
    /// Task whose cancel hotkey should be unregistered after ESC is released.
    pending_cancel_release_owner_task_id: Mutex<Option<u64>>,
    /// Listener used for hotkey recording capture.
    recording_listener: Mutex<Option<RecordingListener>>,
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
}

impl ShortcutManager {
    /// Create a new shortcut manager without starting the thread.
    ///
    /// Use `start()` to begin the event loop.
    pub fn new() -> Result<Self, String> {
        let (event_tx, event_rx) = std::sync::mpsc::channel();

        let state = Arc::new(ManagerState {
            app_handle: Mutex::new(None),
            pending_commands: Mutex::new(Vec::new()),
            desired_hotkey: Mutex::new(None),
            current_id: Mutex::new(None),
            cancel_ids: Mutex::new(Vec::new()),
            cancel_owner_task_id: Mutex::new(None),
            pending_cancel_release_owner_task_id: Mutex::new(None),
            recording_listener: Mutex::new(None),
            shutdown: AtomicBool::new(false),
        });

        Ok(Self {
            event_tx,
            event_rx: Mutex::new(event_rx),
            thread_handle: None,
            state,
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

        self.state.shutdown.store(false, Ordering::SeqCst);
        *self.state.app_handle.lock() = Some(app_handle.clone());

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
    pub fn register_primary(&self, hotkey: &str) -> Result<(), String> {
        // Store in pending state; background thread will pick it up
        let mut pending = self.state.pending_commands.lock();
        pending.push(ShortcutCommand::Register(hotkey.to_string()));

        tracing::info!(hotkey = %hotkey, "primary_shortcut_register_requested");
        Ok(())
    }

    /// Unregister the current hotkey.
    pub fn unregister_primary(&self) -> Result<(), String> {
        let mut pending = self.state.pending_commands.lock();
        pending.push(ShortcutCommand::Unregister);

        tracing::info!("primary_shortcut_unregister_requested");
        Ok(())
    }

    /// Register the cancel hotkey (ESC).
    pub fn register_cancel(&self, owner_task_id: u64) -> Result<(), String> {
        let mut pending = self.state.pending_commands.lock();
        pending.push(ShortcutCommand::RegisterCancel { owner_task_id });
        tracing::info!(owner_task_id, "shortcut_register_cancel_requested");
        Ok(())
    }

    /// Unregister the cancel hotkey.
    pub fn unregister_cancel(&self) -> Result<(), String> {
        let mut pending = self.state.pending_commands.lock();
        pending.push(ShortcutCommand::UnregisterCancel {
            owner_task_id: None,
        });
        tracing::info!("shortcut_unregister_cancel_requested");
        Ok(())
    }

    /// Unregister the cancel hotkey only if the calling task still owns it.
    pub fn unregister_cancel_for_task(&self, owner_task_id: u64) -> Result<(), String> {
        let mut pending = self.state.pending_commands.lock();
        pending.push(ShortcutCommand::UnregisterCancel {
            owner_task_id: Some(owner_task_id),
        });
        tracing::info!(
            owner_task_id,
            "shortcut_unregister_cancel_requested_for_task"
        );
        Ok(())
    }

    /// Start the hotkey recording capture runtime.
    pub fn start_recording_capture(&self) -> Result<(), String> {
        let app_handle = self.app_handle()?;
        let mut recording_listener = self.state.recording_listener.lock();

        if recording_listener
            .as_ref()
            .is_some_and(|listener| listener.is_active())
        {
            return Err("hotkey recording already in progress".to_string());
        }

        let mut listener = RecordingListener::new()
            .map_err(|error| format!("failed to create recording listener: {error}"))?;
        listener
            .start(app_handle)
            .map_err(|error| format!("failed to start recording listener: {error}"))?;
        *recording_listener = Some(listener);

        tracing::info!("recording_capture_started");
        Ok(())
    }

    /// Stop the recording capture runtime and commit the captured hotkey if present.
    pub fn stop_recording_capture(&self) -> Result<Option<String>, String> {
        let captured_hotkey = stop_recording_capture_listener(&self.state, "explicit_stop");

        if let Some(ref hotkey) = captured_hotkey {
            let app_handle = self.app_handle()?;
            commit_captured_primary_hotkey(self, &app_handle, hotkey)?;
        }

        Ok(captured_hotkey)
    }

    /// Cancel the recording capture runtime without persisting the result.
    pub fn cancel_recording_capture(&self) {
        let _ = stop_recording_capture_listener(&self.state, "cancelled");
    }

    /// Inspect the captured hotkey without stopping capture.
    pub fn peek_recording_capture(&self) -> Option<String> {
        self.state
            .recording_listener
            .lock()
            .as_ref()
            .and_then(RecordingListener::peek_captured)
    }

    /// Whether the recording capture runtime is active.
    pub fn is_recording_capture_active(&self) -> bool {
        self.state
            .recording_listener
            .lock()
            .as_ref()
            .is_some_and(|listener| listener.is_active())
    }

    /// Stop the background thread and cleanup.
    pub fn stop(&mut self) -> Result<(), String> {
        if self.thread_handle.is_none() {
            return Ok(());
        }

        self.state.shutdown.store(true, Ordering::SeqCst);

        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }

        tracing::info!("shortcut_manager_stopped");
        Ok(())
    }

    /// Get the event receiver for handling shortcut triggers.
    pub fn event_receiver(&self) -> parking_lot::MutexGuard<'_, Receiver<ShortcutEvent>> {
        self.event_rx.lock()
    }

    fn app_handle(&self) -> Result<tauri::AppHandle, String> {
        self.state
            .app_handle
            .lock()
            .clone()
            .ok_or_else(|| "shortcut manager app handle unavailable".to_string())
    }
}

impl Default for ShortcutManager {
    fn default() -> Self {
        Self::new().expect("shortcut manager creation should not fail")
    }
}

#[cfg(target_os = "macos")]
const SHORTCUT_PERMISSION_POLL_INTERVAL: Duration = Duration::from_millis(100);

#[cfg(target_os = "macos")]
const SHORTCUT_RUNTIME_PROBE_INTERVAL: Duration = Duration::from_millis(500);

struct ShortcutRuntime {
    manager: handy_keys::HotkeyManager,
    #[cfg(target_os = "macos")]
    fn_emoji_blocker: FnEmojiBlocker,
}

#[cfg(target_os = "macos")]
impl Drop for ShortcutRuntime {
    fn drop(&mut self) {
        self.fn_emoji_blocker.stop();
        tracing::info!("fn_emoji_blocker_stopped_for_shortcut_manager");
    }
}

#[cfg(target_os = "macos")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RuntimePermissionAction {
    Mount,
    Unmount,
    Keep,
}

#[cfg(target_os = "macos")]
fn runtime_permission_action(
    runtime_is_mounted: bool,
    previous_accessibility_granted: Option<bool>,
    accessibility_granted: bool,
) -> RuntimePermissionAction {
    match (
        runtime_is_mounted,
        previous_accessibility_granted,
        accessibility_granted,
    ) {
        (true, _, false) => RuntimePermissionAction::Unmount,
        (false, Some(false), true) | (false, None, true) => RuntimePermissionAction::Mount,
        _ => RuntimePermissionAction::Keep,
    }
}

#[cfg(target_os = "macos")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RuntimeProbeAction {
    Mount,
    Unmount,
    Keep,
}

#[cfg(target_os = "macos")]
fn runtime_probe_action(runtime_is_mounted: bool, probe_ok: bool) -> RuntimeProbeAction {
    match (runtime_is_mounted, probe_ok) {
        (false, true) => RuntimeProbeAction::Mount,
        (true, false) => RuntimeProbeAction::Unmount,
        _ => RuntimeProbeAction::Keep,
    }
}

fn run_hotkey_loop(
    state: Arc<ManagerState>,
    event_tx: Sender<ShortcutEvent>,
    app_handle: tauri::AppHandle,
) {
    #[cfg(target_os = "macos")]
    let mut runtime: Option<ShortcutRuntime> = None;
    #[cfg(not(target_os = "macos"))]
    let mut runtime: Option<ShortcutRuntime> =
        match mount_shortcut_runtime(&state, &event_tx, &app_handle) {
            Ok(runtime) => Some(runtime),
            Err(error) => {
                emit_registration_failure(&event_tx, &app_handle, error);
                return;
            }
        };

    #[cfg(target_os = "macos")]
    let mut last_permission_poll_at = Instant::now() - SHORTCUT_PERMISSION_POLL_INTERVAL;
    #[cfg(target_os = "macos")]
    let mut last_accessibility_granted: Option<bool> = None;
    #[cfg(target_os = "macos")]
    let mut last_probe_at = Instant::now() - SHORTCUT_RUNTIME_PROBE_INTERVAL;

    loop {
        if state.shutdown.load(Ordering::SeqCst) {
            tracing::info!("hotkey_manager_shutdown_requested");
            break;
        }

        #[cfg(target_os = "macos")]
        if last_permission_poll_at.elapsed() >= SHORTCUT_PERMISSION_POLL_INTERVAL {
            let permission_snapshot = crate::permissions::report_permission_snapshot_if_changed(
                "shortcut_runtime_permission_poll",
            );
            let accessibility_granted =
                permission_snapshot.accessibility == crate::permissions::PermissionStatus::Granted;
            let action = runtime_permission_action(
                runtime.is_some(),
                last_accessibility_granted,
                accessibility_granted,
            );

            match action {
                RuntimePermissionAction::Mount => {
                    tracing::info!("shortcut_runtime_permission_available");
                    if let Err(error) =
                        remount_shortcut_runtime(&mut runtime, &state, &event_tx, &app_handle)
                    {
                        tracing::warn!(error = %error, "shortcut_runtime_mount_failed");
                    }
                }
                RuntimePermissionAction::Unmount => {
                    tracing::warn!("shortcut_runtime_permission_lost");
                    unmount_shortcut_runtime(&mut runtime, &state, "accessibility_permission_lost");
                }
                RuntimePermissionAction::Keep => {}
            }

            last_accessibility_granted = Some(accessibility_granted);
            last_permission_poll_at = Instant::now();
        }

        #[cfg(target_os = "macos")]
        if last_probe_at.elapsed() >= SHORTCUT_RUNTIME_PROBE_INTERVAL {
            let probe_result = super::macos::fresh_event_tap_probe();
            let action = runtime_probe_action(runtime.is_some(), probe_result.is_ok());

            match action {
                RuntimeProbeAction::Mount => {
                    tracing::info!("shortcut_runtime_probe_recovered");
                    if let Err(error) =
                        remount_shortcut_runtime(&mut runtime, &state, &event_tx, &app_handle)
                    {
                        tracing::warn!(error = %error, "shortcut_runtime_mount_failed");
                    }
                }
                RuntimeProbeAction::Unmount => {
                    let error = probe_result
                        .err()
                        .unwrap_or_else(|| "fresh event tap probe failed".to_string());
                    tracing::warn!(error = %error, "shortcut_runtime_probe_failed");
                    unmount_shortcut_runtime(&mut runtime, &state, "permission_probe_failed");
                }
                RuntimeProbeAction::Keep => {}
            }

            last_probe_at = Instant::now();
        }

        process_pending_commands(&mut runtime, &state, &event_tx, &app_handle);

        if let Some(runtime) = runtime.as_ref() {
            match runtime.manager.try_recv() {
                Some(event) => handle_hotkey_event(event, &state, &event_tx, &app_handle),
                None => thread::sleep(Duration::from_millis(50)),
            }
        } else {
            thread::sleep(Duration::from_millis(50));
        }
    }

    unmount_shortcut_runtime(&mut runtime, &state, "shutdown");
    tracing::info!("hotkey_manager_loop_exited");
}

fn process_pending_commands(
    runtime: &mut Option<ShortcutRuntime>,
    state: &Arc<ManagerState>,
    event_tx: &Sender<ShortcutEvent>,
    app_handle: &tauri::AppHandle,
) {
    let mut pending = state.pending_commands.lock();
    let commands: Vec<ShortcutCommand> = pending.drain(..).collect();
    drop(pending);

    for command in commands {
        match command {
            ShortcutCommand::Register(hotkey_str) => {
                *state.desired_hotkey.lock() = Some(hotkey_str);

                if runtime.is_some() {
                    if let Err(error) =
                        remount_shortcut_runtime(runtime, state, event_tx, app_handle)
                    {
                        tracing::warn!(error = %error, "shortcut_runtime_remount_failed");
                    }
                } else {
                    tracing::info!("shortcut_register_deferred_until_runtime_available");
                }
            }
            ShortcutCommand::Unregister => {
                *state.desired_hotkey.lock() = None;
                if let Some(runtime) = runtime.as_ref() {
                    unregister_current_hotkey(&runtime.manager, state, "explicit");
                } else {
                    clear_live_registrations(state);
                }
            }
            ShortcutCommand::RegisterCancel { owner_task_id } => {
                *state.cancel_owner_task_id.lock() = Some(owner_task_id);

                if let Some(runtime) = runtime.as_ref() {
                    let mut current = state.cancel_ids.lock();
                    unregister_cancel_hotkeys(&runtime.manager, &mut current);
                    let cancel_hotkeys = build_cancel_hotkeys_for_app(app_handle);
                    *current = register_cancel_hotkeys(&runtime.manager, &cancel_hotkeys);
                }
            }
            ShortcutCommand::UnregisterCancel { owner_task_id } => {
                let mut cancel_owner_task_id = state.cancel_owner_task_id.lock();
                if should_unregister_cancel_hotkeys(*cancel_owner_task_id, owner_task_id) {
                    *cancel_owner_task_id = None;
                    drop(cancel_owner_task_id);

                    if let Some(runtime) = runtime.as_ref() {
                        let mut current = state.cancel_ids.lock();
                        unregister_cancel_hotkeys(&runtime.manager, &mut current);
                    } else {
                        state.cancel_ids.lock().clear();
                    }
                } else {
                    tracing::info!(
                        current_owner_task_id = ?*cancel_owner_task_id,
                        requested_owner_task_id = ?owner_task_id,
                        "stale_cancel_hotkey_unregister_ignored"
                    );
                }
            }
        }
    }
}

fn handle_hotkey_event(
    event: handy_keys::HotkeyEvent,
    state: &Arc<ManagerState>,
    event_tx: &Sender<ShortcutEvent>,
    app_handle: &tauri::AppHandle,
) {
    let state_enum = match event.state {
        handy_keys::HotkeyState::Pressed => ShortcutState::Pressed,
        handy_keys::HotkeyState::Released => ShortcutState::Released,
    };

    let current_id = *state.current_id.lock();
    let cancel_ids = state.cancel_ids.lock().clone();

    if Some(event.id) == current_id {
        tracing::info!(state = %state_enum.as_str(), "hotkey_triggered");
        let _ = event_tx.send(ShortcutEvent::Triggered { state: state_enum });
        let _ = app_handle.emit(EventName::SHORTCUT_TRIGGERED, state_enum.as_str());
        handle_recording_trigger(app_handle, state_enum);
    } else if cancel_ids.contains(&event.id) {
        tracing::info!(state = %state_enum.as_str(), "cancel_hotkey_triggered");
        let _ = event_tx.send(ShortcutEvent::CancelTriggered { state: state_enum });
        handle_cancel_trigger(app_handle, state_enum);
    }
}

fn remount_shortcut_runtime(
    runtime: &mut Option<ShortcutRuntime>,
    state: &Arc<ManagerState>,
    event_tx: &Sender<ShortcutEvent>,
    app_handle: &tauri::AppHandle,
) -> Result<(), String> {
    unmount_shortcut_runtime(runtime, state, "remount");
    let new_runtime = mount_shortcut_runtime(state, event_tx, app_handle)?;
    *runtime = Some(new_runtime);
    Ok(())
}

fn mount_shortcut_runtime(
    state: &Arc<ManagerState>,
    event_tx: &Sender<ShortcutEvent>,
    app_handle: &tauri::AppHandle,
) -> Result<ShortcutRuntime, String> {
    #[cfg(target_os = "macos")]
    super::macos::fresh_event_tap_probe()?;

    let manager = create_hotkey_manager().map_err(|e| e.to_string())?;

    #[cfg(target_os = "macos")]
    let runtime = {
        let mut blocker = FnEmojiBlocker::new();
        blocker.start()?;
        tracing::info!("fn_emoji_blocker_started_for_shortcut_manager");
        ShortcutRuntime {
            manager,
            fn_emoji_blocker: blocker,
        }
    };

    #[cfg(not(target_os = "macos"))]
    let runtime = ShortcutRuntime { manager };

    apply_runtime_registrations(&runtime, state, event_tx, app_handle);
    tracing::info!("shortcut_runtime_mounted");
    Ok(runtime)
}

fn unmount_shortcut_runtime(
    runtime: &mut Option<ShortcutRuntime>,
    state: &Arc<ManagerState>,
    reason: &'static str,
) {
    if runtime.take().is_some() {
        tracing::info!(reason, "shortcut_runtime_unmounted");
    }
    let _ = stop_recording_capture_listener(state, reason);
    clear_live_registrations(state);
}

fn clear_live_registrations(state: &Arc<ManagerState>) {
    *state.current_id.lock() = None;
    state.cancel_ids.lock().clear();
}

fn apply_runtime_registrations(
    runtime: &ShortcutRuntime,
    state: &Arc<ManagerState>,
    event_tx: &Sender<ShortcutEvent>,
    app_handle: &tauri::AppHandle,
) {
    clear_live_registrations(state);

    if state.cancel_owner_task_id.lock().is_some() {
        let cancel_hotkeys = build_cancel_hotkeys_for_app(app_handle);
        let registered_ids = register_cancel_hotkeys(&runtime.manager, &cancel_hotkeys);
        tracing::info!(
            cancel_hotkeys = ?cancel_hotkeys,
            count = registered_ids.len(),
            "cancel_hotkeys_registered"
        );
        *state.cancel_ids.lock() = registered_ids;
    }

    if let Some(hotkey_str) = state.desired_hotkey.lock().clone() {
        match register_hotkey(&runtime.manager, &hotkey_str) {
            Ok(id) => {
                tracing::info!(hotkey = %hotkey_str, id = id.as_u32(), "hotkey_registered");
                *state.current_id.lock() = Some(id);
            }
            Err(error) => {
                *state.desired_hotkey.lock() = None;
                tracing::error!(
                    hotkey = %hotkey_str,
                    error = %error,
                    "hotkey_registration_failed"
                );
                emit_registration_failure(event_tx, app_handle, error);
            }
        }
    }
}

fn unregister_current_hotkey(
    manager: &handy_keys::HotkeyManager,
    state: &Arc<ManagerState>,
    reason: &'static str,
) {
    let mut current = state.current_id.lock();
    if let Some(old_id) = current.take() {
        tracing::info!(old_id = old_id.as_u32(), reason, "unregistering_old_hotkey");
        if let Err(error) = manager.unregister(old_id) {
            tracing::warn!(error = ?error, reason, "old_hotkey_unregister_failed");
        } else {
            tracing::info!(reason, "old_hotkey_unregistered");
        }
    }
}

fn take_recording_listener(state: &Arc<ManagerState>) -> Option<RecordingListener> {
    state.recording_listener.lock().take()
}

fn stop_recording_capture_listener(
    state: &Arc<ManagerState>,
    reason: &'static str,
) -> Option<String> {
    let mut listener = take_recording_listener(state)?;
    let captured_hotkey = listener.stop();
    tracing::info!(reason, captured = ?captured_hotkey, "recording_capture_stopped");
    captured_hotkey
}

fn commit_captured_primary_hotkey(
    manager: &ShortcutManager,
    app_handle: &tauri::AppHandle,
    new_hotkey: &str,
) -> Result<(), String> {
    let app_state = app_handle
        .try_state::<crate::state::app_state::AppState>()
        .ok_or_else(|| "app state unavailable".to_string())?;

    manager.unregister_primary()?;
    manager.register_primary(new_hotkey)?;

    {
        let mut settings = app_state.settings.lock();
        settings.hotkey = new_hotkey.to_string();
    }

    save_settings_internal(app_handle)?;

    let settings = app_state.settings.lock().clone();
    app_handle
        .emit(EventName::SETTINGS_CHANGED, settings)
        .map_err(|error| format!("failed to emit settings changed: {error}"))?;

    Ok(())
}

fn emit_registration_failure(
    event_tx: &Sender<ShortcutEvent>,
    app_handle: &tauri::AppHandle,
    error: String,
) {
    let _ = event_tx.send(ShortcutEvent::RegistrationFailed {
        error: error.clone(),
    });
    let _ = app_handle.emit(EventName::SHORTCUT_REGISTRATION_FAILED, error);
}

fn create_hotkey_manager() -> Result<handy_keys::HotkeyManager, handy_keys::Error> {
    // The blocking constructor is required so the active app does not also receive
    // the shortcut's key events, such as Slash in Cmd+Slash.
    handy_keys::HotkeyManager::new_with_blocking()
}

fn handle_cancel_trigger(app_handle: &tauri::AppHandle, state: ShortcutState) {
    match state {
        ShortcutState::Pressed => {
            if let Some(app_state) = app_handle.try_state::<crate::state::app_state::AppState>() {
                let is_recording = app_state.is_recording.load(Ordering::SeqCst);
                let is_transcribing = app_state.is_transcribing.load(Ordering::SeqCst);
                let task_id = app_state.task_counter.load(Ordering::SeqCst);

                if let Some(shortcut_manager) =
                    app_handle.try_state::<crate::shortcut::ShortcutManager>()
                {
                    *shortcut_manager
                        .state
                        .pending_cancel_release_owner_task_id
                        .lock() =
                        capture_cancel_hotkey_release_owner(is_recording, is_transcribing, task_id);
                }
            }

            tracing::info!("cancel_hotkey_pressed, canceling recording");
            let _ = crate::commands::audio::cancel_recording_from_hotkey_sync(app_handle.clone());
        }
        ShortcutState::Released => {
            let Some(app_state) = app_handle.try_state::<crate::state::app_state::AppState>()
            else {
                return;
            };

            let is_recording = app_state.is_recording.load(Ordering::SeqCst);
            let is_transcribing = app_state.is_transcribing.load(Ordering::SeqCst);

            let pending_owner_task_id = app_handle
                .try_state::<crate::shortcut::ShortcutManager>()
                .and_then(|shortcut_manager| {
                    shortcut_manager
                        .state
                        .pending_cancel_release_owner_task_id
                        .lock()
                        .take()
                });

            if let Some(owner_task_id) = cancel_hotkey_release_unregister_owner(
                is_recording,
                is_transcribing,
                pending_owner_task_id,
            ) {
                tracing::info!("cancel_hotkey_released_after_cancel, unregistering_cancel_hotkey");
                if let Some(shortcut_manager) =
                    app_handle.try_state::<crate::shortcut::ShortcutManager>()
                {
                    let _ = shortcut_manager.unregister_cancel_for_task(owner_task_id);
                }
            }
        }
    }
}

fn build_cancel_hotkeys_for_app(app_handle: &tauri::AppHandle) -> Vec<String> {
    let Some(app_state) = app_handle.try_state::<crate::state::app_state::AppState>() else {
        return vec!["Escape".to_string()];
    };

    let settings = app_state.settings.lock();
    build_cancel_hotkeys(&settings.recording_mode, &settings.hotkey)
}

fn register_cancel_hotkeys(
    manager: &handy_keys::HotkeyManager,
    hotkeys: &[String],
) -> Vec<handy_keys::HotkeyId> {
    let mut ids = Vec::new();

    for hotkey in hotkeys {
        match register_hotkey(manager, hotkey) {
            Ok(id) => {
                tracing::info!(hotkey = %hotkey, id = id.as_u32(), "cancel_hotkey_registered");
                ids.push(id);
            }
            Err(e) => {
                tracing::error!(hotkey = %hotkey, error = %e, "cancel_hotkey_registration_failed");
            }
        }
    }

    ids
}

fn unregister_cancel_hotkeys(
    manager: &handy_keys::HotkeyManager,
    current: &mut Vec<handy_keys::HotkeyId>,
) {
    for old_id in current.drain(..) {
        tracing::info!(old_id = old_id.as_u32(), "unregistering_cancel_hotkey");
        if let Err(e) = manager.unregister(old_id) {
            tracing::warn!(error = ?e, "cancel_hotkey_unregister_failed");
        } else {
            tracing::info!("cancel_hotkey_unregistered");
        }
    }
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
    tracing::debug!(state = %state.as_str(), "handle_recording_trigger_entered");

    let Some(app_state) = app_handle.try_state::<crate::state::app_state::AppState>() else {
        tracing::error!("app_state_unavailable_for_recording_trigger");
        return;
    };

    let capture_active = app_handle
        .try_state::<crate::shortcut::ShortcutManager>()
        .is_some_and(|shortcut_manager| shortcut_manager.is_recording_capture_active());
    let context = primary_shortcut_context(&app_state, capture_active);
    tracing::debug!(
        capture_active = context.capture_active,
        is_recording = context.is_recording,
        recording_mode = ?context.recording_mode,
        "handle_recording_trigger_state"
    );

    match primary_shortcut_action(context, state) {
        PrimaryShortcutAction::Ignore => {
            if context.capture_active {
                tracing::info!("capture_mode_active_hotkey_trigger_ignored");
            }
        }
        PrimaryShortcutAction::StartRecording => {
            tracing::info!("shortcut_start_recording_requested");
            match crate::commands::audio::start_recording_sync(app_handle.clone()) {
                Ok(_) => tracing::info!("shortcut_recording_started"),
                Err(e) => tracing::error!(error = %e, "shortcut_start_failed"),
            }
        }
        PrimaryShortcutAction::StopRecording => {
            tracing::info!("shortcut_stop_recording_requested");
            match crate::commands::audio::stop_recording_sync(app_handle.clone()) {
                Ok(_) => tracing::info!("shortcut_recording_stopped"),
                Err(e) => tracing::error!(error = %e, "shortcut_stop_failed"),
            }
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

fn build_cancel_hotkeys(recording_mode: &str, hotkey: &str) -> Vec<String> {
    let mut cancel_hotkeys = vec!["Escape".to_string()];

    if recording_mode.eq_ignore_ascii_case("hold") {
        let modifiers = hotkey
            .split('+')
            .map(str::trim)
            .filter(|token| !token.is_empty() && is_modifier_token(token))
            .collect::<Vec<_>>();

        if !modifiers.is_empty() {
            let modifier_escape = format!("{}+Escape", modifiers.join("+"));
            if !cancel_hotkeys.contains(&modifier_escape) {
                cancel_hotkeys.push(modifier_escape);
            }
        }
    }

    cancel_hotkeys
}

fn is_modifier_token(token: &str) -> bool {
    matches!(
        token.to_ascii_lowercase().as_str(),
        "cmd"
            | "command"
            | "meta"
            | "super"
            | "win"
            | "ctrl"
            | "control"
            | "opt"
            | "option"
            | "alt"
            | "shift"
            | "fn"
            | "function"
            | "cmdleft"
            | "cmdright"
            | "ctrlleft"
            | "ctrlright"
            | "optleft"
            | "optright"
            | "shiftleft"
            | "shiftright"
    )
}

/// FN/Globe key name constant
const FN_KEY_NAME: &str = "fn";

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "macos")]
    #[test]
    fn runtime_permission_action_mounts_on_startup_when_accessibility_is_granted() {
        assert_eq!(
            runtime_permission_action(false, None, true),
            RuntimePermissionAction::Mount
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn runtime_permission_action_unmounts_when_accessibility_is_revoked() {
        assert_eq!(
            runtime_permission_action(true, Some(true), false),
            RuntimePermissionAction::Unmount
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn runtime_permission_action_mounts_when_accessibility_recovers() {
        assert_eq!(
            runtime_permission_action(false, Some(false), true),
            RuntimePermissionAction::Mount
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn runtime_permission_action_keeps_existing_state_without_transition() {
        assert_eq!(
            runtime_permission_action(true, Some(true), true),
            RuntimePermissionAction::Keep
        );
        assert_eq!(
            runtime_permission_action(false, Some(true), true),
            RuntimePermissionAction::Keep
        );
        assert_eq!(
            runtime_permission_action(false, None, false),
            RuntimePermissionAction::Keep
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn runtime_probe_action_mounts_when_probe_recovers() {
        assert_eq!(runtime_probe_action(false, true), RuntimeProbeAction::Mount);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn runtime_probe_action_unmounts_when_probe_fails_while_active() {
        assert_eq!(
            runtime_probe_action(true, false),
            RuntimeProbeAction::Unmount
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn runtime_probe_action_keeps_existing_state_when_no_transition_is_needed() {
        assert_eq!(runtime_probe_action(true, true), RuntimeProbeAction::Keep);
        assert_eq!(runtime_probe_action(false, false), RuntimeProbeAction::Keep);
    }

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
        let result = manager.register_primary("Shift+Space");
        assert!(result.is_ok());

        let pending = manager.state.pending_commands.lock();
        if let Some(ShortcutCommand::Register(ref h)) = pending.last() {
            assert_eq!(h, "Shift+Space");
        } else {
            panic!("Expected ShortcutCommand::Register");
        }
    }

    #[test]
    fn recording_capture_is_inactive_by_default() {
        let manager = ShortcutManager::new().unwrap();
        assert!(!manager.is_recording_capture_active());
        assert_eq!(manager.peek_recording_capture(), None);
    }

    #[test]
    fn stop_recording_capture_without_listener_is_noop() {
        let manager = ShortcutManager::new().unwrap();
        assert_eq!(manager.stop_recording_capture().unwrap(), None);
    }

    #[test]
    fn cancel_recording_capture_clears_listener_state() {
        let manager = ShortcutManager::new().unwrap();
        *manager.state.recording_listener.lock() = Some(RecordingListener::default());

        manager.cancel_recording_capture();

        assert!(manager.state.recording_listener.lock().is_none());
    }

    #[test]
    fn test_manager_stop_without_start() {
        let mut manager = ShortcutManager::new().unwrap();
        let result = manager.stop();
        assert!(result.is_ok()); // Should handle gracefully
    }

    #[test]
    fn test_cancel_hotkeys_toggle_mode_uses_plain_escape_only() {
        assert_eq!(
            build_cancel_hotkeys("toggle", "Shift+Space"),
            vec!["Escape".to_string()]
        );
    }

    #[test]
    fn test_cancel_hotkeys_hold_mode_includes_active_modifiers() {
        assert_eq!(
            build_cancel_hotkeys("hold", "Cmd+Shift+Space"),
            vec!["Escape".to_string(), "Cmd+Shift+Escape".to_string()]
        );
    }

    #[test]
    fn test_cancel_hotkeys_hold_mode_ignores_non_modifier_key() {
        assert_eq!(
            build_cancel_hotkeys("hold", "F13"),
            vec!["Escape".to_string()]
        );
    }

    #[test]
    fn stale_unregister_does_not_clear_new_cancel_owner() {
        let mut cancel_owner_task_id = Some(2);

        if should_unregister_cancel_hotkeys(cancel_owner_task_id, Some(1)) {
            cancel_owner_task_id = None;
        }

        assert_eq!(cancel_owner_task_id, Some(2));
    }

    #[test]
    fn matching_unregister_clears_current_cancel_owner() {
        let mut cancel_owner_task_id = Some(3);

        if should_unregister_cancel_hotkeys(cancel_owner_task_id, Some(3)) {
            cancel_owner_task_id = None;
        }

        assert_eq!(cancel_owner_task_id, None);
    }

    #[test]
    fn unconditional_unregister_still_clears_current_cancel_owner() {
        let mut cancel_owner_task_id = Some(4);

        if should_unregister_cancel_hotkeys(cancel_owner_task_id, None) {
            cancel_owner_task_id = None;
        }

        assert_eq!(cancel_owner_task_id, None);
    }

    #[test]
    fn cancel_hotkey_press_captures_owner_only_while_session_is_active() {
        assert_eq!(capture_cancel_hotkey_release_owner(true, false, 8), Some(8));
        assert_eq!(capture_cancel_hotkey_release_owner(false, true, 8), Some(8));
        assert_eq!(capture_cancel_hotkey_release_owner(false, false, 8), None);
    }

    #[test]
    fn cancel_hotkey_release_unregisters_the_observed_task_only_when_idle() {
        assert_eq!(
            cancel_hotkey_release_unregister_owner(false, false, Some(7)),
            Some(7)
        );
        assert_eq!(
            cancel_hotkey_release_unregister_owner(true, false, Some(7)),
            None
        );
        assert_eq!(
            cancel_hotkey_release_unregister_owner(false, true, Some(7)),
            None
        );
    }

    #[test]
    fn cancel_hotkey_release_drops_when_no_cancel_owner_was_captured() {
        assert_eq!(
            cancel_hotkey_release_unregister_owner(false, false, None),
            None
        );
    }
}
