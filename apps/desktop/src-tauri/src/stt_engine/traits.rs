use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;

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

/// Transcription request parameters
#[derive(Debug, Clone)]
pub struct TranscriptionRequest {
    pub audio_path: PathBuf,
    pub language: Option<String>,
    pub model_name: Option<String>,
    pub initial_prompt: Option<String>,
    pub denoise_mode: String,
    pub vad_enabled: bool,
    pub cloud_config: Option<crate::commands::settings::CloudSttConfig>,
}

impl TranscriptionRequest {
    pub fn new(audio_path: impl Into<PathBuf>) -> Self {
        Self {
            audio_path: audio_path.into(),
            language: None,
            model_name: None,
            initial_prompt: None,
            denoise_mode: "auto".to_string(),
            vad_enabled: true,
            cloud_config: None,
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

    pub fn with_denoise_mode(mut self, mode: impl Into<String>) -> Self {
        self.denoise_mode = mode.into();
        self
    }

    pub fn with_vad_enabled(mut self, enabled: bool) -> Self {
        self.vad_enabled = enabled;
        self
    }

    pub fn with_cloud_config(mut self, config: crate::commands::settings::CloudSttConfig) -> Self {
        self.cloud_config = Some(config);
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

/// Streaming STT engine trait for cloud engines with streaming-only lifecycle.
///
/// This trait defines the lifecycle for streaming speech-to-text engines:
/// - `start()` initializes the streaming session
/// - `send_chunk()` sends audio chunks as they become available
/// - `finish()` finalizes the session and returns the complete transcription
///
/// # Usage Example
///
/// ```ignore
/// let mut engine = MyStreamingEngine::new(config);
/// engine.set_partial_callback(|result| {
///     println!("Partial: {}", result.text);
/// });
///
/// engine.start().await?;
/// for chunk in audio_chunks {
///     engine.send_chunk(chunk).await?;
/// }
/// let final_text = engine.finish().await?;
/// ```
#[async_trait]
pub trait StreamingSttEngine: Send + Sync {
    /// Initialize and start the streaming session.
    ///
    /// Returns `Ok(())` if the session was started successfully,
    /// or an error string if initialization failed.
    async fn start(&mut self) -> Result<(), String>;

    /// Send an audio chunk to the streaming engine.
    ///
    /// The `pcm_data` should contain 16-bit signed PCM audio samples.
    /// Returns `Ok(())` if the chunk was sent successfully.
    async fn send_chunk(&self, pcm_data: Vec<i16>) -> Result<(), String>;

    /// Finish the streaming session and get the final transcription.
    ///
    /// This signals the end of audio input and returns the complete
    /// transcribed text. Returns `Ok(text)` on success or an error string.
    async fn finish(&self) -> Result<String, String>;

    /// Set the callback for receiving partial transcription results.
    ///
    /// The callback will be invoked whenever a partial result is available
    /// during streaming.
    fn set_partial_callback(&mut self, callback: PartialResultCallback);

    /// Get the audio sender channel for this streaming session.
    ///
    /// Returns `Some(Sender)` if the engine supports external audio feeding,
    /// or `None` if the engine manages audio internally.
    async fn get_audio_sender(&self) -> Option<mpsc::Sender<Vec<i16>>>;
}

/// STT engine unified interface
#[allow(async_fn_in_trait)]
pub trait SttEngine: Send + Sync {
    /// Engine type
    fn engine_type(&self) -> EngineType;

    /// Async transcription
    async fn transcribe(
        &self,
        request: TranscriptionRequest,
    ) -> Result<TranscriptionResult, String>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transcription_request_new() {
        let request = TranscriptionRequest::new("/path/to/audio.wav");
        assert_eq!(request.audio_path, PathBuf::from("/path/to/audio.wav"));
        assert!(request.language.is_none());
        assert!(request.model_name.is_none());
        assert!(request.initial_prompt.is_none());
        assert_eq!(request.denoise_mode, "auto");
        assert!(request.vad_enabled);
        assert!(request.cloud_config.is_none());
    }

    #[test]
    fn test_transcription_request_with_model() {
        let request = TranscriptionRequest::new("/path/to/audio.wav").with_model("base");
        assert_eq!(request.model_name, Some("base".to_string()));
    }

    #[test]
    fn test_transcription_request_with_language() {
        let request = TranscriptionRequest::new("/path/to/audio.wav").with_language("en");
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
