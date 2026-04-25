//! Shortcut manager implementation.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;
use std::time::Instant;

use parking_lot::{Mutex, RwLock};
use tauri::{Emitter, Manager};

use crate::events::EventName;
use crate::services::shortcut::{
    cancel_hotkey_release_unregister_owner, capture_cancel_hotkey_release_owner,
    primary_shortcut_action, primary_shortcut_context, should_unregister_cancel_hotkeys,
    PrimaryShortcutAction,
};

use super::hotkey_codec::{analyze_pressed_sequence, PressedInput};
use super::hotkey_codec::{canonicalize_hotkey_string, parse_hotkey_pattern};
use super::matcher::{MatcherEvent, MatcherSnapshot};
use super::platform::{
    start_platform_runner, PlatformRunner, RunnerMode, RuntimeEvent, SharedMatcherSnapshot,
};
use super::profile_types::{ShortcutProfile, ShortcutTriggerMode};
use super::types::{ShortcutEvent, ShortcutState};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct RuntimeSnapshot {
    desired_profiles: HashMap<String, String>,
    live_profiles: HashMap<String, String>,
    capture_active: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum MutationMode {
    Immediate,
    Deferred,
}

trait RuntimeApplier {
    fn apply_profiles(&mut self, desired_profiles: &HashMap<String, String>) -> Result<(), String>;
}

#[derive(Default)]
struct ManagerState {
    desired_profiles: HashMap<String, ShortcutProfile>,
    live_profiles: HashMap<String, String>,
}

impl ManagerState {
    fn new() -> Self {
        Self::default()
    }

    fn register_profile(
        &mut self,
        profile_id: &str,
        profile: ShortcutProfile,
        runtime_applier: Option<&mut dyn RuntimeApplier>,
    ) -> Result<MutationMode, String> {
        let previous_desired = self.desired_profiles.clone();
        let previous_live = self.live_profiles.clone();

        self.desired_profiles
            .retain(|existing_profile_id, existing_profile| {
                existing_profile_id == profile_id || existing_profile.hotkey != profile.hotkey
            });

        self.desired_profiles
            .insert(profile_id.to_string(), profile.clone());

        let mode = if let Some(runtime_applier) = runtime_applier {
            let desired_hotkeys = self
                .desired_profiles
                .iter()
                .map(|(desired_profile_id, desired_profile)| {
                    (desired_profile_id.clone(), desired_profile.hotkey.clone())
                })
                .collect::<HashMap<_, _>>();

            if let Err(error) = runtime_applier.apply_profiles(&desired_hotkeys) {
                self.desired_profiles = previous_desired;
                self.live_profiles = previous_live;
                return Err(error);
            }

            self.live_profiles = desired_hotkeys
                .into_iter()
                .filter(|(_, hotkey)| !hotkey.is_empty())
                .collect();
            MutationMode::Immediate
        } else {
            MutationMode::Deferred
        };

        Ok(mode)
    }

    fn unregister_profile(
        &mut self,
        profile_id: &str,
        runtime_applier: Option<&mut dyn RuntimeApplier>,
    ) -> Result<MutationMode, String> {
        let previous_desired = self.desired_profiles.clone();
        let previous_live = self.live_profiles.clone();

        self.desired_profiles.remove(profile_id);

        let mode = if let Some(runtime_applier) = runtime_applier {
            let desired_hotkeys = self
                .desired_profiles
                .iter()
                .map(|(desired_profile_id, desired_profile)| {
                    (desired_profile_id.clone(), desired_profile.hotkey.clone())
                })
                .collect::<HashMap<_, _>>();

            if let Err(error) = runtime_applier.apply_profiles(&desired_hotkeys) {
                self.desired_profiles = previous_desired;
                self.live_profiles = previous_live;
                return Err(error);
            }

            self.live_profiles = desired_hotkeys;
            MutationMode::Immediate
        } else {
            self.live_profiles.remove(profile_id);
            MutationMode::Deferred
        };

        Ok(mode)
    }

    fn runtime_became_unavailable(&mut self) {
        self.live_profiles.clear();
    }

    fn replayed_live_profiles(&mut self) {
        self.live_profiles = self
            .desired_profiles
            .iter()
            .map(|(profile_id, profile)| (profile_id.clone(), profile.hotkey.clone()))
            .collect();
    }

    fn snapshot(&self) -> RuntimeSnapshot {
        RuntimeSnapshot {
            desired_profiles: self
                .desired_profiles
                .iter()
                .map(|(profile_id, profile)| (profile_id.clone(), profile.hotkey.clone()))
                .collect(),
            live_profiles: self.live_profiles.clone(),
            capture_active: false,
        }
    }
}

#[derive(Debug)]
enum ManagerCommand {
    Start {
        app_handle: tauri::AppHandle,
        reply: Sender<Result<(), String>>,
    },
    Shutdown {
        reply: Sender<Result<(), String>>,
    },
    RegisterProfile {
        profile_id: String,
        profile: ShortcutProfile,
        reply: Sender<Result<(), String>>,
    },
    UnregisterProfile {
        profile_id: String,
        reply: Sender<Result<(), String>>,
    },
    RegisterCancel {
        owner_task_id: u64,
        reply: Sender<Result<(), String>>,
    },
    UnregisterCancel {
        owner_task_id: Option<u64>,
        reply: Sender<Result<(), String>>,
    },
    StartCapture {
        reply: Sender<Result<(), String>>,
    },
    StopCapture {
        reply: Sender<Result<Option<String>, String>>,
    },
    CancelCapture {
        reply: Sender<()>,
    },
}

struct FacadeState {
    capture_active: AtomicBool,
    last_captured_hotkey: Mutex<Option<String>>,
}

struct OwnerState {
    app_handle: Option<tauri::AppHandle>,
    started: bool,
    shutdown: bool,
    command_rx: Receiver<ManagerCommand>,
    runtime_event_rx: Receiver<RuntimeEvent>,
    runtime_event_tx: Sender<RuntimeEvent>,
    event_tx: Sender<ShortcutEvent>,
    facade_state: Arc<FacadeState>,
    manager_state: ManagerState,
    matcher_snapshot: SharedMatcherSnapshot,
    main_runner: Option<Box<dyn PlatformRunner>>,
    main_runner_generation: Option<u64>,
    capture_runner: Option<Box<dyn PlatformRunner>>,
    capture_runner_generation: Option<u64>,
    next_runner_generation: u64,
    cancel_owner_task_id: Option<u64>,
    pending_cancel_release_owner_task_id: Option<u64>,
    capture_sequence: Vec<PressedInput>,
}

/// Public shortcut manager facade.
pub struct ShortcutManager {
    command_tx: Sender<ManagerCommand>,
    event_rx: Mutex<Receiver<ShortcutEvent>>,
    thread_handle: Option<JoinHandle<()>>,
    facade_state: Arc<FacadeState>,
}

impl ShortcutManager {
    pub fn new() -> Result<Self, String> {
        let (command_tx, command_rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();
        let (runtime_event_tx, runtime_event_rx) = mpsc::channel();

        let facade_state = Arc::new(FacadeState {
            capture_active: AtomicBool::new(false),
            last_captured_hotkey: Mutex::new(None),
        });

        let owner_state = OwnerState {
            app_handle: None,
            started: false,
            shutdown: false,
            command_rx,
            runtime_event_rx,
            runtime_event_tx,
            event_tx,
            facade_state: Arc::clone(&facade_state),
            manager_state: ManagerState::new(),
            matcher_snapshot: Arc::new(RwLock::new(MatcherSnapshot::default())),
            main_runner: None,
            main_runner_generation: None,
            capture_runner: None,
            capture_runner_generation: None,
            next_runner_generation: 1,
            cancel_owner_task_id: None,
            pending_cancel_release_owner_task_id: None,
            capture_sequence: Vec::new(),
        };

        let handle = thread::spawn(move || owner_loop(owner_state));

        Ok(Self {
            command_tx,
            event_rx: Mutex::new(event_rx),
            thread_handle: Some(handle),
            facade_state,
        })
    }

    pub fn start(&mut self, app_handle: tauri::AppHandle) -> Result<(), String> {
        self.request(|reply| ManagerCommand::Start { app_handle, reply })
    }

    pub fn register_profile(&self, key: &str, profile: &ShortcutProfile) -> Result<(), String> {
        self.request(|reply| ManagerCommand::RegisterProfile {
            profile_id: key.to_string(),
            profile: profile.clone(),
            reply,
        })
    }

    pub fn unregister_profile(&self, key: &str) -> Result<(), String> {
        self.request(|reply| ManagerCommand::UnregisterProfile {
            profile_id: key.to_string(),
            reply,
        })
    }

    pub fn register_cancel(&self, owner_task_id: u64) -> Result<(), String> {
        self.request(|reply| ManagerCommand::RegisterCancel {
            owner_task_id,
            reply,
        })
    }

    pub fn unregister_cancel(&self) -> Result<(), String> {
        self.request(|reply| ManagerCommand::UnregisterCancel {
            owner_task_id: None,
            reply,
        })
    }

    pub fn unregister_cancel_for_task(&self, owner_task_id: u64) -> Result<(), String> {
        self.request(|reply| ManagerCommand::UnregisterCancel {
            owner_task_id: Some(owner_task_id),
            reply,
        })
    }

    pub fn start_recording_capture(&self) -> Result<(), String> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.command_tx
            .send(ManagerCommand::StartCapture { reply: reply_tx })
            .map_err(|error| format!("shortcut manager command send failed: {error}"))?;
        reply_rx
            .recv()
            .map_err(|error| format!("shortcut manager reply receive failed: {error}"))?
    }

    pub fn stop_recording_capture(&self) -> Result<Option<String>, String> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.command_tx
            .send(ManagerCommand::StopCapture { reply: reply_tx })
            .map_err(|error| format!("shortcut manager command send failed: {error}"))?;
        reply_rx
            .recv()
            .map_err(|error| format!("shortcut manager reply receive failed: {error}"))?
    }

    pub fn cancel_recording_capture(&self) {
        let (reply_tx, reply_rx) = mpsc::channel();
        let _ = self
            .command_tx
            .send(ManagerCommand::CancelCapture { reply: reply_tx });
        let _ = reply_rx.recv();
    }

    pub fn peek_recording_capture(&self) -> Option<String> {
        self.facade_state.last_captured_hotkey.lock().clone()
    }

    pub fn is_recording_capture_active(&self) -> bool {
        self.facade_state.capture_active.load(Ordering::SeqCst)
    }

    pub fn stop(&mut self) -> Result<(), String> {
        if self.thread_handle.is_none() {
            return Ok(());
        }

        let result = self.request(|reply| ManagerCommand::Shutdown { reply });

        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }

        result
    }

    pub fn event_receiver(&self) -> parking_lot::MutexGuard<'_, Receiver<ShortcutEvent>> {
        self.event_rx.lock()
    }

    fn request(
        &self,
        build_command: impl FnOnce(Sender<Result<(), String>>) -> ManagerCommand,
    ) -> Result<(), String> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.command_tx
            .send(build_command(reply_tx))
            .map_err(|error| format!("shortcut manager command send failed: {error}"))?;
        reply_rx
            .recv()
            .map_err(|error| format!("shortcut manager reply receive failed: {error}"))?
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
#[cfg(target_os = "macos")]
const SHORTCUT_CAPTURE_RECONCILE_INTERVAL: Duration = Duration::from_millis(500);
const SHORTCUT_RUNTIME_RESTART_INTERVAL: Duration = Duration::from_millis(500);

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
    Keep,
}

#[cfg(target_os = "macos")]
fn runtime_probe_action(runtime_is_mounted: bool, probe_ok: bool) -> RuntimeProbeAction {
    match (runtime_is_mounted, probe_ok) {
        (false, true) => RuntimeProbeAction::Mount,
        _ => RuntimeProbeAction::Keep,
    }
}

fn owner_loop(mut state: OwnerState) {
    #[cfg(target_os = "macos")]
    let mut last_permission_poll_at = Instant::now() - SHORTCUT_PERMISSION_POLL_INTERVAL;
    #[cfg(target_os = "macos")]
    let mut last_accessibility_granted: Option<bool> = None;
    #[cfg(target_os = "macos")]
    let mut last_probe_at = Instant::now() - SHORTCUT_RUNTIME_PROBE_INTERVAL;
    #[cfg(target_os = "macos")]
    let mut last_capture_reconcile_at = Instant::now() - SHORTCUT_CAPTURE_RECONCILE_INTERVAL;
    let mut last_main_restart_at = Instant::now() - SHORTCUT_RUNTIME_RESTART_INTERVAL;
    let mut last_capture_restart_at = Instant::now() - SHORTCUT_RUNTIME_RESTART_INTERVAL;

    while !state.shutdown {
        while let Ok(command) = state.command_rx.try_recv() {
            handle_command(&mut state, command);
        }

        while let Ok(runtime_event) = state.runtime_event_rx.try_recv() {
            handle_runtime_event(
                &mut state,
                runtime_event,
                &mut last_main_restart_at,
                &mut last_capture_restart_at,
            );
        }

        #[cfg(target_os = "macos")]
        poll_macos_runtime_health(
            &mut state,
            &mut last_permission_poll_at,
            &mut last_accessibility_granted,
            &mut last_probe_at,
            &mut last_capture_reconcile_at,
        );

        #[cfg(not(target_os = "macos"))]
        if state.started && state.main_runner.is_none() {
            let _ = ensure_main_runner(&mut state);
        }

        thread::sleep(Duration::from_millis(20));
    }

    stop_main_runner(&mut state);
}

fn handle_command(state: &mut OwnerState, command: ManagerCommand) {
    match command {
        ManagerCommand::Start { app_handle, reply } => {
            let result = if state.started {
                Err("shortcut manager already started".to_string())
            } else {
                state.app_handle = Some(app_handle);
                state.started = true;
                match ensure_main_runner(state) {
                    Ok(()) => Ok(()),
                    Err(error)
                        if error.contains("Accessibility permission not granted")
                            || error.contains("Failed to create fresh event tap probe") =>
                    {
                        Ok(())
                    }
                    Err(error) => Err(error),
                }
            };
            let _ = reply.send(result);
        }
        ManagerCommand::Shutdown { reply } => {
            state.shutdown = true;
            stop_main_runner(state);
            stop_capture_runtime(state, false);
            let _ = reply.send(Ok(()));
        }
        ManagerCommand::RegisterProfile {
            profile_id,
            mut profile,
            reply,
        } => {
            let result = if profile.hotkey.is_empty() {
                if runtime_is_live(state.main_runner.is_some(), state.capture_runner.is_some()) {
                    let mut applier = SnapshotApplier {
                        matcher_snapshot: Arc::clone(&state.matcher_snapshot),
                        app_handle: state.app_handle.clone(),
                        cancel_owner_task_id: state.cancel_owner_task_id,
                        capture_active: state.facade_state.capture_active.load(Ordering::SeqCst),
                    };
                    state
                        .manager_state
                        .register_profile(&profile_id, profile, Some(&mut applier))
                        .map(|_| ())
                } else {
                    state
                        .manager_state
                        .register_profile(&profile_id, profile, None)
                        .map(|_| ())
                }
            } else {
                match canonicalize_hotkey_string(&profile.hotkey) {
                    Ok(canonical_hotkey) => {
                        profile.hotkey = canonical_hotkey;
                        if runtime_is_live(
                            state.main_runner.is_some(),
                            state.capture_runner.is_some(),
                        ) {
                            let mut applier = SnapshotApplier {
                                matcher_snapshot: Arc::clone(&state.matcher_snapshot),
                                app_handle: state.app_handle.clone(),
                                cancel_owner_task_id: state.cancel_owner_task_id,
                                capture_active: state
                                    .facade_state
                                    .capture_active
                                    .load(Ordering::SeqCst),
                            };
                            state
                                .manager_state
                                .register_profile(&profile_id, profile, Some(&mut applier))
                                .map(|_| ())
                        } else {
                            state
                                .manager_state
                                .register_profile(&profile_id, profile, None)
                                .map(|_| ())
                        }
                    }
                    Err(error) => Err(error),
                }
            };
            let _ = reply.send(result);
        }
        ManagerCommand::UnregisterProfile { profile_id, reply } => {
            let result =
                if runtime_is_live(state.main_runner.is_some(), state.capture_runner.is_some()) {
                    let mut applier = SnapshotApplier {
                        matcher_snapshot: Arc::clone(&state.matcher_snapshot),
                        app_handle: state.app_handle.clone(),
                        cancel_owner_task_id: state.cancel_owner_task_id,
                        capture_active: state.facade_state.capture_active.load(Ordering::SeqCst),
                    };
                    state
                        .manager_state
                        .unregister_profile(&profile_id, Some(&mut applier))
                        .map(|_| ())
                } else {
                    state
                        .manager_state
                        .unregister_profile(&profile_id, None)
                        .map(|_| ())
                };
            let _ = reply.send(result);
        }
        ManagerCommand::RegisterCancel {
            owner_task_id,
            reply,
        } => {
            state.cancel_owner_task_id = Some(owner_task_id);
            let result = refresh_matcher_snapshot(state);
            let _ = reply.send(result);
        }
        ManagerCommand::UnregisterCancel {
            owner_task_id,
            reply,
        } => {
            let result =
                if should_unregister_cancel_hotkeys(state.cancel_owner_task_id, owner_task_id) {
                    state.cancel_owner_task_id = None;
                    refresh_matcher_snapshot(state)
                } else {
                    Ok(())
                };
            let _ = reply.send(result);
        }
        ManagerCommand::StartCapture { reply } => {
            let result = start_capture(state);
            let _ = reply.send(result);
        }
        ManagerCommand::StopCapture { reply } => {
            let captured = stop_capture_runtime(state, true);
            let _ = reply.send(Ok(captured));
        }
        ManagerCommand::CancelCapture { reply } => {
            stop_capture_runtime(state, false);
            let _ = reply.send(());
        }
    }
}

fn handle_runtime_event(
    state: &mut OwnerState,
    runtime_event: RuntimeEvent,
    last_main_restart_at: &mut Instant,
    last_capture_restart_at: &mut Instant,
) {
    if let Some((mode, generation)) = runtime_restart_request(&runtime_event) {
        if !restart_request_matches_current_generation(
            current_runner_generation(state, mode),
            generation,
        ) {
            return;
        }

        let last_restart_at = match mode {
            RunnerMode::Main => last_main_restart_at,
            RunnerMode::CaptureOnly => last_capture_restart_at,
        };
        match runtime_restart_action(last_restart_at.elapsed(), SHORTCUT_RUNTIME_RESTART_INTERVAL) {
            RuntimeRestartAction::TeardownOnly => teardown_runner(state, mode),
            RuntimeRestartAction::TeardownAndRemount => {
                restart_runner(state, mode);
                *last_restart_at = Instant::now();
            }
        }
        return;
    }

    let RuntimeEvent::Matcher(matcher_event) = runtime_event else {
        return;
    };

    match matcher_event {
        MatcherEvent::ProfilePressed { profile_id } => {
            tracing::info!(profile_id = %profile_id, "shortcut_profile_pressed");
            emit_shortcut_event(state, ShortcutState::Pressed, &profile_id);
            if let Some(app_handle) = state.app_handle.clone() {
                handle_recording_trigger_owner_loop(
                    state,
                    &app_handle,
                    ShortcutState::Pressed,
                    &profile_id,
                );
                if let Some(app_state) = app_handle.try_state::<crate::state::app_state::AppState>()
                {
                    if app_state.is_recording.load(Ordering::SeqCst) {
                        let task_id = app_state.task_counter.load(Ordering::SeqCst);
                        state.cancel_owner_task_id = Some(task_id);
                        let _ = refresh_matcher_snapshot(state);
                    }
                }
            }
        }
        MatcherEvent::ProfileReleased { profile_id } => {
            tracing::info!(profile_id = %profile_id, "shortcut_profile_released");
            emit_shortcut_event(state, ShortcutState::Released, &profile_id);
            if let Some(app_handle) = state.app_handle.clone() {
                handle_recording_trigger_owner_loop(
                    state,
                    &app_handle,
                    ShortcutState::Released,
                    &profile_id,
                );
            }
        }
        MatcherEvent::CancelPressed => {
            let _ = state.event_tx.send(ShortcutEvent::CancelTriggered {
                state: ShortcutState::Pressed,
            });
            if let Some(app_handle) = state.app_handle.as_ref() {
                handle_cancel_trigger(app_handle, &mut state.pending_cancel_release_owner_task_id);
            }
        }
        MatcherEvent::CancelReleased => {
            let _ = state.event_tx.send(ShortcutEvent::CancelTriggered {
                state: ShortcutState::Released,
            });
            if let Some(app_handle) = state.app_handle.clone() {
                handle_cancel_release_owner_loop(state, &app_handle);
            }
        }
        MatcherEvent::CapturePressed(input) => {
            if state.facade_state.capture_active.load(Ordering::SeqCst) {
                state.capture_sequence.push(input);
            }
        }
        MatcherEvent::CaptureReleased => {
            if !state.facade_state.capture_active.load(Ordering::SeqCst) {
                return;
            }

            match analyze_pressed_sequence(&state.capture_sequence) {
                Ok(hotkey) => {
                    *state.facade_state.last_captured_hotkey.lock() = Some(hotkey.clone());
                    stop_capture_runtime(state, true);
                    if let Some(app_handle) = state.app_handle.as_ref() {
                        let _ = app_handle.emit(EventName::HOTKEY_CAPTURED, hotkey);
                    }
                }
                Err(_) => {
                    state.capture_sequence.clear();
                }
            }
        }
    }
}

fn runtime_restart_request(runtime_event: &RuntimeEvent) -> Option<(RunnerMode, u64)> {
    match runtime_event {
        RuntimeEvent::RunnerNeedsRestart { mode, generation } => Some((*mode, *generation)),
        RuntimeEvent::Matcher(_) => None,
    }
}

fn current_runner_generation(state: &OwnerState, mode: RunnerMode) -> Option<u64> {
    match mode {
        RunnerMode::Main => state.main_runner_generation,
        RunnerMode::CaptureOnly => state.capture_runner_generation,
    }
}

fn capture_runner_reconcile_needed(
    capture_active: bool,
    main_runner_is_present: bool,
    capture_runner_is_present: bool,
) -> bool {
    capture_active && !main_runner_is_present && !capture_runner_is_present
}

fn main_runner_mount_allowed(
    main_runner_is_present: bool,
    capture_runner_is_present: bool,
    capture_active: bool,
) -> bool {
    !main_runner_is_present && !capture_runner_is_present && !capture_active
}

fn runtime_is_live(main_runner_is_present: bool, capture_runner_is_present: bool) -> bool {
    main_runner_is_present || capture_runner_is_present
}

fn restart_request_matches_current_generation(
    current_generation: Option<u64>,
    request_generation: u64,
) -> bool {
    current_generation == Some(request_generation)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RuntimeRestartAction {
    TeardownOnly,
    TeardownAndRemount,
}

fn runtime_restart_action(elapsed: Duration, min_interval: Duration) -> RuntimeRestartAction {
    if runtime_restart_allowed(elapsed, min_interval) {
        RuntimeRestartAction::TeardownAndRemount
    } else {
        RuntimeRestartAction::TeardownOnly
    }
}

fn runtime_restart_allowed(elapsed: Duration, min_interval: Duration) -> bool {
    elapsed >= min_interval
}

fn teardown_runner(state: &mut OwnerState, mode: RunnerMode) {
    match mode {
        RunnerMode::Main => stop_main_runner(state),
        RunnerMode::CaptureOnly => stop_capture_runner(state),
    }
}

fn allocate_runner_generation(state: &mut OwnerState, mode: RunnerMode) -> u64 {
    let generation = state.next_runner_generation;
    state.next_runner_generation += 1;

    match mode {
        RunnerMode::Main => state.main_runner_generation = Some(generation),
        RunnerMode::CaptureOnly => state.capture_runner_generation = Some(generation),
    }

    generation
}

fn restart_runner(state: &mut OwnerState, mode: RunnerMode) {
    match mode {
        RunnerMode::Main => {
            teardown_runner(state, RunnerMode::Main);
            match replacement_runner_mode_after_restart(
                RunnerMode::Main,
                state.facade_state.capture_active.load(Ordering::SeqCst),
            ) {
                RunnerMode::Main => {
                    let _ = ensure_main_runner(state);
                }
                RunnerMode::CaptureOnly => {
                    let _ = ensure_capture_runner(state);
                }
            }
        }
        RunnerMode::CaptureOnly => {
            teardown_runner(state, RunnerMode::CaptureOnly);
            if state.facade_state.capture_active.load(Ordering::SeqCst)
                && state.main_runner.is_none()
            {
                let _ = ensure_capture_runner(state);
            }
        }
    }
}

fn replacement_runner_mode_after_restart(mode: RunnerMode, capture_active: bool) -> RunnerMode {
    match (mode, capture_active) {
        (RunnerMode::Main, true) => RunnerMode::CaptureOnly,
        _ => mode,
    }
}

fn emit_shortcut_event(state: &mut OwnerState, shortcut_state: ShortcutState, profile_id: &str) {
    let _ = state.event_tx.send(ShortcutEvent::Triggered {
        state: shortcut_state,
        profile_id: profile_id.to_string(),
    });
    if let Some(app_handle) = state.app_handle.as_ref() {
        let _ = app_handle.emit(
            EventName::SHORTCUT_TRIGGERED,
            serde_json::json!({ "state": shortcut_state.as_str(), "profile_id": profile_id }),
        );
    }
}

fn ensure_main_runner(state: &mut OwnerState) -> Result<(), String> {
    if !state.started
        || !main_runner_mount_allowed(
            state.main_runner.is_some(),
            state.capture_runner.is_some(),
            state.facade_state.capture_active.load(Ordering::SeqCst),
        )
    {
        return Ok(());
    }

    refresh_matcher_snapshot(state)?;
    let generation = allocate_runner_generation(state, RunnerMode::Main);
    let runner = start_platform_runner(
        RunnerMode::Main,
        Arc::clone(&state.matcher_snapshot),
        state.runtime_event_tx.clone(),
        generation,
    )?;
    state.main_runner = Some(runner);
    state.manager_state.replayed_live_profiles();
    Ok(())
}

fn ensure_capture_runner(state: &mut OwnerState) -> Result<(), String> {
    if state.main_runner.is_some() || state.capture_runner.is_some() {
        return Ok(());
    }

    let generation = allocate_runner_generation(state, RunnerMode::CaptureOnly);
    let runner = start_platform_runner(
        RunnerMode::CaptureOnly,
        Arc::clone(&state.matcher_snapshot),
        state.runtime_event_tx.clone(),
        generation,
    )?;
    state.capture_runner = Some(runner);
    Ok(())
}

fn stop_main_runner(state: &mut OwnerState) {
    if let Some(mut runner) = state.main_runner.take() {
        let _ = runner.stop();
    }
    state.main_runner_generation = None;
    state.manager_state.runtime_became_unavailable();
}

fn stop_capture_runner(state: &mut OwnerState) {
    if let Some(mut runner) = state.capture_runner.take() {
        let _ = runner.stop();
    }
    state.capture_runner_generation = None;
}

fn start_capture(state: &mut OwnerState) -> Result<(), String> {
    if state.facade_state.capture_active.load(Ordering::SeqCst) {
        return Err("hotkey recording already in progress".to_string());
    }

    if state.main_runner.is_none() && state.capture_runner.is_none() {
        ensure_capture_runner(state)?;
    }

    state
        .facade_state
        .capture_active
        .store(true, Ordering::SeqCst);
    *state.facade_state.last_captured_hotkey.lock() = None;
    state.capture_sequence.clear();
    if let Err(error) = refresh_matcher_snapshot(state) {
        state
            .facade_state
            .capture_active
            .store(false, Ordering::SeqCst);
        stop_capture_runner(state);
        return Err(error);
    }

    Ok(())
}

fn stop_capture_runtime(state: &mut OwnerState, preserve_captured: bool) -> Option<String> {
    let captured = state.facade_state.last_captured_hotkey.lock().clone();
    state
        .facade_state
        .capture_active
        .store(false, Ordering::SeqCst);
    if !preserve_captured {
        *state.facade_state.last_captured_hotkey.lock() = None;
    }
    state.capture_sequence.clear();
    stop_capture_runner(state);
    let _ = refresh_matcher_snapshot(state);
    if preserve_captured {
        captured
    } else {
        None
    }
}

fn refresh_matcher_snapshot(state: &mut OwnerState) -> Result<(), String> {
    let desired_hotkeys = state.manager_state.snapshot().desired_profiles;
    *state.matcher_snapshot.write() = build_matcher_snapshot(
        &desired_hotkeys,
        state.app_handle.as_ref(),
        state.cancel_owner_task_id,
        state.facade_state.capture_active.load(Ordering::SeqCst),
    )?;

    Ok(())
}

#[cfg(target_os = "macos")]
fn poll_macos_runtime_health(
    state: &mut OwnerState,
    last_permission_poll_at: &mut Instant,
    last_accessibility_granted: &mut Option<bool>,
    last_probe_at: &mut Instant,
    last_capture_reconcile_at: &mut Instant,
) {
    if !state.started {
        return;
    }

    if last_permission_poll_at.elapsed() >= SHORTCUT_PERMISSION_POLL_INTERVAL {
        let permission_snapshot = crate::permissions::report_permission_snapshot_if_changed(
            "shortcut_runtime_permission_poll",
        );
        let accessibility_granted =
            permission_snapshot.accessibility == crate::permissions::PermissionStatus::Granted;
        let action = runtime_permission_action(
            state.main_runner.is_some(),
            *last_accessibility_granted,
            accessibility_granted,
        );

        match action {
            RuntimePermissionAction::Mount => {
                let _ = ensure_main_runner(state);
            }
            RuntimePermissionAction::Unmount => {
                stop_main_runner(state);
            }
            RuntimePermissionAction::Keep => {}
        }

        *last_accessibility_granted = Some(accessibility_granted);
        *last_permission_poll_at = Instant::now();
    }

    if last_probe_at.elapsed() >= SHORTCUT_RUNTIME_PROBE_INTERVAL {
        let probe_result = super::macos::fresh_event_tap_probe();
        let action = runtime_probe_action(state.main_runner.is_some(), probe_result.is_ok());
        if let Err(error) = &probe_result {
            tracing::warn!(
                runtime_is_mounted = state.main_runner.is_some(),
                action = ?action,
                error = %error,
                "shortcut_runtime_probe_failed"
            );
        }
        match action {
            RuntimeProbeAction::Mount => {
                tracing::info!("shortcut_runtime_probe_mount_requested");
                let _ = ensure_main_runner(state);
            }
            RuntimeProbeAction::Keep => {}
        }
        *last_probe_at = Instant::now();
    }

    if last_capture_reconcile_at.elapsed() >= SHORTCUT_CAPTURE_RECONCILE_INTERVAL {
        if capture_runner_reconcile_needed(
            state.facade_state.capture_active.load(Ordering::SeqCst),
            state.main_runner.is_some(),
            state.capture_runner.is_some(),
        ) {
            let _ = ensure_capture_runner(state);
        }

        *last_capture_reconcile_at = Instant::now();
    }
}

struct SnapshotApplier {
    matcher_snapshot: SharedMatcherSnapshot,
    app_handle: Option<tauri::AppHandle>,
    cancel_owner_task_id: Option<u64>,
    capture_active: bool,
}

impl RuntimeApplier for SnapshotApplier {
    fn apply_profiles(&mut self, desired_profiles: &HashMap<String, String>) -> Result<(), String> {
        *self.matcher_snapshot.write() = build_matcher_snapshot(
            desired_profiles,
            self.app_handle.as_ref(),
            self.cancel_owner_task_id,
            self.capture_active,
        )?;

        Ok(())
    }
}

fn build_matcher_snapshot(
    desired_profiles: &HashMap<String, String>,
    app_handle: Option<&tauri::AppHandle>,
    cancel_owner_task_id: Option<u64>,
    capture_active: bool,
) -> Result<MatcherSnapshot, String> {
    let cancel_patterns = if cancel_owner_task_id.is_some() {
        build_cancel_hotkeys_for_app(app_handle)
            .into_iter()
            .map(|hotkey| parse_hotkey_pattern(&hotkey))
            .collect::<Result<Vec<_>, _>>()?
    } else {
        Vec::new()
    };

    let mut profiles = HashMap::new();
    for (profile_id, hotkey) in desired_profiles {
        if hotkey.is_empty() {
            continue;
        }
        profiles.insert(profile_id.clone(), parse_hotkey_pattern(hotkey)?);
    }

    Ok(MatcherSnapshot {
        profiles,
        cancel: cancel_patterns,
        capture_active,
    })
}

fn build_cancel_hotkeys_for_app(app_handle: Option<&tauri::AppHandle>) -> Vec<String> {
    let Some(app_handle) = app_handle else {
        return vec!["Escape".to_string()];
    };
    let Some(app_state) = app_handle.try_state::<crate::state::app_state::AppState>() else {
        return vec!["Escape".to_string()];
    };

    match app_state.current_cancel_profile() {
        Some((hotkey, trigger_mode)) => build_cancel_hotkeys(trigger_mode, &hotkey),
        None => vec!["Escape".to_string()],
    }
}

fn handle_cancel_trigger(
    app_handle: &tauri::AppHandle,
    pending_cancel_release_owner_task_id: &mut Option<u64>,
) {
    if let Some(app_state) = app_handle.try_state::<crate::state::app_state::AppState>() {
        let is_recording = app_state.is_recording.load(Ordering::SeqCst);
        let is_transcribing = app_state.is_transcribing.load(Ordering::SeqCst);
        let task_id = app_state.task_counter.load(Ordering::SeqCst);
        *pending_cancel_release_owner_task_id =
            capture_cancel_hotkey_release_owner(is_recording, is_transcribing, task_id);
    }
    let _ = crate::commands::audio::cancel_recording_from_hotkey_sync(app_handle.clone());
}

fn handle_cancel_release_owner_loop(state: &mut OwnerState, app_handle: &tauri::AppHandle) {
    let Some(app_state) = app_handle.try_state::<crate::state::app_state::AppState>() else {
        return;
    };

    let is_recording = app_state.is_recording.load(Ordering::SeqCst);
    let is_transcribing = app_state.is_transcribing.load(Ordering::SeqCst);
    let pending_owner_task_id = state.pending_cancel_release_owner_task_id.take();
    if cancel_hotkey_release_unregister_owner(is_recording, is_transcribing, pending_owner_task_id)
        .is_some()
    {
        state.cancel_owner_task_id = None;
        let _ = refresh_matcher_snapshot(state);
    }
}

fn handle_recording_trigger_owner_loop(
    owner_state: &mut OwnerState,
    app_handle: &tauri::AppHandle,
    state: ShortcutState,
    profile_id: &str,
) {
    let Some(app_state) = app_handle.try_state::<crate::state::app_state::AppState>() else {
        return;
    };

    let capture_active = owner_state
        .facade_state
        .capture_active
        .load(Ordering::SeqCst);
    let profile = {
        let settings = app_state.settings.lock();
        crate::services::shortcut::get_profile_by_key(&settings.shortcut_profiles, profile_id)
            .cloned()
    };
    let context = primary_shortcut_context(&app_state, capture_active, profile.as_ref());

    match primary_shortcut_action(context, state) {
        PrimaryShortcutAction::Ignore => {}
        PrimaryShortcutAction::StartRecording => {
            let _ = crate::commands::audio::start_recording_sync_internal(
                app_handle,
                profile.as_ref(),
                false,
            );
        }
        PrimaryShortcutAction::StopRecording => {
            let _ = crate::commands::audio::stop_recording_sync(app_handle.clone());
        }
    }
}

fn build_cancel_hotkeys(trigger_mode: ShortcutTriggerMode, hotkey: &str) -> Vec<String> {
    let mut cancel_hotkeys = vec!["Escape".to_string()];

    if trigger_mode == ShortcutTriggerMode::Hold {
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::shortcut::{ShortcutAction, ShortcutProfile, ShortcutTriggerMode};

    struct FakeRuntimeApplier {
        should_fail: bool,
        applied: Vec<HashMap<String, String>>,
    }

    impl FakeRuntimeApplier {
        fn successful() -> Self {
            Self {
                should_fail: false,
                applied: Vec::new(),
            }
        }

        fn failing() -> Self {
            Self {
                should_fail: true,
                applied: Vec::new(),
            }
        }
    }

    impl RuntimeApplier for FakeRuntimeApplier {
        fn apply_profiles(
            &mut self,
            desired_profiles: &HashMap<String, String>,
        ) -> Result<(), String> {
            self.applied.push(desired_profiles.clone());
            if self.should_fail {
                Err("runtime_apply_failed".to_string())
            } else {
                Ok(())
            }
        }
    }

    fn profile(hotkey: &str) -> ShortcutProfile {
        ShortcutProfile {
            hotkey: hotkey.to_string(),
            trigger_mode: ShortcutTriggerMode::Hold,
            action: ShortcutAction::Record {
                polish_template_id: None,
            },
        }
    }

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
    fn runtime_probe_action_keeps_mounted_runner_on_probe_failure() {
        assert_eq!(
            runtime_probe_action(true, false),
            RuntimeProbeAction::Keep
        );
    }

    #[test]
    fn runtime_restart_request_targets_the_signaled_runner() {
        assert_eq!(
            runtime_restart_request(&RuntimeEvent::RunnerNeedsRestart {
                mode: RunnerMode::Main,
                generation: 7,
            }),
            Some((RunnerMode::Main, 7))
        );
        assert_eq!(
            runtime_restart_request(&RuntimeEvent::RunnerNeedsRestart {
                mode: RunnerMode::CaptureOnly,
                generation: 9,
            }),
            Some((RunnerMode::CaptureOnly, 9))
        );
    }

    #[test]
    fn capture_runner_reconcile_requires_active_capture_and_no_live_runner() {
        assert!(capture_runner_reconcile_needed(true, false, false));
        assert!(!capture_runner_reconcile_needed(false, false, false));
        assert!(!capture_runner_reconcile_needed(true, true, false));
        assert!(!capture_runner_reconcile_needed(true, false, true));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn main_runner_mount_waits_for_capture_runtime_to_release_ownership() {
        assert!(main_runner_mount_allowed(false, false, false));
        assert!(!main_runner_mount_allowed(true, false, false));
        assert!(!main_runner_mount_allowed(false, true, false));
        assert!(!main_runner_mount_allowed(false, false, true));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn runtime_restart_is_rate_limited() {
        assert!(runtime_restart_allowed(
            Duration::from_millis(500),
            Duration::from_millis(500)
        ));
        assert!(!runtime_restart_allowed(
            Duration::from_millis(499),
            Duration::from_millis(500)
        ));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn throttled_restart_still_requires_runner_teardown() {
        assert_eq!(
            runtime_restart_action(Duration::from_millis(499), Duration::from_millis(500)),
            RuntimeRestartAction::TeardownOnly
        );
        assert_eq!(
            runtime_restart_action(Duration::from_millis(500), Duration::from_millis(500)),
            RuntimeRestartAction::TeardownAndRemount
        );
    }

    #[test]
    fn restart_request_must_match_current_runner_generation() {
        assert!(restart_request_matches_current_generation(Some(7), 7));
        assert!(!restart_request_matches_current_generation(Some(8), 7));
        assert!(!restart_request_matches_current_generation(None, 7));
    }

    #[test]
    fn main_restart_hands_off_to_capture_runner_during_active_capture() {
        assert_eq!(
            replacement_runner_mode_after_restart(RunnerMode::Main, true),
            RunnerMode::CaptureOnly
        );
        assert_eq!(
            replacement_runner_mode_after_restart(RunnerMode::Main, false),
            RunnerMode::Main
        );
        assert_eq!(
            replacement_runner_mode_after_restart(RunnerMode::CaptureOnly, true),
            RunnerMode::CaptureOnly
        );
    }

    #[test]
    fn runtime_is_live_includes_capture_only_runner() {
        assert!(runtime_is_live(true, false));
        assert!(runtime_is_live(false, true));
        assert!(!runtime_is_live(false, false));
    }

    #[test]
    fn manager_new_has_empty_event_receiver() {
        let manager = ShortcutManager::new().unwrap();
        assert!(manager.event_receiver().try_recv().is_err());
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
        manager
            .facade_state
            .capture_active
            .store(true, Ordering::SeqCst);
        *manager.facade_state.last_captured_hotkey.lock() = Some("Cmd+Slash".to_string());

        manager.cancel_recording_capture();

        assert!(!manager.is_recording_capture_active());
        assert_eq!(manager.peek_recording_capture(), None);
    }

    #[test]
    fn test_manager_stop_without_start() {
        let mut manager = ShortcutManager::new().unwrap();
        assert!(manager.stop().is_ok());
    }

    #[test]
    fn test_cancel_hotkeys_hold_mode_includes_active_modifiers() {
        assert_eq!(
            build_cancel_hotkeys(ShortcutTriggerMode::Hold, "Cmd+Shift+Space"),
            vec!["Escape".to_string(), "Cmd+Shift+Escape".to_string()]
        );
    }

    #[test]
    fn test_cancel_hotkeys_toggle_mode_uses_escape_only() {
        assert_eq!(
            build_cancel_hotkeys(ShortcutTriggerMode::Toggle, "Cmd+Shift+Space"),
            vec!["Escape".to_string()]
        );
    }

    #[test]
    fn register_profile_before_start_is_deferred_but_succeeds() {
        let manager = ShortcutManager::new().unwrap();
        let profile = profile("Cmd+Slash");

        assert!(manager.register_profile("dictate", &profile).is_ok());
    }

    #[test]
    fn unregister_profile_is_deferred_when_runtime_is_unavailable() {
        let mut state = ManagerState::new();

        state
            .register_profile("dictate", profile("Cmd+Slash"), None)
            .unwrap();

        let result = state.unregister_profile("dictate", None).unwrap();

        assert_eq!(result, MutationMode::Deferred);
        let snapshot = state.snapshot();
        assert!(!snapshot.desired_profiles.contains_key("dictate"));
        assert!(!snapshot.live_profiles.contains_key("dictate"));
    }

    #[test]
    fn register_profile_updates_desired_and_live_when_runtime_is_healthy() {
        let mut state = ManagerState::new();
        let mut runtime = FakeRuntimeApplier::successful();

        let result = state
            .register_profile("dictate", profile("Cmd+Slash"), Some(&mut runtime))
            .unwrap();

        assert_eq!(result, MutationMode::Immediate);
        let snapshot = state.snapshot();
        assert_eq!(
            snapshot.desired_profiles.get("dictate"),
            Some(&"Cmd+Slash".to_string())
        );
        assert_eq!(
            snapshot.live_profiles.get("dictate"),
            Some(&"Cmd+Slash".to_string())
        );
        assert_eq!(runtime.applied.len(), 1);
    }

    #[test]
    fn register_profile_keeps_previous_authoritative_value_when_runtime_apply_fails() {
        let mut state = ManagerState::new();
        let mut healthy_runtime = FakeRuntimeApplier::successful();
        state
            .register_profile("dictate", profile("Cmd+Slash"), Some(&mut healthy_runtime))
            .unwrap();

        let mut failing_runtime = FakeRuntimeApplier::failing();
        let error = state
            .register_profile("dictate", profile("Opt+Slash"), Some(&mut failing_runtime))
            .unwrap_err();

        assert_eq!(error, "runtime_apply_failed");
        let snapshot = state.snapshot();
        assert_eq!(
            snapshot.desired_profiles.get("dictate"),
            Some(&"Cmd+Slash".to_string())
        );
        assert_eq!(
            snapshot.live_profiles.get("dictate"),
            Some(&"Cmd+Slash".to_string())
        );
    }

    #[test]
    fn register_profile_evicts_existing_duplicate_hotkey() {
        let mut state = ManagerState::new();
        let mut runtime = FakeRuntimeApplier::successful();

        state
            .register_profile("dictate", profile("Cmd+Slash"), Some(&mut runtime))
            .unwrap();
        state
            .register_profile("chat", profile("Cmd+Slash"), Some(&mut runtime))
            .unwrap();

        let snapshot = state.snapshot();
        assert_eq!(
            snapshot.live_profiles.get("chat"),
            Some(&"Cmd+Slash".to_string())
        );
        assert!(!snapshot.live_profiles.contains_key("dictate"));
    }

    #[test]
    fn empty_hotkey_removes_live_binding_when_runtime_is_healthy() {
        let mut state = ManagerState::new();
        let mut runtime = FakeRuntimeApplier::successful();

        state
            .register_profile("dictate", profile("Cmd+Slash"), Some(&mut runtime))
            .unwrap();
        state
            .register_profile("dictate", profile(""), Some(&mut runtime))
            .unwrap();

        let snapshot = state.snapshot();
        assert_eq!(
            snapshot.desired_profiles.get("dictate"),
            Some(&"".to_string())
        );
        assert!(!snapshot.live_profiles.contains_key("dictate"));
    }
}
