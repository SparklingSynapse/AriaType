use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Polish engine type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PolishEngineType {
    Qwen,
    Lfm,
    Gemma,
    Cloud,
}

impl PolishEngineType {
    pub fn as_str(&self) -> &'static str {
        match self {
            PolishEngineType::Qwen => "qwen",
            PolishEngineType::Lfm => "lfm",
            PolishEngineType::Gemma => "gemma",
            PolishEngineType::Cloud => "cloud",
        }
    }

    pub fn all() -> Vec<PolishEngineType> {
        vec![
            PolishEngineType::Qwen,
            PolishEngineType::Lfm,
            PolishEngineType::Gemma,
            PolishEngineType::Cloud,
        ]
    }
}

impl std::str::FromStr for PolishEngineType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "qwen" => Ok(PolishEngineType::Qwen),
            "lfm" => Ok(PolishEngineType::Lfm),
            "gemma" => Ok(PolishEngineType::Gemma),
            "cloud" => Ok(PolishEngineType::Cloud),
            _ => Err(format!("Unknown polish engine type: {}", s)),
        }
    }
}

impl std::fmt::Display for PolishEngineType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// System context provided to polish engines.
///
/// Contains structured context information that engines can consume flexibly:
/// - `system_prompt`: Complete prompt assembled by upper layer (template + app system info)
/// - `window_context`: OCR text from focused window at recording start
///
/// Engines decide how to incorporate these fields into their final prompt.
#[derive(Debug, Clone, Default)]
pub struct SystemContext {
    pub system_prompt: String,
    pub window_context: Option<String>,
}

impl SystemContext {
    pub fn new(system_prompt: impl Into<String>) -> Self {
        Self {
            system_prompt: system_prompt.into(),
            window_context: None,
        }
    }

    pub fn with_window_context(mut self, ctx: impl Into<String>) -> Self {
        self.window_context = Some(ctx.into());
        self
    }

    /// Resolve effective prompt by prepending window context if present.
    pub fn effective_prompt(&self) -> String {
        match &self.window_context {
            Some(ctx) if !ctx.is_empty() => format!(
                "The user is currently looking at a window containing the following text:\n\"\"\"\n{}\n\"\"\"\n\n{}",
                ctx,
                self.system_prompt
            ),
            _ => self.system_prompt.clone(),
        }
    }
}

/// Polish request parameters
#[derive(Debug, Clone)]
pub struct PolishRequest {
    pub text: String,
    pub system_context: SystemContext,
    pub language: String,
    pub model_name: Option<String>,
}

impl PolishRequest {
    pub fn new(
        text: impl Into<String>,
        system_prompt: impl Into<String>,
        language: impl Into<String>,
    ) -> Self {
        Self {
            text: text.into(),
            system_context: SystemContext::new(system_prompt),
            language: language.into(),
            model_name: None,
        }
    }

    pub fn with_model(mut self, model_name: impl Into<String>) -> Self {
        self.model_name = Some(model_name.into());
        self
    }

    pub fn with_window_context(mut self, ctx: impl Into<String>) -> Self {
        self.system_context.window_context = Some(ctx.into());
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
        assert_eq!(all.len(), 4);
        assert!(all.contains(&PolishEngineType::Qwen));
        assert!(all.contains(&PolishEngineType::Lfm));
        assert!(all.contains(&PolishEngineType::Gemma));
        assert!(all.contains(&PolishEngineType::Cloud));
    }

    #[test]
    fn test_polish_engine_type_from_str() {
        assert_eq!(
            "qwen".parse::<PolishEngineType>().unwrap(),
            PolishEngineType::Qwen
        );
        assert_eq!(
            "lfm".parse::<PolishEngineType>().unwrap(),
            PolishEngineType::Lfm
        );
        assert_eq!(
            "QWEN".parse::<PolishEngineType>().unwrap(),
            PolishEngineType::Qwen
        );
        assert_eq!(
            "LFM".parse::<PolishEngineType>().unwrap(),
            PolishEngineType::Lfm
        );
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
        assert_eq!(request.system_context.system_prompt, "system prompt");
        assert_eq!(request.language, "en");
        assert!(request.model_name.is_none());
        assert!(request.system_context.window_context.is_none());
    }

    #[test]
    fn test_polish_request_with_model() {
        let request = PolishRequest::new("test", "prompt", "zh").with_model("model.gguf");
        assert_eq!(request.text, "test");
        assert_eq!(request.model_name, Some("model.gguf".to_string()));
    }

    #[test]
    fn test_polish_request_with_window_context() {
        let request = PolishRequest::new("test", "prompt", "en")
            .with_window_context("window text");
        assert_eq!(request.system_context.window_context, Some("window text".to_string()));
    }

    #[test]
    fn test_system_context_effective_prompt() {
        let ctx = SystemContext::new("base prompt");
        assert_eq!(ctx.effective_prompt(), "base prompt");

        let ctx_with_window = SystemContext::new("base prompt")
            .with_window_context("window content");
        let effective = ctx_with_window.effective_prompt();
        assert!(effective.contains("window content"));
        assert!(effective.contains("base prompt"));
    }

    #[test]
    fn test_polish_result_new() {
        let result = PolishResult::new("polished text".to_string(), PolishEngineType::Qwen, 1500);
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
