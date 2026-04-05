use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

#[test]
fn test_downloader_invalid_url() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let result = runtime.block_on(async {
        let url = "http://invalid-domain-that-does-not-exist-12345.com/file.bin";
        let output_path = PathBuf::from("/tmp/test_download_invalid.bin");
        let cancel_flag = Arc::new(AtomicBool::new(false));

        ariatype_lib::utils::downloader::download_file(url, &output_path, cancel_flag, |_, _| {})
            .await
    });

    assert!(result.is_err(), "Download should fail for invalid URL");
}

#[test]
fn test_downloader_cancel_flag() {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    let cancel_flag = Arc::new(AtomicBool::new(true));

    let result = runtime.block_on(async {
        let url = "http://example.com/file.bin";
        let output_path = PathBuf::from("/tmp/test_download_cancel.bin");

        ariatype_lib::utils::downloader::download_file(url, &output_path, cancel_flag, |_, _| {})
            .await
    });

    assert!(result.is_err(), "Download should fail when cancelled");
}

#[test]
fn test_downloader_progress_callback() {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    let progress_calls = Arc::new(std::sync::Mutex::new(Vec::new()));
    let progress_calls_clone = progress_calls.clone();

    let progress_callback = move |downloaded: u64, total: u64| {
        progress_calls_clone
            .lock()
            .unwrap()
            .push((downloaded, total));
    };

    let cancel_flag = Arc::new(AtomicBool::new(false));

    let result = runtime.block_on(async {
        let url = "https://httpbin.org/bytes/1024";
        let output_path = PathBuf::from("/tmp/test_download_progress.bin");

        ariatype_lib::utils::downloader::download_file(
            url,
            &output_path,
            cancel_flag,
            progress_callback,
        )
        .await
    });

    if result.is_ok() {
        let calls = progress_calls.lock().unwrap();
        assert!(
            !calls.is_empty(),
            "Progress callback should be called at least once"
        );
    }
}

#[test]
fn test_downloader_output_path_handling() {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    let result = runtime.block_on(async {
        let url = "https://httpbin.org/bytes/100";
        let output_path = PathBuf::from("/tmp/nonexistent_dir/test.bin");
        let cancel_flag = Arc::new(AtomicBool::new(false));

        ariatype_lib::utils::downloader::download_file(url, &output_path, cancel_flag, |_, _| {})
            .await
    });

    if result.is_ok() {
        let path = result.unwrap();
        assert!(path.exists(), "Downloaded file should exist");
        let _ = std::fs::remove_file(path);
    }
}
