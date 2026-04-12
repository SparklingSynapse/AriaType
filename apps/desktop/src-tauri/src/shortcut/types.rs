//! Internal types for the shortcut module.
//!
//! These types represent commands sent to the background manager thread
//! and events emitted back to the main thread.

use serde::{Deserialize, Serialize};

/// Configuration for a keyboard shortcut.
///
/// Wraps the string representation (e.g., "Shift+Space") with optional
/// metadata for future extensibility.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HotkeyConfig {
    /// The hotkey string in handy-keys format (e.g., "Shift+Space", "Cmd+K").
    /// Use platform-appropriate modifier names: Cmd/Ctrl, Alt/Opt, Shift.
    pub hotkey: String,
}

impl HotkeyConfig {
    /// Creates a new hotkey configuration.
    pub fn new(hotkey: impl Into<String>) -> Self {
        Self {
            hotkey: hotkey.into(),
        }
    }

    /// Returns the hotkey string.
    pub fn as_str(&self) -> &str {
        &self.hotkey
    }
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        Self::new("Shift+Space")
    }
}

/// State of a triggered shortcut.
///
/// Mirrors handy-keys' HotkeyState for internal use without
/// exposing external library types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ShortcutState {
    /// Key combination was pressed down.
    Pressed,
    /// Key combination was released.
    Released,
}

impl ShortcutState {
    pub fn as_str(&self) -> &'static str {
        match self {
            ShortcutState::Pressed => "pressed",
            ShortcutState::Released => "released",
        }
    }
}

/// Commands sent to the ShortcutManager background thread.
#[derive(Debug, Clone)]
pub enum ShortcutCommand {
    /// Register a new hotkey, replacing any existing one.
    Register { hotkey: String },
    /// Unregister the current hotkey.
    Unregister,
    /// Signal the thread to shut down.
    Shutdown,
}

/// Events emitted from the ShortcutManager to the main thread.
#[derive(Debug, Clone)]
pub enum ShortcutEvent {
    /// Hotkey was triggered with the given state.
    Triggered { state: ShortcutState },
    /// Cancel hotkey (ESC) was triggered.
    CancelTriggered { state: ShortcutState },
    /// Hotkey registration failed with an error message.
    RegistrationFailed { error: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hotkey_config_new() {
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
    fn test_shortcut_state_as_str() {
        assert_eq!(ShortcutState::Pressed.as_str(), "pressed");
        assert_eq!(ShortcutState::Released.as_str(), "released");
    }

    #[test]
    fn test_shortcut_state_serde() {
        let pressed = ShortcutState::Pressed;
        let json = serde_json::to_string(&pressed).unwrap();
        assert_eq!(json, "\"pressed\"");

        let decoded: ShortcutState = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, ShortcutState::Pressed);
    }

    #[test]
    fn test_shortcut_command_register() {
        let cmd = ShortcutCommand::Register {
            hotkey: "Ctrl+Space".to_string(),
        };
        assert!(matches!(cmd, ShortcutCommand::Register { .. }));
    }

    #[test]
    fn test_shortcut_event_triggered() {
        let event = ShortcutEvent::Triggered {
            state: ShortcutState::Pressed,
        };
        assert!(matches!(event, ShortcutEvent::Triggered { .. }));
    }
}
