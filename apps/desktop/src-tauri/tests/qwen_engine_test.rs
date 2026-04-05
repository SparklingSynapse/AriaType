use ariatype_lib::polish_engine::qwen::QwenPolishEngine;
use ariatype_lib::polish_engine::{PolishEngine, PolishEngineType, PolishRequest};

#[tokio::test]
async fn test_qwen_engine_creation() {
    let engine = QwenPolishEngine::new();
    assert_eq!(engine.engine_type(), PolishEngineType::Qwen);
}

#[tokio::test]
async fn test_qwen_engine_polish_basic() {
    let engine = QwenPolishEngine::new();

    let request =
        PolishRequest::new("Hello world", "Polish this text", "en").with_model("qwen3.5-0.8b");

    let result = engine.polish(request).await;
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.contains("Model not found") || error.contains("Model name required"));
}

#[tokio::test]
async fn test_qwen_engine_polish_empty_input() {
    let engine = QwenPolishEngine::new();

    let request = PolishRequest::new("", "Polish this", "en").with_model("nonexistent-model");

    let result = engine.polish(request).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_qwen_engine_polish_chinese() {
    let engine = QwenPolishEngine::new();

    let chinese_text = "你好世界，这是一个测试";
    let request =
        PolishRequest::new(chinese_text, "Polish this", "zh").with_model("nonexistent-model");

    let result = engine.polish(request).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_qwen_engine_error_handling() {
    let engine = QwenPolishEngine::new();

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
async fn test_qwen_engine_type_values() {
    let engine = QwenPolishEngine::new();

    assert_eq!(engine.engine_type(), PolishEngineType::Qwen);
    assert_eq!(engine.engine_type().as_str(), "qwen");

    assert_eq!(
        "qwen".parse::<PolishEngineType>().unwrap(),
        PolishEngineType::Qwen
    );
}
