use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::sync::{LazyLock, Mutex};
use std::time::Instant;
use tracing::{debug, error, info, warn};

const PROGRESS_LOG_THRESHOLD_PERCENT: u32 = 10;
const DOWNLOAD_SLOT_WAIT_MS: u64 = 250;
const COMPLETION_MARKER_SUFFIX: &str = ".download-complete.json";

static ACTIVE_DOWNLOAD_PATHS: LazyLock<Mutex<HashSet<PathBuf>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

pub struct DownloadResult {
    pub path: PathBuf,
    pub bytes: u64,
}

#[derive(Debug, Deserialize, Serialize)]
struct DownloadCompletionMarker {
    version: u8,
    bytes: u64,
    expected_bytes: Option<u64>,
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
    /// Conservative lower bound used to reject partial legacy files
    pub minimum_bytes: Option<u64>,
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
            minimum_bytes: None,
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

    /// Set a conservative lower bound for a valid completed file
    pub fn with_minimum_bytes(mut self, bytes: u64) -> Self {
        self.minimum_bytes = Some(bytes);
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

    let _slot = acquire_download_slot(&options.output_path, &options).await?;
    if options.output_path.exists() {
        let bytes = options
            .output_path
            .metadata()
            .map(|metadata| metadata.len())
            .unwrap_or(0);
        if downloaded_file_is_complete(&options.output_path, options.minimum_bytes) {
            if let Some(callback) = &options.progress_callback {
                callback(bytes, bytes);
            }
            info!(
                model = model_name,
                filename = %filename,
                output_path = ?options.output_path,
                bytes,
                "download_skipped-existing_file_after_wait"
            );
            return Ok(DownloadResult {
                path: options.output_path,
                bytes,
            });
        }

        warn!(
            model = model_name,
            filename = %filename,
            output_path = ?options.output_path,
            bytes,
            minimum_bytes = ?options.minimum_bytes,
            "download_existing_file_incomplete-redownloading"
        );
        remove_download_artifacts(&options.output_path);
    }

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
            options.minimum_bytes,
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

struct DownloadSlot {
    path: PathBuf,
}

impl Drop for DownloadSlot {
    fn drop(&mut self) {
        if let Ok(mut paths) = ACTIVE_DOWNLOAD_PATHS.lock() {
            paths.remove(&self.path);
        }
    }
}

async fn acquire_download_slot(
    output_path: &Path,
    options: &DownloadOptions,
) -> Result<DownloadSlot, String> {
    let path = output_path.to_path_buf();
    let model_name = options.model_name.as_deref().unwrap_or("unknown");
    let mut logged_wait = false;

    loop {
        if options.is_cancelled() {
            return Err("cancelled".to_string());
        }

        let acquired = {
            let mut paths = ACTIVE_DOWNLOAD_PATHS
                .lock()
                .map_err(|_| "download slot lock poisoned".to_string())?;
            paths.insert(path.clone())
        };

        if acquired {
            return Ok(DownloadSlot { path });
        }

        if !logged_wait {
            info!(
                model = model_name,
                output_path = ?path,
                "download_waiting_for_active_path"
            );
            logged_wait = true;
        }

        tokio::time::sleep(std::time::Duration::from_millis(DOWNLOAD_SLOT_WAIT_MS)).await;
    }
}

/// Download from a single URL
async fn download_single(
    url: &str,
    output_path: &Path,
    cancel_flag: Option<&Arc<AtomicBool>>,
    progress_callback: Option<&ProgressCallback>,
    minimum_bytes: Option<u64>,
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
    if let Some(marker_path) = completion_marker_path(output_path) {
        remove_file_if_exists(&marker_path, "completion_marker_removal_failed");
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

    if total_size > 0 && downloaded != total_size {
        warn!(
            filename = %filename,
            downloaded_bytes = downloaded,
            expected_bytes = total_size,
            "download_incomplete_content_length"
        );
        remove_file_if_exists(&tmp_path, "temp_file_cleanup_failed");
        return Err(format!(
            "Download incomplete: expected {} bytes, got {} bytes",
            total_size, downloaded
        ));
    }

    if let Some(minimum_bytes) = minimum_bytes {
        if downloaded < minimum_bytes {
            warn!(
                filename = %filename,
                downloaded_bytes = downloaded,
                minimum_bytes,
                "download_incomplete_minimum_size"
            );
            remove_file_if_exists(&tmp_path, "temp_file_cleanup_failed");
            return Err(format!(
                "Download incomplete: expected at least {} bytes, got {} bytes",
                minimum_bytes, downloaded
            ));
        }
    }

    file.flush().map_err(|e| {
        error!(tmp_path = ?tmp_path, error = %e, "download_flush_failed");
        let message = format!("Failed to flush temp file: {}", e);
        remove_file_if_exists(&tmp_path, "temp_file_cleanup_failed");
        message
    })?;
    file.sync_all().map_err(|e| {
        error!(tmp_path = ?tmp_path, error = %e, "download_sync_failed");
        let message = format!("Failed to sync temp file: {}", e);
        remove_file_if_exists(&tmp_path, "temp_file_cleanup_failed");
        message
    })?;
    drop(file);

    std::fs::rename(&tmp_path, output_path).map_err(|e| {
        error!(
            tmp_path = ?tmp_path,
            output_path = ?output_path,
            error = %e,
            "download_finalize_failed"
        );
        format!("Failed to finalize file: {}", e)
    })?;

    if let Err(e) = write_completion_marker(
        output_path,
        downloaded,
        (total_size > 0).then_some(total_size),
    ) {
        warn!(
            filename = %filename,
            output_path = ?output_path,
            error = %e,
            "download_completion_marker_write_failed"
        );
    }

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

pub fn downloaded_file_is_complete(output_path: &Path, minimum_bytes: Option<u64>) -> bool {
    let metadata = match output_path.metadata() {
        Ok(metadata) if metadata.is_file() => metadata,
        _ => return false,
    };

    let bytes = metadata.len();
    if bytes == 0 {
        return false;
    }

    if let Some(minimum_bytes) = minimum_bytes {
        if bytes < minimum_bytes {
            return false;
        }
    }

    let Some(marker_path) = completion_marker_path(output_path) else {
        return true;
    };

    if !marker_path.exists() {
        return true;
    }

    let marker = match read_completion_marker(&marker_path) {
        Ok(marker) => marker,
        Err(e) => {
            warn!(
                marker_path = ?marker_path,
                output_path = ?output_path,
                error = %e,
                "download_completion_marker_invalid"
            );
            return false;
        }
    };

    if marker.bytes != bytes {
        warn!(
            marker_path = ?marker_path,
            output_path = ?output_path,
            marker_bytes = marker.bytes,
            actual_bytes = bytes,
            "download_completion_marker_size_mismatch"
        );
        return false;
    }

    if let Some(expected_bytes) = marker.expected_bytes.filter(|bytes| *bytes > 0) {
        if expected_bytes != bytes {
            warn!(
                marker_path = ?marker_path,
                output_path = ?output_path,
                expected_bytes,
                actual_bytes = bytes,
                "download_completion_marker_expected_size_mismatch"
            );
            return false;
        }
    }

    true
}

pub fn remove_download_artifacts(output_path: &Path) {
    remove_file_if_exists(output_path, "partial_download_cleanup_failed");
    if let Some(parent) = output_path.parent() {
        let filename = output_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();
        let tmp_path = parent.join(format!("{}.tmp", filename));
        remove_file_if_exists(&tmp_path, "temp_file_cleanup_failed");
    }
    if let Some(marker_path) = completion_marker_path(output_path) {
        remove_file_if_exists(&marker_path, "completion_marker_cleanup_failed");
    }
}

fn cleanup_partial_download(output_path: &Path) {
    remove_download_artifacts(output_path);
}

fn completion_marker_path(output_path: &Path) -> Option<PathBuf> {
    let parent = output_path.parent()?;
    let filename = output_path.file_name()?.to_string_lossy();
    Some(parent.join(format!("{filename}{COMPLETION_MARKER_SUFFIX}")))
}

fn read_completion_marker(path: &Path) -> Result<DownloadCompletionMarker, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("Failed to read marker: {}", e))?;
    serde_json::from_slice(&bytes).map_err(|e| format!("Failed to parse marker: {}", e))
}

fn write_completion_marker(
    output_path: &Path,
    bytes: u64,
    expected_bytes: Option<u64>,
) -> Result<(), String> {
    let marker_path = completion_marker_path(output_path)
        .ok_or_else(|| "Invalid output path: no marker parent".to_string())?;
    let marker = DownloadCompletionMarker {
        version: 1,
        bytes,
        expected_bytes,
    };
    let payload = serde_json::to_vec_pretty(&marker)
        .map_err(|e| format!("Failed to serialize marker: {}", e))?;
    std::fs::write(&marker_path, payload).map_err(|e| format!("Failed to write marker: {}", e))
}

fn remove_file_if_exists(path: &Path, event: &'static str) {
    match std::fs::remove_file(path) {
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => {
            debug!(
                error = %e,
                path = ?path,
                cleanup_event = event,
                "download_artifact_cleanup_failed"
            );
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

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tempfile::TempDir;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::{download, downloaded_file_is_complete, DownloadOptions};

    #[tokio::test]
    async fn concurrent_downloads_to_same_path_reuse_completed_file() {
        let mock_server = MockServer::start().await;
        let body = b"downloaded model bytes";

        Mock::given(method("GET"))
            .and(path("/model.bin"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_bytes(body.as_slice())
                    .set_delay(Duration::from_millis(100)),
            )
            .expect(1)
            .mount(&mock_server)
            .await;

        let dir = TempDir::new().unwrap();
        let output_path = dir.path().join("model.bin");
        let url = format!("{}/model.bin", mock_server.uri());

        let first = download(DownloadOptions::new(url.clone(), output_path.clone()));
        let second = download(DownloadOptions::new(url, output_path.clone()));

        let (first, second) = tokio::join!(first, second);

        assert_eq!(first.unwrap().path, output_path);
        assert_eq!(second.unwrap().path, output_path);
        assert_eq!(std::fs::read(&output_path).unwrap(), body);
        assert!(downloaded_file_is_complete(
            &output_path,
            Some(body.len() as u64)
        ));
    }

    #[tokio::test]
    async fn incomplete_existing_file_is_redownloaded() {
        let mock_server = MockServer::start().await;
        let body = b"complete downloaded model bytes";

        Mock::given(method("GET"))
            .and(path("/model.bin"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(body.as_slice()))
            .expect(1)
            .mount(&mock_server)
            .await;

        let dir = TempDir::new().unwrap();
        let output_path = dir.path().join("model.bin");
        std::fs::write(&output_path, b"partial").unwrap();

        let url = format!("{}/model.bin", mock_server.uri());
        let result = download(
            DownloadOptions::new(url, output_path.clone()).with_minimum_bytes(body.len() as u64),
        )
        .await
        .unwrap();

        assert_eq!(result.path, output_path);
        assert_eq!(std::fs::read(output_path).unwrap(), body);
    }
}
