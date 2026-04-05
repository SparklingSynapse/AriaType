use ariatype_lib::polish_engine::lfm::LfmPolishEngine;
use ariatype_lib::polish_engine::{PolishEngine, PolishEngineType, PolishRequest};

#[tokio::test]
async fn test_lfm_engine_creation() {
    let engine = LfmPolishEngine::new();
    assert_eq!(engine.engine_type(), PolishEngineType::Lfm);
}

#[tokio::test]
async fn test_lfm_engine_polish_basic() {
    let engine = LfmPolishEngine::new();

    let request =
        PolishRequest::new("Hello world", "Polish this text", "en").with_model("lfm2.5-1.2b");

    let result = engine.polish(request).await;
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.contains("Model not found") || error.contains("Model name required"));
}

#[tokio::test]
async fn test_lfm_engine_polish_empty_input() {
    let engine = LfmPolishEngine::new();

    let request = PolishRequest::new("", "Polish this", "en").with_model("nonexistent-model");

    let result = engine.polish(request).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_lfm_engine_polish_long_text() {
    let engine = LfmPolishEngine::new();

    let long_text =
        "This is a longer piece of text that should be handled by the engine. ".repeat(100);
    let request = PolishRequest::new(long_text, "Polish", "en").with_model("nonexistent-model");

    let result = engine.polish(request).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_lfm_engine_error_handling() {
    let engine = LfmPolishEngine::new();

    let request = PolishRequest::new("Some text", "System prompt", "en");
    let result = engine.polish(request).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Model name required"));

    let request = PolishRequest::new("Some text", "System prompt", "en")
        .with_model("definitely-does-not-exist.gguf");
    let result = engine.polish(request).await;
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.contains("Model not found"));
}

#[tokio::test]
async fn test_lfm_engine_type_values() {
    let engine = LfmPolishEngine::new();

    assert_eq!(engine.engine_type(), PolishEngineType::Lfm);
    assert_eq!(engine.engine_type().as_str(), "lfm");

    assert_eq!(
        "lfm".parse::<PolishEngineType>().unwrap(),
        PolishEngineType::Lfm
    );
}
