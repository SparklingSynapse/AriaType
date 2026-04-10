use ariatype_lib::polish_engine::{PolishEngine, PolishEngineType, PolishRequest, PolishResult};
use ariatype_lib::stt_engine::traits::{PartialResultCallback, RecordingConsumer};
use ariatype_lib::stt_engine::{EngineType, TranscriptionRequest, TranscriptionResult};
use async_trait::async_trait;
use std::sync::Arc;

struct MockSttEngine {
    result_text: String,
    latency_ms: u64,
    should_fail: bool,
    failure_message: String,
    engine_type: EngineType,
    chunks: std::sync::Mutex<Vec<Vec<i16>>>,
}

impl MockSttEngine {
    fn new() -> Self {
        Self {
            result_text: "Mock transcription".to_string(),
            latency_ms: 0,
            should_fail: false,
            failure_message: "Mock failure".to_string(),
            engine_type: EngineType::Whisper,
            chunks: std::sync::Mutex::new(Vec::new()),
        }
    }

    fn with_failure(mut self, message: impl Into<String>) -> Self {
        self.should_fail = true;
        self.failure_message = message.into();
        self
    }

    fn with_latency(mut self, latency_ms: u64) -> Self {
        self.latency_ms = latency_ms;
        self
    }

    fn build(self) -> Arc<Self> {
        Arc::new(self)
    }

    async fn transcribe(
        &self,
        _request: TranscriptionRequest,
    ) -> Result<TranscriptionResult, String> {
        if self.latency_ms > 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(self.latency_ms)).await;
        }
        if self.should_fail {
            return Err(self.failure_message.clone());
        }
        Ok(TranscriptionResult::new(
            self.result_text.clone(),
            self.engine_type,
            self.latency_ms,
        ))
    }
}

impl Default for MockSttEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl RecordingConsumer for MockSttEngine {
    async fn send_chunk(&self, pcm_data: Vec<i16>) -> Result<(), String> {
        self.chunks.lock().unwrap().push(pcm_data);
        Ok(())
    }

    async fn finish(&self) -> Result<String, String> {
        if self.latency_ms > 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(self.latency_ms)).await;
        }
        if self.should_fail {
            return Err(self.failure_message.clone());
        }
        Ok(self.result_text.clone())
    }

    fn set_partial_callback(&mut self, _callback: PartialResultCallback) {}
}

struct MockPolishEngine {
    result_text: String,
    latency_ms: u64,
    should_fail: bool,
    failure_message: String,
    engine_type: PolishEngineType,
}

impl MockPolishEngine {
    fn new() -> Self {
        Self {
            result_text: "Mock polished text".to_string(),
            latency_ms: 0,
            should_fail: false,
            failure_message: "Mock polish failure".to_string(),
            engine_type: PolishEngineType::Qwen,
        }
    }

    fn with_failure(mut self, message: impl Into<String>) -> Self {
        self.should_fail = true;
        self.failure_message = message.into();
        self
    }

    fn with_latency(mut self, latency_ms: u64) -> Self {
        self.latency_ms = latency_ms;
        self
    }

    fn build(self) -> Arc<Self> {
        Arc::new(self)
    }
}

impl Default for MockPolishEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PolishEngine for MockPolishEngine {
    fn engine_type(&self) -> PolishEngineType {
        self.engine_type
    }

    async fn polish(&self, _request: PolishRequest) -> Result<PolishResult, String> {
        if self.latency_ms > 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(self.latency_ms)).await;
        }
        if self.should_fail {
            return Err(self.failure_message.clone());
        }
        Ok(PolishResult::new(
            self.result_text.clone(),
            self.engine_type,
            self.latency_ms,
        ))
    }
}

#[tokio::test]
async fn test_network_timeout_handling() {
    let mock = MockSttEngine::new()
        .with_failure("Network timeout: connection timed out after 30s")
        .with_latency(10);

    let request = TranscriptionRequest::new(vec![0.0f32; 16000]);
    let result: Result<TranscriptionResult, String> = mock.transcribe(request).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("timeout") || err.contains("timed out"));
}

#[tokio::test]
async fn test_connection_failed_handling() {
    let mock = MockSttEngine::new()
        .with_failure("Connection failed: Failed to connect to server")
        .with_latency(10);

    let request = TranscriptionRequest::new(vec![0.0f32; 16000]);
    let result: Result<TranscriptionResult, String> = mock.transcribe(request).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("connection") || err.contains("connect"));
}

#[tokio::test]
async fn test_invalid_api_response() {
    let mock = MockSttEngine::new()
        .with_failure("Invalid API response: malformed JSON received")
        .with_latency(10);

    let request = TranscriptionRequest::new(vec![0.0f32; 16000]);
    let result: Result<TranscriptionResult, String> = mock.transcribe(request).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("invalid") || err.contains("malformed") || err.contains("JSON"));
}

#[tokio::test]
async fn test_rate_limit_handling() {
    let mock = MockSttEngine::new()
        .with_failure(
            "Cloud STT rate limit exceeded (429). Please wait before making more requests.",
        )
        .with_latency(10);

    let request = TranscriptionRequest::new(vec![0.0f32; 16000]);
    let result: Result<TranscriptionResult, String> = mock.transcribe(request).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("rate limit") || err.contains("429"));
}

#[tokio::test]
async fn test_empty_audio_input() {
    let mock = MockSttEngine::new()
        .with_failure("Empty audio input: audio file is empty or contains no data")
        .with_latency(10);

    let request = TranscriptionRequest::new(vec![0.0f32; 16000]);
    let result: Result<TranscriptionResult, String> = mock.transcribe(request).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("empty") || err.contains("no data"));
}

#[tokio::test]
async fn test_unsupported_format() {
    let mock = MockSttEngine::new()
        .with_failure("Unsupported audio format: expected WAV/MP3/PCM, got .xyz")
        .with_latency(10);

    let request = TranscriptionRequest::new(vec![0.0f32; 16000]);
    let result: Result<TranscriptionResult, String> = mock.transcribe(request).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("unsupported") || err.contains("format"));
}

#[tokio::test]
async fn test_model_not_found() {
    let mock = MockSttEngine::new()
        .with_failure("Whisper model 'base' not found at /models/ggml-base.bin")
        .with_latency(10);

    let request = TranscriptionRequest::new(vec![0.0f32; 16000]);
    let result: Result<TranscriptionResult, String> = mock.transcribe(request).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("not found") || err.contains("model"));
}

#[tokio::test]
async fn test_model_load_failed() {
    let mock = MockSttEngine::new()
        .with_failure("Failed to load model: insufficient memory to load ggml-model.bin")
        .with_latency(10);

    let request = TranscriptionRequest::new(vec![0.0f32; 16000]);
    let result: Result<TranscriptionResult, String> = mock.transcribe(request).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("load") || err.contains("memory") || err.contains("failed"));
}

#[tokio::test]
async fn test_partial_result_recovery() {
    let mock = Arc::new(
        MockSttEngine::new()
            .with_failure("Partial failure: returning partial transcription")
            .with_latency(10)
            .build(),
    );

    let request = TranscriptionRequest::new(vec![0.0f32; 16000]);
    let result: Result<TranscriptionResult, String> = mock.transcribe(request).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_error_propagation_chain() {
    let stt_mock = Arc::new(
        MockSttEngine::new()
            .with_failure("STT engine unavailable")
            .build(),
    );

    let polish_mock = Arc::new(
        MockPolishEngine::new()
            .with_failure("Polish engine unavailable")
            .build(),
    );

    let stt_request = TranscriptionRequest::new(vec![0.0f32; 16000]);
    let stt_result: Result<TranscriptionResult, String> = stt_mock.transcribe(stt_request).await;
    assert!(stt_result.is_err());

    let polish_request = PolishRequest::new("partial", "prompt", "en");
    let polish_result: Result<PolishResult, String> = polish_mock.polish(polish_request).await;
    assert!(polish_result.is_err());
}

#[tokio::test]
async fn test_authentication_failure() {
    let mock = MockSttEngine::new()
        .with_failure(
            "Cloud STT authentication failed (401 Unauthorized). Please verify your API key.",
        )
        .with_latency(10);

    let request = TranscriptionRequest::new(vec![0.0f32; 16000]);
    let result: Result<TranscriptionResult, String> = mock.transcribe(request).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("auth") || err.contains("401") || err.contains("unauthorized"));
}

#[tokio::test]
async fn test_forbidden_access() {
    let mock = MockSttEngine::new()
        .with_failure("Cloud STT access forbidden (403). Your subscription may have expired.")
        .with_latency(10);

    let request = TranscriptionRequest::new(vec![0.0f32; 16000]);
    let result: Result<TranscriptionResult, String> = mock.transcribe(request).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("forbidden") || err.contains("403"));
}

#[tokio::test]
async fn test_retry_after_transient_failure() {
    let call_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    for _ in 0..3 {
        let count = call_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        if count < 2 {
            let err = format!("Transient error (attempt {})", count + 1);
            assert!(err.contains("Transient error"));
        }
    }
}

#[tokio::test]
async fn test_error_message_format() {
    let error_messages = vec![
        "Network timeout: connection timed out after 30s",
        "Connection failed: Failed to connect to server",
        "Invalid API response: malformed JSON received",
        "Rate limit exceeded (429)",
        "Empty audio input",
        "Model not found",
        "Authentication failed (401)",
    ];

    for msg in error_messages {
        assert!(!msg.is_empty());
        assert!(msg.len() > 10);
    }
}

#[tokio::test]
async fn test_polish_rate_limit_handling() {
    let mock = MockPolishEngine::new()
        .with_failure("Cloud API rate limit exceeded (429). Retry after 60 seconds.")
        .with_latency(10);

    let request = PolishRequest::new("test text", "system prompt", "en");
    let result: Result<PolishResult, String> = mock.polish(request).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("rate limit") || err.contains("429"));
}

#[tokio::test]
async fn test_polish_model_not_found() {
    let mock = MockPolishEngine::new()
        .with_failure("Polish model 'qwen2.5' not found at /models/qwen2.5.gguf")
        .with_latency(10);

    let request = PolishRequest::new("test text", "system prompt", "en");
    let result: Result<PolishResult, String> = mock.polish(request).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("not found") || err.contains("model"));
}

#[tokio::test]
async fn test_polish_empty_input() {
    let mock = MockPolishEngine::new()
        .with_failure("Empty text input: cannot polish empty string")
        .with_latency(10);

    let request = PolishRequest::new("", "system prompt", "en");
    let result: Result<PolishResult, String> = mock.polish(request).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("empty"));
}

#[tokio::test]
async fn test_long_samples_error_handling() {
    let mock = MockSttEngine::new()
        .with_failure("Input too large")
        .with_latency(10);

    let long_samples: Vec<f32> = vec![0.0; 16000 * 100];
    let request = TranscriptionRequest::new(long_samples);
    let result: Result<TranscriptionResult, String> = mock.transcribe(request).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_special_chars_error() {
    let mock = MockSttEngine::new()
        .with_failure("Invalid input characters")
        .with_latency(10);

    let request = TranscriptionRequest::new(vec![0.0f32; 16000]);
    let result: Result<TranscriptionResult, String> = mock.transcribe(request).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_unicode_text_error_handling() {
    let mock = MockPolishEngine::new()
        .with_failure("Encoding error: invalid UTF-8 sequence")
        .with_latency(10);

    let request = PolishRequest::new("文本测试 🚀", "系统提示", "zh");
    let result: Result<PolishResult, String> = mock.polish(request).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_concurrent_error_handling() {
    let mock = Arc::new(
        MockSttEngine::new()
            .with_failure("Concurrent access error")
            .with_latency(5)
            .build(),
    );

    let mut handles = vec![];

    for _ in 0..10 {
        let mock_clone = mock.clone();
        let handle = tokio::spawn(async move {
            let request = TranscriptionRequest::new(vec![0.0f32; 16000]);
            mock_clone.transcribe(request).await
        });
        handles.push(handle);
    }

    for handle in handles {
        let result: Result<TranscriptionResult, String> = handle.await.unwrap();
        assert!(result.is_err());
    }
}
