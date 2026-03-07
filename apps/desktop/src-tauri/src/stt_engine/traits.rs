use std::path::PathBuf;
use serde::{Deserialize, Serialize};

/// STT engine type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EngineType {
    Whisper,
    SenseVoice,
}

impl EngineType {
    pub fn as_str(&self) -> &'static str {
        match self {
            EngineType::Whisper => "whisper",
            EngineType::SenseVoice => "sensevoice",
        }
    }

    pub fn all() -> Vec<EngineType> {
        vec![EngineType::Whisper, EngineType::SenseVoice]
    }
}

impl std::str::FromStr for EngineType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "whisper" => Ok(EngineType::Whisper),
            "sensevoice" => Ok(EngineType::SenseVoice),
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
}

impl TranscriptionRequest {
    pub fn new(audio_path: impl Into<PathBuf>) -> Self {
        Self {
            audio_path: audio_path.into(),
            language: None,
            model_name: None,
            initial_prompt: None,
        }
    }

    pub fn with_language(mut self, language: impl Into<String>) -> Self {
        self.language = Some(language.into());
        self
    }

    pub fn with_model(mut self, model_name: impl Into<String>) -> Self {
        self.model_name = Some(model_name.into());
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

/// STT engine unified interface
#[allow(async_fn_in_trait)]
pub trait SttEngine: Send + Sync {
    /// Engine type
    fn engine_type(&self) -> EngineType;

    /// Async transcription
    async fn transcribe(&self, request: TranscriptionRequest) -> Result<TranscriptionResult, String>;
}
