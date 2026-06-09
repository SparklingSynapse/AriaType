use ariatype_lib::polish_engine::{
    CloudPolishEngine, CloudProviderConfig, PolishEngine, PolishRequest, CORE_POLISH_CONSTRAINT,
};
use std::sync::{Arc, Mutex};
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

    let request = PolishRequest::new(
        "User text here".to_string(),
        "System instruction here".to_string(),
        "en".to_string(),
    );

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

    let request = PolishRequest::new(
        "User text here".to_string(),
        "System instruction here".to_string(),
        "en".to_string(),
    );

    let result = engine
        .polish(request)
        .await
        .expect("OpenAI polish failed due to incorrect request format or other error");

    assert_eq!(result.text, "OpenAI mock format correct");
}

#[tokio::test]
async fn test_openai_polish_streams_preview_chunks() {
    let mock_server = MockServer::start().await;
    let expected_system_prompt = format!(
        "{}\n\nUSER RULES:\n{}",
        CORE_POLISH_CONSTRAINT, "System instruction here"
    );
    let expected_body = serde_json::json!({
        "model": "gpt-4o-mini",
        "max_tokens": 4096,
        "stream": true,
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
    let stream_body = concat!(
        "data: {\"choices\":[{\"delta\":{\"content\":\"Open\"}}]}\n\n",
        "data: {\"choices\":[{\"delta\":{\"content\":\"AI\"}}]}\n\n",
        "data: [DONE]\n\n",
    );

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(header("Authorization", "Bearer test_openai_api_key"))
        .and(header("Content-Type", "application/json"))
        .and(body_partial_json(expected_body))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "text/event-stream")
                .set_body_string(stream_body),
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
    let updates = Arc::new(Mutex::new(Vec::new()));
    let callback_updates = Arc::clone(&updates);
    let callback: ariatype_lib::polish_engine::PolishPreviewCallback =
        Arc::new(move |update| callback_updates.lock().unwrap().push(update));

    let request = PolishRequest::new(
        "User text here".to_string(),
        "System instruction here".to_string(),
        "en".to_string(),
    )
    .with_preview_callback(callback);

    let result = engine
        .polish(request)
        .await
        .expect("OpenAI streaming polish should succeed");

    assert_eq!(result.text, "OpenAI");
    assert!(result.time_to_first_token_ms.is_some());
    assert!(result.generation_ms.is_some());
    let updates = updates.lock().unwrap();
    assert_eq!(updates[0].text, "Open");
    assert_eq!(updates[1].text, "OpenAI");
    assert!(updates.last().unwrap().is_final);
}

#[tokio::test]
async fn test_openai_connection_check_uses_configured_endpoint() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(header("Authorization", "Bearer test_openai_api_key"))
        .and(header("Content-Type", "application/json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "choices": [{
                "message": {"content": "ok"}
            }]
        })))
        .expect(1)
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
    engine
        .check_connection()
        .await
        .expect("connection check should use the configured endpoint");
}

#[tokio::test]
async fn test_short_cloud_polish_times_out_after_core_prompt_timeout() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(Duration::from_secs(11))
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
    let request = PolishRequest::new(
        "User text here".to_string(),
        "System instruction here".to_string(),
        "en".to_string(),
    );

    let err = engine
        .polish(request)
        .await
        .expect_err("cloud polish should time out at the bounded short-request timeout");

    assert_eq!(
        err,
        format!(
            "Cloud polish request timed out after 10s during HTTP request (provider=openai, model=gpt-4o-mini, url={}/v1/chat/completions)",
            mock_server.uri()
        )
    );
}

#[tokio::test]
async fn test_long_cloud_polish_can_complete_after_five_seconds() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(Duration::from_secs(6))
                .set_body_json(serde_json::json!({
                    "choices": [{
                        "message": {"content": "Delayed long polish result"}
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
    let request = PolishRequest::new(
        "Long user text ".repeat(140),
        "System instruction here".to_string(),
        "en".to_string(),
    );

    let result = engine
        .polish(request)
        .await
        .expect("long cloud polish should get a bounded timeout above 5 seconds");

    assert_eq!(result.text, "Delayed long polish result");
}
