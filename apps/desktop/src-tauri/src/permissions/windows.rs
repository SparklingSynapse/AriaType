use std::process::Command;

use super::{PermissionProvider, PermissionStatus};

pub struct WindowsPermissions;

impl PermissionProvider for WindowsPermissions {
    fn check_accessibility(&self) -> PermissionStatus {
        PermissionStatus::Granted
    }

    fn check_input_monitoring(&self) -> PermissionStatus {
        PermissionStatus::Granted
    }

    fn check_microphone(&self) -> PermissionStatus {
        let host = cpal::default_host();
        match host.default_input_device() {
            Some(_) => PermissionStatus::Granted,
            None => PermissionStatus::NotDetermined,
        }
    }

    fn apply_accessibility(&self) -> Result<(), String> {
        Command::new("cmd")
            .args(["/c", "start", "ms-settings:easeofaccess"])
            .spawn()
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    fn apply_input_monitoring(&self) -> Result<(), String> {
        Ok(())
    }

    fn apply_microphone(&self) -> Result<(), String> {
        Command::new("cmd")
            .args(["/c", "start", "ms-settings:privacy-microphone"])
            .spawn()
            .map_err(|error| error.to_string())?;
        Ok(())
    }
}
