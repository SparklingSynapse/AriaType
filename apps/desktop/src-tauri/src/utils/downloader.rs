use futures_util::StreamExt;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Instant;
use tracing::{debug, error, info, warn};

const PROGRESS_LOG_THRESHOLD_PERCENT: u32 = 10;

pub struct DownloadResult {
    pub path: PathBuf,
    pub bytes: u64,
}

/// Progress callback type for download operations
pub type ProgressCallback = Arc<dyn Fn(u64, u64) + Send + Sync>;

/// Download options with support for fallback URLs and cancellation
pub struct DownloadOptions {
    /// Primary and fallback URLs (tried in order)
    pub urls: Vec<String>,
    /// Output file path
    pub output_path: PathBuf,
    /// Optional cancellation flag
    pub cancel_flag: Option<Arc<AtomicBool>>,
    /// Optional progress callback (downloaded bytes, total bytes)
    pub progress_callback: Option<ProgressCallback>,
    /// Model/display name for logging (optional)
    pub model_name: Option<String>,
}

impl DownloadOptions {
    /// Create download options with a single URL
    pub fn new(url: impl Into<String>, output_path: impl Into<PathBuf>) -> Self {
        Self {
            urls: vec![url.into()],
            output_path: output_path.into(),
            cancel_flag: None,
            progress_callback: None,
            model_name: None,
        }
    }

    /// Add fallback URLs
    pub fn with_fallbacks(mut self, fallback_urls: Vec<String>) -> Self {
        self.urls.extend(fallback_urls);
        self
    }

    /// Set cancellation flag
    pub fn with_cancel_flag(mut self, flag: Arc<AtomicBool>) -> Self {
        self.cancel_flag = Some(flag);
        self
    }

    /// Set progress callback
    pub fn with_progress_callback(mut self, callback: ProgressCallback) -> Self {
        self.progress_callback = Some(callback);
        self
    }

    /// Set model name for logging
    pub fn with_model_name(mut self, name: impl Into<String>) -> Self {
        self.model_name = Some(name.into());
        self
    }

    /// Check if cancelled
    fn is_cancelled(&self) -> bool {
        self.cancel_flag
            .as_ref()
            .map(|f| f.load(Ordering::Relaxed))
            .unwrap_or(false)
    }
}

/// Download a file with automatic fallback support
pub async fn download(options: DownloadOptions) -> Result<DownloadResult, String> {
    if options.urls.is_empty() {
        return Err("No download URLs provided".to_string());
    }

    if options.is_cancelled() {
        return Err("cancelled".to_string());
    }

    let model_name = options.model_name.as_deref().unwrap_or("unknown");
    let filename = options
        .output_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    info!(
        model = model_name,
        filename = %filename,
        output_path = ?options.output_path,
        source_count = options.urls.len(),
        "download_started"
    );

    let mut last_error = String::new();

    for (attempt, url) in options.urls.iter().enumerate() {
        let attempt_num = attempt + 1;

        if options.is_cancelled() {
            info!(model = model_name, "download_cancelled_before_source");
            return Err("cancelled".to_string());
        }

        info!(
            model = model_name,
            attempt = attempt_num,
            total_sources = options.urls.len(),
            url = url,
            "download_source_attempt"
        );

        match download_single(
            url,
            &options.output_path,
            options.cancel_flag.as_ref(),
            options.progress_callback.as_ref(),
            model_name,
        )
        .await
        {
            Ok(result) => {
                info!(
                    model = model_name,
                    attempt = attempt_num,
                    output_path = ?result.path,
                    bytes = result.bytes,
                    "download_completed"
                );
                return Ok(result);
            }
            Err(e) => {
                if e == "cancelled" {
                    return Err(e);
                }
                warn!(
                    model = model_name,
                    attempt = attempt_num,
                    url = url,
                    error = %e,
                    "download_source_failed"
                );
                last_error = e;
                cleanup_partial_download(&options.output_path);
            }
        }
    }

    error!(
        model = model_name,
        attempts = options.urls.len(),
        last_error = %last_error,
        "download_all_sources_failed"
    );
    Err(format!(
        "All download sources failed. Last error: {}",
        last_error
    ))
}

/// Download from a single URL
async fn download_single(
    url: &str,
    output_path: &Path,
    cancel_flag: Option<&Arc<AtomicBool>>,
    progress_callback: Option<&ProgressCallback>,
    _model_name: &str,
) -> Result<DownloadResult, String> {
    let start_time = Instant::now();
    let filename = output_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            error!(path = ?parent, error = %e, "download_directory_creation_failed");
            format!("Failed to create directory: {}", e)
        })?;
    }

    let tmp_path = output_path
        .parent()
        .map(|p| p.join(format!("{}.tmp", filename)))
        .ok_or_else(|| "Invalid output path: no parent directory".to_string())?;

    if tmp_path.exists() {
        info!(tmp_path = ?tmp_path, "temp_file_removing");
        if let Err(e) = std::fs::remove_file(&tmp_path) {
            debug!(error = %e, path = ?tmp_path, "temp_file_removal_failed");
        }
    }

    let client = reqwest::Client::new();
    let response = client.get(url).send().await.map_err(|e| {
        warn!(url = url, error = %e, "download_request_failed");
        format!("Download request failed: {}", e)
    })?;

    if !response.status().is_success() {
        warn!(
            url = url,
            status = %response.status(),
            status_code = response.status().as_u16(),
            "download_failed_http_error"
        );
        return Err(format!(
            "Download failed with status: {}",
            response.status()
        ));
    }

    let total_size = response.content_length().unwrap_or(0);
    let total_size_mb = total_size as f64 / 1_048_576.0;

    info!(
        url = url,
        filename = %filename,
        total_bytes = total_size,
        total_mb = format!("{:.2}", total_size_mb),
        "download_response_received"
    );

    let mut file = std::fs::File::create(&tmp_path).map_err(|e| {
        error!(tmp_path = ?tmp_path, error = %e, "temp_file_creation_failed");
        format!("Failed to create temp file: {}", e)
    })?;

    let mut stream = response.bytes_stream();
    let mut downloaded = 0u64;
    let mut last_logged_percent = 0u32;

    let result = async {
        while let Some(chunk) = stream.next().await {
            if cancel_flag.map(|f| f.load(Ordering::Relaxed)).unwrap_or(false) {
                info!(
                    filename = %filename,
                    downloaded_mb = format!("{:.2}", downloaded as f64 / 1_048_576.0),
                    "download_cancelled_by_user"
                );
                return Err("cancelled".to_string());
            }

            let chunk = chunk.map_err(|e| {
                warn!(url = url, downloaded_bytes = downloaded, error = %e, "download_stream_error");
                format!("Download error: {}", e)
            })?;

            file.write_all(&chunk).map_err(|e| {
                error!(tmp_path = ?tmp_path, error = %e, "download_write_error");
                format!("Write error: {}", e)
            })?;

            downloaded += chunk.len() as u64;

            if let Some(cb) = progress_callback {
                cb(downloaded, total_size);
            }

            if total_size > 0 {
                let current_percent = (downloaded as f64 / total_size as f64 * 100.0) as u32;
                if current_percent >= last_logged_percent + PROGRESS_LOG_THRESHOLD_PERCENT {
                    last_logged_percent =
                        (current_percent / PROGRESS_LOG_THRESHOLD_PERCENT) * PROGRESS_LOG_THRESHOLD_PERCENT;
                    let elapsed = start_time.elapsed();
                    let speed_mbps = if elapsed.as_secs() > 0 {
                        (downloaded as f64 / 1_048_576.0) / elapsed.as_secs_f64()
                    } else {
                        0.0
                    };
                    info!(
                        filename = %filename,
                        progress_percent = current_percent,
                        downloaded_mb = format!("{:.2}", downloaded as f64 / 1_048_576.0),
                        total_mb = format!("{:.2}", total_size_mb),
                        elapsed_secs = elapsed.as_secs(),
                        speed_mbps = format!("{:.2}", speed_mbps),
                        "download_progress"
                    );
                }
            }
        }
        Ok::<(), String>(())
    }
    .await;

    if let Err(e) = result {
        if let Err(e) = std::fs::remove_file(&tmp_path) {
            debug!(error = %e, path = ?tmp_path, "temp_file_cleanup_failed");
        }
        return Err(e);
    }

    std::fs::rename(&tmp_path, output_path).map_err(|e| {
        error!(
            tmp_path = ?tmp_path,
            output_path = ?output_path,
            error = %e,
            "download_finalize_failed"
        );
        format!("Failed to finalize file: {}", e)
    })?;

    let elapsed = start_time.elapsed();
    let avg_speed_mbps = if elapsed.as_secs() > 0 {
        (downloaded as f64 / 1_048_576.0) / elapsed.as_secs_f64()
    } else {
        0.0
    };

    info!(
        filename = %filename,
        total_bytes = downloaded,
        total_mb = format!("{:.2}", downloaded as f64 / 1_048_576.0),
        elapsed_secs = elapsed.as_secs(),
        elapsed_ms = elapsed.as_millis(),
        avg_speed_mbps = format!("{:.2}", avg_speed_mbps),
        output_path = ?output_path,
        "download_completed_successfully"
    );

    Ok(DownloadResult {
        path: output_path.to_path_buf(),
        bytes: downloaded,
    })
}

fn cleanup_partial_download(output_path: &Path) {
    if let Err(e) = std::fs::remove_file(output_path) {
        debug!(error = %e, path = ?output_path, "partial_download_cleanup_failed");
    }
    if let Some(parent) = output_path.parent() {
        let filename = output_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();
        let tmp_path = parent.join(format!("{}.tmp", filename));
        if let Err(e) = std::fs::remove_file(&tmp_path) {
            debug!(error = %e, path = ?tmp_path, "temp_file_cleanup_failed");
        }
    }
}

/// Download a single file (legacy API for backwards compatibility)
pub async fn download_file<F>(
    url: &str,
    output_path: &Path,
    cancel_flag: Arc<AtomicBool>,
    progress_callback: F,
) -> Result<PathBuf, String>
where
    F: Fn(u64, u64) + Send + Sync + 'static,
{
    let options = DownloadOptions::new(url, output_path)
        .with_cancel_flag(cancel_flag)
        .with_progress_callback(Arc::new(progress_callback));

    download(options).await.map(|r| r.path)
}
