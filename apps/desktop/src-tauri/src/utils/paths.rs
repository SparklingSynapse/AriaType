use std::path::PathBuf;
use std::sync::OnceLock;

/// Application data directory, initialized from Tauri's PathResolver at startup.
/// Falls back to "ariatype" if not initialized (for tests or early bootstrapping).
static APP_DATA_DIR: OnceLock<PathBuf> = OnceLock::new();

pub struct AppPaths;

impl AppPaths {
    /// Initialize from Tauri's PathResolver. Called once during app setup.
    pub fn init_from_tauri(data_dir: PathBuf) {
        let _ = APP_DATA_DIR.set(data_dir);
    }

    pub fn data_dir() -> PathBuf {
        APP_DATA_DIR
            .get()
            .cloned()
            .unwrap_or_else(|| dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")).join("ariatype"))
    }

    pub fn models_dir() -> PathBuf {
        Self::data_dir().join("models")
    }

    pub fn recordings_dir() -> PathBuf {
        Self::data_dir().join("recordings")
    }

    pub fn cache_dir() -> PathBuf {
        APP_DATA_DIR
            .get()
            .cloned()
            .unwrap_or_else(|| dirs::cache_dir().unwrap_or_else(|| PathBuf::from(".")).join("ariatype"))
    }

    pub fn temp_dir() -> PathBuf {
        let path = Self::cache_dir().join("temp");
        if let Err(e) = std::fs::create_dir_all(&path) {
            tracing::warn!(error = %e, path = ?path, "temp_directory_creation_failed");
        }
        path
    }

    pub fn log_dir() -> PathBuf {
        #[cfg(target_os = "macos")]
        {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("Library/Logs")
                .join(Self::data_dir().file_name().unwrap_or_default())
        }
        #[cfg(target_os = "windows")]
        {
            Self::data_dir().join("logs")
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            Self::data_dir().join("logs")
        }
    }

    pub fn ensure_dirs() {
        if let Err(e) = std::fs::create_dir_all(Self::data_dir()) {
            tracing::warn!(error = %e, "data_directory_creation_failed");
        }
        if let Err(e) = std::fs::create_dir_all(Self::models_dir()) {
            tracing::warn!(error = %e, "models_directory_creation_failed");
        }
        if let Err(e) = std::fs::create_dir_all(Self::recordings_dir()) {
            tracing::warn!(error = %e, "recordings_directory_creation_failed");
        }
        if let Err(e) = std::fs::create_dir_all(Self::cache_dir()) {
            tracing::warn!(error = %e, "cache_directory_creation_failed");
        }
        if let Err(e) = std::fs::create_dir_all(Self::temp_dir()) {
            tracing::warn!(error = %e, "temp_directory_creation_failed");
        }
        if let Err(e) = std::fs::create_dir_all(Self::log_dir()) {
            tracing::warn!(error = %e, "log_directory_creation_failed");
        }
    }

    pub fn cleanup_temp_dir(max_age_secs: u64) {
        let temp_dir = Self::temp_dir();
        let Ok(entries) = std::fs::read_dir(&temp_dir) else {
            return;
        };

        let cutoff = std::time::SystemTime::now() - std::time::Duration::from_secs(max_age_secs);

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            if let Ok(meta) = std::fs::metadata(&path) {
                if let Ok(modified) = meta.modified() {
                    if modified < cutoff {
                        if let Err(e) = std::fs::remove_file(&path) {
                            tracing::debug!(error = %e, path = ?path, "stale_temp_file_removal_failed");
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paths_fallback_without_init() {
        // Without init, falls back to "ariatype"
        let data = AppPaths::data_dir();
        assert!(data.ends_with("ariatype"));

        let models = AppPaths::models_dir();
        assert!(models.ends_with("models"));
        assert!(models.starts_with(&data));

        let recordings = AppPaths::recordings_dir();
        assert!(recordings.ends_with("recordings"));
        assert!(recordings.starts_with(&data));

        let temp = AppPaths::temp_dir();
        assert!(temp.ends_with("temp"));
    }
}
