use serde::{Deserialize, Serialize};
use async_trait::async_trait;

/// Polish engine type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PolishEngineType {
    Qwen,
    Lfm,
}

impl PolishEngineType {
    pub fn as_str(&self) -> &'static str {
        match self {
            PolishEngineType::Qwen => "qwen",
            PolishEngineType::Lfm => "lfm",
        }
    }

    pub fn all() -> Vec<PolishEngineType> {
        vec![PolishEngineType::Qwen, PolishEngineType::Lfm]
    }
}

impl std::str::FromStr for PolishEngineType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "qwen" => Ok(PolishEngineType::Qwen),
            "lfm" => Ok(PolishEngineType::Lfm),
            _ => Err(format!("Unknown polish engine type: {}", s)),
        }
    }
}

impl std::fmt::Display for PolishEngineType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Polish request parameters
#[derive(Debug, Clone)]
pub struct PolishRequest {
    pub text: String,
    pub system_prompt: String,
    pub language: String,
    pub model_name: Option<String>,
}

impl PolishRequest {
    pub fn new(text: impl Into<String>, system_prompt: impl Into<String>, language: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            system_prompt: system_prompt.into(),
            language: language.into(),
            model_name: None,
        }
    }

    pub fn with_model(mut self, model_name: impl Into<String>) -> Self {
        self.model_name = Some(model_name.into());
        self
    }
}

/// Polish result
#[derive(Debug, Clone)]
pub struct PolishResult {
    pub text: String,
    pub engine: PolishEngineType,
    /// Total time in milliseconds
    pub total_ms: u64,
    /// Model load time in milliseconds
    pub model_load_ms: Option<u64>,
    /// Inference time in milliseconds
    pub inference_ms: Option<u64>,
}

impl PolishResult {
    /// Create basic result with total time only
    pub fn new(text: String, engine: PolishEngineType, total_ms: u64) -> Self {
        Self {
            text,
            engine,
            total_ms,
            model_load_ms: None,
            inference_ms: None,
        }
    }

    /// Create result with detailed metrics
    pub fn with_metrics(
        text: String,
        engine: PolishEngineType,
        total_ms: u64,
        model_load_ms: Option<u64>,
        inference_ms: Option<u64>,
    ) -> Self {
        Self {
            text,
            engine,
            total_ms,
            model_load_ms,
            inference_ms,
        }
    }
}

/// Polish engine unified interface
#[async_trait]
pub trait PolishEngine: Send + Sync {
    /// Engine type
    fn engine_type(&self) -> PolishEngineType;

    /// Async polish
    async fn polish(&self, request: PolishRequest) -> Result<PolishResult, String>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_polish_engine_type_as_str() {
        assert_eq!(PolishEngineType::Qwen.as_str(), "qwen");
        assert_eq!(PolishEngineType::Lfm.as_str(), "lfm");
    }

    #[test]
    fn test_polish_engine_type_all() {
        let all = PolishEngineType::all();
        assert_eq!(all.len(), 2);
        assert!(all.contains(&PolishEngineType::Qwen));
        assert!(all.contains(&PolishEngineType::Lfm));
    }

    #[test]
    fn test_polish_engine_type_from_str() {
        assert_eq!("qwen".parse::<PolishEngineType>().unwrap(), PolishEngineType::Qwen);
        assert_eq!("lfm".parse::<PolishEngineType>().unwrap(), PolishEngineType::Lfm);
        assert_eq!("QWEN".parse::<PolishEngineType>().unwrap(), PolishEngineType::Qwen);
        assert_eq!("LFM".parse::<PolishEngineType>().unwrap(), PolishEngineType::Lfm);
    }

    #[test]
    fn test_polish_engine_type_from_str_invalid() {
        assert!("invalid".parse::<PolishEngineType>().is_err());
        assert!("".parse::<PolishEngineType>().is_err());
        assert!("gpt".parse::<PolishEngineType>().is_err());
    }

    #[test]
    fn test_polish_engine_type_display() {
        assert_eq!(format!("{}", PolishEngineType::Qwen), "qwen");
        assert_eq!(format!("{}", PolishEngineType::Lfm), "lfm");
    }

    #[test]
    fn test_polish_engine_type_serde() {
        // Test serialization
        let qwen = PolishEngineType::Qwen;
        let json = serde_json::to_string(&qwen).unwrap();
        assert_eq!(json, "\"qwen\"");

        let lfm = PolishEngineType::Lfm;
        let json = serde_json::to_string(&lfm).unwrap();
        assert_eq!(json, "\"lfm\"");

        // Test deserialization
        let qwen: PolishEngineType = serde_json::from_str("\"qwen\"").unwrap();
        assert_eq!(qwen, PolishEngineType::Qwen);

        let lfm: PolishEngineType = serde_json::from_str("\"lfm\"").unwrap();
        assert_eq!(lfm, PolishEngineType::Lfm);
    }

    #[test]
    fn test_polish_request_new() {
        let request = PolishRequest::new("test text", "system prompt", "en");
        assert_eq!(request.text, "test text");
        assert_eq!(request.system_prompt, "system prompt");
        assert_eq!(request.language, "en");
        assert!(request.model_name.is_none());
    }

    #[test]
    fn test_polish_request_with_model() {
        let request = PolishRequest::new("test", "prompt", "zh")
            .with_model("model.gguf");
        assert_eq!(request.text, "test");
        assert_eq!(request.model_name, Some("model.gguf".to_string()));
    }

    #[test]
    fn test_polish_result_new() {
        let result = PolishResult::new(
            "polished text".to_string(),
            PolishEngineType::Qwen,
            1500,
        );
        assert_eq!(result.text, "polished text");
        assert_eq!(result.engine, PolishEngineType::Qwen);
        assert_eq!(result.total_ms, 1500);
        assert!(result.model_load_ms.is_none());
        assert!(result.inference_ms.is_none());
    }

    #[test]
    fn test_polish_result_with_metrics() {
        let result = PolishResult::with_metrics(
            "polished".to_string(),
            PolishEngineType::Lfm,
            2000,
            Some(500),
            Some(1500),
        );
        assert_eq!(result.text, "polished");
        assert_eq!(result.engine, PolishEngineType::Lfm);
        assert_eq!(result.total_ms, 2000);
        assert_eq!(result.model_load_ms, Some(500));
        assert_eq!(result.inference_ms, Some(1500));
    }
}
