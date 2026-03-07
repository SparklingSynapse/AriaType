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

use ariatype_lib::stt_engine::{UnifiedEngineManager, EngineType};
use std::path::PathBuf;
use std::fs::File;
use std::io::Write;

// ==================== Test helper functions ====================

/// Create temporary test directory
fn create_test_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("unified_manager_test_{}_{}", name, uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).expect("Failed to create test directory");
    dir
}

/// Create fake model file
fn create_fake_model(dir: &PathBuf, filename: &str, size: usize) -> PathBuf {
    let path = dir.join(filename);
    let mut file = File::create(&path).expect("Failed to create fake model file");
    let data = vec![0u8; size];
    file.write_all(&data).expect("Failed to write fake model data");
    path
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

    // Verify at least 2 models (Q4 and Q8)
    assert!(models.len() >= 2, "Should have at least 2 SenseVoice models");

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_is_model_downloaded_false() {
    let test_dir = create_test_dir("is_downloaded_false");
    let manager = UnifiedEngineManager::new(test_dir.clone());

    // Undownloaded models should return false
    assert!(!manager.is_model_downloaded(EngineType::Whisper, "tiny"));
    assert!(!manager.is_model_downloaded(EngineType::Whisper, "base"));
    assert!(!manager.is_model_downloaded(EngineType::SenseVoice, "sense-voice-small-q4_k"));

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_is_model_downloaded_true() {
    let test_dir = create_test_dir("is_downloaded_true");
    let manager = UnifiedEngineManager::new(test_dir.clone());

    // Create fake model file
    create_fake_model(&test_dir, "ggml-tiny.bin", 1024);

    // Downloaded models should return true
    assert!(manager.is_model_downloaded(EngineType::Whisper, "tiny"));

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_get_model_path() {
    let test_dir = create_test_dir("get_model_path");
    let manager = UnifiedEngineManager::new(test_dir.clone());

    // Whisper model paths
    let path = manager.get_model_path(EngineType::Whisper, "tiny");
    assert!(path.to_string_lossy().contains("ggml-tiny.bin"));

    let path = manager.get_model_path(EngineType::Whisper, "base");
    assert!(path.to_string_lossy().contains("ggml-base.bin"));

    // SenseVoice model paths
    let path = manager.get_model_path(EngineType::SenseVoice, "sense-voice-small-q4_k");
    assert!(path.to_string_lossy().contains("sense-voice-small-q4_k.gguf"));

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_get_model_path_unknown() {
    let test_dir = create_test_dir("get_model_path_unknown");
    let manager = UnifiedEngineManager::new(test_dir.clone());

    // Unknown models should fallback to default naming
    let path = manager.get_model_path(EngineType::Whisper, "unknown");
    assert!(path.to_string_lossy().contains("ggml-unknown.bin"));

    let path = manager.get_model_path(EngineType::SenseVoice, "unknown");
    assert!(path.to_string_lossy().contains("unknown.gguf"));

    cleanup_test_dir(&test_dir);
}

// ==================== D. Model preloading tests ====================

#[test]
fn test_load_model_not_downloaded() {
    let test_dir = create_test_dir("load_not_downloaded");
    let manager = UnifiedEngineManager::new(test_dir.clone());

    // Trying to load undownloaded model should fail
    let result = manager.load_model(EngineType::Whisper, "tiny");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not downloaded"));

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_load_model_unknown() {
    let test_dir = create_test_dir("load_unknown");
    let manager = UnifiedEngineManager::new(test_dir.clone());

    // Create a fake unknown model file
    create_fake_model(&test_dir, "ggml-unknown.bin", 1024);

    // Try loading unknown model (file exists but model definition doesn't)
    let result = manager.load_model(EngineType::Whisper, "unknown");
    // Should fail since model definition doesn't have "unknown"
    assert!(result.is_err());

    cleanup_test_dir(&test_dir);
}

// ==================== E. Model deletion tests ====================

#[test]
fn test_delete_model_not_exists() {
    let test_dir = create_test_dir("delete_not_exists");
    let manager = UnifiedEngineManager::new(test_dir.clone());

    // Trying to delete non-existent model should fail
    let result = manager.delete_model(EngineType::Whisper, "tiny");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not found"));

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_delete_model_success() {
    let test_dir = create_test_dir("delete_success");
    let manager = UnifiedEngineManager::new(test_dir.clone());

    // Create fake model file
    let model_path = create_fake_model(&test_dir, "ggml-tiny.bin", 1024);

    // Verify file exists
    assert!(model_path.exists());

    // Delete model
    let result = manager.delete_model(EngineType::Whisper, "tiny");
    assert!(result.is_ok());

    // Verify file is deleted
    assert!(!model_path.exists());

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_delete_model_multiple_types() {
    let test_dir = create_test_dir("delete_multiple");
    let manager = UnifiedEngineManager::new(test_dir.clone());

    // Create model files for different engines
    let whisper_path = create_fake_model(&test_dir, "ggml-tiny.bin", 1024);
    let sensevoice_path = create_fake_model(&test_dir, "sense-voice-small-q4_k.gguf", 2048);

    // Delete Whisper model
    assert!(manager.delete_model(EngineType::Whisper, "tiny").is_ok());
    assert!(!whisper_path.exists());
    assert!(sensevoice_path.exists());

    // Delete SenseVoice model
    assert!(manager.delete_model(EngineType::SenseVoice, "sense-voice-small-q4_k").is_ok());
    assert!(!sensevoice_path.exists());

    cleanup_test_dir(&test_dir);
}

// ==================== F. Language recommendation tests ====================

#[test]
fn test_recommend_by_language_zh() {
    let test_dir = create_test_dir("recommend_zh");
    let manager = UnifiedEngineManager::new(test_dir.clone());

    let recommendations = manager.recommend_by_language("zh");

    // Should have recommendations
    assert!(!recommendations.is_empty(), "Should have recommendations for Chinese");

    // Should include SenseVoice model (optimized for Chinese)
    let has_sensevoice = recommendations.iter().any(|r| r.engine_type == EngineType::SenseVoice);
    assert!(has_sensevoice, "Should recommend SenseVoice for Chinese");

    // Verify sorted by accuracy descending
    for i in 1..recommendations.len() {
        let prev = &recommendations[i - 1];
        let curr = &recommendations[i];
        assert!(
            prev.accuracy_score >= curr.accuracy_score,
            "Should be sorted by accuracy score (descending)"
        );
    }

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_recommend_by_language_en() {
    let test_dir = create_test_dir("recommend_en");
    let manager = UnifiedEngineManager::new(test_dir.clone());

    let recommendations = manager.recommend_by_language("en");

    // Should have recommendations
    assert!(!recommendations.is_empty(), "Should have recommendations for English");

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

    // Japanese should have recommendations (SenseVoice and some Whisper models support it)
    assert!(!recommendations.is_empty(), "Should have recommendations for Japanese");

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_recommend_by_language_ko() {
    let test_dir = create_test_dir("recommend_ko");
    let manager = UnifiedEngineManager::new(test_dir.clone());

    let recommendations = manager.recommend_by_language("ko");

    // Korean should have recommendations
    assert!(!recommendations.is_empty(), "Should have recommendations for Korean");

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_recommend_by_language_unsupported() {
    let test_dir = create_test_dir("recommend_unsupported");
    let manager = UnifiedEngineManager::new(test_dir.clone());

    // Use unsupported language code
    let recommendations = manager.recommend_by_language("xyz");

    // Should return empty list
    assert!(recommendations.is_empty(), "Should return empty list for unsupported language");

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_recommend_includes_download_status() {
    let test_dir = create_test_dir("recommend_download_status");
    let manager = UnifiedEngineManager::new(test_dir.clone());

    // Create a fake SenseVoice model file
    create_fake_model(&test_dir, "sense-voice-small-q4_k.gguf", 2048);

    let recommendations = manager.recommend_by_language("zh");

    // Find SenseVoice Small Q4 model
    let sensevoice_q4 = recommendations.iter()
        .find(|r| r.model_name == "sense-voice-small-q4_k");

    assert!(sensevoice_q4.is_some());
    assert!(sensevoice_q4.unwrap().downloaded, "Should detect downloaded model");

    // Other models should be undownloaded
    let other_models: Vec<_> = recommendations.iter()
        .filter(|r| r.model_name != "sense-voice-small-q4_k")
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

    let recommendations = manager.recommend_by_language("zh");

    // Verify sorting: accuracy descending, speed descending for same accuracy
    for i in 1..recommendations.len() {
        let prev = &recommendations[i - 1];
        let curr = &recommendations[i];

        if prev.accuracy_score == curr.accuracy_score {
            // When accuracy is same, speed should be descending
            assert!(
                prev.speed_score >= curr.speed_score,
                "When accuracy is equal, should sort by speed (descending)"
            );
        } else {
            // Accuracy should be descending
            assert!(
                prev.accuracy_score > curr.accuracy_score,
                "Should sort by accuracy (descending)"
            );
        }
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
        let path = manager.get_model_path(EngineType::Whisper, &model.name);
        let is_downloaded = manager.is_model_downloaded(EngineType::Whisper, &model.name);

        assert_eq!(
            model.downloaded,
            is_downloaded,
            "Model info downloaded status should match is_model_downloaded()"
        );

        assert_eq!(
            model.downloaded,
            path.exists(),
            "Model info downloaded status should match file existence"
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

    // Create model file in first manager
    create_fake_model(&test_dir, "ggml-tiny.bin", 1024);

    // Both managers should detect the model
    assert!(manager1.is_model_downloaded(EngineType::Whisper, "tiny"));
    assert!(manager2.is_model_downloaded(EngineType::Whisper, "tiny"));

    cleanup_test_dir(&test_dir);
}

// ==================== Integration tests (marked as ignore) ====================

#[test]
fn test_real_download_whisper_tiny() {
    // This test requires real network connection and takes a long time
    // Run: cargo test test_real_download_whisper_tiny -- --ignored

    let test_dir = create_test_dir("test_model_download");
    let manager = UnifiedEngineManager::new(test_dir.clone());

    let cancel_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let progress_called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let progress_called_clone = progress_called.clone();

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        manager.download_model(
            EngineType::Whisper,
            "tiny",
            cancel_flag,
            move |downloaded, total| {
                println!("Progress: {}/{} bytes", downloaded, total);
                progress_called_clone.store(true, std::sync::atomic::Ordering::SeqCst);
            }
        ).await
    });

    assert!(result.is_ok(), "Download should succeed");
    assert!(progress_called.load(std::sync::atomic::Ordering::SeqCst), "Progress callback should be called");

    // Verify file is downloaded
    assert!(manager.is_model_downloaded(EngineType::Whisper, "tiny"));

    cleanup_test_dir(&test_dir);
}
