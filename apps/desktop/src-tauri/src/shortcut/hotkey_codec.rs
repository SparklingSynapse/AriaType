use std::collections::BTreeSet;

use rdev::Key;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SideRequirement {
    Any,
    LeftOnly,
    RightOnly,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ModifierState {
    pub ctrl_left: bool,
    pub ctrl_right: bool,
    pub opt_left: bool,
    pub opt_right: bool,
    pub shift_left: bool,
    pub shift_right: bool,
    pub cmd_left: bool,
    pub cmd_right: bool,
    pub function: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HotkeyPattern {
    canonical: String,
    ctrl: Option<SideRequirement>,
    opt: Option<SideRequirement>,
    shift: Option<SideRequirement>,
    cmd: Option<SideRequirement>,
    function: bool,
    key: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PressedInput {
    Modifier(String),
    Key(String),
}

pub fn canonicalize_hotkey_string(_input: &str) -> Result<String, String> {
    let pattern = parse_hotkey_pattern(_input)?;
    Ok(pattern.canonical.clone())
}

pub fn parse_hotkey_pattern(_input: &str) -> Result<HotkeyPattern, String> {
    let tokens = _input
        .split('+')
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();

    if tokens.is_empty() {
        return Err("Hotkey cannot be empty.".to_string());
    }

    let mut canonical_modifiers = BTreeSet::new();
    let mut ctrl = None;
    let mut opt = None;
    let mut shift = None;
    let mut cmd = None;
    let mut function = false;
    let mut key = None;

    for token in tokens {
        if let Ok((canonical_modifier, requirement)) = normalize_modifier_token(token) {
            match canonical_modifier.as_str() {
                "Ctrl" | "CtrlLeft" | "CtrlRight" => ctrl = Some(requirement),
                "Opt" | "OptLeft" | "OptRight" => opt = Some(requirement),
                "Shift" | "ShiftLeft" | "ShiftRight" => shift = Some(requirement),
                "Cmd" | "CmdLeft" | "CmdRight" => cmd = Some(requirement),
                "Fn" => function = true,
                _ => {}
            }
            canonical_modifiers.insert(canonical_modifier);
            continue;
        }

        let normalized_key = normalize_key_token(token);
        if key.replace(normalized_key).is_some() {
            return Err("Multiple keys not supported. Use modifiers + single key.".to_string());
        }
    }

    let canonical = build_hotkey_string(&canonical_modifiers, key.as_deref());

    Ok(HotkeyPattern {
        canonical,
        ctrl,
        opt,
        shift,
        cmd,
        function,
        key,
    })
}

pub fn pattern_matches_state(
    _pattern: &HotkeyPattern,
    _state: &ModifierState,
    _key: Option<&str>,
) -> bool {
    key_matches(&_pattern.key, _key)
        && modifier_matches(_pattern.ctrl, _state.ctrl_left, _state.ctrl_right)
        && modifier_matches(_pattern.opt, _state.opt_left, _state.opt_right)
        && modifier_matches(_pattern.shift, _state.shift_left, _state.shift_right)
        && modifier_matches(_pattern.cmd, _state.cmd_left, _state.cmd_right)
        && _pattern.function == _state.function
}

pub fn analyze_pressed_sequence(_sequence: &[PressedInput]) -> Result<String, String> {
    let mut canonical_modifiers = BTreeSet::new();
    let mut key = None;

    for input in _sequence {
        match input {
            PressedInput::Modifier(token) => {
                let (canonical_modifier, _) = normalize_modifier_token(token)?;
                canonical_modifiers.insert(canonical_modifier);
            }
            PressedInput::Key(token) => {
                let normalized_key = normalize_key_token(token);
                if key.replace(normalized_key).is_some() {
                    return Err(
                        "Multiple keys not supported. Use modifiers + single key.".to_string()
                    );
                }
            }
        }
    }

    let has_fn = canonical_modifiers.contains("Fn");
    let has_regular_modifier = canonical_modifiers.iter().any(|token| token != "Fn");

    match key.as_deref() {
        None if has_fn && !has_regular_modifier => Ok("Fn".to_string()),
        None => Err("Modifier-only hotkey not supported. Press a key after modifiers.".to_string()),
        Some(key_token) if !has_fn && !has_regular_modifier && !is_function_key_name(key_token) => {
            Err("Single key requires a modifier (e.g., Cmd+A, Shift+Space). F1-F20 and Fn are exceptions.".to_string())
        }
        Some(_) => Ok(build_hotkey_string(&canonical_modifiers, key.as_deref())),
    }
}

pub fn key_token_from_rdev_key(key: Key) -> Option<String> {
    let token = match key {
        Key::Backspace => "Delete",
        Key::Delete => "ForwardDelete",
        Key::DownArrow => "DownArrow",
        Key::End => "End",
        Key::Escape => "Escape",
        Key::F1 => "F1",
        Key::F2 => "F2",
        Key::F3 => "F3",
        Key::F4 => "F4",
        Key::F5 => "F5",
        Key::F6 => "F6",
        Key::F7 => "F7",
        Key::F8 => "F8",
        Key::F9 => "F9",
        Key::F10 => "F10",
        Key::F11 => "F11",
        Key::F12 => "F12",
        Key::Home => "Home",
        Key::Insert => "Insert",
        Key::KpReturn => "Return",
        Key::LeftArrow => "LeftArrow",
        Key::PageDown => "PageDown",
        Key::PageUp => "PageUp",
        Key::Return => "Return",
        Key::RightArrow => "RightArrow",
        Key::Slash => "Slash",
        Key::BackSlash | Key::IntlBackslash => "Backslash",
        Key::Space => "Space",
        Key::Tab => "Tab",
        Key::UpArrow => "UpArrow",
        Key::BackQuote => "BackQuote",
        Key::Minus | Key::KpMinus => "Minus",
        Key::Equal => "Equal",
        Key::Comma => "Comma",
        Key::Dot => "Dot",
        Key::SemiColon => "SemiColon",
        Key::Quote => "Quote",
        Key::LeftBracket => "LeftBracket",
        Key::RightBracket => "RightBracket",
        Key::Num0 => "0",
        Key::Num1 => "1",
        Key::Num2 => "2",
        Key::Num3 => "3",
        Key::Num4 => "4",
        Key::Num5 => "5",
        Key::Num6 => "6",
        Key::Num7 => "7",
        Key::Num8 => "8",
        Key::Num9 => "9",
        Key::KeyA => "A",
        Key::KeyB => "B",
        Key::KeyC => "C",
        Key::KeyD => "D",
        Key::KeyE => "E",
        Key::KeyF => "F",
        Key::KeyG => "G",
        Key::KeyH => "H",
        Key::KeyI => "I",
        Key::KeyJ => "J",
        Key::KeyK => "K",
        Key::KeyL => "L",
        Key::KeyM => "M",
        Key::KeyN => "N",
        Key::KeyO => "O",
        Key::KeyP => "P",
        Key::KeyQ => "Q",
        Key::KeyR => "R",
        Key::KeyS => "S",
        Key::KeyT => "T",
        Key::KeyU => "U",
        Key::KeyV => "V",
        Key::KeyW => "W",
        Key::KeyX => "X",
        Key::KeyY => "Y",
        Key::KeyZ => "Z",
        Key::Kp0 => "Kp0",
        Key::Kp1 => "Kp1",
        Key::Kp2 => "Kp2",
        Key::Kp3 => "Kp3",
        Key::Kp4 => "Kp4",
        Key::Kp5 => "Kp5",
        Key::Kp6 => "Kp6",
        Key::Kp7 => "Kp7",
        Key::Kp8 => "Kp8",
        Key::Kp9 => "Kp9",
        Key::KpDelete => "KpDelete",
        Key::KpDivide => "KpDivide",
        Key::KpMultiply => "KpMultiply",
        Key::KpPlus => "KpPlus",
        _ => return None,
    };
    Some(token.to_string())
}

fn normalize_key_token(token: &str) -> String {
    let trimmed = token.trim();
    let lowered = trimmed.to_ascii_lowercase();
    match lowered.as_str() {
        "slash" => "Slash".to_string(),
        "/" => "Slash".to_string(),
        "backslash" => "Backslash".to_string(),
        "\\" => "Backslash".to_string(),
        "space" => "Space".to_string(),
        "return" | "enter" => "Return".to_string(),
        "tab" => "Tab".to_string(),
        "escape" | "esc" => "Escape".to_string(),
        "delete" => "Delete".to_string(),
        "forwarddelete" | "forward_delete" => "ForwardDelete".to_string(),
        _ if is_function_key_name(trimmed) => trimmed.to_ascii_uppercase(),
        _ if trimmed.len() == 1 => trimmed.to_ascii_uppercase(),
        _ => {
            let mut chars = trimmed.chars();
            if let Some(first) = chars.next() {
                format!("{}{}", first.to_ascii_uppercase(), chars.as_str())
            } else {
                String::new()
            }
        }
    }
}

fn normalize_modifier_token(token: &str) -> Result<(String, SideRequirement), String> {
    let lowered = token.trim().to_ascii_lowercase();
    match lowered.as_str() {
        "ctrl" | "control" => Ok(("Ctrl".to_string(), SideRequirement::Any)),
        "ctrlleft" | "controlleft" => Ok(("CtrlLeft".to_string(), SideRequirement::LeftOnly)),
        "ctrlright" | "controlright" => Ok(("CtrlRight".to_string(), SideRequirement::RightOnly)),
        "opt" | "option" | "alt" => Ok(("Opt".to_string(), SideRequirement::Any)),
        "optleft" | "optionleft" | "altleft" => {
            Ok(("OptLeft".to_string(), SideRequirement::LeftOnly))
        }
        "optright" | "optionright" | "altright" | "altgr" => {
            Ok(("OptRight".to_string(), SideRequirement::RightOnly))
        }
        "shift" => Ok(("Shift".to_string(), SideRequirement::Any)),
        "shiftleft" => Ok(("ShiftLeft".to_string(), SideRequirement::LeftOnly)),
        "shiftright" => Ok(("ShiftRight".to_string(), SideRequirement::RightOnly)),
        "cmd" | "command" | "meta" | "super" | "win" => {
            Ok(("Cmd".to_string(), SideRequirement::Any))
        }
        "cmdleft" | "commandleft" | "metaleft" | "superleft" | "winleft" => {
            Ok(("CmdLeft".to_string(), SideRequirement::LeftOnly))
        }
        "cmdright" | "commandright" | "metaright" | "superright" | "winright" => {
            Ok(("CmdRight".to_string(), SideRequirement::RightOnly))
        }
        "fn" | "function" | "globe" => Ok(("Fn".to_string(), SideRequirement::Any)),
        _ => Err(format!("unsupported modifier token: {token}")),
    }
}

fn ordered_modifier_tokens(tokens: &BTreeSet<String>) -> Vec<String> {
    let mut ordered = Vec::new();
    for group in ["Ctrl", "Opt", "Shift", "Cmd", "Fn"] {
        for token in tokens {
            if token.starts_with(group) {
                ordered.push(token.clone());
            }
        }
    }
    ordered
}

fn build_hotkey_string(modifiers: &BTreeSet<String>, key: Option<&str>) -> String {
    let mut parts = ordered_modifier_tokens(modifiers);
    if let Some(key_token) = key {
        parts.push(key_token.to_string());
    }
    parts.join("+")
}

fn key_matches(expected: &Option<String>, actual: Option<&str>) -> bool {
    match (expected.as_deref(), actual) {
        (None, None) => true,
        (Some(expected_key), Some(actual_key)) => expected_key == normalize_key_token(actual_key),
        _ => false,
    }
}

fn modifier_matches(requirement: Option<SideRequirement>, left: bool, right: bool) -> bool {
    match requirement {
        None => !left && !right,
        Some(SideRequirement::Any) => left || right,
        Some(SideRequirement::LeftOnly) => left && !right,
        Some(SideRequirement::RightOnly) => right && !left,
    }
}

fn is_function_key_name(token: &str) -> bool {
    let upper = token.trim().to_ascii_uppercase();
    let Some(number) = upper.strip_prefix('F') else {
        return false;
    };
    number
        .parse::<u8>()
        .is_ok_and(|value| (1..=20).contains(&value))
}

#[cfg(test)]
mod tests {
    use super::{
        analyze_pressed_sequence, canonicalize_hotkey_string, parse_hotkey_pattern,
        pattern_matches_state, ModifierState, PressedInput,
    };

    #[test]
    fn canonicalizes_modifier_aliases_and_order() {
        let canonical = canonicalize_hotkey_string("command+shift+slash").unwrap();
        assert_eq!(canonical, "Shift+Cmd+Slash");
    }

    #[test]
    fn canonicalizes_side_specific_modifiers() {
        let canonical = canonicalize_hotkey_string("metaright+slash").unwrap();
        assert_eq!(canonical, "CmdRight+Slash");
    }

    #[test]
    fn generic_modifier_binding_matches_either_side() {
        let pattern = parse_hotkey_pattern("Cmd+Slash").unwrap();

        let left_state = ModifierState {
            cmd_left: true,
            ..ModifierState::default()
        };
        let right_state = ModifierState {
            cmd_right: true,
            ..ModifierState::default()
        };

        assert!(pattern_matches_state(&pattern, &left_state, Some("Slash")));
        assert!(pattern_matches_state(&pattern, &right_state, Some("Slash")));
    }

    #[test]
    fn side_specific_binding_rejects_wrong_side() {
        let pattern = parse_hotkey_pattern("CmdRight+Slash").unwrap();

        let left_state = ModifierState {
            cmd_left: true,
            ..ModifierState::default()
        };
        let right_state = ModifierState {
            cmd_right: true,
            ..ModifierState::default()
        };

        assert!(!pattern_matches_state(&pattern, &left_state, Some("Slash")));
        assert!(pattern_matches_state(&pattern, &right_state, Some("Slash")));
    }

    #[test]
    fn analyzes_fn_only_capture() {
        let hotkey = analyze_pressed_sequence(&[PressedInput::Modifier("Fn".to_string())]).unwrap();
        assert_eq!(hotkey, "Fn");
    }

    #[test]
    fn analyzes_modifier_plus_key_capture() {
        let hotkey = analyze_pressed_sequence(&[
            PressedInput::Modifier("CmdRight".to_string()),
            PressedInput::Key("Slash".to_string()),
        ])
        .unwrap();
        assert_eq!(hotkey, "CmdRight+Slash");
    }

    #[test]
    fn rejects_single_non_function_key_without_modifier() {
        let error = analyze_pressed_sequence(&[PressedInput::Key("A".to_string())]).unwrap_err();
        assert!(error.contains("requires a modifier"));
    }
}
