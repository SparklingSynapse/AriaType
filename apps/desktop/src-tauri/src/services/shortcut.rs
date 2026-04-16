use crate::shortcut::ShortcutState;
use crate::state::app_state::AppState;
use std::sync::atomic::Ordering;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShortcutRecordingMode {
    Hold,
    Toggle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrimaryShortcutContext {
    pub capture_active: bool,
    pub is_recording: bool,
    pub recording_mode: ShortcutRecordingMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimaryShortcutAction {
    Ignore,
    StartRecording,
    StopRecording,
}

pub fn primary_shortcut_context(state: &AppState, capture_active: bool) -> PrimaryShortcutContext {
    let is_recording = state.is_recording.load(Ordering::SeqCst);
    let recording_mode = ShortcutRecordingMode::from_setting(&state.settings.lock().recording_mode);

    PrimaryShortcutContext {
        capture_active,
        is_recording,
        recording_mode,
    }
}

pub fn primary_shortcut_action(
    context: PrimaryShortcutContext,
    shortcut_state: ShortcutState,
) -> PrimaryShortcutAction {
    if context.capture_active {
        return PrimaryShortcutAction::Ignore;
    }

    match context.recording_mode {
        ShortcutRecordingMode::Hold => match (shortcut_state, context.is_recording) {
            (ShortcutState::Pressed, false) => PrimaryShortcutAction::StartRecording,
            (ShortcutState::Released, true) => PrimaryShortcutAction::StopRecording,
            _ => PrimaryShortcutAction::Ignore,
        },
        ShortcutRecordingMode::Toggle => {
            if shortcut_state != ShortcutState::Pressed {
                return PrimaryShortcutAction::Ignore;
            }

            if context.is_recording {
                PrimaryShortcutAction::StopRecording
            } else {
                PrimaryShortcutAction::StartRecording
            }
        }
    }
}

pub fn capture_cancel_hotkey_release_owner(
    is_recording: bool,
    is_transcribing: bool,
    task_id: u64,
) -> Option<u64> {
    if is_recording || is_transcribing {
        Some(task_id)
    } else {
        None
    }
}

pub fn should_unregister_cancel_hotkeys(
    current_owner_task_id: Option<u64>,
    requested_owner_task_id: Option<u64>,
) -> bool {
    requested_owner_task_id.is_none() || current_owner_task_id == requested_owner_task_id
}

pub fn cancel_hotkey_release_unregister_owner(
    is_recording: bool,
    is_transcribing: bool,
    pending_owner_task_id: Option<u64>,
) -> Option<u64> {
    if is_recording || is_transcribing {
        None
    } else {
        pending_owner_task_id
    }
}

impl ShortcutRecordingMode {
    fn from_setting(recording_mode: &str) -> Self {
        if recording_mode.eq_ignore_ascii_case("hold") {
            ShortcutRecordingMode::Hold
        } else {
            ShortcutRecordingMode::Toggle
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        cancel_hotkey_release_unregister_owner, capture_cancel_hotkey_release_owner,
        primary_shortcut_action, should_unregister_cancel_hotkeys, PrimaryShortcutAction,
        PrimaryShortcutContext, ShortcutRecordingMode,
    };
    use crate::shortcut::ShortcutState;

    #[test]
    fn primary_shortcut_ignores_trigger_during_capture() {
        let context = PrimaryShortcutContext {
            capture_active: true,
            is_recording: false,
            recording_mode: ShortcutRecordingMode::Toggle,
        };

        assert_eq!(
            primary_shortcut_action(context, ShortcutState::Pressed),
            PrimaryShortcutAction::Ignore
        );
    }

    #[test]
    fn primary_shortcut_hold_mode_starts_on_press_and_stops_on_release() {
        let idle_context = PrimaryShortcutContext {
            capture_active: false,
            is_recording: false,
            recording_mode: ShortcutRecordingMode::Hold,
        };
        let recording_context = PrimaryShortcutContext {
            capture_active: false,
            is_recording: true,
            recording_mode: ShortcutRecordingMode::Hold,
        };

        assert_eq!(
            primary_shortcut_action(idle_context, ShortcutState::Pressed),
            PrimaryShortcutAction::StartRecording
        );
        assert_eq!(
            primary_shortcut_action(recording_context, ShortcutState::Released),
            PrimaryShortcutAction::StopRecording
        );
        assert_eq!(
            primary_shortcut_action(idle_context, ShortcutState::Released),
            PrimaryShortcutAction::Ignore
        );
    }

    #[test]
    fn primary_shortcut_toggle_mode_toggles_only_on_press() {
        let idle_context = PrimaryShortcutContext {
            capture_active: false,
            is_recording: false,
            recording_mode: ShortcutRecordingMode::Toggle,
        };
        let recording_context = PrimaryShortcutContext {
            capture_active: false,
            is_recording: true,
            recording_mode: ShortcutRecordingMode::Toggle,
        };

        assert_eq!(
            primary_shortcut_action(idle_context, ShortcutState::Pressed),
            PrimaryShortcutAction::StartRecording
        );
        assert_eq!(
            primary_shortcut_action(recording_context, ShortcutState::Pressed),
            PrimaryShortcutAction::StopRecording
        );
        assert_eq!(
            primary_shortcut_action(recording_context, ShortcutState::Released),
            PrimaryShortcutAction::Ignore
        );
    }

    #[test]
    fn cancel_owner_is_captured_only_for_active_session() {
        assert_eq!(capture_cancel_hotkey_release_owner(true, false, 8), Some(8));
        assert_eq!(capture_cancel_hotkey_release_owner(false, true, 8), Some(8));
        assert_eq!(capture_cancel_hotkey_release_owner(false, false, 8), None);
    }

    #[test]
    fn cancel_owner_is_released_only_when_session_is_idle() {
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
        assert_eq!(
            cancel_hotkey_release_unregister_owner(false, false, None),
            None
        );
    }

    #[test]
    fn cancel_unregister_ignores_stale_owner() {
        assert!(!should_unregister_cancel_hotkeys(Some(2), Some(1)));
        assert!(should_unregister_cancel_hotkeys(Some(2), Some(2)));
        assert!(should_unregister_cancel_hotkeys(Some(2), None));
    }
}
