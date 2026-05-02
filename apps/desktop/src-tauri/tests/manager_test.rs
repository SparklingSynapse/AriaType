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
    assert!(!models.is_empty(), "Should have Whisper models");
    
    let sensevoice_models = manager.get_models(EngineType::SenseVoice);
    assert!(!sensevoice_models.is_empty(), "Should have SenseVoice models");
}

#[test]
fn test_manager_model_info() {
    let models_dir = temp_models_dir();
    let manager = UnifiedEngineManager::new(models_dir);

    let models = manager.get_models(EngineType::Whisper);

    let base = models.iter().find(|m| m.name == "whisper-base");
    assert!(base.is_some(), "Should have whisper-base model");

    let base = base.unwrap();
    assert!(base.size_mb > 0);
    assert!(!base.filename.is_empty());
    assert!(base.display_name.contains("Base"));
}

#[test]
fn test_manager_get_model_path() {
    let models_dir = temp_models_dir();
    let manager = UnifiedEngineManager::new(models_dir);

    let path = manager.get_model_path(EngineType::Whisper, "whisper-base");
    assert!(path.to_string_lossy().contains("whisper-base"));
}

#[test]
fn test_manager_is_model_downloaded() {
    let models_dir = temp_models_dir();
    let manager = UnifiedEngineManager::new(models_dir.clone());

    assert!(
        !manager.is_model_downloaded(EngineType::Whisper, "whisper-base"),
        "Model should not be downloaded initially"
    );

    let model_subdir = models_dir.join("whisper-base");
    std::fs::create_dir_all(&model_subdir).ok();
    std::fs::write(model_subdir.join("base-encoder.onnx"), "fake encoder").ok();
    std::fs::write(model_subdir.join("base-decoder.onnx"), "fake decoder").ok();
    std::fs::write(model_subdir.join("base-tokens.txt"), "fake tokens").ok();

    let manager2 = UnifiedEngineManager::new(models_dir);
    assert!(
        manager2.is_model_downloaded(EngineType::Whisper, "whisper-base"),
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

    assert!(names.contains(&"whisper-base"));
    assert!(names.contains(&"whisper-small"));
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

    let result = manager.load_model(EngineType::Whisper, "whisper-base");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not downloaded"));
}

#[test]
fn test_delete_model_not_exists() {
    let models_dir = temp_models_dir();
    let manager = UnifiedEngineManager::new(models_dir);

    let result = manager.delete_model(EngineType::Whisper, "whisper-base");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not found"));
}

#[test]
fn test_delete_model_success() {
    let models_dir = temp_models_dir();
    let manager = UnifiedEngineManager::new(models_dir.clone());

    let model_subdir = models_dir.join("whisper-base");
    std::fs::create_dir_all(&model_subdir).ok();
    std::fs::write(model_subdir.join("base-encoder.onnx"), "fake encoder").unwrap();
    std::fs::write(model_subdir.join("base-decoder.onnx"), "fake decoder").unwrap();
    std::fs::write(model_subdir.join("base-tokens.txt"), "fake tokens").unwrap();

    assert!(model_subdir.exists());

    let result = manager.delete_model(EngineType::Whisper, "whisper-base");
    assert!(result.is_ok());

    assert!(!model_subdir.exists());
}
