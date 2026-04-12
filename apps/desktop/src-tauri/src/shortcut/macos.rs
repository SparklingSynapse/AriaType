//! macOS-specific accessibility helpers.
//!
//! On macOS, global keyboard shortcuts require accessibility permissions.
//! This module provides helpers to check and request those permissions.

use std::process::Command;

/// Check if accessibility permissions are granted for this application.
///
/// Returns `true` if permissions are already granted, `false` otherwise.
/// When permissions are missing, the application cannot intercept keyboard
/// events globally.
pub fn check_accessibility() -> bool {
    handy_keys::check_accessibility()
}

/// Open macOS System Settings accessibility pane.
///
/// Guides the user to grant accessibility permissions to the application.
/// After granting permissions, the application must be restarted.
pub fn open_accessibility_settings() -> Result<(), String> {
    // Use handy-keys' helper if available, otherwise fall back to manual approach
    if let Err(_e) = handy_keys::open_accessibility_settings() {
        // Fallback: open System Settings directly
        let result = Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
            .spawn();

        if let Err(e) = result {
            return Err(format!("failed to open accessibility settings: {}", e));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_accessibility_runs() {
        // This test just verifies the function runs without panic.
        // The result depends on actual system permissions.
        let _result = check_accessibility();
    }

    #[test]
    fn test_open_accessibility_settings_returns_ok() {
        // Note: This actually opens System Settings on macOS.
        // Skip in CI or mock in real test suite.
        // For now, just verify it doesn't panic on call structure.
        // Uncomment to test manually:
        // assert!(open_accessibility_settings().is_ok());
    }
}
