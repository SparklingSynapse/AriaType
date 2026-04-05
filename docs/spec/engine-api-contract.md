# Engine API Contract Testing

Engines (STT and Polish) abstract vendor API differences behind unified interfaces. Each engine must correctly construct requests for its vendor and parse vendor-specific responses into unified types.

## Engine Traits

**SttEngine** (batch processing for local engines like Whisper, SenseVoice):

```rust
pub trait SttEngine: Send + Sync {
    fn engine_type(&self) -> EngineType;
    async fn transcribe(&self, request: TranscriptionRequest) -> Result<TranscriptionResult, String>;
}
```

**StreamingSttEngine** (cloud streaming engines like Volcengine, Qwen Omni, ElevenLabs):

```rust
pub trait StreamingSttEngine: Send + Sync {
    fn set_partial_callback(&mut self, callback: PartialResultCallback);
    async fn connect(&mut self) -> Result<(), String>;
    async fn get_audio_sender(&self) -> Option<Sender<Vec<i16>>>;
    async fn finish(&self) -> Result<String, String>;
}
```

## Auth Error Verification Pattern

Tests verify API contract correctness by expecting authentication errors from real endpoints with mock credentials.

```rust
mod mock_credentials {
    pub const API_KEY: &str = "mock_api_key_for_testing";
    pub const APP_ID: &str = "mock_app_id_for_testing";
}
```

| Error Type | Proves |
|------------|--------|
| 401/403 Auth Error | Endpoint URL, headers, and request body are correctly formed |
| 400 Bad Request | Request body or headers are malformed |
| 404 Not Found | Endpoint URL is incorrect |

## Test Requirements by Engine Type

| Engine Category | Test Requirement | Location |
|-----------------|------------------|----------|
| Local engines (Whisper, SenseVoice) | Unit tests for model loading, inference, parsing | `src/` inline + `tests/` |
| Cloud STT engines | Real API calls with mock credentials to verify auth errors | `tests/cloud_provider_api_test.rs` |
| Cloud Polish engines | Real API calls with mock credentials to verify auth errors | `tests/cloud_provider_api_test.rs` |
| Response parsing (all vendors) | Mock server or recorded responses | `tests/common/mock_server.rs` |

## Test Locations

| Purpose | Location |
|---------|----------|
| Cloud STT API contract tests | `tests/cloud_provider_api_test.rs` |
| Cloud STT integration tests | `tests/cloud_stt_test.rs` |
| Streaming client tests | `tests/volcengine_streaming_test.rs` |
| Pipeline integration | `tests/pipeline_integration_test.rs` |

## Core Testing Principle

For cloud engines, tests must verify the API contract without requiring valid credentials or successful API responses. An auth error proves the endpoint, headers, and request body are correctly formed.
