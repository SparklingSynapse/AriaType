use super::storage::CorrectionStore;
use std::path::Path;
use std::process::Command;

#[tauri::command]
pub fn clear_correction_memory() -> Result<(), String> {
    CorrectionStore::shared().clear()
}

#[tauri::command]
pub fn open_correction_memory_directory() -> Result<(), String> {
    let store = CorrectionStore::shared();
    store.ensure_file()?;
    let directory = store
        .path()
        .parent()
        .ok_or_else(|| "correction memory directory is unavailable".to_string())?;
    open_directory(directory)
}

fn open_directory(path: &Path) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(path)
            .spawn()
            .map(|_| ())
            .map_err(|error| format!("failed to open correction memory directory: {error}"))
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", ""])
            .arg(path)
            .spawn()
            .map(|_| ())
            .map_err(|error| format!("failed to open correction memory directory: {error}"))
    }

    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map(|_| ())
            .map_err(|error| format!("failed to open correction memory directory: {error}"))
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        let _ = path;
        Err("opening correction memory directory is not supported on this platform".to_string())
    }
}
