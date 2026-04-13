use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Structured STT context passed to each engine.
///
/// Each cloud engine interprets these fields differently based on its API capabilities:
/// - **Volcengine**: glossary → `context.hotwords[]`, domain → `context.context_data[]`
/// - **ElevenLabs**: all fields combined → `previous_text` (first audio chunk)
/// - **Aliyun Realtime**: currently unused (STT model is opaque)
/// - **Whisper**: all fields combined into `initial_prompt` string
#[derive(Debug, Clone, Default)]
pub struct SttContext {
    pub initial_prompt: Option<String>,
    pub domain: Option<String>,
    pub subdomain: Option<String>,
    pub glossary: Option<String>,
}

/// STT engine type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EngineType {
    Whisper,
    SenseVoice,
    Cloud,
}

impl EngineType {
    pub fn as_str(&self) -> &'static str {
        match self {
            EngineType::Whisper => "whisper",
            EngineType::SenseVoice => "sensevoice",
            EngineType::Cloud => "cloud",
        }
    }

    pub fn all() -> Vec<EngineType> {
        vec![
            EngineType::Whisper,
            EngineType::SenseVoice,
            EngineType::Cloud,
        ]
    }
}

impl std::str::FromStr for EngineType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "whisper" => Ok(EngineType::Whisper),
            "sensevoice" => Ok(EngineType::SenseVoice),
            "cloud" => Ok(EngineType::Cloud),
            _ => Err(format!("Unknown engine type: {}", s)),
        }
    }
}

impl std::fmt::Display for EngineType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Transcription request parameters for batch (local) engines.
///
/// Denoise and VAD are handled upstream by `StreamAudioProcessor` before
/// audio reaches this point, so they are not part of the request.
/// Cloud STT uses the `RecordingConsumer` streaming lifecycle directly
/// (send_chunk + finish), not this batch request.
#[derive(Debug, Clone)]
pub struct TranscriptionRequest {
    /// In-memory f32 samples (16kHz mono, already preprocessed).
    pub samples: Vec<f32>,
    pub language: Option<String>,
    pub model_name: Option<String>,
    pub initial_prompt: Option<String>,
}

impl TranscriptionRequest {
    pub fn new(samples: Vec<f32>) -> Self {
        Self {
            samples,
            language: None,
            model_name: None,
            initial_prompt: None,
        }
    }

    pub fn with_language(mut self, language: impl Into<String>) -> Self {
        self.language = Some(language.into());
        self
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model_name = Some(model.into());
        self
    }

    pub fn with_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.initial_prompt = Some(prompt.into());
        self
    }
}

/// Transcription result
#[derive(Debug, Clone)]
pub struct TranscriptionResult {
    pub text: String,
    pub engine: EngineType,
    /// Total time in milliseconds
    pub total_ms: u64,
    /// Model load time in milliseconds
    pub model_load_ms: Option<u64>,
    /// Audio preprocessing time in milliseconds
    pub preprocess_ms: Option<u64>,
    /// Inference time in milliseconds
    pub inference_ms: Option<u64>,
}

impl TranscriptionResult {
    /// Create basic result with total time only
    pub fn new(text: String, engine: EngineType, total_ms: u64) -> Self {
        Self {
            text,
            engine,
            total_ms,
            model_load_ms: None,
            preprocess_ms: None,
            inference_ms: None,
        }
    }

    /// Create result with detailed metrics
    pub fn with_metrics(
        text: String,
        engine: EngineType,
        total_ms: u64,
        model_load_ms: Option<u64>,
        preprocess_ms: Option<u64>,
        inference_ms: Option<u64>,
    ) -> Self {
        Self {
            text,
            engine,
            total_ms,
            model_load_ms,
            preprocess_ms,
            inference_ms,
        }
    }
}

/// Partial transcription result for streaming callbacks
#[derive(Debug, Clone, Serialize)]
pub struct PartialResult {
    /// The transcribed text so far
    pub text: String,
    /// Whether this result is definite (finalized by VAD)
    pub is_definite: bool,
    /// Whether this is a final result
    pub is_final: bool,
}

/// Callback type for receiving partial transcription results
pub type PartialResultCallback = Arc<dyn Fn(PartialResult) + Send + Sync>;

/// Consumer of PCM audio chunks during a recording session.
///
/// This is the unified abstraction for the recording pipeline's downstream
/// STT processing. Two implementations exist with fundamentally different
/// semantics:
///
/// - **`BufferingConsumer`** (local models): `send_chunk` buffers PCM in memory;
///   `finish` batch-transcribes via `UnifiedEngineManager`. No partial results.
///
/// - **`StreamingConsumer`** (cloud STT): `send_chunk` forwards PCM to a live
///   WebSocket; `finish` signals end-of-stream and awaits the final result.
///   Partial results arrive via the callback set before the first `send_chunk`.
///
/// # Lifecycle
///
/// ```text
/// 1. Create consumer (connect is implicit for streaming)
/// 2. [optional] set_partial_callback (streaming only)
/// 3. send_chunk × N   (per audio chunk from recorder)
/// 4. finish()          (recording stopped → final transcription)
/// ```
#[async_trait]
pub trait RecordingConsumer: Send + Sync {
    /// Feed one audio chunk (16-bit PCM, 16kHz mono) into the consumer.
    async fn send_chunk(&self, pcm_data: Vec<i16>) -> Result<(), String>;

    /// Signal end-of-recording and obtain the final transcription text.
    async fn finish(&self) -> Result<String, String>;

    /// Register a callback for partial transcription results.
    ///
    /// Only meaningful for streaming consumers; buffering consumers ignore this.
    fn set_partial_callback(&mut self, _callback: PartialResultCallback) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transcription_request_new() {
        let request = TranscriptionRequest::new(vec![0.0; 16000]);
        assert_eq!(request.samples.len(), 16000);
        assert!(request.language.is_none());
        assert!(request.model_name.is_none());
        assert!(request.initial_prompt.is_none());
    }

    #[test]
    fn test_transcription_request_with_model() {
        let request = TranscriptionRequest::new(vec![]).with_model("base");
        assert_eq!(request.model_name, Some("base".to_string()));
    }

    #[test]
    fn test_transcription_request_with_language() {
        let request = TranscriptionRequest::new(vec![]).with_language("en");
        assert_eq!(request.language, Some("en".to_string()));
    }

    #[test]
    fn test_transcription_result_new() {
        let result = TranscriptionResult::new("Hello world".to_string(), EngineType::Whisper, 1000);
        assert_eq!(result.text, "Hello world");
        assert_eq!(result.engine, EngineType::Whisper);
        assert_eq!(result.total_ms, 1000);
        assert!(result.model_load_ms.is_none());
        assert!(result.preprocess_ms.is_none());
        assert!(result.inference_ms.is_none());
    }

    #[test]
    fn test_transcription_result_with_metrics() {
        let result = TranscriptionResult::with_metrics(
            "Hello world".to_string(),
            EngineType::Cloud,
            500,
            Some(100),
            Some(50),
            Some(350),
        );
        assert_eq!(result.text, "Hello world");
        assert_eq!(result.engine, EngineType::Cloud);
        assert_eq!(result.total_ms, 500);
        assert_eq!(result.model_load_ms, Some(100));
        assert_eq!(result.preprocess_ms, Some(50));
        assert_eq!(result.inference_ms, Some(350));
    }

    #[test]
    fn test_engine_type_values() {
        assert_eq!(EngineType::Whisper.as_str(), "whisper");
        assert_eq!(EngineType::SenseVoice.as_str(), "sensevoice");
        assert_eq!(EngineType::Cloud.as_str(), "cloud");

        let all = EngineType::all();
        assert_eq!(all.len(), 3);
        assert!(all.contains(&EngineType::Whisper));
        assert!(all.contains(&EngineType::SenseVoice));
        assert!(all.contains(&EngineType::Cloud));

        // Test FromStr
        assert_eq!(
            "whisper".parse::<EngineType>().unwrap(),
            EngineType::Whisper
        );
        assert_eq!(
            "SENSEVOICE".parse::<EngineType>().unwrap(),
            EngineType::SenseVoice
        );
        assert!("unknown".parse::<EngineType>().is_err());
    }
}
