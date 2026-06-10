use crate::polish_engine::streaming::{collect_openai_streaming_response, PolishRuntimeTimings};
use crate::polish_engine::traits::{PolishEngineType, PolishRequest, PolishResult, SystemContext};
use crate::utils::{downloaded_file_is_complete, AppPaths};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;
use tracing::{debug, error, info, warn};

const LOCAL_POLISH_DEFAULT_BASE_URL: &str = "http://127.0.0.1:8000/v1";
const LOCAL_POLISH_BASE_URL_ENV: &str = "ARIATYPE_LOCAL_POLISH_BASE_URL";
const LOCAL_POLISH_API_KEY_ENV: &str = "ARIATYPE_LOCAL_POLISH_API_KEY";
const LOCAL_POLISH_MAX_OUTPUT_TOKENS: u32 = 20_480;
const LOCAL_POLISH_DEFAULT_TIMEOUT: Duration = Duration::from_secs(12);
const LOCAL_POLISH_FALLBACK_MAX_TIMEOUT: Duration = Duration::from_secs(30);
const LOCAL_POLISH_BASE_TIMEOUT_CHARS: usize = 500;
const LOCAL_POLISH_TIMEOUT_STEP_CHARS: usize = 800;
const LOCAL_POLISH_TIMEOUT_STEP: Duration = Duration::from_secs(5);
const LOCAL_POLISH_CORE_PROMPT: &str = "Polish transcript. Fix clear STT mistakes, punctuation, grammar, names and terms. Preserve meaning, facts, order, language and tone. Do not answer, summarize or add info. Output plain text only.";
const NO_THINK_DIRECTIVE: &str = "/no_think";
const THINK_START_TAG: &str = "<think>";
const THINK_END_TAG: &str = "</think>";
const MIB: u64 = 1024 * 1024;

#[derive(Debug, Clone)]
pub(crate) struct LocalHttpPolishConfig {
    pub engine_type: PolishEngineType,
    pub engine_label: &'static str,
    pub model_filename: String,
    pub model_alias: String,
    pub min_model_size_mb: u64,
    pub no_think_directive: bool,
}

#[derive(Debug, Clone)]
struct LocalOpenAiConfig {
    base_url: String,
    api_key: Option<String>,
    model: String,
    engine_label: &'static str,
    no_think_directive: bool,
}

#[derive(Debug, Serialize)]
struct Message {
    role: &'static str,
    content: String,
}

#[derive(Debug, Serialize)]
struct RequestBody {
    model: String,
    max_tokens: u32,
    temperature: f32,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    enable_thinking: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    chat_template_kwargs: Option<ChatTemplateKwargs>,
    messages: Vec<Message>,
}

#[derive(Debug, Serialize)]
struct ChatTemplateKwargs {
    enable_thinking: bool,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: OpenAiResponseMessage,
}

#[derive(Debug, Deserialize)]
struct OpenAiResponseMessage {
    content: String,
}

#[derive(Debug, Deserialize)]
struct ResponseBody {
    choices: Vec<Choice>,
    #[serde(default)]
    timings: Option<serde_json::Value>,
    #[serde(default)]
    ariatype_timings: Option<serde_json::Value>,
}

pub(crate) async fn polish_via_local_http(
    request: PolishRequest,
    config: LocalHttpPolishConfig,
) -> Result<PolishResult, String> {
    let model_path = AppPaths::models_dir().join(&config.model_filename);
    validate_model_file(
        &model_path,
        &config.model_filename,
        config.min_model_size_mb,
        config.engine_label,
    )?;

    let timeout = request
        .timeout
        .unwrap_or_else(|| fallback_timeout(&request.text));
    let client = Client::builder()
        .build()
        .expect("local polish reqwest client should build");
    let runtime_config = crate::polish_engine::local_runtime::current_config();
    let http_config = LocalOpenAiConfig {
        base_url: runtime_config.base_url,
        api_key: runtime_config.api_key,
        model: config.model_alias,
        engine_label: config.engine_label,
        no_think_directive: config.no_think_directive,
    };

    let t0 = std::time::Instant::now();
    let preview_callback = request.preview_callback.clone();
    let response = call_local_openai_api(
        &client,
        &http_config,
        &request.system_context,
        &request.text,
        timeout,
        preview_callback.as_ref(),
    )
    .await?;
    let total_ms = t0.elapsed().as_millis() as u64;
    let text = strip_think_block(&response.text).unwrap_or_else(|| {
        warn!(
            engine = config.engine_label,
            "local_polish_incomplete_think_block"
        );
        String::new()
    });

    Ok(PolishResult::new(text, config.engine_type, total_ms)
        .with_runtime_metrics(
            response.timings.model_load_ms,
            response.timings.context_create_ms,
            response.timings.prefill_ms,
            response.timings.inference_ms,
        )
        .with_streaming_metrics(
            response.time_to_first_token_ms,
            Some(response.generation_ms),
        ))
}

async fn call_local_openai_api(
    client: &Client,
    config: &LocalOpenAiConfig,
    system_context: &SystemContext,
    user_message: &str,
    timeout: Duration,
    preview_callback: Option<&crate::polish_engine::PolishPreviewCallback>,
) -> Result<crate::polish_engine::streaming::StreamingPolishResponse, String> {
    let url = local_api_url(&config.base_url);
    let system_prompt = build_local_system_prompt(system_context, config.no_think_directive);
    let body = RequestBody {
        model: config.model.clone(),
        max_tokens: LOCAL_POLISH_MAX_OUTPUT_TOKENS,
        temperature: 0.0,
        stream: preview_callback.is_some(),
        enable_thinking: config.no_think_directive.then_some(false),
        chat_template_kwargs: config.no_think_directive.then_some(ChatTemplateKwargs {
            enable_thinking: false,
        }),
        messages: vec![
            Message {
                role: "system",
                content: system_prompt,
            },
            Message {
                role: "user",
                content: user_message.to_string(),
            },
        ],
    };

    debug!(
        engine = config.engine_label,
        url = %url,
        model = %config.model,
        timeout_secs = timeout.as_secs(),
        max_tokens = LOCAL_POLISH_MAX_OUTPUT_TOKENS,
        "local_polish_http_request_start"
    );
    info!(
        engine = config.engine_label,
        system_prompt_len = body
            .messages
            .first()
            .map(|message| message.content.len())
            .unwrap_or(0),
        user_message_len = user_message.len(),
        "local_polish_http_request_prepared"
    );

    let mut builder = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header(
            "User-Agent",
            concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION")),
        )
        .timeout(timeout)
        .json(&body);

    if let Some(api_key) = &config.api_key {
        builder = builder.header("Authorization", format!("Bearer {api_key}"));
    }

    let response = builder
        .send()
        .await
        .map_err(|e| format_local_request_error(config, &url, e, timeout))?;

    let status = response.status();
    if !status.is_success() {
        let response_text = response
            .text()
            .await
            .map_err(|e| format_local_request_error(config, &url, e, timeout))?;
        error!(
            engine = config.engine_label,
            status = %status,
            body_len = response_text.len(),
            "local_polish_http_api_error"
        );
        return Err(format!("Local polish API error ({status})"));
    }

    if preview_callback.is_some() {
        return collect_openai_streaming_response(response, preview_callback, config.engine_label)
            .await;
    }

    let response_text = response
        .text()
        .await
        .map_err(|e| format_local_request_error(config, &url, e, timeout))?;

    debug!(
        engine = config.engine_label,
        response_len = response_text.len(),
        "local_polish_http_response_received"
    );

    let response_body: ResponseBody = serde_json::from_str(&response_text)
        .map_err(|e| format!("Failed to parse local polish response: {}", e))?;

    Ok(crate::polish_engine::streaming::StreamingPolishResponse {
        text: response_body
            .choices
            .first()
            .map(|choice| choice.message.content.clone())
            .unwrap_or_default(),
        time_to_first_token_ms: None,
        generation_ms: 0,
        timings: PolishRuntimeTimings::from_response_parts(
            response_body.timings.as_ref(),
            response_body.ariatype_timings.as_ref(),
        ),
    })
}

pub(crate) fn local_base_url() -> String {
    std::env::var(LOCAL_POLISH_BASE_URL_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| LOCAL_POLISH_DEFAULT_BASE_URL.to_string())
}

pub(crate) fn local_api_key() -> Option<String> {
    std::env::var(LOCAL_POLISH_API_KEY_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub(crate) fn local_api_url(base_url: &str) -> String {
    let base = base_url.trim_end_matches('/');

    if base.ends_with("/chat/completions") {
        return base.to_string();
    }

    if base.ends_with("/v1") {
        return format!("{base}/chat/completions");
    }

    format!("{base}/v1/chat/completions")
}

pub(crate) fn local_models_url(base_url: &str) -> String {
    let base = base_url.trim_end_matches('/');

    if let Some(base) = base.strip_suffix("/chat/completions") {
        return format!("{base}/models");
    }

    if base.ends_with("/v1") {
        return format!("{base}/models");
    }

    format!("{base}/v1/models")
}

fn fallback_timeout(text: &str) -> Duration {
    let extra_chars = text
        .chars()
        .count()
        .saturating_sub(LOCAL_POLISH_BASE_TIMEOUT_CHARS);
    let extra_steps = extra_chars.div_ceil(LOCAL_POLISH_TIMEOUT_STEP_CHARS);
    (LOCAL_POLISH_DEFAULT_TIMEOUT
        + Duration::from_secs(LOCAL_POLISH_TIMEOUT_STEP.as_secs() * extra_steps as u64))
    .min(LOCAL_POLISH_FALLBACK_MAX_TIMEOUT)
}

fn build_local_system_prompt(_system_context: &SystemContext, no_think_directive: bool) -> String {
    let mut system_prompt = LOCAL_POLISH_CORE_PROMPT.to_string();
    if no_think_directive {
        system_prompt.push('\n');
        system_prompt.push_str(NO_THINK_DIRECTIVE);
    }

    system_prompt
}

fn validate_model_file(
    model_path: &Path,
    model_filename: &str,
    min_model_size_mb: u64,
    engine_label: &'static str,
) -> Result<(), String> {
    if !model_path.exists() {
        error!(
            engine = engine_label,
            path = ?model_path,
            "local_polish_model_not_found"
        );
        return Err(format!("Model not found: {model_filename}"));
    }

    let minimum_bytes = min_model_size_mb.saturating_mul(MIB);
    let complete = downloaded_file_is_complete(model_path, Some(minimum_bytes));
    let size_mb = model_path
        .metadata()
        .map(|metadata| metadata.len() / MIB)
        .unwrap_or(0);

    info!(
        engine = engine_label,
        path = ?model_path,
        size_mb,
        min_model_size_mb,
        complete,
        "local_polish_model_file_checked"
    );

    if !complete {
        error!(
            engine = engine_label,
            size_mb, min_model_size_mb, "local_polish_model_file_incomplete"
        );
        return Err(format!(
            "Model file appears incomplete: {}MB (expected at least {}MB)",
            size_mb, min_model_size_mb
        ));
    }

    Ok(())
}

fn format_local_request_error(
    config: &LocalOpenAiConfig,
    url: &str,
    error: reqwest::Error,
    timeout: Duration,
) -> String {
    if error.is_timeout() {
        error!(
            engine = config.engine_label,
            model = %config.model,
            url = %url,
            timeout_secs = timeout.as_secs(),
            error = %error,
            "local_polish_http_request_timeout"
        );
        return format!(
            "Local polish timed out after {}s (model={}, url={})",
            timeout.as_secs(),
            config.model,
            url
        );
    }

    error!(
        engine = config.engine_label,
        model = %config.model,
        url = %url,
        error = %error,
        "local_polish_http_request_failed"
    );
    format!(
        "Local polish server unavailable (model={}, url={}): {}",
        config.model, url, error
    )
}

fn strip_think_block(result: &str) -> Option<String> {
    if let Some(end_idx) = result.find(THINK_END_TAG) {
        Some(result[end_idx + THINK_END_TAG.len()..].trim().to_string())
    } else if result.contains(THINK_START_TAG) {
        None
    } else {
        Some(result.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use wiremock::matchers::{body_partial_json, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn test_config(base_url: String) -> LocalOpenAiConfig {
        LocalOpenAiConfig {
            base_url,
            api_key: None,
            model: "qwen3.5-0.8b".to_string(),
            engine_label: "polish:qwen",
            no_think_directive: true,
        }
    }

    #[test]
    fn normalizes_local_openai_endpoint() {
        assert_eq!(
            local_api_url("http://127.0.0.1:8000"),
            "http://127.0.0.1:8000/v1/chat/completions"
        );
        assert_eq!(
            local_api_url("http://127.0.0.1:8000/v1"),
            "http://127.0.0.1:8000/v1/chat/completions"
        );
        assert_eq!(
            local_api_url("http://127.0.0.1:8000/v1/chat/completions"),
            "http://127.0.0.1:8000/v1/chat/completions"
        );
    }

    #[test]
    fn normalizes_local_models_endpoint() {
        assert_eq!(
            local_models_url("http://127.0.0.1:8000"),
            "http://127.0.0.1:8000/v1/models"
        );
        assert_eq!(
            local_models_url("http://127.0.0.1:8000/v1"),
            "http://127.0.0.1:8000/v1/models"
        );
        assert_eq!(
            local_models_url("http://127.0.0.1:8000/v1/chat/completions"),
            "http://127.0.0.1:8000/v1/models"
        );
    }

    #[test]
    fn fallback_timeout_is_capped() {
        assert_eq!(fallback_timeout("short text"), LOCAL_POLISH_DEFAULT_TIMEOUT);
        assert_eq!(
            fallback_timeout(&"a".repeat(20_000)),
            LOCAL_POLISH_FALLBACK_MAX_TIMEOUT
        );
    }

    #[test]
    fn local_prompt_uses_concise_core_constraints() {
        let context = SystemContext::new("Long template rules should not be copied.")
            .with_window_context("Visible window text should not be copied.");
        let prompt = build_local_system_prompt(&context, true);

        assert!(prompt.len() <= 220);
        assert!(prompt.contains("Fix clear STT mistakes"));
        assert!(prompt.contains("Preserve meaning"));
        assert!(prompt.contains("Do not answer"));
        assert!(!prompt.contains("Long template rules"));
        assert!(!prompt.contains("Visible window text"));
        assert!(!prompt.contains("USER RULES"));
        assert!(!prompt.contains("REFERENCE CONTEXT"));
        assert!(prompt.contains(NO_THINK_DIRECTIVE));
    }

    #[test]
    fn local_prompt_omits_no_think_directive_when_not_requested() {
        let prompt = build_local_system_prompt(&SystemContext::new("Fix typos."), false);

        assert!(prompt.len() <= 200);
        assert!(!prompt.contains(NO_THINK_DIRECTIVE));
    }

    #[test]
    fn strips_complete_think_block() {
        assert_eq!(
            strip_think_block("<think>\nreasoning\n</think>\nPolished text.").unwrap(),
            "Polished text."
        );
        assert_eq!(strip_think_block("<think>\nreasoning"), None);
        assert_eq!(
            strip_think_block("Polished text.").unwrap(),
            "Polished text."
        );
    }

    #[tokio::test]
    async fn sends_openai_compatible_local_request_without_auth_by_default() {
        let mock_server = MockServer::start().await;
        let expected_system_prompt =
            build_local_system_prompt(&SystemContext::new("System instruction here"), true);
        let expected_body = serde_json::json!({
            "model": "qwen3.5-0.8b",
            "max_tokens": LOCAL_POLISH_MAX_OUTPUT_TOKENS,
            "temperature": 0.0,
            "stream": false,
            "enable_thinking": false,
            "chat_template_kwargs": {
                "enable_thinking": false
            },
            "messages": [
                {
                    "role": "system",
                    "content": expected_system_prompt
                },
                {
                    "role": "user",
                    "content": "User text here"
                }
            ]
        });
        let response_body = serde_json::json!({
            "choices": [
                {
                    "message": {
                        "content": "<think>hidden</think>\nLocal mock format correct"
                    }
                }
            ],
            "timings": {
                "prompt_ms": 31.2,
                "predicted_ms": 662.4
            },
            "ariatype_timings": {
                "model_load_ms": 120,
                "context_create_ms": 45
            }
        });

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .and(header("Content-Type", "application/json"))
            .and(body_partial_json(expected_body))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .mount(&mock_server)
            .await;

        let client = Client::builder().build().unwrap();
        let result = call_local_openai_api(
            &client,
            &test_config(mock_server.uri()),
            &SystemContext::new("System instruction here"),
            "User text here",
            Duration::from_secs(2),
            None,
        )
        .await
        .expect("local polish request should succeed");

        assert_eq!(
            strip_think_block(&result.text).unwrap(),
            "Local mock format correct"
        );
        assert_eq!(result.timings.model_load_ms, Some(120));
        assert_eq!(result.timings.context_create_ms, Some(45));
        assert_eq!(result.timings.prefill_ms, Some(31));
        assert_eq!(result.timings.inference_ms, Some(662));
    }

    #[tokio::test]
    async fn sends_auth_header_when_configured() {
        let mock_server = MockServer::start().await;
        let response_body = serde_json::json!({
            "choices": [
                {
                    "message": {
                        "content": "ok"
                    }
                }
            ]
        });

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .and(header("Authorization", "Bearer local-test-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .mount(&mock_server)
            .await;

        let mut config = test_config(mock_server.uri());
        config.api_key = Some("local-test-key".to_string());
        let client = Client::builder().build().unwrap();

        let result = call_local_openai_api(
            &client,
            &config,
            &SystemContext::new("System instruction here"),
            "User text here",
            Duration::from_secs(2),
            None,
        )
        .await
        .expect("local polish request should succeed");

        assert_eq!(result.text, "ok");
    }

    #[tokio::test]
    async fn streams_openai_chunks_to_preview_callback() {
        let mock_server = MockServer::start().await;
        let expected_body = serde_json::json!({
            "model": "qwen3.5-0.8b",
            "max_tokens": LOCAL_POLISH_MAX_OUTPUT_TOKENS,
            "temperature": 0.0,
            "stream": true,
            "enable_thinking": false,
            "chat_template_kwargs": {
                "enable_thinking": false
            },
        });
        let stream_body = concat!(
            "data: {\"choices\":[{\"delta\":{\"content\":\"Hel\"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\"lo\"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{}}],\"timings\":{\"prompt_ms\":20.1,\"predicted_ms\":80.9},\"ariatype_timings\":{\"model_load_ms\":100,\"context_create_ms\":40}}\n\n",
            "data: [DONE]\n\n",
        );

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .and(body_partial_json(expected_body))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("Content-Type", "text/event-stream")
                    .set_body_string(stream_body),
            )
            .mount(&mock_server)
            .await;

        let client = Client::builder().build().unwrap();
        let updates = Arc::new(Mutex::new(Vec::new()));
        let callback_updates = Arc::clone(&updates);
        let callback: crate::polish_engine::PolishPreviewCallback =
            Arc::new(move |update: crate::polish_engine::PolishPreviewUpdate| {
                callback_updates.lock().unwrap().push(update);
            });

        let result = call_local_openai_api(
            &client,
            &test_config(mock_server.uri()),
            &SystemContext::new("System instruction here"),
            "User text here",
            Duration::from_secs(2),
            Some(&callback),
        )
        .await
        .expect("streaming local polish request should succeed");

        assert_eq!(result.text, "Hello");
        assert!(result.time_to_first_token_ms.is_some());
        assert_eq!(result.timings.model_load_ms, Some(100));
        assert_eq!(result.timings.context_create_ms, Some(40));
        assert_eq!(result.timings.prefill_ms, Some(20));
        assert_eq!(result.timings.inference_ms, Some(81));
        let updates = updates.lock().unwrap();
        assert_eq!(updates[0].text, "Hel");
        assert_eq!(updates[1].text, "Hello");
        assert!(updates.last().unwrap().is_final);
    }
}
