use crate::commands::settings::parse_hotkey;
use tauri_plugin_global_shortcut::Modifiers;

fn assert_valid_hotkey(hotkey: &str) {
    assert!(
        parse_hotkey(hotkey).is_ok(),
        "expected valid hotkey: {hotkey}"
    );
}

fn assert_invalid_hotkey(hotkey: &str, expected_message: &str) {
    let error = parse_hotkey(hotkey).unwrap_err();
    assert!(
        error.contains(expected_message),
        "expected '{hotkey}' to contain '{expected_message}', got '{error}'"
    );
}

#[test]
fn test_parse_hotkey_valid_shift_space() {
    let (modifiers, _code) = parse_hotkey("shift+space").unwrap();
    assert!(modifiers.is_some());
    assert!(modifiers.unwrap().contains(Modifiers::SHIFT));
}

#[test]
fn test_parse_hotkey_accepts_supported_keys() {
    for hotkey in [
        "a",
        "space",
        "enter",
        "1",
        "f1",
        "cmd+space",
        "ctrl+c",
        "ctrl+a",
        "ctrl+shift+a",
        "ctrl+shift+alt+x",
        "ctrl+shift+alt+cmd+x",
        "ctrl+alt+shift+f1",
        "ctrl+arrowup",
        "cmd+f12",
        "ctrl+comma",
        "cmd+backquote",
        "alt+slash",
        "numpadenter",
        "ctrl+numpad1",
        "command+a",
        "cmd+a",
        "meta+a",
    ] {
        assert_valid_hotkey(hotkey);
    }
}

#[test]
fn test_parse_hotkey_rejects_modifier_only_keys() {
    for hotkey in [
        "cmd",
        "ctrl",
        "shift",
        "alt",
        "command",
        "meta",
        "ctrlleft",
        "ctrlright",
        "altleft",
        "altright",
        "shiftleft",
        "shiftright",
        "cmdleft",
        "cmdright",
    ] {
        assert_invalid_hotkey(hotkey, "not supported");
    }
}

#[test]
fn test_parse_hotkey_rejects_empty_string() {
    assert_invalid_hotkey("", "must have 1-5 keys");
}

#[test]
fn test_parse_hotkey_rejects_unknown_modifier() {
    assert_invalid_hotkey("super+space", "unknown modifier");
    assert_invalid_hotkey("unknown+a", "unknown modifier");
}

#[test]
fn test_parse_hotkey_rejects_unknown_key() {
    assert_invalid_hotkey("shift+unknownkey123", "unknown key");
}

#[test]
fn test_parse_hotkey_rejects_invalid_format_with_empty_part() {
    assert_invalid_hotkey("ctrl++a", "invalid format");
}

#[test]
fn test_parse_hotkey_rejects_too_many_keys() {
    assert_invalid_hotkey("ctrl+alt+shift+cmd+space+extra", "must have 1-5 keys");
    assert_invalid_hotkey("ctrl+shift+alt+cmd+x+v", "must have 1-5 keys");
}

#[test]
fn test_parse_hotkey_rejects_duplicate_modifier() {
    assert_invalid_hotkey("ctrl+ctrl+space", "duplicate modifier");
    assert_invalid_hotkey("ctrl+ctrl+a", "duplicate modifier");
}
