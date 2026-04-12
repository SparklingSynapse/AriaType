//! Integration tests for shortcut module.
//!
//! Tests the full flow of shortcut manager and listener.

use crate::shortcut::{HotkeyConfig, ShortcutEvent, ShortcutManager, ShortcutState};

#[test]
fn test_hotkey_config_creation() {
    let config = HotkeyConfig::new("Cmd+Shift+K");
    assert_eq!(config.hotkey, "Cmd+Shift+K");
    assert_eq!(config.as_str(), "Cmd+Shift+K");
}

#[test]
fn test_hotkey_config_default() {
    let config = HotkeyConfig::default();
    assert_eq!(config.hotkey, "Shift+Space");
}

#[test]
fn test_shortcut_manager_creation() {
    let manager = ShortcutManager::new();
    assert!(manager.is_ok());
}

#[test]
fn test_shortcut_manager_register() {
    let manager = ShortcutManager::new().unwrap();
    // Register should succeed even without app handle (stores in pending state)
    let result = manager.register("Cmd+Space");
    assert!(result.is_ok());
}

#[test]
fn test_shortcut_manager_unregister() {
    let manager = ShortcutManager::new().unwrap();
    let result = manager.unregister();
    assert!(result.is_ok());
}

#[test]
fn test_shortcut_manager_stop_without_start() {
    let mut manager = ShortcutManager::new().unwrap();
    let result = manager.stop();
    assert!(result.is_ok()); // Should handle gracefully
}

#[test]
fn test_shortcut_state_serde() {
    let pressed = ShortcutState::Pressed;
    let json = serde_json::to_string(&pressed).unwrap();
    assert_eq!(json, "\"pressed\"");

    let decoded: ShortcutState = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded, ShortcutState::Pressed);

    let released = ShortcutState::Released;
    let json = serde_json::to_string(&released).unwrap();
    assert_eq!(json, "\"released\"");
}

#[test]
fn test_shortcut_event_creation() {
    let event = ShortcutEvent::Triggered {
        state: ShortcutState::Pressed,
    };
    // Just verify it can be created
    assert!(matches!(event, ShortcutEvent::Triggered { .. }));

    let error_event = ShortcutEvent::RegistrationFailed {
        error: "test error".to_string(),
    };
    assert!(matches!(
        error_event,
        ShortcutEvent::RegistrationFailed { .. }
    ));
}

// Note: Full integration tests with KeyboardListener require
// accessibility permissions on macOS and cannot be run in CI.
// The following tests verify the structural integrity only.
