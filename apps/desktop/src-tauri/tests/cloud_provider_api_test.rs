//! Cloud Provider API Contract Tests
//!
//! Tests verify that CloudSttEngine correctly rejects batch transcription requests
//! and guides users to use the streaming lifecycle instead.

use ariatype_lib::commands::settings::CloudSttConfig;
use ariatype_lib::polish_engine::{PolishRequest, UnifiedPolishManager};
use ariatype_lib::stt_engine::cloud::CloudSttEngine;
use ariatype_lib::stt_engine::traits::{SttEngine, TranscriptionRequest};
use std::io::Cursor;
use std::path::PathBuf;

mod mock_credentials {
    pub const API_KEY: &str = "mock_api_key";
    pub const APP_ID: &str = "mock_app_id";
}

fn create_test_wav() -> Vec<u8> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 16000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut writer = hound::WavWriter::new(&mut cursor, spec).unwrap();
        for _ in 0..1600 {
            writer.write_sample(0i16).unwrap();
        }
    }
    cursor.into_inner()
}

fn write_temp_wav(data: &[u8]) -> PathBuf {
    let temp_dir = std::env::temp_dir();
    let file_path = temp_dir.join(format!("test_audio_{}.wav", uuid::Uuid::new_v4()));
    std::fs::write(&file_path, data).unwrap();
    file_path
}

// ==================== Cloud STT Batch Mode Rejection Tests ====================
// CloudSttEngine::transcribe() should reject batch requests and guide users to streaming

#[tokio::test]
async fn test_cloud_stt_batch_mode_rejected_volcengine() {
    let wav_data = create_test_wav();
    let temp_path = write_temp_wav(&wav_data);

    let config = CloudSttConfig {
        enabled: true,
        provider_type: "volcengine-streaming".to_string(),
        api_key: mock_credentials::API_KEY.to_string(),
        app_id: mock_credentials::APP_ID.to_string(),
        base_url: "wss://openspeech.bytedance.com/api/v3/sauc/bigmodel_nostream".to_string(),
        model: "volc.bigasr.sauc.duration".to_string(),
        language: "zh".to_string(),
    };

    let engine = CloudSttEngine::new().unwrap();
    let mut request = TranscriptionRequest::new(temp_path.to_str().unwrap());
    request.cloud_config = Some(config);

    let result = engine.transcribe(request).await;
    assert!(result.is_err());
    let err = result.unwrap_err();

    assert!(
        err.contains("streaming lifecycle") || err.contains("StreamingSttEngine"),
        "Expected streaming lifecycle guidance, got: {}",
        err
    );
}

#[tokio::test]
async fn test_cloud_stt_batch_mode_rejected_qwen() {
    let wav_data = create_test_wav();
    let temp_path = write_temp_wav(&wav_data);

    let config = CloudSttConfig {
        enabled: true,
        provider_type: "qwen-omni-realtime".to_string(),
        api_key: mock_credentials::API_KEY.to_string(),
        app_id: "".to_string(),
        base_url: "wss://dashscope.aliyuncs.com/api-ws/v1/realtime".to_string(),
        model: "qwen3-asr-flash-realtime".to_string(),
        language: "zh".to_string(),
    };

    let engine = CloudSttEngine::new().unwrap();
    let mut request = TranscriptionRequest::new(temp_path.to_str().unwrap());
    request.cloud_config = Some(config);

    let result = engine.transcribe(request).await;
    assert!(result.is_err());
    let err = result.unwrap_err();

    assert!(
        err.contains("streaming lifecycle") || err.contains("StreamingSttEngine"),
        "Expected streaming lifecycle guidance, got: {}",
        err
    );
}

#[tokio::test]
async fn test_cloud_stt_batch_mode_rejected_elevenlabs() {
    let wav_data = create_test_wav();
    let temp_path = write_temp_wav(&wav_data);

    let config = CloudSttConfig {
        enabled: true,
        provider_type: "elevenlabs".to_string(),
        api_key: mock_credentials::API_KEY.to_string(),
        app_id: "".to_string(),
        base_url: "wss://api.elevenlabs.io/v1/speech-to-text/realtime".to_string(),
        model: "scribe_v2_realtime".to_string(),
        language: "en".to_string(),
    };

    let engine = CloudSttEngine::new().unwrap();
    let mut request = TranscriptionRequest::new(temp_path.to_str().unwrap());
    request.cloud_config = Some(config);

    let result = engine.transcribe(request).await;
    assert!(result.is_err());
    let err = result.unwrap_err();

    assert!(
        err.contains("streaming lifecycle") || err.contains("StreamingSttEngine"),
        "Expected streaming lifecycle guidance, got: {}",
        err
    );
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
