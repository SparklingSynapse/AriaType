use std::collections::HashMap;

use super::hotkey_codec::{pattern_matches_state, HotkeyPattern, ModifierState, PressedInput};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MatcherEvent {
    ProfilePressed { profile_id: String },
    ProfileReleased { profile_id: String },
    CancelPressed,
    CancelReleased,
    CapturePressed(PressedInput),
    CaptureReleased,
}

#[derive(Clone, Debug, Default)]
pub struct MatcherSnapshot {
    pub profiles: HashMap<String, HotkeyPattern>,
    pub cancel: Vec<HotkeyPattern>,
    pub capture_active: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MatcherInput {
    ModifierPressed(ModifierKey),
    ModifierReleased(ModifierKey),
    KeyPressed(String),
    KeyReleased(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModifierKey {
    CtrlLeft,
    CtrlRight,
    OptLeft,
    OptRight,
    ShiftLeft,
    ShiftRight,
    CmdLeft,
    CmdRight,
    Function,
}

#[derive(Clone, Debug, Default)]
pub struct MatcherState {
    pub(crate) modifiers: ModifierState,
    pub(crate) pressed_key: Option<String>,
    pub(crate) active_profile: Option<String>,
    pub(crate) active_cancel: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MatcherOutcome {
    pub swallow: bool,
    pub events: Vec<MatcherEvent>,
}

pub fn handle_input(
    state: &mut MatcherState,
    snapshot: &MatcherSnapshot,
    input: MatcherInput,
) -> MatcherOutcome {
    let mut outcome = MatcherOutcome::default();

    if snapshot.capture_active {
        match &input {
            MatcherInput::ModifierPressed(modifier) => {
                outcome
                    .events
                    .push(MatcherEvent::CapturePressed(PressedInput::Modifier(
                        modifier_token(*modifier).to_string(),
                    )));
            }
            MatcherInput::KeyPressed(key) => {
                outcome
                    .events
                    .push(MatcherEvent::CapturePressed(PressedInput::Key(key.clone())));
            }
            MatcherInput::ModifierReleased(_) | MatcherInput::KeyReleased(_) => {
                outcome.events.push(MatcherEvent::CaptureReleased);
            }
        }
    }

    apply_input_to_state(state, &input);

    let matches_cancel = snapshot.cancel.iter().any(|pattern| {
        pattern_matches_state(pattern, &state.modifiers, state.pressed_key.as_deref())
    });

    if matches_cancel && state.active_profile.is_some() {
        state.active_profile = None;
        state.active_cancel = true;
        outcome.swallow = true;
        outcome.events.push(MatcherEvent::CancelPressed);
        return outcome;
    }

    if let Some(active_profile_id) = state.active_profile.clone() {
        let Some(active_pattern) = snapshot.profiles.get(&active_profile_id) else {
            state.active_profile = None;
            return outcome;
        };

        if !pattern_matches_state(
            active_pattern,
            &state.modifiers,
            state.pressed_key.as_deref(),
        ) {
            state.active_profile = None;
            outcome.swallow = true;
            outcome.events.push(MatcherEvent::ProfileReleased {
                profile_id: active_profile_id,
            });
        } else {
            outcome.swallow = true;
        }

        return outcome;
    }

    if state.active_cancel {
        if !matches_cancel {
            state.active_cancel = false;
            outcome.swallow = true;
            outcome.events.push(MatcherEvent::CancelReleased);
        } else {
            outcome.swallow = true;
        }
        return outcome;
    }

    for (profile_id, pattern) in &snapshot.profiles {
        if pattern_matches_state(pattern, &state.modifiers, state.pressed_key.as_deref()) {
            state.active_profile = Some(profile_id.clone());
            outcome.swallow = true;
            outcome.events.push(MatcherEvent::ProfilePressed {
                profile_id: profile_id.clone(),
            });
            return outcome;
        }
    }

    if matches_cancel {
        state.active_cancel = true;
        outcome.swallow = true;
        outcome.events.push(MatcherEvent::CancelPressed);
    }

    outcome
}

fn apply_input_to_state(state: &mut MatcherState, input: &MatcherInput) {
    match input {
        MatcherInput::ModifierPressed(modifier) => {
            set_modifier_state(&mut state.modifiers, *modifier, true)
        }
        MatcherInput::ModifierReleased(modifier) => {
            set_modifier_state(&mut state.modifiers, *modifier, false)
        }
        MatcherInput::KeyPressed(key) => state.pressed_key = Some(key.clone()),
        MatcherInput::KeyReleased(key) => {
            if state.pressed_key.as_deref() == Some(key.as_str()) {
                state.pressed_key = None;
            }
        }
    }
}

fn set_modifier_state(state: &mut ModifierState, modifier: ModifierKey, pressed: bool) {
    match modifier {
        ModifierKey::CtrlLeft => state.ctrl_left = pressed,
        ModifierKey::CtrlRight => state.ctrl_right = pressed,
        ModifierKey::OptLeft => state.opt_left = pressed,
        ModifierKey::OptRight => state.opt_right = pressed,
        ModifierKey::ShiftLeft => state.shift_left = pressed,
        ModifierKey::ShiftRight => state.shift_right = pressed,
        ModifierKey::CmdLeft => state.cmd_left = pressed,
        ModifierKey::CmdRight => state.cmd_right = pressed,
        ModifierKey::Function => state.function = pressed,
    }
}

fn modifier_token(modifier: ModifierKey) -> &'static str {
    match modifier {
        ModifierKey::CtrlLeft => "CtrlLeft",
        ModifierKey::CtrlRight => "CtrlRight",
        ModifierKey::OptLeft => "OptLeft",
        ModifierKey::OptRight => "OptRight",
        ModifierKey::ShiftLeft => "ShiftLeft",
        ModifierKey::ShiftRight => "ShiftRight",
        ModifierKey::CmdLeft => "CmdLeft",
        ModifierKey::CmdRight => "CmdRight",
        ModifierKey::Function => "Fn",
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::shortcut::hotkey_codec::parse_hotkey_pattern;

    use super::{
        handle_input, MatcherEvent, MatcherInput, MatcherSnapshot, MatcherState, ModifierKey,
    };

    #[test]
    fn pressing_matching_profile_swallow_event_and_emits_press() {
        let mut profiles = HashMap::new();
        profiles.insert(
            "dictate".to_string(),
            parse_hotkey_pattern("Cmd+Slash").unwrap(),
        );
        let snapshot = MatcherSnapshot {
            profiles,
            cancel: Vec::new(),
            capture_active: false,
        };
        let mut state = MatcherState::default();

        let _ = handle_input(
            &mut state,
            &snapshot,
            MatcherInput::ModifierPressed(ModifierKey::CmdLeft),
        );
        let outcome = handle_input(
            &mut state,
            &snapshot,
            MatcherInput::KeyPressed("Slash".to_string()),
        );

        assert!(outcome.swallow);
        assert_eq!(
            outcome.events,
            vec![MatcherEvent::ProfilePressed {
                profile_id: "dictate".to_string()
            }]
        );
    }

    #[test]
    fn releasing_active_profile_emits_release_and_swallow() {
        let mut profiles = HashMap::new();
        profiles.insert(
            "dictate".to_string(),
            parse_hotkey_pattern("Cmd+Slash").unwrap(),
        );
        let snapshot = MatcherSnapshot {
            profiles,
            cancel: Vec::new(),
            capture_active: false,
        };
        let mut state = MatcherState::default();

        let _ = handle_input(
            &mut state,
            &snapshot,
            MatcherInput::ModifierPressed(ModifierKey::CmdLeft),
        );
        let _ = handle_input(
            &mut state,
            &snapshot,
            MatcherInput::KeyPressed("Slash".to_string()),
        );
        let outcome = handle_input(
            &mut state,
            &snapshot,
            MatcherInput::KeyReleased("Slash".to_string()),
        );

        assert!(outcome.swallow);
        assert_eq!(
            outcome.events,
            vec![MatcherEvent::ProfileReleased {
                profile_id: "dictate".to_string()
            }]
        );
    }

    #[test]
    fn capture_active_emits_capture_events_from_same_stream() {
        let snapshot = MatcherSnapshot {
            profiles: HashMap::new(),
            cancel: Vec::new(),
            capture_active: true,
        };
        let mut state = MatcherState::default();

        let pressed = handle_input(
            &mut state,
            &snapshot,
            MatcherInput::ModifierPressed(ModifierKey::CmdRight),
        );
        let key_pressed = handle_input(
            &mut state,
            &snapshot,
            MatcherInput::KeyPressed("Slash".to_string()),
        );
        let released = handle_input(
            &mut state,
            &snapshot,
            MatcherInput::KeyReleased("Slash".to_string()),
        );

        assert_eq!(
            pressed.events,
            vec![MatcherEvent::CapturePressed(
                crate::shortcut::hotkey_codec::PressedInput::Modifier("CmdRight".to_string())
            )]
        );
        assert_eq!(
            key_pressed.events,
            vec![MatcherEvent::CapturePressed(
                crate::shortcut::hotkey_codec::PressedInput::Key("Slash".to_string())
            )]
        );
        assert_eq!(released.events, vec![MatcherEvent::CaptureReleased]);
    }

    #[test]
    fn side_specific_pattern_matches_correct_side_only() {
        let mut profiles = HashMap::new();
        profiles.insert(
            "dictate".to_string(),
            parse_hotkey_pattern("CmdRight+Slash").unwrap(),
        );
        let snapshot = MatcherSnapshot {
            profiles,
            cancel: Vec::new(),
            capture_active: false,
        };
        let mut state = MatcherState::default();

        let _ = handle_input(
            &mut state,
            &snapshot,
            MatcherInput::ModifierPressed(ModifierKey::CmdLeft),
        );
        let wrong_side = handle_input(
            &mut state,
            &snapshot,
            MatcherInput::KeyPressed("Slash".to_string()),
        );

        assert!(!wrong_side.swallow);
        assert!(wrong_side.events.is_empty());
    }

    #[test]
    fn cancel_hotkey_takes_precedence_over_active_fn_profile() {
        let mut profiles = HashMap::new();
        profiles.insert("dictate".to_string(), parse_hotkey_pattern("Fn").unwrap());
        let snapshot = MatcherSnapshot {
            profiles,
            cancel: vec![
                parse_hotkey_pattern("Escape").unwrap(),
                parse_hotkey_pattern("Fn+Escape").unwrap(),
            ],
            capture_active: false,
        };
        let mut state = MatcherState::default();

        let pressed = handle_input(
            &mut state,
            &snapshot,
            MatcherInput::ModifierPressed(ModifierKey::Function),
        );
        assert_eq!(
            pressed.events,
            vec![MatcherEvent::ProfilePressed {
                profile_id: "dictate".to_string()
            }]
        );

        let cancel = handle_input(
            &mut state,
            &snapshot,
            MatcherInput::KeyPressed("Escape".to_string()),
        );

        assert!(cancel.swallow);
        assert_eq!(cancel.events, vec![MatcherEvent::CancelPressed]);
    }

    #[test]
    fn fn_escape_cancel_releases_when_escape_is_lifted() {
        let mut profiles = HashMap::new();
        profiles.insert("dictate".to_string(), parse_hotkey_pattern("Fn").unwrap());
        let snapshot = MatcherSnapshot {
            profiles,
            cancel: vec![
                parse_hotkey_pattern("Escape").unwrap(),
                parse_hotkey_pattern("Fn+Escape").unwrap(),
            ],
            capture_active: false,
        };
        let mut state = MatcherState::default();

        let _ = handle_input(
            &mut state,
            &snapshot,
            MatcherInput::ModifierPressed(ModifierKey::Function),
        );
        let _ = handle_input(
            &mut state,
            &snapshot,
            MatcherInput::KeyPressed("Escape".to_string()),
        );

        let escape_release = handle_input(
            &mut state,
            &snapshot,
            MatcherInput::KeyReleased("Escape".to_string()),
        );

        assert!(escape_release.swallow);
        assert_eq!(escape_release.events, vec![MatcherEvent::CancelReleased]);

        let fn_release = handle_input(
            &mut state,
            &snapshot,
            MatcherInput::ModifierReleased(ModifierKey::Function),
        );

        assert!(!fn_release.swallow);
        assert!(fn_release.events.is_empty());
    }
}
