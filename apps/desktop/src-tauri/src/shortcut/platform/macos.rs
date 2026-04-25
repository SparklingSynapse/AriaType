#[cfg(target_os = "macos")]
use std::ffi::c_void;
#[cfg(target_os = "macos")]
use std::ptr::NonNull;
#[cfg(target_os = "macos")]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(target_os = "macos")]
use std::sync::mpsc::Sender;
#[cfg(target_os = "macos")]
use std::sync::Arc;
#[cfg(target_os = "macos")]
use std::thread::{self, JoinHandle};
#[cfg(target_os = "macos")]
use std::time::{Duration, Instant};

#[cfg(target_os = "macos")]
use parking_lot::Mutex;

#[cfg(target_os = "macos")]
use crate::permissions::{check_permission, PermissionKind, PermissionStatus};
#[cfg(target_os = "macos")]
use crate::shortcut::hotkey_codec::key_token_from_rdev_key;
#[cfg(target_os = "macos")]
use crate::shortcut::matcher::{handle_input, MatcherInput, MatcherState, ModifierKey};

#[cfg(target_os = "macos")]
use super::{PlatformRunner, RunnerMode, RuntimeEvent, SharedMatcherSnapshot};

#[cfg(target_os = "macos")]
const EVENT_TYPE_KEY_DOWN: u32 = 10;
#[cfg(target_os = "macos")]
const EVENT_TYPE_KEY_UP: u32 = 11;
#[cfg(target_os = "macos")]
const EVENT_TYPE_FLAGS_CHANGED: u32 = 12;
#[cfg(target_os = "macos")]
const EVENT_TYPE_TAP_DISABLED_BY_TIMEOUT: u32 = 15;
#[cfg(target_os = "macos")]
const EVENT_TYPE_TAP_DISABLED_BY_USER_INPUT: u32 = 16;
#[cfg(target_os = "macos")]
const KEYCODE_FN: u16 = 0x3F;
#[cfg(target_os = "macos")]
const KEYCODE_FN_HIDDEN_TRIGGER: u16 = 179;
#[cfg(target_os = "macos")]
const NX_SYSDEFINED: u32 = 14;
#[cfg(target_os = "macos")]
const FN_RELEASE_BLOCK_WINDOW: Duration = Duration::from_millis(200);

#[cfg(target_os = "macos")]
#[derive(Debug, Default)]
struct FnBlockState {
    // Use a monotonic clock so the FN block window is not distorted by wall-clock jumps.
    release_deadline: Option<Instant>,
}

#[cfg(target_os = "macos")]
impl FnBlockState {
    fn record_fn_flags_change(&mut self, keycode: u16, has_fn: bool, now: Instant) {
        if keycode != KEYCODE_FN {
            return;
        }

        if has_fn {
            self.release_deadline = None;
            tracing::debug!("fn_key_pressed_tracking_started");
        } else {
            self.release_deadline = Some(now + FN_RELEASE_BLOCK_WINDOW);
            tracing::debug!("fn_key_released_block_window_started");
        }
    }

    fn should_block_event(&self, event_type: u32, keycode: u16, now: Instant) -> bool {
        let Some(release_deadline) = self.release_deadline else {
            return false;
        };

        if now >= release_deadline {
            return false;
        }

        event_type == NX_SYSDEFINED
            || matches!(event_type, EVENT_TYPE_KEY_DOWN | EVENT_TYPE_KEY_UP)
                && keycode == KEYCODE_FN_HIDDEN_TRIGGER
    }
}

#[cfg(target_os = "macos")]
fn should_passthrough_swallowed_event(event_type: u32, keycode: u16, has_fn: bool) -> bool {
    let _ = event_type;
    let _ = keycode;
    let _ = has_fn;
    false
}

#[cfg(target_os = "macos")]
fn should_swallow_event(
    outcome_swallow: bool,
    mode: RunnerMode,
    event_type: u32,
    keycode: u16,
    has_fn: bool,
) -> bool {
    outcome_swallow
        && mode == RunnerMode::Main
        && !should_passthrough_swallowed_event(event_type, keycode, has_fn)
}

#[cfg(target_os = "macos")]
// Ignore our own synthesized keyboard traffic so text injection does not poison
// the global shortcut matcher state between real user key presses.
fn should_ignore_self_generated_event(event_type: u32, source_pid: i64) -> bool {
    source_pid == i64::from(std::process::id())
        && matches!(event_type, EVENT_TYPE_KEY_DOWN | EVENT_TYPE_KEY_UP | EVENT_TYPE_FLAGS_CHANGED)
}

#[cfg(target_os = "macos")]
fn should_synthesize_fn_release_on_blocked_followup(
    fn_block_state: &FnBlockState,
    matcher_state: &MatcherState,
    event_type: u32,
    keycode: u16,
    now: Instant,
) -> bool {
    fn_block_state.should_block_event(event_type, keycode, now) && matcher_state.modifiers.function
}

#[cfg(target_os = "macos")]
fn blocked_fn_followup_matcher_outcome(
    fn_block_state: &FnBlockState,
    matcher_state: &mut MatcherState,
    snapshot: &crate::shortcut::matcher::MatcherSnapshot,
    event_type: u32,
    keycode: u16,
    now: Instant,
) -> Option<Vec<crate::shortcut::matcher::MatcherEvent>> {
    if !should_synthesize_fn_release_on_blocked_followup(
        fn_block_state,
        matcher_state,
        event_type,
        keycode,
        now,
    ) {
        return None;
    }

    let outcome = handle_input(
        matcher_state,
        snapshot,
        MatcherInput::ModifierReleased(ModifierKey::Function),
    );
    Some(outcome.events)
}

#[cfg(target_os = "macos")]
pub struct MacosRunner {
    thread_handle: Option<JoinHandle<()>>,
    running: Arc<AtomicBool>,
    active: Arc<AtomicBool>,
}

#[cfg(target_os = "macos")]
struct CallbackState {
    fn_block_state: Mutex<FnBlockState>,
    snapshot: SharedMatcherSnapshot,
    matcher_state: Mutex<MatcherState>,
    event_tx: Sender<RuntimeEvent>,
    mode: RunnerMode,
    generation: u64,
}

#[cfg(target_os = "macos")]
pub fn start_runner(
    mode: RunnerMode,
    snapshot: SharedMatcherSnapshot,
    event_tx: Sender<RuntimeEvent>,
    generation: u64,
) -> Result<MacosRunner, String> {
    if check_permission(PermissionKind::Accessibility) != PermissionStatus::Granted {
        return Err("Accessibility permission not granted".to_string());
    }

    let running = Arc::new(AtomicBool::new(true));
    let active = Arc::new(AtomicBool::new(true));
    let thread_running = Arc::clone(&running);
    let thread_active = Arc::clone(&active);

    let handle = thread::spawn(move || {
        run_macos_runner(
            mode,
            snapshot,
            event_tx,
            thread_running,
            thread_active,
            generation,
        );
    });

    Ok(MacosRunner {
        thread_handle: Some(handle),
        running,
        active,
    })
}

#[cfg(target_os = "macos")]
impl PlatformRunner for MacosRunner {
    fn stop(&mut self) -> Result<(), String> {
        self.running.store(false, Ordering::SeqCst);
        self.active.store(false, Ordering::SeqCst);

        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }

        Ok(())
    }
}

#[cfg(target_os = "macos")]
fn run_macos_runner(
    mode: RunnerMode,
    snapshot: SharedMatcherSnapshot,
    event_tx: Sender<RuntimeEvent>,
    running: Arc<AtomicBool>,
    active: Arc<AtomicBool>,
    generation: u64,
) {
    use objc2_core_foundation::{CFMachPort, CFRetained, CFRunLoop, CFRunLoopSource};
    use objc2_core_graphics::{
        CGEvent, CGEventMask, CGEventTapCallBack, CGEventTapLocation, CGEventTapOptions,
        CGEventTapPlacement, CGEventType,
    };

    let callback_state = Arc::new(CallbackState {
        fn_block_state: Mutex::new(FnBlockState::default()),
        snapshot,
        matcher_state: Mutex::new(MatcherState::default()),
        event_tx: event_tx.clone(),
        mode,
        generation,
    });
    let state_ptr = Arc::into_raw(Arc::clone(&callback_state)) as *mut c_void;
    let callback: CGEventTapCallBack = Some(macos_runner_callback);

    let event_mask: CGEventMask = (1 << CGEventType::KeyDown.0)
        | (1 << CGEventType::KeyUp.0)
        | (1 << CGEventType::FlagsChanged.0)
        | (1 << NX_SYSDEFINED);

    let tap: Option<CFRetained<CFMachPort>> = unsafe {
        CGEvent::tap_create(
            CGEventTapLocation::SessionEventTap,
            CGEventTapPlacement::HeadInsertEventTap,
            CGEventTapOptions::Default,
            event_mask,
            callback,
            state_ptr,
        )
    };

    let Some(tap) = tap else {
        unsafe {
            let _ = Arc::from_raw(state_ptr as *const CallbackState);
        }
        finish_runner_exit(&event_tx, &running, &active, mode, generation);
        return;
    };

    let Some(source): Option<CFRetained<CFRunLoopSource>> =
        CFMachPort::new_run_loop_source(None, Some(&tap), 0)
    else {
        unsafe {
            CFMachPort::invalidate(&tap);
            let _ = Arc::from_raw(state_ptr as *const CallbackState);
        }
        finish_runner_exit(&event_tx, &running, &active, mode, generation);
        return;
    };

    let Some(run_loop) = CFRunLoop::current() else {
        unsafe {
            CFMachPort::invalidate(&tap);
            let _ = Arc::from_raw(state_ptr as *const CallbackState);
        }
        finish_runner_exit(&event_tx, &running, &active, mode, generation);
        return;
    };

    run_loop.add_source(Some(&source), unsafe {
        objc2_core_foundation::kCFRunLoopCommonModes
    });
    CGEvent::tap_enable(&tap, true);

    while running.load(Ordering::SeqCst) {
        CFRunLoop::run_in_mode(
            unsafe { objc2_core_foundation::kCFRunLoopDefaultMode },
            0.1,
            true,
        );
    }

    run_loop.remove_source(Some(&source), unsafe {
        objc2_core_foundation::kCFRunLoopCommonModes
    });
    CGEvent::tap_enable(&tap, false);
    CFMachPort::invalidate(&tap);
    unsafe {
        let _ = Arc::from_raw(state_ptr as *const CallbackState);
    }
    finish_runner_exit(&event_tx, &running, &active, mode, generation);
}

#[cfg(target_os = "macos")]
fn runtime_event_for_runner_exit(
    unexpected_exit: bool,
    mode: RunnerMode,
    generation: u64,
) -> Option<RuntimeEvent> {
    if unexpected_exit {
        Some(RuntimeEvent::RunnerNeedsRestart { mode, generation })
    } else {
        None
    }
}

#[cfg(target_os = "macos")]
fn finish_runner_exit(
    event_tx: &Sender<RuntimeEvent>,
    running: &Arc<AtomicBool>,
    active: &Arc<AtomicBool>,
    mode: RunnerMode,
    generation: u64,
) {
    let unexpected_exit = running.load(Ordering::SeqCst);
    if let Some(runtime_event) = runtime_event_for_runner_exit(unexpected_exit, mode, generation) {
        let _ = event_tx.send(runtime_event);
    }

    running.store(false, Ordering::SeqCst);
    active.store(false, Ordering::SeqCst);
}

#[cfg(target_os = "macos")]
unsafe extern "C-unwind" fn macos_runner_callback(
    _proxy: objc2_core_graphics::CGEventTapProxy,
    event_type: objc2_core_graphics::CGEventType,
    event: NonNull<objc2_core_graphics::CGEvent>,
    user_info: *mut c_void,
) -> *mut objc2_core_graphics::CGEvent {
    use objc2_core_graphics::{CGEvent, CGEventField, CGEventFlags};
    let state = &*(user_info as *const CallbackState);

    if matches!(
        event_type.0,
        EVENT_TYPE_TAP_DISABLED_BY_TIMEOUT | EVENT_TYPE_TAP_DISABLED_BY_USER_INPUT
    ) {
        let _ = state.event_tx.send(RuntimeEvent::RunnerNeedsRestart {
            mode: state.mode,
            generation: state.generation,
        });
        return event.as_ptr();
    }

    let cg_event = event.as_ref();
    let now = Instant::now();
    let source_pid =
        CGEvent::integer_value_field(Some(cg_event), CGEventField::EventSourceUnixProcessID);

    if should_ignore_self_generated_event(event_type.0, source_pid) {
        tracing::info!(
            mode = ?state.mode,
            generation = state.generation,
            event_type = event_type.0,
            source_pid,
            "shortcut_ignoring_self_generated_event"
        );
        return event.as_ptr();
    }

    if event_type.0 == NX_SYSDEFINED {
        if state
            .fn_block_state
            .lock()
            .should_block_event(event_type.0, 0, now)
        {
            tracing::debug!("blocking_nx_sysdefined_after_fn_release");
            return std::ptr::null_mut();
        }
        return event.as_ptr();
    }

    let keycode =
        CGEvent::integer_value_field(Some(cg_event), CGEventField::KeyboardEventKeycode) as u16;
    let flags = CGEvent::flags(Some(cg_event));
    let has_fn = flags.contains(CGEventFlags::MaskSecondaryFn);

    if event_type.0 == EVENT_TYPE_FLAGS_CHANGED && keycode == KEYCODE_FN {
        let matcher_state = state.matcher_state.lock();
        tracing::info!(
            mode = ?state.mode,
            generation = state.generation,
            event_type = event_type.0,
            keycode,
            source_pid,
            has_fn,
            function_pressed = matcher_state.modifiers.function,
            active_profile = ?matcher_state.active_profile,
            "fn_flags_changed_observed"
        );
        drop(matcher_state);
        state
            .fn_block_state
            .lock()
            .record_fn_flags_change(keycode, has_fn, now);
    }

    if state
        .fn_block_state
        .lock()
        .should_block_event(event_type.0, keycode, now)
    {
        let snapshot = state.snapshot.read().clone();
        let mut matcher_state = state.matcher_state.lock();
        let function_pressed_before = matcher_state.modifiers.function;
        let active_profile_before = matcher_state.active_profile.clone();
        if let Some(events) = blocked_fn_followup_matcher_outcome(
            &state.fn_block_state.lock(),
            &mut matcher_state,
            &snapshot,
            event_type.0,
            keycode,
            now,
        ) {
            tracing::info!(
                mode = ?state.mode,
                generation = state.generation,
                event_type = event_type.0,
                keycode,
                source_pid,
                function_pressed_before,
                active_profile_before = ?active_profile_before,
                synthesized_events = ?events,
                "fn_blocked_followup_synthesized_release"
            );
            drop(matcher_state);
            for matcher_event in events {
                let _ = state.event_tx.send(RuntimeEvent::Matcher(matcher_event));
            }
        } else {
            tracing::info!(
                mode = ?state.mode,
                generation = state.generation,
                event_type = event_type.0,
                keycode,
                source_pid,
                function_pressed_before,
                active_profile_before = ?active_profile_before,
                "fn_blocked_followup_without_synthesized_release"
            );
        }
        tracing::debug!(
            event_type = event_type.0,
            "blocking_hidden_keycode_179_after_fn_release"
        );
        return std::ptr::null_mut();
    }

    let Some(input) = matcher_input_from_event(event_type.0, keycode, flags, &state.matcher_state)
    else {
        if event_type.0 == EVENT_TYPE_FLAGS_CHANGED && keycode == KEYCODE_FN {
            let matcher_state = state.matcher_state.lock();
            tracing::info!(
                mode = ?state.mode,
                generation = state.generation,
                event_type = event_type.0,
                keycode,
                source_pid,
                has_fn,
                function_pressed = matcher_state.modifiers.function,
                active_profile = ?matcher_state.active_profile,
                "fn_matcher_input_ignored"
            );
        }
        return event.as_ptr();
    };

    let snapshot = state.snapshot.read().clone();
    let snapshot_profile_ids = snapshot.profiles.keys().cloned().collect::<Vec<_>>();
    let snapshot_profile_count = snapshot_profile_ids.len();
    let snapshot_profiles_for_log = snapshot.profiles.clone();
    let mut matcher_state = state.matcher_state.lock();
    let active_profile_before = matcher_state.active_profile.clone();
    let function_pressed_before = matcher_state.modifiers.function;
    let modifiers_before = matcher_state.modifiers.clone();
    let input_for_log = input.clone();
    let outcome = handle_input(&mut matcher_state, &snapshot, input);
    let active_profile_after = matcher_state.active_profile.clone();
    let function_pressed_after = matcher_state.modifiers.function;
    let modifiers_after = matcher_state.modifiers.clone();
    drop(matcher_state);

    if event_type.0 == EVENT_TYPE_FLAGS_CHANGED && keycode == KEYCODE_FN {
        tracing::info!(
            mode = ?state.mode,
            generation = state.generation,
            event_type = event_type.0,
            keycode,
            source_pid,
            has_fn,
            matcher_input = ?input_for_log,
            snapshot_profile_count,
            snapshot_profile_ids = ?snapshot_profile_ids,
            snapshot_profiles = ?snapshot_profiles_for_log,
            modifiers_before = ?modifiers_before,
            modifiers_after = ?modifiers_after,
            function_pressed_before,
            function_pressed_after,
            active_profile_before = ?active_profile_before,
            active_profile_after = ?active_profile_after,
            outcome_swallow = outcome.swallow,
            outcome_events = ?outcome.events,
            "fn_matcher_input_processed"
        );
    }

    for matcher_event in outcome.events {
        let _ = state.event_tx.send(RuntimeEvent::Matcher(matcher_event));
    }

    if should_swallow_event(outcome.swallow, state.mode, event_type.0, keycode, has_fn) {
        std::ptr::null_mut()
    } else {
        event.as_ptr()
    }
}

#[cfg(target_os = "macos")]
fn matcher_input_from_event(
    event_type: u32,
    keycode: u16,
    flags: objc2_core_graphics::CGEventFlags,
    matcher_state: &Mutex<MatcherState>,
) -> Option<MatcherInput> {
    match event_type {
        EVENT_TYPE_KEY_DOWN => key_token_from_macos_keycode(keycode).map(MatcherInput::KeyPressed),
        EVENT_TYPE_KEY_UP => key_token_from_macos_keycode(keycode).map(MatcherInput::KeyReleased),
        EVENT_TYPE_FLAGS_CHANGED => {
            let modifier = modifier_from_keycode(keycode)?;
            let was_pressed = modifier_is_pressed(&matcher_state.lock(), modifier);
            let is_pressed = modifier_flag_is_pressed(flags, modifier);
            if is_pressed == was_pressed {
                None
            } else if is_pressed {
                Some(MatcherInput::ModifierPressed(modifier))
            } else {
                Some(MatcherInput::ModifierReleased(modifier))
            }
        }
        _ => None,
    }
}

#[cfg(target_os = "macos")]
fn modifier_is_pressed(state: &MatcherState, modifier: ModifierKey) -> bool {
    match modifier {
        ModifierKey::CtrlLeft => state.clone().modifiers.ctrl_left,
        ModifierKey::CtrlRight => state.clone().modifiers.ctrl_right,
        ModifierKey::OptLeft => state.clone().modifiers.opt_left,
        ModifierKey::OptRight => state.clone().modifiers.opt_right,
        ModifierKey::ShiftLeft => state.clone().modifiers.shift_left,
        ModifierKey::ShiftRight => state.clone().modifiers.shift_right,
        ModifierKey::CmdLeft => state.clone().modifiers.cmd_left,
        ModifierKey::CmdRight => state.clone().modifiers.cmd_right,
        ModifierKey::Function => state.clone().modifiers.function,
    }
}

#[cfg(target_os = "macos")]
fn modifier_flag_is_pressed(
    flags: objc2_core_graphics::CGEventFlags,
    modifier: ModifierKey,
) -> bool {
    use objc2_core_graphics::CGEventFlags;

    match modifier {
        ModifierKey::CtrlLeft | ModifierKey::CtrlRight => flags.contains(CGEventFlags::MaskControl),
        ModifierKey::OptLeft | ModifierKey::OptRight => flags.contains(CGEventFlags::MaskAlternate),
        ModifierKey::ShiftLeft | ModifierKey::ShiftRight => flags.contains(CGEventFlags::MaskShift),
        ModifierKey::CmdLeft | ModifierKey::CmdRight => flags.contains(CGEventFlags::MaskCommand),
        ModifierKey::Function => flags.contains(CGEventFlags::MaskSecondaryFn),
    }
}

#[cfg(target_os = "macos")]
fn modifier_from_keycode(keycode: u16) -> Option<ModifierKey> {
    match keycode {
        59 => Some(ModifierKey::CtrlLeft),
        62 => Some(ModifierKey::CtrlRight),
        58 => Some(ModifierKey::OptLeft),
        61 => Some(ModifierKey::OptRight),
        56 => Some(ModifierKey::ShiftLeft),
        60 => Some(ModifierKey::ShiftRight),
        55 => Some(ModifierKey::CmdLeft),
        54 => Some(ModifierKey::CmdRight),
        63 => Some(ModifierKey::Function),
        _ => None,
    }
}

#[cfg(target_os = "macos")]
fn key_token_from_macos_keycode(keycode: u16) -> Option<String> {
    let key = match keycode {
        58 | 61 | 56 | 60 | 59 | 62 | 55 | 54 | 63 => return None,
        122 => rdev::Key::F1,
        120 => rdev::Key::F2,
        99 => rdev::Key::F3,
        118 => rdev::Key::F4,
        96 => rdev::Key::F5,
        97 => rdev::Key::F6,
        98 => rdev::Key::F7,
        100 => rdev::Key::F8,
        101 => rdev::Key::F9,
        109 => rdev::Key::F10,
        103 => rdev::Key::F11,
        111 => rdev::Key::F12,
        105 => return Some("F13".to_string()),
        107 => return Some("F14".to_string()),
        113 => return Some("F15".to_string()),
        106 => return Some("F16".to_string()),
        64 => return Some("F17".to_string()),
        79 => return Some("F18".to_string()),
        80 => return Some("F19".to_string()),
        90 => return Some("F20".to_string()),
        36 => rdev::Key::Return,
        48 => rdev::Key::Tab,
        49 => rdev::Key::Space,
        51 => rdev::Key::Backspace,
        53 => rdev::Key::Escape,
        117 => rdev::Key::Delete,
        123 => rdev::Key::LeftArrow,
        124 => rdev::Key::RightArrow,
        125 => rdev::Key::DownArrow,
        126 => rdev::Key::UpArrow,
        50 => rdev::Key::BackQuote,
        18 => rdev::Key::Num1,
        19 => rdev::Key::Num2,
        20 => rdev::Key::Num3,
        21 => rdev::Key::Num4,
        23 => rdev::Key::Num5,
        22 => rdev::Key::Num6,
        26 => rdev::Key::Num7,
        28 => rdev::Key::Num8,
        25 => rdev::Key::Num9,
        29 => rdev::Key::Num0,
        27 => rdev::Key::Minus,
        24 => rdev::Key::Equal,
        12 => rdev::Key::KeyQ,
        13 => rdev::Key::KeyW,
        14 => rdev::Key::KeyE,
        15 => rdev::Key::KeyR,
        17 => rdev::Key::KeyT,
        16 => rdev::Key::KeyY,
        32 => rdev::Key::KeyU,
        34 => rdev::Key::KeyI,
        31 => rdev::Key::KeyO,
        35 => rdev::Key::KeyP,
        33 => rdev::Key::LeftBracket,
        30 => rdev::Key::RightBracket,
        0 => rdev::Key::KeyA,
        1 => rdev::Key::KeyS,
        2 => rdev::Key::KeyD,
        3 => rdev::Key::KeyF,
        5 => rdev::Key::KeyG,
        4 => rdev::Key::KeyH,
        38 => rdev::Key::KeyJ,
        40 => rdev::Key::KeyK,
        37 => rdev::Key::KeyL,
        41 => rdev::Key::SemiColon,
        39 => rdev::Key::Quote,
        42 => rdev::Key::BackSlash,
        6 => rdev::Key::KeyZ,
        7 => rdev::Key::KeyX,
        8 => rdev::Key::KeyC,
        9 => rdev::Key::KeyV,
        11 => rdev::Key::KeyB,
        45 => rdev::Key::KeyN,
        46 => rdev::Key::KeyM,
        43 => rdev::Key::Comma,
        47 => rdev::Key::Dot,
        44 => rdev::Key::Slash,
        _ => return None,
    };

    key_token_from_rdev_key(key)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(target_os = "macos")]
    use objc2_core_graphics::CGEventFlags;
    #[cfg(target_os = "macos")]
    use std::collections::HashMap;

    #[test]
    fn unexpected_runner_exit_requests_restart_for_same_mode() {
        assert_eq!(
            runtime_event_for_runner_exit(true, RunnerMode::CaptureOnly, 11),
            Some(RuntimeEvent::RunnerNeedsRestart {
                mode: RunnerMode::CaptureOnly,
                generation: 11,
            })
        );
        assert_eq!(
            runtime_event_for_runner_exit(false, RunnerMode::Main, 7),
            None
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn unchanged_fn_flags_changed_does_not_emit_matcher_input() {
        let pressed_state = Mutex::new(MatcherState {
            modifiers: crate::shortcut::hotkey_codec::ModifierState {
                function: true,
                ..Default::default()
            },
            ..Default::default()
        });

        assert_eq!(
            matcher_input_from_event(
                EVENT_TYPE_FLAGS_CHANGED,
                KEYCODE_FN,
                CGEventFlags::MaskSecondaryFn,
                &pressed_state,
            ),
            None
        );

        let released_state = Mutex::new(MatcherState::default());
        assert_eq!(
            matcher_input_from_event(
                EVENT_TYPE_FLAGS_CHANGED,
                KEYCODE_FN,
                CGEventFlags::empty(),
                &released_state,
            ),
            None
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn fn_block_state_blocks_emoji_followups_after_release() {
        let mut state = FnBlockState::default();
        let released_at = Instant::now();

        state.record_fn_flags_change(KEYCODE_FN, false, released_at);

        assert!(state.should_block_event(NX_SYSDEFINED, 0, released_at + Duration::from_millis(50)));
        assert!(state.should_block_event(
            EVENT_TYPE_KEY_DOWN,
            KEYCODE_FN_HIDDEN_TRIGGER,
            released_at + Duration::from_millis(50),
        ));
        assert!(state.should_block_event(
            EVENT_TYPE_KEY_UP,
            KEYCODE_FN_HIDDEN_TRIGGER,
            released_at + Duration::from_millis(50),
        ));
        assert!(!state.should_block_event(
            NX_SYSDEFINED,
            0,
            released_at + Duration::from_millis(250),
        ));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn fn_block_state_clears_window_on_new_press() {
        let mut state = FnBlockState::default();
        let released_at = Instant::now();

        state.record_fn_flags_change(KEYCODE_FN, false, released_at);
        assert!(state.should_block_event(
            NX_SYSDEFINED,
            0,
            released_at + Duration::from_millis(50),
        ));

        state.record_fn_flags_change(
            KEYCODE_FN,
            true,
            released_at + Duration::from_millis(60),
        );
        assert!(!state.should_block_event(
            NX_SYSDEFINED,
            0,
            released_at + Duration::from_millis(70),
        ));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn swallowed_fn_flags_changed_is_not_passed_through() {
        assert!(!should_passthrough_swallowed_event(
            EVENT_TYPE_FLAGS_CHANGED,
            KEYCODE_FN,
            false,
        ));
        assert!(!should_passthrough_swallowed_event(
            EVENT_TYPE_FLAGS_CHANGED,
            KEYCODE_FN,
            true,
        ));
        assert!(!should_passthrough_swallowed_event(
            EVENT_TYPE_KEY_DOWN,
            KEYCODE_FN_HIDDEN_TRIGGER,
            false,
        ));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn swallow_decision_blocks_fn_flags_changed_while_hotkey_active() {
        assert!(should_swallow_event(
            true,
            RunnerMode::Main,
            EVENT_TYPE_FLAGS_CHANGED,
            KEYCODE_FN,
            false,
        ));
        assert!(should_swallow_event(
            true,
            RunnerMode::Main,
            EVENT_TYPE_FLAGS_CHANGED,
            KEYCODE_FN,
            true,
        ));
        assert!(should_swallow_event(
            true,
            RunnerMode::Main,
            EVENT_TYPE_KEY_DOWN,
            KEYCODE_FN_HIDDEN_TRIGGER,
            false,
        ));
        assert!(!should_swallow_event(
            false,
            RunnerMode::Main,
            EVENT_TYPE_KEY_DOWN,
            KEYCODE_FN_HIDDEN_TRIGGER,
            false,
        ));
        assert!(!should_swallow_event(
            true,
            RunnerMode::CaptureOnly,
            EVENT_TYPE_KEY_DOWN,
            KEYCODE_FN_HIDDEN_TRIGGER,
            false,
        ));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn blocked_fn_followup_can_synthesize_missing_release() {
        let mut fn_block_state = FnBlockState::default();
        let released_at = Instant::now();
        fn_block_state.record_fn_flags_change(KEYCODE_FN, false, released_at);

        let matcher_state = MatcherState {
            modifiers: crate::shortcut::hotkey_codec::ModifierState {
                function: true,
                ..Default::default()
            },
            active_profile: Some("dictate".to_string()),
            ..Default::default()
        };

        assert!(should_synthesize_fn_release_on_blocked_followup(
            &fn_block_state,
            &matcher_state,
            EVENT_TYPE_KEY_DOWN,
            KEYCODE_FN_HIDDEN_TRIGGER,
            released_at + Duration::from_millis(50),
        ));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn blocked_fn_followup_releases_active_fn_profile() {
        let mut profiles = HashMap::new();
        profiles.insert(
            "dictate".to_string(),
            crate::shortcut::hotkey_codec::parse_hotkey_pattern("Fn").unwrap(),
        );
        let snapshot = crate::shortcut::matcher::MatcherSnapshot {
            profiles,
            cancel: Vec::new(),
            capture_active: false,
        };
        let mut fn_block_state = FnBlockState::default();
        let released_at = Instant::now();
        fn_block_state.record_fn_flags_change(KEYCODE_FN, false, released_at);
        let mut matcher_state = MatcherState {
            modifiers: crate::shortcut::hotkey_codec::ModifierState {
                function: true,
                ..Default::default()
            },
            active_profile: Some("dictate".to_string()),
            ..Default::default()
        };

        let outcome = blocked_fn_followup_matcher_outcome(
            &fn_block_state,
            &mut matcher_state,
            &snapshot,
            EVENT_TYPE_KEY_DOWN,
            KEYCODE_FN_HIDDEN_TRIGGER,
            released_at + Duration::from_millis(50),
        );

        assert_eq!(matcher_state.active_profile, None);
        assert!(!matcher_state.modifiers.function);
        assert_eq!(
            outcome,
            Some(vec![crate::shortcut::matcher::MatcherEvent::ProfileReleased {
                profile_id: "dictate".to_string(),
            }])
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn self_generated_keyboard_events_are_ignored_by_shortcut_runtime() {
        let current_pid = std::process::id() as i64;

        assert!(should_ignore_self_generated_event(
            EVENT_TYPE_KEY_DOWN,
            current_pid,
        ));
        assert!(should_ignore_self_generated_event(
            EVENT_TYPE_KEY_UP,
            current_pid,
        ));
        assert!(should_ignore_self_generated_event(
            EVENT_TYPE_FLAGS_CHANGED,
            current_pid,
        ));
        assert!(!should_ignore_self_generated_event(NX_SYSDEFINED, current_pid));
        assert!(!should_ignore_self_generated_event(EVENT_TYPE_KEY_DOWN, 0));
    }
}

#[cfg(not(target_os = "macos"))]
pub struct MacosRunner;

#[cfg(not(target_os = "macos"))]
pub fn start_runner(
    _mode: RunnerMode,
    _snapshot: SharedMatcherSnapshot,
    _event_tx: Sender<RuntimeEvent>,
    _generation: u64,
) -> Result<MacosRunner, String> {
    Err("macOS runner unavailable on this platform".to_string())
}

#[cfg(not(target_os = "macos"))]
impl PlatformRunner for MacosRunner {
    fn stop(&mut self) -> Result<(), String> {
        Ok(())
    }
}
