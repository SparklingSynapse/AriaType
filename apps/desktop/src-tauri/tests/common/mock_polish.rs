//! Mock Polish engine for testing
//!
//! Provides a configurable mock implementation of the Polish engine trait
//! for use in integration and unit tests.

use ariatype_lib::polish_engine::{PolishEngine, PolishEngineType, PolishRequest, PolishResult};
use async_trait::async_trait;

/// Mock Polish engine with configurable behavior
///
/// # Example
/// ```rust
/// use common::MockPolishEngine;
///
/// let mock = MockPolishEngine::new()
///     .with_result_text("Polished text")
///     .with_latency(50);
///
/// let result = mock.polish(request).await;
/// ```
pub struct MockPolishEngine {
    result_text: String,
    latency_ms: u64,
    should_fail: bool,
    failure_message: String,
    engine_type: PolishEngineType,
}

impl MockPolishEngine {
    /// Create a new MockPolishEngine with default values
    pub fn new() -> Self {
        Self {
            result_text: "Mock polished text".to_string(),
            latency_ms: 0,
            should_fail: false,
            failure_message: "Mock polish failure".to_string(),
            engine_type: PolishEngineType::Qwen,
        }
    }

    /// Set the text that should be returned by polish
    pub fn with_result_text(mut self, text: impl Into<String>) -> Self {
        self.result_text = text.into();
        self
    }

    /// Set an artificial latency for polish calls
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
    pub fn with_engine_type(mut self, engine_type: PolishEngineType) -> Self {
        self.engine_type = engine_type;
        self
    }

    /// Build the mock engine as Arc for trait object compatibility
    pub fn build(self) -> std::sync::Arc<Self> {
        std::sync::Arc::new(self)
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
        // Apply artificial latency if configured
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

/// Mock that tracks call count and last request
pub struct MockPolishEngineWithTracking {
    inner: MockPolishEngine,
    pub call_count: std::sync::atomic::AtomicUsize,
    pub last_request: std::sync::Mutex<Option<PolishRequest>>,
}

impl MockPolishEngineWithTracking {
    pub fn new() -> Self {
        Self {
            inner: MockPolishEngine::new(),
            call_count: std::sync::atomic::AtomicUsize::new(0),
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
        self.call_count.load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn last_request(&self) -> Option<PolishRequest> {
        self.last_request.lock().unwrap().clone()
    }
}

impl Default for MockPolishEngineWithTracking {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PolishEngine for MockPolishEngineWithTracking {
    fn engine_type(&self) -> PolishEngineType {
        self.inner.engine_type()
    }

    async fn polish(&self, request: PolishRequest) -> Result<PolishResult, String> {
        self.call_count
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        *self.last_request.lock().unwrap() = Some(request.clone());
        self.inner.polish(request).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ariatype_lib::polish_engine::PolishEngineType;

    #[tokio::test]
    async fn test_mock_polish_default() {
        let mock = MockPolishEngine::new();
        let request = PolishRequest::new("原始文本", "系统提示", "zh");

        let result = mock.polish(request).await.unwrap();
        assert_eq!(result.text, "Mock polished text");
        assert_eq!(result.engine, PolishEngineType::Qwen);
    }

    #[tokio::test]
    async fn test_mock_polish_with_text() {
        let mock = MockPolishEngine::new().with_result_text("Polished output");
        let request = PolishRequest::new("input", "prompt", "en");

        let result = mock.polish(request).await.unwrap();
        assert_eq!(result.text, "Polished output");
    }

    #[tokio::test]
    async fn test_mock_polish_with_latency() {
        let mock = MockPolishEngine::new().with_latency(50);
        let request = PolishRequest::new("input", "prompt", "en");

        let start = std::time::Instant::now();
        mock.polish(request).await.unwrap();
        let elapsed = start.elapsed().as_millis() as u64;

        assert!(
            elapsed >= 50,
            "Expected at least 50ms latency, got {}",
            elapsed
        );
    }

    #[tokio::test]
    async fn test_mock_polish_with_failure() {
        let mock = MockPolishEngine::new().with_failure("Polish error");
        let request = PolishRequest::new("input", "prompt", "en");

        let result = mock.polish(request).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Polish error");
    }

    #[tokio::test]
    async fn test_mock_polish_with_engine_type() {
        let mock = MockPolishEngine::new().with_engine_type(PolishEngineType::Lfm);
        let request = PolishRequest::new("input", "prompt", "en");

        let result = mock.polish(request).await.unwrap();
        assert_eq!(result.engine, PolishEngineType::Lfm);
    }

    #[tokio::test]
    async fn test_mock_polish_build() {
        let mock = MockPolishEngine::new()
            .with_result_text("Built mock")
            .build();

        let request = PolishRequest::new("input", "prompt", "en");
        let result = mock.polish(request).await.unwrap();
        assert_eq!(result.text, "Built mock");
    }

    #[tokio::test]
    async fn test_mock_polish_tracking() {
        let mock = std::sync::Arc::new(MockPolishEngineWithTracking::new());

        let request1 = PolishRequest::new("input1", "prompt", "en");
        let request2 = PolishRequest::new("input2", "prompt", "en");

        mock.polish(request1.clone()).await.unwrap();
        mock.polish(request2.clone()).await.unwrap();

        assert_eq!(mock.call_count(), 2);
    }
}
