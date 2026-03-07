/// Platform-agnostic permission interface.
pub trait PermissionProvider: Send + Sync {
    /// Returns the current status: "granted" | "denied" | "not_determined"
    fn check_accessibility(&self) -> String;
    fn check_input_monitoring(&self) -> String;
    fn check_microphone(&self) -> String;

    /// Opens system settings or shows the permission dialog.
    fn apply_accessibility(&self) -> Result<(), String>;
    fn apply_input_monitoring(&self) -> Result<(), String>;
    /// Async because macOS microphone dialog requires waiting for user response.
    fn apply_microphone(&self) -> Result<(), String>;
}

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

fn provider() -> Box<dyn PermissionProvider> {
    #[cfg(target_os = "macos")]
    return Box::new(macos::MacosPermissions);
    #[cfg(target_os = "windows")]
    return Box::new(windows::WindowsPermissions);
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    compile_error!("permissions: unsupported platform");
}

/// Check the current status of a permission.
///
/// `kind`: "accessibility" | "input_monitoring" | "microphone"
/// Returns: "granted" | "denied" | "not_determined"
#[tauri::command]
pub fn check_permission(kind: String) -> String {
    let p = provider();
    match kind.as_str() {
        "accessibility" => p.check_accessibility(),
        "input_monitoring" => p.check_input_monitoring(),
        "microphone" => p.check_microphone(),
        _ => "not_determined".to_string(),
    }
}

/// Apply (request or open settings for) a permission.
///
/// For microphone on macOS when status is "not_determined": shows the system dialog.
/// Otherwise: opens the relevant system settings page.
#[tauri::command]
pub async fn apply_permission(kind: String) -> Result<(), String> {
    let p = provider();
    match kind.as_str() {
        "accessibility" => p.apply_accessibility(),
        "input_monitoring" => p.apply_input_monitoring(),
        "microphone" => p.apply_microphone(),
        _ => Err(format!("Unknown permission kind: {}", kind)),
    }
}
