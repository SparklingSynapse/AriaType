use futures_util::StreamExt;
use reqwest::Response;
use serde::Deserialize;
use serde_json::Value;
use std::time::Instant;
use tracing::{debug, warn};

use crate::polish_engine::traits::{PolishPreviewCallback, PolishPreviewUpdate};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StreamingPolishResponse {
    pub text: String,
    pub time_to_first_token_ms: Option<u64>,
    pub generation_ms: u64,
    pub timings: PolishRuntimeTimings,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct PolishRuntimeTimings {
    pub model_load_ms: Option<u64>,
    pub context_create_ms: Option<u64>,
    pub prefill_ms: Option<u64>,
    pub inference_ms: Option<u64>,
}

impl PolishRuntimeTimings {
    pub(crate) fn merge(&mut self, other: Self) {
        self.model_load_ms = self.model_load_ms.or(other.model_load_ms);
        self.context_create_ms = self.context_create_ms.or(other.context_create_ms);
        self.prefill_ms = self.prefill_ms.or(other.prefill_ms);
        self.inference_ms = self.inference_ms.or(other.inference_ms);
    }

    pub(crate) fn from_response_parts(
        timings: Option<&Value>,
        ariatype_timings: Option<&Value>,
    ) -> Self {
        let mut parsed = Self::from_value(timings);
        parsed.merge(Self::from_ariatype_value(ariatype_timings));
        parsed
    }

    fn from_value(value: Option<&Value>) -> Self {
        let Some(value) = value else {
            return Self::default();
        };

        Self {
            model_load_ms: timing_ms(value, &["model_load_ms", "model_load", "load_ms"]),
            context_create_ms: timing_ms(
                value,
                &["context_create_ms", "context_ms", "context_load_ms"],
            ),
            prefill_ms: timing_ms(value, &["prefill_ms", "prompt_ms", "prompt_eval_ms"]),
            inference_ms: timing_ms(
                value,
                &["inference_ms", "predicted_ms", "generation_ms", "decode_ms"],
            ),
        }
    }

    fn from_ariatype_value(value: Option<&Value>) -> Self {
        Self::from_value(value)
    }
}

fn timing_ms(value: &Value, keys: &[&str]) -> Option<u64> {
    keys.iter()
        .find_map(|key| value.get(*key))
        .and_then(value_to_ms)
}

fn value_to_ms(value: &Value) -> Option<u64> {
    value.as_u64().or_else(|| {
        value
            .as_f64()
            .filter(|value| value.is_finite() && *value >= 0.0)
            .map(|value| value.round() as u64)
    })
}

#[derive(Debug, Deserialize)]
struct StreamChoice {
    delta: Option<StreamDelta>,
    message: Option<StreamMessage>,
}

#[derive(Debug, Deserialize)]
struct StreamDelta {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct StreamMessage {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct StreamBody {
    choices: Vec<StreamChoice>,
    #[serde(default)]
    timings: Option<Value>,
    #[serde(default)]
    ariatype_timings: Option<Value>,
}

struct StreamEvent {
    content: Option<String>,
    timings: PolishRuntimeTimings,
}

pub(crate) async fn collect_openai_streaming_response(
    response: Response,
    preview_callback: Option<&PolishPreviewCallback>,
    label: &'static str,
) -> Result<StreamingPolishResponse, String> {
    let started_at = Instant::now();
    let mut stream = response.bytes_stream();
    let mut pending = Vec::new();
    let mut text = String::new();
    let mut time_to_first_token_ms = None;
    let mut timings = PolishRuntimeTimings::default();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("{label} streaming response read failed: {e}"))?;
        pending.extend_from_slice(&chunk);

        while let Some(newline_index) = pending.iter().position(|byte| *byte == b'\n') {
            let line_bytes: Vec<u8> = pending.drain(..=newline_index).collect();
            consume_stream_line(
                &line_bytes,
                label,
                preview_callback,
                &mut text,
                &mut time_to_first_token_ms,
                &mut timings,
                started_at,
            )?;
        }
    }

    if !pending.is_empty() {
        consume_stream_line(
            &pending,
            label,
            preview_callback,
            &mut text,
            &mut time_to_first_token_ms,
            &mut timings,
            started_at,
        )?;
    }

    if let Some(callback) = preview_callback {
        callback(PolishPreviewUpdate::final_text(text.clone()));
    }

    Ok(StreamingPolishResponse {
        text,
        time_to_first_token_ms,
        generation_ms: started_at.elapsed().as_millis() as u64,
        timings,
    })
}

fn consume_stream_line(
    line_bytes: &[u8],
    label: &'static str,
    preview_callback: Option<&PolishPreviewCallback>,
    text: &mut String,
    time_to_first_token_ms: &mut Option<u64>,
    timings: &mut PolishRuntimeTimings,
    started_at: Instant,
) -> Result<(), String> {
    let line = std::str::from_utf8(line_bytes)
        .map_err(|e| format!("{label} streaming response was not valid UTF-8: {e}"))?
        .trim();

    if let Some(event) = parse_openai_stream_event(line)? {
        timings.merge(event.timings);
        let Some(delta) = event.content else {
            return Ok(());
        };
        if time_to_first_token_ms.is_none() {
            *time_to_first_token_ms = Some(started_at.elapsed().as_millis() as u64);
        }
        text.push_str(&delta);
        if let Some(callback) = preview_callback {
            callback(PolishPreviewUpdate::chunk(text.clone(), delta));
        }
    }

    Ok(())
}

#[cfg(test)]
fn parse_openai_stream_delta(line: &str) -> Result<Option<String>, String> {
    Ok(parse_openai_stream_event(line)?.and_then(|event| event.content))
}

fn parse_openai_stream_event(line: &str) -> Result<Option<StreamEvent>, String> {
    let line = line.trim();
    if line.is_empty() || line.starts_with(':') {
        return Ok(None);
    }

    let Some(payload) = line.strip_prefix("data:") else {
        debug!(line, "openai_stream_ignored_non_data_line");
        return Ok(None);
    };
    let payload = payload.trim();
    if payload.is_empty() || payload == "[DONE]" {
        return Ok(None);
    }

    let body: StreamBody = serde_json::from_str(payload)
        .map_err(|e| format!("Failed to parse OpenAI stream chunk: {e}"))?;

    let Some(choice) = body.choices.first() else {
        warn!(payload, "openai_stream_chunk_without_choice");
        return Ok(None);
    };

    let content = choice
        .delta
        .as_ref()
        .and_then(|delta| delta.content.clone())
        .or_else(|| {
            choice
                .message
                .as_ref()
                .and_then(|message| message.content.clone())
        })
        .filter(|content| !content.is_empty());

    Ok(Some(StreamEvent {
        content,
        timings: PolishRuntimeTimings::from_response_parts(
            body.timings.as_ref(),
            body.ariatype_timings.as_ref(),
        ),
    }))
}

#[cfg(test)]
mod tests {
    use super::{parse_openai_stream_delta, parse_openai_stream_event};

    #[test]
    fn parses_openai_delta_content() {
        let line = r#"data: {"choices":[{"delta":{"content":"Hello"}}]}"#;

        assert_eq!(
            parse_openai_stream_delta(line).unwrap(),
            Some("Hello".to_string())
        );
    }

    #[test]
    fn ignores_done_and_comment_lines() {
        assert_eq!(parse_openai_stream_delta("data: [DONE]").unwrap(), None);
        assert_eq!(parse_openai_stream_delta(": keep-alive").unwrap(), None);
        assert_eq!(parse_openai_stream_delta("").unwrap(), None);
    }

    #[test]
    fn supports_message_content_fallback() {
        let line = r#"data: {"choices":[{"message":{"content":"Full text"}}]}"#;

        assert_eq!(
            parse_openai_stream_delta(line).unwrap(),
            Some("Full text".to_string())
        );
    }

    #[test]
    fn parses_stream_timing_metadata() {
        let line = r#"data: {"choices":[{"delta":{}}],"timings":{"prompt_ms":30.4,"predicted_ms":661.6},"ariatype_timings":{"model_load_ms":120,"context_create_ms":45}}"#;

        let event = parse_openai_stream_event(line).unwrap().unwrap();

        assert_eq!(event.timings.model_load_ms, Some(120));
        assert_eq!(event.timings.context_create_ms, Some(45));
        assert_eq!(event.timings.prefill_ms, Some(30));
        assert_eq!(event.timings.inference_ms, Some(662));
    }
}
