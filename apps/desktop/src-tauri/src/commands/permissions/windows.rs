use std::process::Command;

pub struct WindowsPermissions;

impl super::PermissionProvider for WindowsPermissions {
    fn check_accessibility(&self) -> String {
        // Windows does not restrict accessibility APIs the same way as macOS
        "granted".to_string()
    }

    fn check_input_monitoring(&self) -> String {
        // Not applicable on Windows
        "granted".to_string()
    }

    fn check_microphone(&self) -> String {
        let host = cpal::default_host();
        match host.default_input_device() {
            Some(_) => "granted".to_string(),
            None => "not_determined".to_string(),
        }
    }

    fn apply_accessibility(&self) -> Result<(), String> {
        Command::new("cmd")
            .args(["/c", "start", "ms-settings:easeofaccess"])
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn apply_input_monitoring(&self) -> Result<(), String> {
        Ok(()) // Not applicable on Windows
    }

    fn apply_microphone(&self) -> Result<(), String> {
        Command::new("cmd")
            .args(["/c", "start", "ms-settings:privacy-microphone"])
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}
