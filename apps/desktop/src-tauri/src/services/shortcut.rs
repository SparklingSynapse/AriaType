use crate::shortcut::{
    ShortcutAction, ShortcutProfile, ShortcutProfilesMap, ShortcutState, ShortcutTriggerMode,
};
use crate::state::app_state::AppState;
use std::sync::atomic::Ordering;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShortcutRecordingMode {
    Hold,
    Toggle,
    DoubleTap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrimaryShortcutContext {
    pub capture_active: bool,
    pub is_recording: bool,
    pub trigger_mode: ShortcutRecordingMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimaryShortcutAction {
    Ignore,
    StartRecording,
    StopRecording,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoubleTapShortcutAction {
    Ignore,
    ArmFirstTap,
    StartRecording,
    StopRecording,
}

pub fn resolve_profile_template(profiles: &ShortcutProfilesMap, key: &str) -> Option<String> {
    let profile = match key {
        "dictate" => &profiles.dictate,
        "riff" => &profiles.riff,
        "custom" => profiles.custom.as_ref()?,
        _ => return None,
    };

    match &profile.action {
        ShortcutAction::Record { polish_template_id } => polish_template_id.clone(),
    }
}

pub fn get_profile_by_key<'a>(
    profiles: &'a ShortcutProfilesMap,
    key: &str,
) -> Option<&'a ShortcutProfile> {
    match key {
        "dictate" => Some(&profiles.dictate),
        "riff" => Some(&profiles.riff),
        "custom" => profiles.custom.as_ref(),
        _ => None,
    }
}

pub fn validate_hotkey_uniqueness(
    profiles: &ShortcutProfilesMap,
    hotkey: &str,
    exclude_key: Option<&str>,
) -> Result<(), String> {
    if hotkey.is_empty() {
        return Ok(());
    }

    let all_profiles: Vec<(&str, &ShortcutProfile)> =
        vec![("dictate", &profiles.dictate), ("riff", &profiles.riff)]
            .into_iter()
            .chain(profiles.custom.as_ref().map(|p| ("custom", p)))
            .collect();

    for (key, profile) in all_profiles {
        if exclude_key.is_none_or(|k| key != k) && profile.hotkey == hotkey {
            return Err(format!(
                "Hotkey '{}' is already used by profile '{}'",
                hotkey, key
            ));
        }
    }
    Ok(())
}

pub fn find_profile_key_by_hotkey<'a>(
    profiles: &'a ShortcutProfilesMap,
    hotkey: &str,
) -> Option<&'a str> {
    if hotkey.is_empty() {
        return None;
    }

    if profiles.dictate.hotkey == hotkey {
        return Some("dictate");
    }
    if profiles.riff.hotkey == hotkey {
        return Some("riff");
    }
    if profiles.custom.as_ref().is_some_and(|p| p.hotkey == hotkey) {
        return Some("custom");
    }
    None
}

pub fn primary_shortcut_context(
    state: &AppState,
    capture_active: bool,
    profile: Option<&ShortcutProfile>,
) -> PrimaryShortcutContext {
    let is_recording = state.is_recording.load(Ordering::SeqCst);
    let trigger_mode = ShortcutRecordingMode::from_profile(profile);

    PrimaryShortcutContext {
        capture_active,
        is_recording,
        trigger_mode,
    }
}

pub fn primary_shortcut_action(
    context: PrimaryShortcutContext,
    shortcut_state: ShortcutState,
) -> PrimaryShortcutAction {
    if context.capture_active {
        return PrimaryShortcutAction::Ignore;
    }

    match context.trigger_mode {
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
        ShortcutRecordingMode::DoubleTap => PrimaryShortcutAction::Ignore,
    }
}

pub fn double_tap_shortcut_action(
    context: PrimaryShortcutContext,
    shortcut_state: ShortcutState,
    current_profile_id: &str,
    pending_profile_id: Option<&str>,
    pending_elapsed: Option<Duration>,
    max_interval: Duration,
) -> DoubleTapShortcutAction {
    if context.capture_active
        || context.trigger_mode != ShortcutRecordingMode::DoubleTap
        || shortcut_state != ShortcutState::Pressed
    {
        return DoubleTapShortcutAction::Ignore;
    }

    let second_tap_matches = pending_profile_id == Some(current_profile_id)
        && pending_elapsed.is_some_and(|elapsed| elapsed <= max_interval);

    if !second_tap_matches {
        return DoubleTapShortcutAction::ArmFirstTap;
    }

    if context.is_recording {
        DoubleTapShortcutAction::StopRecording
    } else {
        DoubleTapShortcutAction::StartRecording
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
    fn from_profile(profile: Option<&ShortcutProfile>) -> Self {
        match profile.map(|item| item.trigger_mode) {
            Some(ShortcutTriggerMode::Hold) => ShortcutRecordingMode::Hold,
            Some(ShortcutTriggerMode::Toggle) => ShortcutRecordingMode::Toggle,
            Some(ShortcutTriggerMode::DoubleTap) => ShortcutRecordingMode::DoubleTap,
            None => ShortcutRecordingMode::Hold,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        cancel_hotkey_release_unregister_owner, capture_cancel_hotkey_release_owner,
        double_tap_shortcut_action, primary_shortcut_action, primary_shortcut_context,
        should_unregister_cancel_hotkeys, DoubleTapShortcutAction, PrimaryShortcutAction,
        PrimaryShortcutContext, ShortcutRecordingMode,
    };
    use crate::shortcut::{ShortcutProfile, ShortcutState};
    use crate::state::app_state::AppState;
    use std::time::Duration;

    #[test]
    fn primary_shortcut_ignores_trigger_during_capture() {
        let context = PrimaryShortcutContext {
            capture_active: true,
            is_recording: false,
            trigger_mode: ShortcutRecordingMode::Toggle,
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
            trigger_mode: ShortcutRecordingMode::Hold,
        };
        let recording_context = PrimaryShortcutContext {
            capture_active: false,
            is_recording: true,
            trigger_mode: ShortcutRecordingMode::Hold,
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
            trigger_mode: ShortcutRecordingMode::Toggle,
        };
        let recording_context = PrimaryShortcutContext {
            capture_active: false,
            is_recording: true,
            trigger_mode: ShortcutRecordingMode::Toggle,
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
    fn primary_shortcut_double_tap_mode_does_not_trigger_without_second_tap() {
        let context = PrimaryShortcutContext {
            capture_active: false,
            is_recording: false,
            trigger_mode: ShortcutRecordingMode::DoubleTap,
        };

        assert_eq!(
            primary_shortcut_action(context, ShortcutState::Pressed),
            PrimaryShortcutAction::Ignore
        );
        assert_eq!(
            double_tap_shortcut_action(
                context,
                ShortcutState::Pressed,
                "dictate",
                None,
                None,
                Duration::from_millis(500),
            ),
            DoubleTapShortcutAction::ArmFirstTap
        );
    }

    #[test]
    fn double_tap_shortcut_triggers_on_second_matching_press_within_window() {
        let idle_context = PrimaryShortcutContext {
            capture_active: false,
            is_recording: false,
            trigger_mode: ShortcutRecordingMode::DoubleTap,
        };
        let recording_context = PrimaryShortcutContext {
            is_recording: true,
            ..idle_context
        };

        assert_eq!(
            double_tap_shortcut_action(
                idle_context,
                ShortcutState::Pressed,
                "dictate",
                Some("dictate"),
                Some(Duration::from_millis(200)),
                Duration::from_millis(500),
            ),
            DoubleTapShortcutAction::StartRecording
        );
        assert_eq!(
            double_tap_shortcut_action(
                recording_context,
                ShortcutState::Pressed,
                "dictate",
                Some("dictate"),
                Some(Duration::from_millis(200)),
                Duration::from_millis(500),
            ),
            DoubleTapShortcutAction::StopRecording
        );
    }

    #[test]
    fn double_tap_shortcut_rearms_on_timeout_or_different_profile() {
        let context = PrimaryShortcutContext {
            capture_active: false,
            is_recording: false,
            trigger_mode: ShortcutRecordingMode::DoubleTap,
        };

        assert_eq!(
            double_tap_shortcut_action(
                context,
                ShortcutState::Pressed,
                "dictate",
                Some("dictate"),
                Some(Duration::from_millis(501)),
                Duration::from_millis(500),
            ),
            DoubleTapShortcutAction::ArmFirstTap
        );
        assert_eq!(
            double_tap_shortcut_action(
                context,
                ShortcutState::Pressed,
                "riff",
                Some("dictate"),
                Some(Duration::from_millis(200)),
                Duration::from_millis(500),
            ),
            DoubleTapShortcutAction::ArmFirstTap
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

    #[test]
    fn primary_shortcut_context_uses_profile_trigger_mode() {
        let state = AppState::new();
        {
            let mut settings = state.settings.lock();
            settings.recording_mode = "hold".to_string();
        }
        let profile = ShortcutProfile::default_riff();

        let context = primary_shortcut_context(&state, false, Some(&profile));

        assert_eq!(context.trigger_mode, ShortcutRecordingMode::Toggle);
    }
}
