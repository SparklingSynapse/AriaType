use ariatype_lib::stt_engine::{EngineType, UnifiedEngineManager};
use std::path::PathBuf;

fn temp_models_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("ariatype_test_models_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).ok();
    dir
}

#[test]
fn test_manager_new() {
    let models_dir = temp_models_dir();
    let manager = UnifiedEngineManager::new(models_dir.clone());

    let models = manager.get_models(EngineType::Whisper);
    assert!(!models.is_empty(), "Should have default models");
}

#[test]
fn test_manager_model_info() {
    let models_dir = temp_models_dir();
    let manager = UnifiedEngineManager::new(models_dir);

    let models = manager.get_models(EngineType::Whisper);

    let tiny = models.iter().find(|m| m.name == "tiny");
    assert!(tiny.is_some(), "Should have tiny model");

    let tiny = tiny.unwrap();
    assert!(tiny.size_mb > 0);
    assert!(!tiny.filename.is_empty());
    assert!(tiny.display_name.contains("Tiny"));
}

#[test]
fn test_manager_get_model_path() {
    let models_dir = temp_models_dir();
    let manager = UnifiedEngineManager::new(models_dir);

    let path = manager.get_model_path(EngineType::Whisper, "tiny");
    assert!(path.to_string_lossy().contains("ggml-tiny.bin"));
}

#[test]
fn test_manager_is_model_downloaded() {
    let models_dir = temp_models_dir();
    let manager = UnifiedEngineManager::new(models_dir.clone());

    assert!(
        !manager.is_model_downloaded(EngineType::Whisper, "tiny"),
        "Model should not be downloaded initially"
    );

    std::fs::write(models_dir.join("ggml-tiny.bin"), "fake model data").ok();

    let manager2 = UnifiedEngineManager::new(models_dir);
    assert!(
        manager2.is_model_downloaded(EngineType::Whisper, "tiny"),
        "Model should be detected as downloaded"
    );
}

#[test]
fn test_manager_models_sorted_by_size() {
    let models_dir = temp_models_dir();
    let manager = UnifiedEngineManager::new(models_dir);

    let models = manager.get_models(EngineType::Whisper);

    for i in 1..models.len() {
        assert!(
            models[i - 1].size_mb <= models[i].size_mb,
            "Models should be sorted by size"
        );
    }
}

#[test]
fn test_manager_all_model_names() {
    let models_dir = temp_models_dir();
    let manager = UnifiedEngineManager::new(models_dir);

    let models = manager.get_models(EngineType::Whisper);
    let names: Vec<&str> = models.iter().map(|m| m.name.as_str()).collect();

    assert!(names.contains(&"tiny"));
    assert!(names.contains(&"base"));
}

#[test]
fn test_manager_unknown_model() {
    let models_dir = temp_models_dir();
    let manager = UnifiedEngineManager::new(models_dir);

    let path = manager.get_model_path(EngineType::Whisper, "nonexistent");
    assert!(path.to_string_lossy().contains("nonexistent"));
}

#[test]
fn test_load_model_not_downloaded() {
    let models_dir = temp_models_dir();
    let manager = UnifiedEngineManager::new(models_dir);

    let result = manager.load_model(EngineType::Whisper, "tiny");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not downloaded"));
}

#[test]
fn test_delete_model_not_exists() {
    let models_dir = temp_models_dir();
    let manager = UnifiedEngineManager::new(models_dir);

    let result = manager.delete_model(EngineType::Whisper, "tiny");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not found"));
}

#[test]
fn test_delete_model_success() {
    let models_dir = temp_models_dir();
    let manager = UnifiedEngineManager::new(models_dir.clone());

    // Create a fake model file
    let model_path = models_dir.join("ggml-tiny.bin");
    std::fs::write(&model_path, "fake model data").unwrap();

    // Verify file exists
    assert!(model_path.exists());

    // Delete model
    let result = manager.delete_model(EngineType::Whisper, "tiny");
    assert!(result.is_ok());

    // Verify file is deleted
    assert!(!model_path.exists());
}
