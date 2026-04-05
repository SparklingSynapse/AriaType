# Contributing to @ariatype/desktop (STT Engine)

This document covers the Speech-to-Text (STT) engine architecture for contributors working on transcription functionality.

---

## 1. STT Engine Architecture Overview

The codebase uses two distinct engine traits to handle different transcription paradigms:

### Engine Traits

| Trait | Purpose | Engine Types |
|-------|---------|--------------|
| `SttEngine` | Batch transcription for complete audio files | Whisper, SenseVoice |
| `StreamingSttEngine` | Streaming lifecycle for real-time audio chunks | Volcengine, Qwen Omni, ElevenLabs |

### Local Engines (Batch Mode)

**Whisper** (`stt_engine/whisper/`) and **SenseVoice** (`stt_engine/sense_voice/`) implement `SttEngine`:

```rust
#[async_trait]
pub trait SttEngine: Send + Sync {
    fn engine_type(&self) -> EngineType;
    async fn transcribe(&self, request: TranscriptionRequest) -> Result<TranscriptionResult, String>;
}
```

These engines:
- Receive a complete audio file path
- Process the entire file in one operation
- Return full transcription result
- Run entirely on-device

### Cloud Engines (Streaming Lifecycle)

Cloud providers implement `StreamingSttEngine` (`stt_engine/traits.rs`):

```rust
#[async_trait]
pub trait StreamingSttEngine: Send + Sync {
    fn set_partial_callback(&mut self, callback: PartialResultCallback);
    async fn connect(&mut self) -> Result<(), String>;
    async fn get_audio_sender(&self) -> Option<tokio::sync::mpsc::Sender<Vec<i16>>>;
    async fn finish(&self) -> Result<String, String>;
}
```

Current cloud providers:
- **Volcengine** (`volcengine_streaming.rs`) - WebSocket streaming
- **Qwen Omni** (`qwen_omni_realtime.rs`) - WebSocket streaming
- **ElevenLabs** (`elevenlabs.rs`) - WebSocket streaming

---

## 2. Streaming Lifecycle Contract

Cloud STT engines follow a strict three-phase lifecycle:

### Lifecycle Phases

```
1. connect()    → Establish WebSocket connection, receive audio sender channel
2. send_chunk() → Stream audio chunks (1 second intervals, 16kHz mono PCM)
3. finish()     → Signal end of audio, receive final transcription
```

### Audio Format Requirements

| Provider | Sample Rate | Channels | Format |
|----------|-------------|----------|--------|
| Volcengine | 16kHz | Mono | 16-bit PCM |
| Qwen Omni | 24kHz (internal) | Mono | 16-bit PCM |
| ElevenLabs | 16kHz | Mono | 16-bit PCM |

**Note**: Qwen Omni resamples to 24kHz internally. Other providers expect 16kHz input.

### Chunk Timing

During recording, audio chunks are sent at **1 second intervals**:

```
Recording start
  ↓
connect() called
  ↓
[send_chunk(1s audio) → partial result]
[send_chunk(1s audio) → partial result]
[send_chunk(1s audio) → partial result]
  ↓
finish() called → final result
```

### Error Handling

Errors must fail immediately and display to the user:

```rust
// Pattern: Fail fast with clear message
if self.config.api_key.is_empty() {
    return Err("Volcengine Access Token is empty. Please configure your credentials.".to_string());
}

// Connection errors provide actionable guidance
if error_str.contains("403") {
    return Err(format!(
        "Volcengine STT authentication failed (403 Forbidden).\n\n\
        Possible causes:\n\
        1. Access Token has expired\n\
        2. Service not activated\n\
        ..."
    ));
}
```

---

## 3. Recording Flow

### Cloud STT (Streaming)

During recording, chunks flow through the system:

```rust
// 1. User holds hotkey → recording starts
// 2. Audio captured at 16kHz mono PCM
// 3. Every 1 second: send_chunk() called via audio sender channel
// 4. Partial results received via callback
// 5. User releases hotkey → finish() called
// 6. Final transcription returned
```

**State management**: `StreamingSttState` in `state/unified_state.rs` manages:
- Active streaming client instance
- Partial transcription results
- Recording session lifecycle

### Local STT (Batch)

After recording completes:

```rust
// 1. User holds hotkey → recording starts
// 2. Audio captured and stored in AudioStorage
// 3. User releases hotkey → recording stops
// 4. Complete audio file passed to transcribe()
// 5. Engine processes entire file
// 6. Full transcription returned
```

**State management**: `AudioStorage` holds complete audio buffer until transcription completes.

---

## 4. Testing Cloud STT Engines

Cloud engine tests use mock credentials to verify API request construction without requiring valid accounts.

### Mock Credentials Pattern

```rust
mod mock_credentials {
    pub const API_KEY: &str = "mock_api_key";
    pub const APP_ID: &str = "mock_app_id";
}
```

### Auth Error Verification

Tests verify the API request is correctly formed by expecting authentication errors:

```rust
#[tokio::test]
async fn test_stt_volcengine_streaming_schema() {
    let config = CloudSttConfig {
        provider_type: "volcengine-streaming".to_string(),
        api_key: mock_credentials::API_KEY.to_string(),
        app_id: mock_credentials::APP_ID.to_string(),
        base_url: "wss://openspeech.bytedance.com/api/v3/sauc/bigmodel_nostream".to_string(),
        // ...
    };

    let result = engine.transcribe(request).await;
    assert!(result.is_err());
    let err = result.unwrap_err();

    // CORRECT: Auth error proves endpoint/headers/body are correctly formed
    assert!(err.contains("403") || err.contains("Forbidden"),
        "Expected auth error (403), got: {}", err);

    // Verify NOT a parameter error
    assert!(!err.contains("400") && !err.contains("Bad Request"),
        "Should not be parameter validation error: {}", err);
}
```

### Why Auth Errors Prove Correctness

| Error Type | Proves |
|------------|--------|
| 401/403 Auth Error | Endpoint URL, headers, and request body are correctly formed |
| 400 Bad Request | Request body or headers are malformed |
| 404 Not Found | Endpoint URL is incorrect |

### Test File Locations

| Purpose | Location |
|---------|----------|
| Cloud STT API contract tests | `tests/cloud_provider_api_test.rs` |
| Cloud STT integration tests | `tests/cloud_stt_test.rs` |
| Streaming client tests | `tests/volcengine_streaming_test.rs` |

---

## 5. Adding New Cloud Providers

To add a new cloud STT provider:

### Step 1: Implement StreamingSttEngine

Create a new file in `stt_engine/cloud/`:

```rust
// stt_engine/cloud/new_provider.rs
pub struct NewProviderClient {
    // Provider-specific fields
}

#[async_trait]
impl StreamingSttEngine for NewProviderClient {
    fn set_partial_callback(&mut self, callback: PartialResultCallback) {
        // Store callback for partial results
    }

    async fn connect(&mut self) -> Result<(), String> {
        // 1. Validate config
        // 2. Establish WebSocket connection
        // 3. Send initialization request
        // 4. Return error with actionable message on failure
    }

    async fn get_audio_sender(&self) -> Option<Sender<Vec<i16>>> {
        // Return channel for sending audio chunks
    }

    async fn finish(&self) -> Result<String, String> {
        // 1. Signal end of audio
        // 2. Wait for final transcription
        // 3. Return result
    }
}
```

### Step 2: Add to StreamingSttClient Enum

In `stt_engine/cloud/mod.rs`:

```rust
pub enum StreamingSttClient {
    Volcengine(VolcengineStreamingClient),
    QwenOmni(QwenOmniRealtimeClient),
    ElevenLabs(ElevenLabsStreamingClient),
    NewProvider(NewProviderClient),  // Add here
}

impl StreamingSttClient {
    pub fn new(config: CloudSttConfig, language: Option<&str>) -> Result<Self, String> {
        match config.provider_type.as_str() {
            "new-provider" => Ok(Self::NewProvider(NewProviderClient::new(config, language))),
            // ...
        }
    }
}
```

### Step 3: Register in CloudSttEngine

In `stt_engine/cloud/engine.rs`:

```rust
match config.provider_type.as_str() {
    "new-provider" => {
        debug!("Using New Provider API (WebSocket)");
        transcribe_new_provider(&config, &request.audio_path, request.language.as_deref()).await
    }
    // ...
}
```

### Step 4: Add Tests

In `tests/cloud_provider_api_test.rs`:

```rust
#[tokio::test]
async fn test_stt_new_provider_schema() {
    let config = CloudSttConfig {
        provider_type: "new-provider".to_string(),
        api_key: mock_credentials::API_KEY.to_string(),
        // ...
    };
    // Verify auth error, not parameter error
}
```

---

## 6. Product Priority Order

From root `AGENTS.md` Section 1.2:

```
STT accuracy > STT stability > user experience > speed
```

**Implications for STT development**:
- Do NOT accept latency gains that reduce accuracy or stability
- Speed optimizations are only acceptable after accuracy, stability, and UX are protected
- Always prefer correct transcription over fast transcription

### Volcengine URL Requirement

**Required URL**: `wss://openspeech.bytedance.com/api/v3/sauc/bigmodel_nostream`

**Reason**: Per product priority order, bidirectional streaming interfaces (`bigmodel_async`, `bigmodel`) have lower accuracy. Only `bigmodel_nostream` meets our accuracy requirements.

| Mode | URL | Accuracy | Recommendation |
|------|-----|----------|---------------|
| NoStream | `bigmodel_nostream` | Highest | **Required** |
| Async | `bigmodel_async` | Lower | Not recommended |
| Standard | `bigmodel` | Lower | Not recommended |

**Frontend placeholder** must show `bigmodel_nostream`, not `bigmodel_async`.

---

## 7. Key Files Reference

| File | Purpose |
|------|---------|
| `stt_engine/traits.rs` | `SttEngine` trait, `EngineType`, request/result types |
| `stt_engine/traits.rs` | `StreamingSttEngine` trait, `PartialResult`, `PartialResultCallback` |
| `stt_engine/cloud/engine.rs` | `CloudSttEngine` implementation |
| `stt_engine/cloud/volcengine_streaming.rs` | Volcengine streaming client |
| `stt_engine/unified_manager.rs` | Engine lifecycle management |
| `state/unified_state.rs` | Runtime state including `StreamingSttState` |
| `tests/cloud_provider_api_test.rs` | Cloud API contract tests |
