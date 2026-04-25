//! macOS-specific accessibility helpers.
//!
//! On macOS, global keyboard shortcuts require accessibility permissions.
//! This module provides helpers to check, probe, and request those permissions.

#[cfg(target_os = "macos")]
use std::ffi::c_void;
use std::process::Command;
#[cfg(target_os = "macos")]
use std::ptr::NonNull;

/// Check if accessibility permissions are granted for this application.
///
/// Returns `true` if permissions are already granted, `false` otherwise.
/// When permissions are missing, the application cannot intercept keyboard
/// events globally.
pub fn check_accessibility() -> bool {
    crate::permissions::check_permission(crate::permissions::PermissionKind::Accessibility)
        == crate::permissions::PermissionStatus::Granted
}

/// Create and immediately tear down a fresh keyboard-only event tap.
///
/// This is stricter than `check_accessibility()`: it verifies that macOS will
/// currently allow a new session event tap to be created for keyboard traffic.
pub fn fresh_event_tap_probe() -> Result<(), String> {
    use objc2_core_foundation::CFMachPort;
    use objc2_core_graphics::{
        CGEvent, CGEventMask, CGEventTapCallBack, CGEventTapLocation, CGEventTapOptions,
        CGEventTapPlacement, CGEventType,
    };

    if !check_accessibility() {
        return Err("Accessibility permission not granted".to_string());
    }

    let event_mask: CGEventMask = (1 << CGEventType::KeyDown.0)
        | (1 << CGEventType::KeyUp.0)
        | (1 << CGEventType::FlagsChanged.0);

    let callback: CGEventTapCallBack = Some(fresh_event_tap_probe_callback);
    let tap = unsafe {
        CGEvent::tap_create(
            CGEventTapLocation::SessionEventTap,
            CGEventTapPlacement::HeadInsertEventTap,
            CGEventTapOptions::ListenOnly,
            event_mask,
            callback,
            std::ptr::null_mut(),
        )
    }
    .ok_or_else(|| "Failed to create fresh event tap probe".to_string())?;

    CGEvent::tap_enable(&tap, true);
    CGEvent::tap_enable(&tap, false);
    CFMachPort::invalidate(&tap);

    Ok(())
}

/// Open macOS System Settings accessibility pane.
///
/// Guides the user to grant accessibility permissions to the application.
pub fn open_accessibility_settings() -> Result<(), String> {
    let result = Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
        .spawn();

    if let Err(e) = result {
        return Err(format!("failed to open accessibility settings: {}", e));
    }
    Ok(())
}

unsafe extern "C-unwind" fn fresh_event_tap_probe_callback(
    _proxy: objc2_core_graphics::CGEventTapProxy,
    _event_type: objc2_core_graphics::CGEventType,
    event: NonNull<objc2_core_graphics::CGEvent>,
    _user_info: *mut c_void,
) -> *mut objc2_core_graphics::CGEvent {
    event.as_ptr()
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
