use crate::polish_engine::traits::{PolishEngine, PolishEngineType, PolishRequest, PolishResult};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, error, info};

const CLOUD_POLISH_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone)]
pub struct CloudProviderConfig {
    pub provider_type: String,
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub enable_thinking: bool,
}

pub struct CloudPolishEngine {
    config: CloudProviderConfig,
    client: Client,
}

pub const CORE_POLISH_CONSTRAINT: &str = r#"You are a text polishing assistant.

ESSENTIAL CONSTRAINTS (MUST follow for ALL tasks):
1. SEMANTIC PRESERVATION: Output MUST convey the SAME meaning as input. Do NOT change the speaker's intent, add interpretations, or hallucinate information, or answer questions.
2. TASK BOUNDARY: Only perform text polishing. Do NOT summarize, expand, or execute tasks unrelated to text polishing.
3. OUTPUT FORMAT: Output ONLY the polished text. No explanations or meta-commentary.

DEFAULT BEHAVIOR (unless user rules specify otherwise):
- Keep output in the SAME language as input

Follow the user rules below for the specific polishing style."#;

impl CloudPolishEngine {
    pub fn new(config: CloudProviderConfig) -> Self {
        Self {
            config,
            client: Client::builder()
                .timeout(CLOUD_POLISH_TIMEOUT)
                .build()
                .expect("cloud polish reqwest client should build"),
        }
    }

    fn format_request_error(&self, stage: &str, url: &str, error: reqwest::Error) -> String {
        if error.is_timeout() {
            error!(
                provider = %self.config.provider_type,
                model = %self.config.model,
                url = %url,
                timeout_secs = CLOUD_POLISH_TIMEOUT.as_secs(),
                stage = stage,
                error = %error,
                "cloud_polish_request_timeout"
            );
            format!(
                "Cloud polish request timed out after {}s during {} (provider={}, model={}, url={})",
                CLOUD_POLISH_TIMEOUT.as_secs(),
                stage,
                self.config.provider_type,
                self.config.model,
                url,
            )
        } else {
            error!(
                provider = %self.config.provider_type,
                model = %self.config.model,
                url = %url,
                stage = stage,
                error = %error,
                "cloud_polish_request_failed"
            );
            format!(
                "{} failed (provider={}, model={}, url={}): {}",
                stage, self.config.provider_type, self.config.model, url, error
            )
        }
    }

    fn get_api_url(&self) -> String {
        if !self.config.base_url.is_empty() {
            let base = self.config.base_url.trim_end_matches('/');
            // Check if base_url already ends with a valid API path
            // Anthropic: /v1/messages, OpenAI: /v1/chat/completions
            if base.ends_with("/messages") || base.ends_with("/chat/completions") {
                base.to_string()
            } else if base.ends_with("/v1") {
                // base_url already has /v1, append the specific endpoint
                match self.config.provider_type.as_str() {
                    "anthropic" => format!("{}/messages", base),
                    "openai" => format!("{}/chat/completions", base),
                    _ => format!("{}/chat/completions", base),
                }
            } else {
                // base_url is just a domain, add full path
                match self.config.provider_type.as_str() {
                    "anthropic" => format!("{}/v1/messages", base),
                    "openai" => format!("{}/v1/chat/completions", base),
                    _ => format!("{}/v1/chat/completions", base),
                }
            }
        } else {
            match self.config.provider_type.as_str() {
                "anthropic" => "https://api.anthropic.com/v1/messages".to_string(),
                "openai" => "https://api.openai.com/v1/chat/completions".to_string(),
                _ => format!("{}/v1/messages", self.config.base_url.trim_end_matches('/')),
            }
        }
    }

    fn get_auth_header(&self) -> (String, String) {
        match self.config.provider_type.as_str() {
            "anthropic" => ("x-api-key".to_string(), self.config.api_key.clone()),
            "openai" => (
                "Authorization".to_string(),
                format!("Bearer {}", self.config.api_key),
            ),
            _ => (
                "Authorization".to_string(),
                format!("Bearer {}", self.config.api_key),
            ),
        }
    }

    fn is_coding_plan_endpoint(&self) -> bool {
        self.config
            .base_url
            .contains("coding.dashscope.aliyuncs.com")
            || self
                .config
                .base_url
                .contains("coding-intl.dashscope.aliyuncs.com")
    }

    fn get_user_agent(&self) -> &'static str {
        if self.is_coding_plan_endpoint() {
            "opencode/1.0.0"
        } else {
            concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"))
        }
    }

    async fn call_anthropic_api(
        &self,
        system_prompt: &str,
        user_message: &str,
    ) -> Result<String, String> {
        let url = self.get_api_url();
        let (header_name, header_value) = self.get_auth_header();

        #[derive(Serialize)]
        struct Message {
            role: String,
            content: String,
        }

        #[derive(Serialize)]
        struct RequestBody {
            model: String,
            max_tokens: u32,
            system: String,
            messages: Vec<Message>,
        }

        let body = RequestBody {
            model: self.config.model.clone(),
            max_tokens: 4096,
            system: system_prompt.to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: user_message.to_string(),
            }],
        };

        debug!(
            url = %url,
            model = %self.config.model,
            timeout_secs = CLOUD_POLISH_TIMEOUT.as_secs(),
            "cloud_polish_anthropic_request_start"
        );

        let response = self
            .client
            .post(&url)
            .header(&header_name, &header_value)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .header("User-Agent", self.get_user_agent())
            .json(&body)
            .send()
            .await
            .map_err(|e| self.format_request_error("HTTP request", &url, e))?;

        let status = response.status();
        let response_text = response
            .text()
            .await
            .map_err(|e| self.format_request_error("HTTP response read", &url, e))?;

        if !status.is_success() {
            error!(status = %status, body = %response_text, "cloud_polish_api_error");
            return Err(format!("API error ({}): {}", status, response_text));
        }

        #[derive(Deserialize)]
        struct ContentBlock {
            text: Option<String>,
        }

        #[derive(Deserialize)]
        struct ResponseBody {
            content: Vec<ContentBlock>,
        }

        let response_body: ResponseBody = serde_json::from_str(&response_text)
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        let text = response_body
            .content
            .first()
            .and_then(|c| c.text.clone())
            .unwrap_or_default();

        Ok(text)
    }

    async fn call_openai_api(
        &self,
        system_prompt: &str,
        user_message: &str,
    ) -> Result<String, String> {
        let url = self.get_api_url();
        let (header_name, header_value) = self.get_auth_header();

        let mut body = serde_json::json!({
            "model": self.config.model,
            "max_tokens": 4096,
            "messages": [
                {
                    "role": "system",
                    "content": system_prompt
                },
                {
                    "role": "user",
                    "content": user_message
                }
            ]
        });

        if self.is_coding_plan_endpoint() {
            body["enable_thinking"] = serde_json::json!(self.config.enable_thinking);
        }

        debug!(
            url = %url,
            model = %self.config.model,
            enable_thinking = self.config.enable_thinking,
            timeout_secs = CLOUD_POLISH_TIMEOUT.as_secs(),
            "cloud_polish_openai_request_start"
        );

        let response = self
            .client
            .post(&url)
            .header(&header_name, &header_value)
            .header("Content-Type", "application/json")
            .header("User-Agent", self.get_user_agent())
            .json(&body)
            .send()
            .await
            .map_err(|e| self.format_request_error("HTTP request", &url, e))?;

        let status = response.status();
        let response_text = response
            .text()
            .await
            .map_err(|e| self.format_request_error("HTTP response read", &url, e))?;

        if !status.is_success() {
            error!(status = %status, body = %response_text, "cloud_polish_api_error");
            return Err(format!("API error ({}): {}", status, response_text));
        }

        #[derive(Deserialize)]
        struct Choice {
            message: OpenAIResponseMessage,
        }

        #[derive(Deserialize)]
        struct OpenAIResponseMessage {
            content: String,
        }

        #[derive(Deserialize)]
        struct ResponseBody {
            choices: Vec<Choice>,
        }

        let response_body: ResponseBody = serde_json::from_str(&response_text)
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        let text = response_body
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        Ok(text)
    }
}

#[async_trait]
impl PolishEngine for CloudPolishEngine {
    fn engine_type(&self) -> PolishEngineType {
        PolishEngineType::Cloud
    }

    async fn polish(&self, request: PolishRequest) -> Result<PolishResult, String> {
        if self.config.api_key.is_empty() {
            return Err("Cloud polish API key not configured".to_string());
        }

        if self.config.model.is_empty() {
            return Err("Cloud polish model not configured".to_string());
        }

        let t0 = std::time::Instant::now();
        let input_text = request.text.clone();
        let input_chars = input_text.len();

        let system_prompt = if request.system_prompt.is_empty() {
            CORE_POLISH_CONSTRAINT.to_string()
        } else {
            format!(
                "{}\n\nUSER RULES:\n{}",
                CORE_POLISH_CONSTRAINT, request.system_prompt
            )
        };

        info!(
            provider = %self.config.provider_type,
            model = %self.config.model,
            base_url = %self.config.base_url,
            enable_thinking = self.config.enable_thinking,
            timeout_secs = CLOUD_POLISH_TIMEOUT.as_secs(),
            system_prompt = %system_prompt,
            input_text = %input_text,
            input_len = input_chars,
            "cloud_polish_request"
        );

        let result = match self.config.provider_type.as_str() {
            "anthropic" => self.call_anthropic_api(&system_prompt, &input_text).await?,
            "openai" => self.call_openai_api(&system_prompt, &input_text).await?,
            _ => self.call_openai_api(&system_prompt, &input_text).await?,
        };

        let total_ms = t0.elapsed().as_millis() as u64;
        let output_chars = result.len();

        info!(
            provider = %self.config.provider_type,
            model = %self.config.model,
            input_len = input_chars,
            output_text = %result,
            output_len = output_chars,
            duration_ms = total_ms,
            "cloud_polish_complete"
        );

        Ok(PolishResult::new(result, PolishEngineType::Cloud, total_ms))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config(
        provider_type: &str,
        api_key: &str,
        base_url: &str,
        model: &str,
    ) -> CloudProviderConfig {
        CloudProviderConfig {
            provider_type: provider_type.to_string(),
            api_key: api_key.to_string(),
            base_url: base_url.to_string(),
            model: model.to_string(),
            enable_thinking: false,
        }
    }

    #[test]
    fn test_cloud_polish_engine_type() {
        let config = test_config("anthropic", "test-key", "", "claude-3-sonnet");
        let engine = CloudPolishEngine::new(config);
        assert_eq!(engine.engine_type(), PolishEngineType::Cloud);
    }

    #[test]
    fn test_get_api_url_anthropic() {
        let config = test_config("anthropic", "test", "", "claude-3");
        let engine = CloudPolishEngine::new(config);
        assert_eq!(
            engine.get_api_url(),
            "https://api.anthropic.com/v1/messages"
        );
    }

    #[test]
    fn test_get_api_url_openai() {
        let config = test_config("openai", "test", "", "gpt-4");
        let engine = CloudPolishEngine::new(config);
        assert_eq!(
            engine.get_api_url(),
            "https://api.openai.com/v1/chat/completions"
        );
    }

    #[test]
    fn test_get_api_url_custom_domain_only() {
        let config = test_config("custom", "test", "https://custom.api.com/", "custom-model");
        let engine = CloudPolishEngine::new(config);
        assert_eq!(
            engine.get_api_url(),
            "https://custom.api.com/v1/chat/completions"
        );
    }

    #[test]
    fn test_get_api_url_custom_with_v1_suffix_anthropic() {
        let config = test_config(
            "anthropic",
            "test",
            "https://coding.dashscope.aliyuncs.com/apps/anthropic/v1",
            "qwen3.5-plus",
        );
        let engine = CloudPolishEngine::new(config);
        assert_eq!(
            engine.get_api_url(),
            "https://coding.dashscope.aliyuncs.com/apps/anthropic/v1/messages"
        );
    }

    #[test]
    fn test_get_api_url_custom_with_v1_suffix_openai() {
        let config = test_config(
            "openai",
            "test",
            "https://dashscope.aliyuncs.com/compatible-mode/v1",
            "qwen3.5-plus",
        );
        let engine = CloudPolishEngine::new(config);
        assert_eq!(
            engine.get_api_url(),
            "https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions"
        );
    }

    #[test]
    fn test_get_api_url_already_has_messages() {
        let config = test_config(
            "custom",
            "test",
            "https://api.example.com/v1/messages",
            "model",
        );
        let engine = CloudPolishEngine::new(config);
        assert_eq!(engine.get_api_url(), "https://api.example.com/v1/messages");
    }

    #[test]
    fn test_is_coding_plan_endpoint() {
        let config_coding = test_config(
            "openai",
            "test",
            "https://coding.dashscope.aliyuncs.com/v1",
            "qwen",
        );
        let config_intl = test_config(
            "openai",
            "test",
            "https://coding-intl.dashscope.aliyuncs.com/v1",
            "qwen",
        );
        let config_other = test_config("openai", "test", "https://api.openai.com/v1", "gpt-4");

        let engine_coding = CloudPolishEngine::new(config_coding);
        let engine_intl = CloudPolishEngine::new(config_intl);
        let engine_other = CloudPolishEngine::new(config_other);

        assert!(engine_coding.is_coding_plan_endpoint());
        assert!(engine_intl.is_coding_plan_endpoint());
        assert!(!engine_other.is_coding_plan_endpoint());
    }
}
