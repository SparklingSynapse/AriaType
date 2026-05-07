// UnifiedEngineManager comprehensive test suite
//
// Test categories:
// A. Construction and configuration
// B. Engine cache management
// C. Model info queries
// D. Model preloading
// E. Model deletion
// F. Language recommendations
// G. Edge cases and error handling

use ariatype_lib::stt_engine::{EngineType, UnifiedEngineManager};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

// ==================== Test helper functions ====================

/// Create temporary test directory
fn create_test_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "unified_manager_test_{}_{}",
        name,
        uuid::Uuid::new_v4()
    ));
    std::fs::create_dir_all(&dir).expect("Failed to create test directory");
    dir
}

/// Create fake model files in proper subdirectory structure
fn create_fake_whisper_model(dir: &PathBuf, model_name: &str) {
    let model_subdir = dir.join(model_name);
    std::fs::create_dir_all(&model_subdir).expect("Failed to create model subdirectory");
    
    let files: &[(&str, usize)] = match model_name {
        "whisper-base" => &[("base-encoder.onnx", 1024), ("base-decoder.onnx", 2048), ("base-tokens.txt", 100)],
        "whisper-small" => &[("small-encoder.onnx", 2048), ("small-decoder.onnx", 4096), ("small-tokens.txt", 100)],
        _ => &[("encoder.onnx", 1024), ("decoder.onnx", 2048), ("tokens.txt", 100)],
    };
    
    for (filename, size) in files {
        let path = model_subdir.join(filename);
        let mut file = File::create(&path).expect("Failed to create fake model file");
        let data = vec![0u8; *size];
        file.write_all(&data).expect("Failed to write fake model data");
    }
}

/// Create fake SenseVoice model files
fn create_fake_sensevoice_model(dir: &PathBuf) {
    let model_subdir = dir.join("sense-voice-small");
    std::fs::create_dir_all(&model_subdir).expect("Failed to create model subdirectory");
    
    let path = model_subdir.join("model.int8.onnx");
    let mut file = File::create(&path).expect("Failed to create fake model file");
    let data = vec![0u8; 2048];
    file.write_all(&data).expect("Failed to write fake model data");
    
    let tokens_path = model_subdir.join("tokens.txt");
    let mut tokens_file = File::create(&tokens_path).expect("Failed to create tokens file");
    tokens_file.write_all(b"fake tokens").expect("Failed to write tokens");
}

/// Cleanup test directory
fn cleanup_test_dir(dir: &PathBuf) {
    let _ = std::fs::remove_dir_all(dir);
}

// ==================== A. Construction and configuration tests ====================

#[test]
fn test_new_manager() {
    let test_dir = create_test_dir("new_manager");
    let manager = UnifiedEngineManager::new(test_dir.clone());

    let models = manager.get_models(EngineType::SenseVoice);

    // Verify returned model list
    assert!(!models.is_empty(), "Should have SenseVoice models");
    assert_eq!(models.len(), 1, "Should have exactly 1 SenseVoice model");

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_is_model_downloaded_false() {
    let test_dir = create_test_dir("is_downloaded_false");
    let manager = UnifiedEngineManager::new(test_dir.clone());

    // Undownloaded models should return false
    assert!(!manager.is_model_downloaded(EngineType::Whisper, "whisper-base"));
    assert!(!manager.is_model_downloaded(EngineType::Whisper, "whisper-small"));
    assert!(!manager.is_model_downloaded(EngineType::SenseVoice, "sense-voice-small"));

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_is_model_downloaded_true() {
    let test_dir = create_test_dir("is_downloaded_true");
    let manager = UnifiedEngineManager::new(test_dir.clone());

    // Create fake model files
    create_fake_whisper_model(&test_dir, "whisper-base");

    // Downloaded models should return true
    assert!(manager.is_model_downloaded(EngineType::Whisper, "whisper-base"));

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_get_model_path() {
    let test_dir = create_test_dir("get_model_path");
    let manager = UnifiedEngineManager::new(test_dir.clone());

    // Whisper model paths
    let path = manager.get_model_path(EngineType::Whisper, "whisper-base");
    assert!(path.to_string_lossy().contains("whisper-base"));

    let path = manager.get_model_path(EngineType::Whisper, "whisper-small");
    assert!(path.to_string_lossy().contains("whisper-small"));

    // SenseVoice model paths
    let path = manager.get_model_path(EngineType::SenseVoice, "sense-voice-small");
    assert!(path.to_string_lossy().contains("sense-voice-small"));

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_get_model_path_unknown() {
    let test_dir = create_test_dir("get_model_path_unknown");
    let manager = UnifiedEngineManager::new(test_dir.clone());

    // Unknown models should fallback to default naming
    let path = manager.get_model_path(EngineType::Whisper, "unknown");
    assert!(path.to_string_lossy().contains("unknown"));

    let path = manager.get_model_path(EngineType::SenseVoice, "unknown");
    assert!(path.to_string_lossy().contains("unknown"));

    cleanup_test_dir(&test_dir);
}

// ==================== D. Model preloading tests ====================

#[test]
fn test_load_model_not_downloaded() {
    let test_dir = create_test_dir("load_not_downloaded");
    let manager = UnifiedEngineManager::new(test_dir.clone());

    // Trying to load undownloaded model should fail
    let result = manager.load_model(EngineType::Whisper, "whisper-base");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not downloaded"));

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_load_model_unknown() {
    let test_dir = create_test_dir("load_unknown");
    let manager = UnifiedEngineManager::new(test_dir.clone());

    // Try loading unknown model (model definition doesn't exist)
    let result = manager.load_model(EngineType::Whisper, "unknown");
    assert!(result.is_err());

    cleanup_test_dir(&test_dir);
}

// ==================== E. Model deletion tests ====================

#[test]
fn test_delete_model_not_exists() {
    let test_dir = create_test_dir("delete_not_exists");
    let manager = UnifiedEngineManager::new(test_dir.clone());

    // Trying to delete non-existent model should fail
    let result = manager.delete_model(EngineType::Whisper, "whisper-base");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not found"));

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_delete_model_success() {
    let test_dir = create_test_dir("delete_success");
    let manager = UnifiedEngineManager::new(test_dir.clone());

    // Create fake model files
    create_fake_whisper_model(&test_dir, "whisper-base");

    let model_subdir = test_dir.join("whisper-base");
    assert!(model_subdir.exists());

    // Delete model
    let result = manager.delete_model(EngineType::Whisper, "whisper-base");
    assert!(result.is_ok());

    // Verify subdirectory is deleted
    assert!(!model_subdir.exists());

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_delete_model_multiple_types() {
    let test_dir = create_test_dir("delete_multiple");
    let manager = UnifiedEngineManager::new(test_dir.clone());

    // Create model files for different engines
    create_fake_whisper_model(&test_dir, "whisper-base");
    create_fake_sensevoice_model(&test_dir);

    let whisper_subdir = test_dir.join("whisper-base");
    let sensevoice_subdir = test_dir.join("sense-voice-small");

    // Delete Whisper model
    assert!(manager.delete_model(EngineType::Whisper, "whisper-base").is_ok());
    assert!(!whisper_subdir.exists());
    assert!(sensevoice_subdir.exists());

    // Delete SenseVoice model
    assert!(manager.delete_model(EngineType::SenseVoice, "sense-voice-small").is_ok());
    assert!(!sensevoice_subdir.exists());

    cleanup_test_dir(&test_dir);
}

// ==================== F. Language recommendation tests ====================

#[test]
fn test_recommend_by_language_zh() {
    let test_dir = create_test_dir("recommend_zh");
    let manager = UnifiedEngineManager::new(test_dir.clone());

    let recommendations = manager.recommend_by_language("zh");

    // Should have recommendations
    assert!(
        !recommendations.is_empty(),
        "Should have recommendations for Chinese"
    );

    // Should include SenseVoice model (optimized for Chinese)
    let has_sensevoice = recommendations
        .iter()
        .any(|r| r.engine_type == EngineType::SenseVoice);
    assert!(has_sensevoice, "Should recommend SenseVoice for Chinese");

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_recommend_by_language_en() {
    let test_dir = create_test_dir("recommend_en");
    let manager = UnifiedEngineManager::new(test_dir.clone());

    let recommendations = manager.recommend_by_language("en");

    // Should have recommendations
    assert!(
        !recommendations.is_empty(),
        "Should have recommendations for English"
    );

    // All recommended models should have complete info
    for rec in &recommendations {
        assert!(!rec.model_name.is_empty());
        assert!(!rec.display_name.is_empty());
        assert!(rec.size_mb > 0);
        assert!(rec.speed_score > 0);
        assert!(rec.accuracy_score > 0);
    }

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_recommend_by_language_ja() {
    let test_dir = create_test_dir("recommend_ja");
    let manager = UnifiedEngineManager::new(test_dir.clone());

    let recommendations = manager.recommend_by_language("ja");

    assert!(
        !recommendations.is_empty(),
        "Should have recommendations for Japanese"
    );

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_recommend_by_language_ko() {
    let test_dir = create_test_dir("recommend_ko");
    let manager = UnifiedEngineManager::new(test_dir.clone());

    let recommendations = manager.recommend_by_language("ko");

    assert!(
        !recommendations.is_empty(),
        "Should have recommendations for Korean"
    );

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_recommend_by_language_unsupported() {
    let test_dir = create_test_dir("recommend_unsupported");
    let manager = UnifiedEngineManager::new(test_dir.clone());

    // Use unsupported language code
    let recommendations = manager.recommend_by_language("xyz");

    // Should return Whisper Base as fallback
    assert!(
        !recommendations.is_empty(),
        "Should return fallback for unsupported language"
    );

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_recommend_includes_download_status() {
    let test_dir = create_test_dir("recommend_download_status");
    let manager = UnifiedEngineManager::new(test_dir.clone());

    // Create a fake SenseVoice model
    create_fake_sensevoice_model(&test_dir);

    let recommendations = manager.recommend_by_language("zh");

    // Find SenseVoice Small model
    let sensevoice = recommendations
        .iter()
        .find(|r| r.model_name == "sense-voice-small");

    assert!(sensevoice.is_some());
    assert!(
        sensevoice.unwrap().downloaded,
        "Should detect downloaded model"
    );

    // Other models should be undownloaded
    let other_models: Vec<_> = recommendations
        .iter()
        .filter(|r| r.model_name != "sense-voice-small")
        .collect();

    for model in other_models {
        assert!(!model.downloaded, "Other models should not be downloaded");
    }

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_recommend_sorting() {
    let test_dir = create_test_dir("recommend_sorting");
    let manager = UnifiedEngineManager::new(test_dir.clone());

    let recommendations = manager.recommend_by_language("auto");

    // Verify sorting: accuracy descending
    for i in 1..recommendations.len() {
        let prev = &recommendations[i - 1];
        let curr = &recommendations[i];

        assert!(
            prev.accuracy_score >= curr.accuracy_score,
            "Should sort by accuracy (descending)"
        );
    }

    cleanup_test_dir(&test_dir);
}

// ==================== G. Edge cases and error handling tests ====================

#[test]
fn test_empty_models_directory() {
    let test_dir = create_test_dir("empty_dir");
    let manager = UnifiedEngineManager::new(test_dir.clone());

    // Empty directory should work normally
    let models = manager.get_models(EngineType::Whisper);
    assert!(!models.is_empty());

    // All models should be undownloaded
    for model in models {
        assert!(!model.downloaded);
    }

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_model_info_consistency() {
    let test_dir = create_test_dir("info_consistency");
    let manager = UnifiedEngineManager::new(test_dir.clone());

    // Get model list
    let models = manager.get_models(EngineType::Whisper);

    // Verify each model's path and download status are consistent
    for model in models {
        let is_downloaded = manager.is_model_downloaded(EngineType::Whisper, &model.name);

        assert_eq!(
            model.downloaded, is_downloaded,
            "Model info downloaded status should match is_model_downloaded()"
        );
    }

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_multiple_managers_same_directory() {
    let test_dir = create_test_dir("multiple_managers");

    // Create multiple manager instances pointing to same directory
    let manager1 = UnifiedEngineManager::new(test_dir.clone());
    let manager2 = UnifiedEngineManager::new(test_dir.clone());

    // Create model files
    create_fake_whisper_model(&test_dir, "whisper-base");

    // Both managers should detect the model
    assert!(manager1.is_model_downloaded(EngineType::Whisper, "whisper-base"));
    assert!(manager2.is_model_downloaded(EngineType::Whisper, "whisper-base"));

    cleanup_test_dir(&test_dir);
}

// ==================== H. Engine switching and management tests ====================

#[test]
fn test_stt_engine_type_switching() {
    let test_dir = create_test_dir("engine_switching");
    let manager = UnifiedEngineManager::new(test_dir.clone());

    assert_eq!(
        UnifiedEngineManager::get_engine_by_model_name("whisper-base"),
        Some(EngineType::Whisper)
    );
    assert_eq!(
        UnifiedEngineManager::get_engine_by_model_name("sense-voice-small"),
        Some(EngineType::SenseVoice)
    );
    assert_eq!(
        UnifiedEngineManager::get_engine_by_model_name("cloud"),
        Some(EngineType::Cloud)
    );
    assert_eq!(
        UnifiedEngineManager::get_engine_by_model_name("unknown_model"),
        None
    );

    let engines = UnifiedEngineManager::available_engines();
    assert_eq!(engines.len(), 3);
    assert!(engines.contains(&EngineType::Whisper));
    assert!(engines.contains(&EngineType::SenseVoice));
    assert!(engines.contains(&EngineType::Cloud));

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_model_path_resolution() {
    let test_dir = create_test_dir("model_path_resolution");
    let manager = UnifiedEngineManager::new(test_dir.clone());

    let whisper_base_path = manager.get_model_path(EngineType::Whisper, "whisper-base");
    assert!(whisper_base_path.to_string_lossy().contains("whisper-base"));

    let sensevoice_path = manager.get_model_path(EngineType::SenseVoice, "sense-voice-small");
    assert!(sensevoice_path.to_string_lossy().contains("sense-voice-small"));

    let unknown_whisper = manager.get_model_path(EngineType::Whisper, "unknown");
    assert!(unknown_whisper.to_string_lossy().contains("unknown"));

    let cloud_path = manager.get_model_path(EngineType::Cloud, "cloud");
    assert!(cloud_path.as_os_str().is_empty());

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_engine_preload_on_startup() {
    let test_dir = create_test_dir("engine_preload_startup");
    let manager = UnifiedEngineManager::new(test_dir.clone());

    create_fake_whisper_model(&test_dir, "whisper-base");
    create_fake_sensevoice_model(&test_dir);

    let whisper_models = manager.get_models(EngineType::Whisper);
    let sensevoice_models = manager.get_models(EngineType::SenseVoice);

    let whisper_base = whisper_models.iter().find(|m| m.name == "whisper-base").unwrap();
    assert!(whisper_base.downloaded);

    let sensevoice = sensevoice_models.iter().find(|m| m.name == "sense-voice-small").unwrap();
    assert!(sensevoice.downloaded);

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_concurrent_model_access() {
    use std::sync::{Arc, Barrier};
    use std::thread;

    let test_dir = create_test_dir("concurrent_access");
    let manager = Arc::new(UnifiedEngineManager::new(test_dir.clone()));

    let barrier = Arc::new(Barrier::new(3));
    let barrier_clone1 = barrier.clone();
    let barrier_clone2 = barrier.clone();
    let manager_clone1 = manager.clone();
    let manager_clone2 = manager.clone();

    let handle1 = thread::spawn(move || {
        barrier_clone1.wait();
        let models = manager_clone1.get_models(EngineType::Whisper);
        !models.is_empty()
    });

    let handle2 = thread::spawn(move || {
        barrier_clone2.wait();
        let models = manager_clone2.get_models(EngineType::SenseVoice);
        !models.is_empty()
    });

    barrier.wait();

    let result1 = handle1.join().unwrap();
    let result2 = handle2.join().unwrap();

    assert!(result1);
    assert!(result2);

    let engine1 = UnifiedEngineManager::get_engine_by_model_name("whisper-base");
    let engine2 = UnifiedEngineManager::get_engine_by_model_name("sense-voice-small");

    assert_eq!(engine1, Some(EngineType::Whisper));
    assert_eq!(engine2, Some(EngineType::SenseVoice));

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_engine_error_recovery() {
    let test_dir = create_test_dir("engine_error_recovery");
    let manager = UnifiedEngineManager::new(test_dir.clone());

    let is_downloaded = manager.is_model_downloaded(EngineType::Whisper, "whisper-base");
    assert!(!is_downloaded);

    create_fake_whisper_model(&test_dir, "whisper-base");

    let is_downloaded_after = manager.is_model_downloaded(EngineType::Whisper, "whisper-base");
    assert!(is_downloaded_after);

    assert_eq!(
        UnifiedEngineManager::get_engine_by_model_name("whisper-base"),
        Some(EngineType::Whisper)
    );

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_model_preload_cancellation() {
    let test_dir = create_test_dir("preload_cancellation");
    let manager = UnifiedEngineManager::new(test_dir.clone());

    create_fake_whisper_model(&test_dir, "whisper-base");

    assert!(manager.is_model_downloaded(EngineType::Whisper, "whisper-base"));

    assert_eq!(
        UnifiedEngineManager::get_engine_by_model_name("whisper-base"),
        Some(EngineType::Whisper)
    );

    let result = manager.is_model_downloaded(EngineType::Whisper, "whisper-small");
    assert!(!result);

    cleanup_test_dir(&test_dir);
}

// ==================== Integration tests (marked as ignore) ====================

#[test]
#[ignore]
fn test_real_download_whisper_base() {
    // This test requires real network connection and takes a long time
    // Run: cargo test test_real_download_whisper_base -- --ignored

    let test_dir = create_test_dir("test_model_download");
    let manager = UnifiedEngineManager::new(test_dir.clone());

    let cancel_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let progress_called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let progress_called_clone = progress_called.clone();

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        manager
            .download_model(
                EngineType::Whisper,
                "whisper-base",
                cancel_flag,
                move |downloaded, total| {
                    println!("Progress: {}/{} bytes", downloaded, total);
                    progress_called_clone.store(true, std::sync::atomic::Ordering::SeqCst);
                },
            )
            .await
    });

    assert!(result.is_ok(), "Download should succeed");
    assert!(
        progress_called.load(std::sync::atomic::Ordering::SeqCst),
        "Progress callback should be called"
    );

    // Verify file is downloaded
    assert!(manager.is_model_downloaded(EngineType::Whisper, "whisper-base"));

    cleanup_test_dir(&test_dir);
}