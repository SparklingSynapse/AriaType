use crate::audio::recorder::AudioRecorder;
use crate::utils::AppPaths;
use std::process::Command;

#[tauri::command]
pub fn get_audio_devices() -> Vec<String> {
    let mut devices = vec!["default".to_string()];
    devices.extend(AudioRecorder::get_devices());
    devices
}

#[tauri::command]
pub fn get_log_content(lines: usize) -> String {
    let log_dir = AppPaths::log_dir();
    // tracing-appender hourly names: "ariatype.log.YYYY-MM-DDTHH"
    // Collect all matching files, sort, read the most recent one
    let Ok(entries) = std::fs::read_dir(&log_dir) else {
        return String::new();
    };
    let mut log_files: Vec<std::path::PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("ariatype.log"))
                    .unwrap_or(false)
        })
        .collect();
    log_files.sort();

    // Read last N lines across the most recent file(s)
    let mut all_lines: Vec<String> = Vec::new();
    for path in log_files.iter().rev() {
        if let Ok(content) = std::fs::read_to_string(path) {
            let mut file_lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
            file_lines.extend(all_lines);
            all_lines = file_lines;
        }
        if all_lines.len() >= lines * 2 {
            break;
        }
    }
    let start = all_lines.len().saturating_sub(lines);
    all_lines[start..].join("\n")
}

#[tauri::command]
pub fn open_log_folder() -> Result<(), String> {
    let log_dir = AppPaths::log_dir();
    std::fs::create_dir_all(&log_dir).ok();
    #[cfg(target_os = "macos")]
    Command::new("open")
        .arg(&log_dir)
        .spawn()
        .map_err(|e| e.to_string())?;
    #[cfg(target_os = "windows")]
    Command::new("explorer")
        .arg(&log_dir)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_platform() -> String {
    #[cfg(target_os = "macos")]
    return "macos".to_string();
    #[cfg(target_os = "windows")]
    return "windows".to_string();
    #[cfg(target_os = "linux")]
    return "linux".to_string();
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    return "unknown".to_string();
}
