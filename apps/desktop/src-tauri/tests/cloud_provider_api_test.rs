//! Cloud Provider API Contract Tests
//!
//! Cloud STT batch mode rejection tests were removed — CloudSttEngine no longer
//! exposes a batch transcribe() method. Cloud STT now exclusively uses the
//! streaming RecordingConsumer trait (send_chunk + finish), making batch cloud STT
//! impossible by design.

use ariatype_lib::polish_engine::{PolishRequest, UnifiedPolishManager};

mod mock_credentials {
    pub const API_KEY: &str = "mock_api_key";
}

// ==================== Polish Engine Tests ====================

#[tokio::test]
async fn test_polish_openai_schema() {
    let manager = UnifiedPolishManager::default();
    let request = PolishRequest::new("test", "test prompt", "en");

    let result = manager
        .polish_cloud(
            request,
            "openai",
            mock_credentials::API_KEY,
            "https://api.openai.com/v1/chat/completions",
            "gpt-4o-mini",
            false,
        )
        .await;

    assert!(result.is_err());
    let err = result.unwrap_err();

    assert!(
        err.contains("401") || err.contains("Unauthorized") || err.contains("invalid_api_key"),
        "Expected auth error (401), got: {}",
        err
    );
    assert!(
        !err.contains("400") && !err.contains("Bad Request"),
        "Should not be parameter error (400): {}",
        err
    );
}

#[tokio::test]
async fn test_polish_anthropic_schema() {
    let manager = UnifiedPolishManager::default();
    let request = PolishRequest::new("test", "test prompt", "en");

    let result = manager
        .polish_cloud(
            request,
            "anthropic",
            mock_credentials::API_KEY,
            "https://api.anthropic.com/v1/messages",
            "claude-3-haiku",
            false,
        )
        .await;

    assert!(result.is_err());
    let err = result.unwrap_err();

    assert!(
        err.contains("401")
            || err.contains("Unauthorized")
            || err.contains("403")
            || err.contains("invalid"),
        "Expected auth error (401/403), got: {}",
        err
    );
    assert!(
        !err.contains("400") && !err.contains("Bad Request"),
        "Should not be parameter error (400): {}",
        err
    );
}
