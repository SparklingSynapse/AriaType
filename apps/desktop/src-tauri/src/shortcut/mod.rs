//! Cross-platform global keyboard shortcuts module.
//!
//! This module provides:
//! - `ShortcutManager`: Background thread handling hotkey registration and triggering
//! - `ShortcutProfile`: Multi-shortcut profile support
//! - `ShortcutAction`: Actions bound to profiles
//! - Internal recording capture runtime for hotkey recording UI
//! - `FnEmojiBlocker`: macOS-specific blocker for FN/Globe key emoji popup
//!
//! Uses platform-native runners built around the `rdev` key model.

mod hotkey_codec;
mod matcher;
mod platform;
mod profile_types;
mod types;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "macos")]
mod fn_emoji_blocker;

mod manager;

// Public API
pub use manager::ShortcutManager;
pub use profile_types::{
    ShortcutAction, ShortcutProfile, ShortcutProfilesMap, ShortcutTriggerMode,
};
pub use types::{HotkeyConfig, ShortcutCommand, ShortcutEvent, ShortcutState};

#[cfg(target_os = "macos")]
pub use macos::{check_accessibility, open_accessibility_settings};

#[cfg(target_os = "macos")]
pub use fn_emoji_blocker::FnEmojiBlocker;
