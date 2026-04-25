//! Profile types for multi-shortcut support.
//!
//! Fixed-key map structure: { dictate, chat, custom? }
//! - dictate: system profile, polish_template_id = null (fixed)
//! - chat: system profile, polish_template_id non-null (default first template)
//! - custom: optional user profile (max 1), polish_template_id can be null

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ShortcutTriggerMode {
    Hold,
    Toggle,
}

impl ShortcutTriggerMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Hold => "hold",
            Self::Toggle => "toggle",
        }
    }
}

/// Map of shortcut profiles with fixed keys.
/// Stored in settings as an object/map, not an array.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShortcutProfilesMap {
    /// System profile: always exists, cannot be deleted.
    /// Fixed polish_template_id = None (no polish).
    pub dictate: ShortcutProfile,

    /// System profile: always exists, cannot be deleted.
    /// polish_template_id defaults to first template, cannot be None.
    pub chat: ShortcutProfile,

    /// Optional user profile: can be created and deleted (max 1).
    /// polish_template_id can be None or any template.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom: Option<ShortcutProfile>,
}

impl Default for ShortcutProfilesMap {
    fn default() -> Self {
        Self {
            dictate: ShortcutProfile::default_dictate(),
            chat: ShortcutProfile::default_chat(),
            custom: None,
        }
    }
}

impl ShortcutProfilesMap {
    pub fn with_migration_hotkey(hotkey: String) -> Self {
        Self {
            dictate: ShortcutProfile {
                hotkey,
                trigger_mode: ShortcutTriggerMode::Hold,
                action: ShortcutAction::Record {
                    polish_template_id: None,
                },
            },
            chat: ShortcutProfile::default_chat(),
            custom: None,
        }
    }
}

/// Single shortcut profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShortcutProfile {
    /// Hotkey string in the canonical shortcut format.
    pub hotkey: String,
    /// Whether this shortcut starts on hold or toggles on press.
    #[serde(default = "default_trigger_mode")]
    pub trigger_mode: ShortcutTriggerMode,
    /// The action this profile triggers.
    pub action: ShortcutAction,
}

fn default_trigger_mode() -> ShortcutTriggerMode {
    ShortcutTriggerMode::Hold
}

impl ShortcutProfile {
    /// Dictate profile: Cmd+Slash, no polish template.
    pub fn default_dictate() -> Self {
        Self {
            hotkey: "Cmd+Slash".to_string(),
            trigger_mode: ShortcutTriggerMode::Hold,
            action: ShortcutAction::Record {
                polish_template_id: None,
            },
        }
    }

    /// Chat profile: Opt+Slash, default polish template.
    pub fn default_chat() -> Self {
        Self {
            hotkey: "Opt+Slash".to_string(),
            trigger_mode: ShortcutTriggerMode::Toggle,
            action: ShortcutAction::Record {
                polish_template_id: Some("filler".to_string()),
            },
        }
    }
}

/// What a shortcut does.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ShortcutAction {
    /// Standard recording action with optional polish template.
    Record {
        /// Polish template ID to use for this profile.
        /// - None: Skip polish (Dictate behavior)
        /// - Some(template_id): Apply polish with template's prompt + global provider/model
        polish_template_id: Option<String>,
    },
}

impl ShortcutAction {
    pub fn is_record(&self) -> bool {
        matches!(self, ShortcutAction::Record { .. })
    }
}

impl Default for ShortcutAction {
    fn default() -> Self {
        ShortcutAction::Record {
            polish_template_id: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profiles_map_serializes_with_fixed_keys() {
        let profiles = ShortcutProfilesMap {
            dictate: ShortcutProfile {
                hotkey: "Shift+Space".to_string(),
                trigger_mode: ShortcutTriggerMode::Hold,
                action: ShortcutAction::Record {
                    polish_template_id: None,
                },
            },
            chat: ShortcutProfile {
                hotkey: "Cmd+Space".to_string(),
                trigger_mode: ShortcutTriggerMode::Toggle,
                action: ShortcutAction::Record {
                    polish_template_id: Some("filler".to_string()),
                },
            },
            custom: None,
        };

        let json = serde_json::to_string(&profiles).unwrap();
        assert!(json.contains("\"dictate\""));
        assert!(json.contains("\"chat\""));
        assert!(!json.contains("\"custom\""));
    }

    #[test]
    fn profiles_map_with_custom_serializes() {
        let profiles = ShortcutProfilesMap {
            dictate: ShortcutProfile::default_dictate(),
            chat: ShortcutProfile::default_chat(),
            custom: Some(ShortcutProfile {
                hotkey: "Cmd+Alt+Space".to_string(),
                trigger_mode: ShortcutTriggerMode::Toggle,
                action: ShortcutAction::Record {
                    polish_template_id: Some("formal".to_string()),
                },
            }),
        };

        let json = serde_json::to_string(&profiles).unwrap();
        assert!(json.contains("\"custom\""));
    }

    #[test]
    fn profile_serialization_roundtrip() {
        let profile = ShortcutProfile {
            hotkey: "Cmd+Shift+Space".to_string(),
            trigger_mode: ShortcutTriggerMode::Toggle,
            action: ShortcutAction::Record {
                polish_template_id: Some("filler".to_string()),
            },
        };

        let json = serde_json::to_string(&profile).unwrap();
        let decoded: ShortcutProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(profile, decoded);
    }

    #[test]
    fn action_serializes_to_pascal_case() {
        let action = ShortcutAction::Record {
            polish_template_id: None,
        };
        let json = serde_json::to_string(&action).unwrap();
        assert_eq!(json, r#"{"Record":{"polish_template_id":null}}"#);
    }

    #[test]
    fn action_with_template_serializes() {
        let action = ShortcutAction::Record {
            polish_template_id: Some("filler".to_string()),
        };
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("filler"));
    }

    #[test]
    fn default_dictate_has_no_template() {
        let profile = ShortcutProfile::default_dictate();
        assert_eq!(profile.trigger_mode, ShortcutTriggerMode::Hold);
        match &profile.action {
            ShortcutAction::Record { polish_template_id } => {
                assert!(polish_template_id.is_none());
            }
        }
    }

    #[test]
    fn default_chat_has_template() {
        let profile = ShortcutProfile::default_chat();
        assert_eq!(profile.trigger_mode, ShortcutTriggerMode::Toggle);
        match &profile.action {
            ShortcutAction::Record { polish_template_id } => {
                assert!(polish_template_id.is_some());
            }
        }
    }

    #[test]
    fn default_action_is_record_without_template() {
        let action = ShortcutAction::default();
        assert!(action.is_record());
        match action {
            ShortcutAction::Record { polish_template_id } => {
                assert!(polish_template_id.is_none());
            }
        }
    }

    #[test]
    fn profiles_map_default_dictate_chat_no_custom() {
        let profiles = ShortcutProfilesMap::default();
        assert_eq!(profiles.dictate.hotkey, "Cmd+Slash");
        assert_eq!(profiles.dictate.trigger_mode, ShortcutTriggerMode::Hold);
        assert_eq!(profiles.chat.hotkey, "Opt+Slash");
        assert_eq!(profiles.chat.trigger_mode, ShortcutTriggerMode::Toggle);
        assert!(profiles.custom.is_none());
    }

    #[test]
    fn default_profiles_serialize_with_expected_trigger_modes() {
        let profiles = ShortcutProfilesMap {
            dictate: ShortcutProfile::default_dictate(),
            chat: ShortcutProfile::default_chat(),
            custom: Some(ShortcutProfile {
                hotkey: String::new(),
                trigger_mode: ShortcutTriggerMode::Toggle,
                action: ShortcutAction::Record {
                    polish_template_id: Some("filler".to_string()),
                },
            }),
        };

        let value = serde_json::to_value(&profiles).unwrap();
        assert_eq!(value["dictate"]["trigger_mode"], "hold");
        assert_eq!(value["chat"]["trigger_mode"], "toggle");
        assert_eq!(value["custom"]["trigger_mode"], "toggle");
    }

    #[test]
    fn profiles_map_deserializes_missing_custom() {
        let json = r#"{"dictate":{"hotkey":"Shift+Space","action":{"Record":{"polish_template_id":null}}},"chat":{"hotkey":"","action":{"Record":{"polish_template_id":"filler"}}}}"#;
        let profiles: ShortcutProfilesMap = serde_json::from_str(json).unwrap();
        assert!(profiles.custom.is_none());
    }
}
