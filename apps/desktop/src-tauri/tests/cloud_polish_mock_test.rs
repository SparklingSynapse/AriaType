use ariatype_lib::polish_engine::{
    CloudPolishEngine, CloudProviderConfig, PolishEngine, PolishRequest, CORE_POLISH_CONSTRAINT,
};
use std::time::Duration;
use wiremock::matchers::{body_partial_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_anthropic_polish_request_format() {
    let mock_server = MockServer::start().await;
    let expected_system_prompt = format!(
        "{}\n\nUSER RULES:\n{}",
        CORE_POLISH_CONSTRAINT, "System instruction here"
    );

    // We expect an Anthropic-compatible JSON body
    let expected_body = serde_json::json!({
        "model": "claude-3-haiku",
        "max_tokens": 4096,
        "system": expected_system_prompt,
        "messages": [
            {
                "role": "user",
                "content": "User text here"
            }
        ]
    });

    // Mock Anthropic response
    let response_body = serde_json::json!({
        "content": [
            {
                "text": "Anthropic mock format correct"
            }
        ]
    });

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(header("x-api-key", "test_anthropic_api_key"))
        .and(header("anthropic-version", "2023-06-01"))
        .and(header("Content-Type", "application/json"))
        .and(body_partial_json(expected_body))
        .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
        .mount(&mock_server)
        .await;

    let config = CloudProviderConfig {
        provider_type: "anthropic".to_string(),
        api_key: "test_anthropic_api_key".to_string(),
        base_url: format!("{}/v1/messages", mock_server.uri()),
        model: "claude-3-haiku".to_string(),
        enable_thinking: false,
    };

    let engine = CloudPolishEngine::new(config);

    let request = PolishRequest {
        text: "User text here".to_string(),
        system_prompt: "System instruction here".to_string(),
        language: "en".to_string(),
        model_name: None,
    };

    let result = engine
        .polish(request)
        .await
        .expect("Anthropic polish failed due to incorrect request format or other error");

    assert_eq!(result.text, "Anthropic mock format correct");
}

#[tokio::test]
async fn test_openai_polish_request_format() {
    let mock_server = MockServer::start().await;
    let expected_system_prompt = format!(
        "{}\n\nUSER RULES:\n{}",
        CORE_POLISH_CONSTRAINT, "System instruction here"
    );

    // We expect an OpenAI-compatible JSON body
    let expected_body = serde_json::json!({
        "model": "gpt-4o-mini",
        "max_tokens": 4096,
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

    // Mock OpenAI response
    let response_body = serde_json::json!({
        "choices": [
            {
                "message": {
                    "content": "OpenAI mock format correct"
                }
            }
        ]
    });

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(header("Authorization", "Bearer test_openai_api_key"))
        .and(header("Content-Type", "application/json"))
        .and(body_partial_json(expected_body))
        .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
        .mount(&mock_server)
        .await;

    let config = CloudProviderConfig {
        provider_type: "openai".to_string(),
        api_key: "test_openai_api_key".to_string(),
        base_url: format!("{}/v1/chat/completions", mock_server.uri()),
        model: "gpt-4o-mini".to_string(),
        enable_thinking: false,
    };

    let engine = CloudPolishEngine::new(config);

    let request = PolishRequest {
        text: "User text here".to_string(),
        system_prompt: "System instruction here".to_string(),
        language: "en".to_string(),
        model_name: None,
    };

    let result = engine
        .polish(request)
        .await
        .expect("OpenAI polish failed due to incorrect request format or other error");

    assert_eq!(result.text, "OpenAI mock format correct");
}

#[tokio::test]
async fn test_cloud_polish_times_out_after_five_seconds() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(Duration::from_secs(6))
                .set_body_json(serde_json::json!({
                    "choices": [{
                        "message": {"content": "Too late"}
                    }]
                })),
        )
        .mount(&mock_server)
        .await;

    let config = CloudProviderConfig {
        provider_type: "openai".to_string(),
        api_key: "test_openai_api_key".to_string(),
        base_url: format!("{}/v1/chat/completions", mock_server.uri()),
        model: "gpt-4o-mini".to_string(),
        enable_thinking: false,
    };

    let engine = CloudPolishEngine::new(config);
    let request = PolishRequest {
        text: "User text here".to_string(),
        system_prompt: "System instruction here".to_string(),
        language: "en".to_string(),
        model_name: None,
    };

    let err = engine
        .polish(request)
        .await
        .expect_err("cloud polish should time out after 5 seconds");

    assert_eq!(
        err,
        format!(
            "Cloud polish request timed out after 5s during HTTP request (provider=openai, model=gpt-4o-mini, url={}/v1/chat/completions)",
            mock_server.uri()
        )
    );
}
