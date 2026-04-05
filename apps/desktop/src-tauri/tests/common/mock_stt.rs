//! Mock STT engine for testing
//!
//! Provides a configurable mock implementation of the STT engine trait
//! for use in integration and unit tests.

use ariatype_lib::stt_engine::{EngineType, SttEngine, TranscriptionRequest, TranscriptionResult};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Mock STT engine with configurable behavior
pub struct MockSttEngine {
    result_text: String,
    latency_ms: u64,
    should_fail: bool,
    failure_message: String,
    engine_type: EngineType,
}

impl MockSttEngine {
    /// Create a new MockSttEngine with default values
    pub fn new() -> Self {
        Self {
            result_text: "Mock transcription".to_string(),
            latency_ms: 0,
            should_fail: false,
            failure_message: "Mock failure".to_string(),
            engine_type: EngineType::Whisper,
        }
    }

    /// Set the text that should be returned by transcribe
    pub fn with_result_text(mut self, text: impl Into<String>) -> Self {
        self.result_text = text.into();
        self
    }

    /// Set an artificial latency for transcribe calls
    pub fn with_latency(mut self, latency_ms: u64) -> Self {
        self.latency_ms = latency_ms;
        self
    }

    /// Configure the mock to always fail
    pub fn with_failure(mut self, message: impl Into<String>) -> Self {
        self.should_fail = true;
        self.failure_message = message.into();
        self
    }

    /// Set the engine type returned by engine_type()
    pub fn with_engine_type(mut self, engine_type: EngineType) -> Self {
        self.engine_type = engine_type;
        self
    }

    /// Build the mock engine
    pub fn build(self) -> Arc<Self> {
        Arc::new(self)
    }
}

impl Default for MockSttEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl SttEngine for MockSttEngine {
    fn engine_type(&self) -> EngineType {
        self.engine_type
    }

    async fn transcribe(
        &self,
        _request: TranscriptionRequest,
    ) -> Result<TranscriptionResult, String> {
        // Apply artificial latency if configured
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

/// Mock that tracks call count and last request
pub struct MockSttEngineWithTracking {
    inner: MockSttEngine,
    pub call_count: AtomicUsize,
    pub last_request: std::sync::Mutex<Option<TranscriptionRequest>>,
}

impl MockSttEngineWithTracking {
    pub fn new() -> Self {
        Self {
            inner: MockSttEngine::new(),
            call_count: AtomicUsize::new(0),
            last_request: std::sync::Mutex::new(None),
        }
    }

    pub fn with_result_text(self, text: impl Into<String>) -> Self {
        Self {
            inner: self.inner.with_result_text(text),
            ..self
        }
    }

    pub fn with_latency(self, latency_ms: u64) -> Self {
        Self {
            inner: self.inner.with_latency(latency_ms),
            ..self
        }
    }

    pub fn with_failure(self, message: impl Into<String>) -> Self {
        Self {
            inner: self.inner.with_failure(message),
            ..self
        }
    }

    pub fn call_count(&self) -> usize {
        self.call_count.load(Ordering::SeqCst)
    }

    pub fn last_request(&self) -> Option<TranscriptionRequest> {
        self.last_request.lock().unwrap().clone()
    }
}

impl Default for MockSttEngineWithTracking {
    fn default() -> Self {
        Self::new()
    }
}

impl SttEngine for MockSttEngineWithTracking {
    fn engine_type(&self) -> EngineType {
        self.inner.engine_type()
    }

    async fn transcribe(
        &self,
        request: TranscriptionRequest,
    ) -> Result<TranscriptionResult, String> {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        *self.last_request.lock().unwrap() = Some(request.clone());
        self.inner.transcribe(request).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_stt_default() {
        let mock = MockSttEngine::new();
        let request = TranscriptionRequest::new("/tmp/test.wav");

        let result = mock.transcribe(request).await.unwrap();
        assert_eq!(result.text, "Mock transcription");
        assert_eq!(result.engine, EngineType::Whisper);
    }

    #[tokio::test]
    async fn test_mock_stt_with_text() {
        let mock = MockSttEngine::new().with_result_text("Hello world");
        let request = TranscriptionRequest::new("/tmp/test.wav");

        let result = mock.transcribe(request).await.unwrap();
        assert_eq!(result.text, "Hello world");
    }

    #[tokio::test]
    async fn test_mock_stt_with_latency() {
        let mock = MockSttEngine::new().with_latency(50);
        let request = TranscriptionRequest::new("/tmp/test.wav");

        let start = std::time::Instant::now();
        mock.transcribe(request).await.unwrap();
        let elapsed = start.elapsed().as_millis() as u64;

        assert!(
            elapsed >= 50,
            "Expected at least 50ms latency, got {}",
            elapsed
        );
    }

    #[tokio::test]
    async fn test_mock_stt_with_failure() {
        let mock = MockSttEngine::new().with_failure("Test error");
        let request = TranscriptionRequest::new("/tmp/test.wav");

        let result = mock.transcribe(request).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Test error");
    }

    #[tokio::test]
    async fn test_mock_stt_with_engine_type() {
        let mock = MockSttEngine::new().with_engine_type(EngineType::SenseVoice);
        let request = TranscriptionRequest::new("/tmp/test.wav");

        let result = mock.transcribe(request).await.unwrap();
        assert_eq!(result.engine, EngineType::SenseVoice);
    }

    #[tokio::test]
    async fn test_mock_stt_build() {
        let mock = MockSttEngine::new().with_result_text("Built mock").build();

        let request = TranscriptionRequest::new("/tmp/test.wav");
        let result = mock.transcribe(request).await.unwrap();
        assert_eq!(result.text, "Built mock");
    }

    #[tokio::test]
    async fn test_mock_stt_tracking() {
        let mock = Arc::new(MockSttEngineWithTracking::new());

        let request1 = TranscriptionRequest::new("/tmp/test1.wav");
        let request2 = TranscriptionRequest::new("/tmp/test2.wav");

        mock.transcribe(request1.clone()).await.unwrap();
        mock.transcribe(request2.clone()).await.unwrap();

        assert_eq!(mock.call_count(), 2);
        assert_eq!(
            mock.last_request().map(|r| r.audio_path),
            Some(request2.audio_path)
        );
    }
}
