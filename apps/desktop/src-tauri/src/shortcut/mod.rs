//! Cross-platform global keyboard shortcuts module.
//!
//! This module provides:
//! - `ShortcutManager`: Background thread handling hotkey registration and triggering
//! - `RecordingListener`: Low-level keyboard listener for hotkey recording UI
//! - `FnEmojiBlocker`: macOS-specific blocker for FN/Globe key emoji popup
//!
//! Uses `handy-keys` library for cross-platform support (macOS, Windows, Linux).

mod types;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "macos")]
mod fn_emoji_blocker;

mod listener;
mod manager;

// Public API
pub use listener::RecordingListener;
pub use manager::ShortcutManager;
pub use types::{HotkeyConfig, ShortcutCommand, ShortcutEvent, ShortcutState};

#[cfg(target_os = "macos")]
pub use macos::{check_accessibility, open_accessibility_settings};

#[cfg(target_os = "macos")]
pub use fn_emoji_blocker::FnEmojiBlocker;
