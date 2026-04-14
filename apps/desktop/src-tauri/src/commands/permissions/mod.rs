use crate::permissions::{self, PermissionKind};

/// Check the current status of a permission.
///
/// `kind`: "accessibility" | "input_monitoring" | "microphone"
/// Returns: "granted" | "denied" | "not_determined"
#[tauri::command]
pub fn check_permission(kind: String) -> String {
    PermissionKind::parse(&kind)
        .map(permissions::check_permission)
        .map(|status| status.as_str().to_string())
        .unwrap_or_else(|| {
            permissions::PermissionStatus::NotDetermined
                .as_str()
                .to_string()
        })
}

/// Apply (request or open settings for) a permission.
///
/// For microphone on macOS when status is "not_determined": shows the system dialog.
/// Otherwise: opens the relevant system settings page.
#[tauri::command]
pub async fn apply_permission(kind: String) -> Result<(), String> {
    let Some(kind) = PermissionKind::parse(&kind) else {
        return Err(format!("Unknown permission kind: {}", kind));
    };

    permissions::apply_permission(kind)
}
