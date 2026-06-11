use std::time::{Duration, Instant};

use tracing::{info, instrument, warn};

use crate::polish_engine::{get_template_by_id, DEFAULT_POLISH_PROMPT};
use crate::state::app_state::AppState;

use super::postprocess::strip_trailing_sentence_period;
use super::shared::ProcessingEventTarget;

struct LocalPolishContext {
    system_prompt: String,
    window_context: Option<String>,
    language: String,
    model_id: String,
    log_context: &'static str,
}

struct PolishAcceptContext {
    polish_wall_ms: u64,
    polish_queue_ms: u64,
    log_context: &'static str,
    direct_stream_inserted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PolishProcessingResult {
    pub text: String,
    pub direct_stream_inserted: bool,
    pub polish_ms: u64,
    pub polish_wall_ms: u64,
    pub polish_queue_ms: u64,
    pub model_load_ms: Option<u64>,
    pub context_create_ms: Option<u64>,
    pub prefill_ms: Option<u64>,
    pub inference_ms: Option<u64>,
    pub time_to_first_token_ms: Option<u64>,
    pub generation_ms: Option<u64>,
    pub fallback_reason: Option<&'static str>,
}

impl PolishProcessingResult {
    pub(super) fn skipped(text: String, fallback_reason: &'static str) -> Self {
        let (text, _) = strip_trailing_sentence_period(&text);
        Self {
            text,
            direct_stream_inserted: false,
            polish_ms: 0,
            polish_wall_ms: 0,
            polish_queue_ms: 0,
            model_load_ms: None,
            context_create_ms: None,
            prefill_ms: None,
            inference_ms: None,
            time_to_first_token_ms: None,
            generation_ms: None,
            fallback_reason: Some(fallback_reason),
        }
    }

    fn from_polish_result(
        result: crate::polish_engine::PolishResult,
        polish_wall_ms: u64,
        polish_queue_ms: u64,
        direct_stream_inserted: bool,
    ) -> Self {
        let (text, _) = strip_trailing_sentence_period(&result.text);
        Self {
            text,
            direct_stream_inserted,
            polish_ms: result.total_ms,
            polish_wall_ms,
            polish_queue_ms,
            model_load_ms: result.model_load_ms,
            context_create_ms: result.context_create_ms,
            prefill_ms: result.prefill_ms,
            inference_ms: result.inference_ms,
            time_to_first_token_ms: result.time_to_first_token_ms,
            generation_ms: result.generation_ms,
            fallback_reason: None,
        }
    }

    fn using_original(
        text: String,
        polish_wall_ms: u64,
        polish_queue_ms: u64,
        fallback_reason: &'static str,
    ) -> Self {
        let (text, _) = strip_trailing_sentence_period(&text);
        Self {
            text,
            direct_stream_inserted: false,
            polish_ms: 0,
            polish_wall_ms,
            polish_queue_ms,
            model_load_ms: None,
            context_create_ms: None,
            prefill_ms: None,
            inference_ms: None,
            time_to_first_token_ms: None,
            generation_ms: None,
            fallback_reason: Some(fallback_reason),
        }
    }
}

const LOCAL_POLISH_BASE_TIMEOUT: Duration = Duration::from_secs(20);
const LOCAL_POLISH_MAX_TIMEOUT: Duration = Duration::from_secs(60);
const LOCAL_POLISH_BASE_TIMEOUT_CHARS: usize = 500;
const LOCAL_POLISH_TIMEOUT_STEP_CHARS: usize = 800;
const LOCAL_POLISH_TIMEOUT_STEP: Duration = Duration::from_secs(10);
const LOCAL_POLISH_TIMEOUT_REASON: &str = "local polish timed out";

fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

fn local_polish_timeout(text: &str) -> Duration {
    let chars = text.chars().count();
    let extra_chars = chars.saturating_sub(LOCAL_POLISH_BASE_TIMEOUT_CHARS);
    let extra_steps = extra_chars.div_ceil(LOCAL_POLISH_TIMEOUT_STEP_CHARS);
    let timeout = LOCAL_POLISH_BASE_TIMEOUT
        + Duration::from_secs(LOCAL_POLISH_TIMEOUT_STEP.as_secs() * extra_steps as u64);

    timeout.min(LOCAL_POLISH_MAX_TIMEOUT)
}

fn elapsed_ms(started: Instant) -> u64 {
    started.elapsed().as_millis() as u64
}

fn has_question_mark(text: &str) -> bool {
    text.contains('?') || text.contains('？')
}

fn is_question_like_text(text: &str) -> bool {
    let lower = text.to_lowercase();
    has_question_mark(text)
        || contains_any(
            &lower,
            &[
                "吗",
                "是不是",
                "是否",
                "哪些",
                "哪个",
                "哪里",
                "哪儿",
                "为什么",
                "怎么",
                "如何",
                "有没有",
                "能不能",
                "可不可以",
                "what",
                "why",
                "how",
                "should",
                "could",
                "would",
            ],
        )
}

fn is_answer_like_text(text: &str) -> bool {
    let lower = text
        .trim_start_matches(|c: char| c.is_whitespace() || matches!(c, ',' | '，' | '.' | '。'))
        .to_lowercase();

    lower.starts_with("我觉得")
        || lower.starts_with("我认为")
        || lower.starts_with("是的")
        || lower.starts_with("不是")
        || lower.starts_with("可以")
        || lower.starts_with("不可以")
        || lower.starts_with("不能")
        || lower.starts_with("还不")
        || lower.starts_with("还没")
        || lower.starts_with("需要")
        || lower.starts_with("不需要")
        || contains_any(
            &lower,
            &[
                "不够完整",
                "还没到",
                "还不是",
                "不是所有",
                "not ready",
                "is ready",
                "is not ready",
                "i think",
                "i believe",
            ],
        )
}

fn should_reject_question_answer_polish(input: &str, output: &str) -> bool {
    is_question_like_text(input) && !has_question_mark(output) && is_answer_like_text(output)
}

fn classify_polish_failure_reason(error: &str) -> &'static str {
    let lower = error.to_lowercase();

    if lower.contains("incomplete") {
        "model download looks incomplete"
    } else if is_local_polish_timeout_error(error) {
        LOCAL_POLISH_TIMEOUT_REASON
    } else if lower.contains("not downloaded") || lower.contains("not found") {
        "model file is missing"
    } else if lower.contains("model load")
        || lower.contains("context")
        || lower.contains("null reference")
        || lower.contains("out of memory")
        || lower.contains("memory")
    {
        "model could not be loaded, likely low memory"
    } else if lower.contains("backend init") {
        "polish runtime failed to start"
    } else if lower.contains("tokenize") || lower.contains("tokenization") {
        "input could not be tokenized"
    } else if lower.contains("decode") || lower.contains("inference") {
        "model inference failed"
    } else if lower.contains("task join") || lower.contains("panic") {
        "polish worker crashed"
    } else if lower.contains("local polish server unavailable") {
        "local polish server unavailable"
    } else if lower.contains("401") || lower.contains("unauthorized") || lower.contains("api key") {
        "cloud polish authentication failed"
    } else if lower.contains("429") || lower.contains("rate limit") {
        "cloud polish was rate limited"
    } else if lower.contains("timeout") || lower.contains("timed out") {
        "cloud polish timed out"
    } else if lower.contains("network")
        || lower.contains("connection")
        || lower.contains("dns")
        || lower.contains("resolve")
    {
        "cloud polish network failed"
    } else {
        "unexpected polish error"
    }
}

fn is_local_polish_timeout_error(error: &str) -> bool {
    error.to_lowercase().contains(LOCAL_POLISH_TIMEOUT_REASON)
}

fn accept_polish_result(
    event_target: &ProcessingEventTarget<'_>,
    task_id: u64,
    accumulated_text: String,
    result: crate::polish_engine::PolishResult,
    context: PolishAcceptContext,
) -> PolishProcessingResult {
    let PolishAcceptContext {
        polish_wall_ms,
        polish_queue_ms,
        log_context,
        direct_stream_inserted,
    } = context;
    let result_text = result.text.as_str();
    if direct_stream_inserted {
        if should_reject_question_answer_polish(&accumulated_text, result_text) {
            warn!(
                task_id,
                context = log_context,
                input_chars = accumulated_text.len(),
                output_chars = result_text.len(),
                "polish_policy_rejection_skipped-direct_stream_inserted"
            );
        }
        return PolishProcessingResult::from_polish_result(
            result,
            polish_wall_ms,
            polish_queue_ms,
            true,
        );
    }

    if should_reject_question_answer_polish(&accumulated_text, result_text) {
        event_target.emit_polish_policy_tooltip(task_id);
        warn!(
            task_id,
            context = log_context,
            input_chars = accumulated_text.len(),
            output_chars = result_text.len(),
            "polish_rejected_answered_question-using_raw"
        );
        PolishProcessingResult::using_original(
            accumulated_text,
            polish_wall_ms,
            polish_queue_ms,
            "polish answered dictated question",
        )
    } else {
        PolishProcessingResult::from_polish_result(result, polish_wall_ms, polish_queue_ms, false)
    }
}

async fn run_local_polish(
    event_target: &ProcessingEventTarget<'_>,
    state: &AppState,
    task_id: u64,
    accumulated_text: String,
    context: LocalPolishContext,
    polish_decision_started: Instant,
) -> PolishProcessingResult {
    let LocalPolishContext {
        system_prompt,
        window_context,
        language,
        model_id,
        log_context,
    } = context;

    if model_id.is_empty() {
        let failure_reason = "no polish model selected";
        event_target.emit_polish_error_tooltip(task_id, Some(failure_reason));
        warn!(
            task_id,
            context = log_context,
            failure_reason,
            "polish_model_not_configured"
        );
        return PolishProcessingResult::using_original(
            accumulated_text,
            0,
            elapsed_ms(polish_decision_started),
            failure_reason,
        );
    }

    match crate::polish_engine::UnifiedPolishManager::get_engine_by_model_id(&model_id) {
        Some(engine_type) => {
            let model_filename = state
                .polish_manager
                .get_model_filename(engine_type, &model_id);

            if let Some(model_filename) = model_filename.filter(|_| {
                state
                    .polish_manager
                    .is_model_downloaded(engine_type, &model_id)
            }) {
                let polish_queue_ms = elapsed_ms(polish_decision_started);
                info!(
                    task_id,
                    engine = ?engine_type,
                    model_id = %model_id,
                    context = log_context,
                    polish_queue_ms,
                    "polish_started-local"
                );

                let timeout = local_polish_timeout(&accumulated_text);
                let request = crate::polish_engine::PolishRequest::new(
                    accumulated_text.clone(),
                    system_prompt,
                    language,
                )
                .with_model(model_filename)
                .with_timeout(timeout);
                let request = match window_context {
                    Some(ref ctx) => request.with_window_context(ctx),
                    None => request,
                };
                let preview_handle = event_target.polish_preview_callback(task_id);
                let request = match preview_handle.as_ref() {
                    Some(handle) => request.with_preview_callback(handle.callback.clone()),
                    None => request,
                };

                event_target.emit_polishing(task_id);

                let polish_call_started = Instant::now();
                match tokio::time::timeout(
                    timeout,
                    state.polish_manager.polish(engine_type, request),
                )
                .await
                {
                    Ok(Ok(result)) if !result.text.is_empty() => {
                        let polish_wall_ms = elapsed_ms(polish_call_started);
                        info!(
                            task_id,
                            chars = result.text.len(),
                            polish_ms = result.total_ms,
                            polish_wall_ms,
                            polish_queue_ms,
                            model_load_ms = result.model_load_ms,
                            context_create_ms = result.context_create_ms,
                            prefill_ms = result.prefill_ms,
                            inference_ms = result.inference_ms,
                            time_to_first_token_ms = result.time_to_first_token_ms,
                            generation_ms = result.generation_ms,
                            context = log_context,
                            "polish_completed-local"
                        );
                        accept_polish_result(
                            event_target,
                            task_id,
                            accumulated_text,
                            result,
                            PolishAcceptContext {
                                polish_wall_ms,
                                polish_queue_ms,
                                log_context,
                                direct_stream_inserted: preview_handle
                                    .as_ref()
                                    .map(|handle| handle.direct_stream_inserted())
                                    .unwrap_or(false),
                            },
                        )
                    }
                    Ok(Ok(_)) => {
                        let polish_wall_ms = elapsed_ms(polish_call_started);
                        let failure_reason = "model returned empty output";
                        event_target.emit_polish_error_tooltip(task_id, Some(failure_reason));
                        warn!(
                            task_id,
                            context = log_context,
                            failure_reason,
                            "polish_empty_result-local_using_raw"
                        );
                        PolishProcessingResult::using_original(
                            accumulated_text,
                            polish_wall_ms,
                            polish_queue_ms,
                            failure_reason,
                        )
                    }
                    Ok(Err(e)) => {
                        let polish_wall_ms = elapsed_ms(polish_call_started);
                        let failure_reason = classify_polish_failure_reason(&e);
                        if is_local_polish_timeout_error(&e) {
                            event_target.emit_local_polish_timeout_tooltip(task_id);
                        } else {
                            event_target.emit_polish_error_tooltip(task_id, Some(failure_reason));
                        }
                        warn!(task_id, error = %e, context = log_context, failure_reason, "polish_failed-local_using_raw");
                        PolishProcessingResult::using_original(
                            accumulated_text,
                            polish_wall_ms,
                            polish_queue_ms,
                            failure_reason,
                        )
                    }
                    Err(_) => {
                        let polish_wall_ms = elapsed_ms(polish_call_started);
                        event_target.emit_local_polish_timeout_tooltip(task_id);
                        warn!(
                            task_id,
                            context = log_context,
                            timeout_secs = timeout.as_secs(),
                            input_chars = accumulated_text.chars().count(),
                            "polish_timeout-local_using_raw"
                        );
                        PolishProcessingResult::using_original(
                            accumulated_text,
                            polish_wall_ms,
                            polish_queue_ms,
                            LOCAL_POLISH_TIMEOUT_REASON,
                        )
                    }
                }
            } else {
                let polish_queue_ms = elapsed_ms(polish_decision_started);
                let failure_reason = "model is not downloaded";
                event_target.emit_polish_error_tooltip(task_id, Some(failure_reason));
                warn!(
                    task_id,
                    context = log_context,
                    failure_reason,
                    "polish_model_not_downloaded-using_raw"
                );
                PolishProcessingResult::using_original(
                    accumulated_text,
                    0,
                    polish_queue_ms,
                    failure_reason,
                )
            }
        }
        None => {
            let polish_queue_ms = elapsed_ms(polish_decision_started);
            let failure_reason = "unknown polish model";
            event_target.emit_polish_error_tooltip(task_id, Some(failure_reason));
            warn!(task_id, model_id = %model_id, context = log_context, failure_reason, "polish_model_unknown-engine_undetermined");
            PolishProcessingResult::using_original(
                accumulated_text,
                0,
                polish_queue_ms,
                failure_reason,
            )
        }
    }
}

#[instrument(
    skip(event_target, state, accumulated_text, resolved_polish_template_id),
    fields(task_id)
)]
pub(super) async fn maybe_polish_transcription_text(
    event_target: &ProcessingEventTarget<'_>,
    state: &AppState,
    task_id: u64,
    accumulated_text: String,
    resolved_polish_template_id: Option<String>,
) -> PolishProcessingResult {
    let polish_decision_started = Instant::now();
    match resolved_polish_template_id {
        None => {
            info!(task_id, "polish_skipped-no_template");
            PolishProcessingResult::skipped(accumulated_text, "no polish template")
        }
        Some(template_id) => {
            let (
                system_prompt,
                language,
                provider_type,
                cloud_config,
                polish_model_id,
                cloud_polish_enabled,
            ) = {
                let settings = state.settings.lock();

                let system_prompt: String = get_template_by_id(&template_id)
                    .map(|t| t.system_prompt.to_string())
                    .or_else(|| {
                        settings
                            .polish_custom_templates
                            .iter()
                            .find(|t| t.id == template_id)
                            .map(|t| t.system_prompt.clone())
                    })
                    .unwrap_or_else(|| {
                        warn!(task_id, template_id = %template_id, "template_not_found_fallback");
                        get_template_by_id("filler")
                            .map(|t| t.system_prompt.to_string())
                            .unwrap_or_else(|| DEFAULT_POLISH_PROMPT.to_string())
                    });

                let language = settings.stt_engine_language.clone();
                let provider_type = settings.active_cloud_polish_provider.clone();
                let cloud_config = settings.cloud_polish_configs.get(&provider_type).cloned();
                let polish_model_id = settings.polish_model.clone();
                let cloud_polish_enabled = settings.cloud_polish_enabled;

                (
                    system_prompt,
                    language,
                    provider_type,
                    cloud_config,
                    polish_model_id,
                    cloud_polish_enabled,
                )
            };

            let window_context = {
                let session = state.session_state.lock();
                session
                    .as_ref()
                    .and_then(|s| s.window_context.as_ref())
                    .and_then(|ctx| ctx.to_polish_context())
            };
            let window_context_chars = window_context
                .as_ref()
                .map(|ctx| ctx.chars().count())
                .unwrap_or(0);

            if cloud_polish_enabled {
                if let Some(cfg) = cloud_config {
                    if !cfg.api_key.is_empty() && !cfg.model.is_empty() {
                        let polish_queue_ms = elapsed_ms(polish_decision_started);
                        info!(
                            task_id,
                            provider = %provider_type,
                            model = %cfg.model,
                            has_window_context = window_context_chars > 0,
                            window_context_chars,
                            polish_queue_ms,
                            "polish_started-cloud"
                        );

                        let request = crate::polish_engine::PolishRequest::new(
                            accumulated_text.clone(),
                            system_prompt,
                            language,
                        );
                        let request = match window_context {
                            Some(ref ctx) => request.with_window_context(ctx),
                            None => request,
                        };
                        let preview_handle = event_target.polish_preview_callback(task_id);
                        let request = match preview_handle.as_ref() {
                            Some(handle) => request.with_preview_callback(handle.callback.clone()),
                            None => request,
                        };

                        event_target.emit_polishing(task_id);

                        let polish_call_started = Instant::now();
                        return match state
                            .polish_manager
                            .polish_cloud(
                                request,
                                &provider_type,
                                &cfg.api_key,
                                &cfg.base_url,
                                &cfg.model,
                                cfg.enable_thinking,
                            )
                            .await
                        {
                            Ok(result) if !result.text.is_empty() => {
                                let polish_wall_ms = elapsed_ms(polish_call_started);
                                info!(
                                    task_id,
                                    chars = result.text.len(),
                                    polish_ms = result.total_ms,
                                    polish_wall_ms,
                                    polish_queue_ms,
                                    model_load_ms = result.model_load_ms,
                                    context_create_ms = result.context_create_ms,
                                    prefill_ms = result.prefill_ms,
                                    inference_ms = result.inference_ms,
                                    time_to_first_token_ms = result.time_to_first_token_ms,
                                    generation_ms = result.generation_ms,
                                    "polish_completed-cloud"
                                );
                                accept_polish_result(
                                    event_target,
                                    task_id,
                                    accumulated_text,
                                    result,
                                    PolishAcceptContext {
                                        polish_wall_ms,
                                        polish_queue_ms,
                                        log_context: "cloud",
                                        direct_stream_inserted: preview_handle
                                            .as_ref()
                                            .map(|handle| handle.direct_stream_inserted())
                                            .unwrap_or(false),
                                    },
                                )
                            }
                            Ok(_) => {
                                let polish_wall_ms = elapsed_ms(polish_call_started);
                                let failure_reason = "cloud model returned empty output";
                                event_target
                                    .emit_polish_error_tooltip(task_id, Some(failure_reason));
                                warn!(task_id, provider = %provider_type, failure_reason, "polish_empty_result-cloud_using_raw");
                                PolishProcessingResult::using_original(
                                    accumulated_text,
                                    polish_wall_ms,
                                    polish_queue_ms,
                                    failure_reason,
                                )
                            }
                            Err(e) => {
                                let polish_wall_ms = elapsed_ms(polish_call_started);
                                let failure_reason = classify_polish_failure_reason(&e);
                                event_target
                                    .emit_polish_error_tooltip(task_id, Some(failure_reason));
                                warn!(task_id, provider = %provider_type, error = %e, failure_reason, "polish_failed-cloud_using_raw");
                                PolishProcessingResult::using_original(
                                    accumulated_text,
                                    polish_wall_ms,
                                    polish_queue_ms,
                                    failure_reason,
                                )
                            }
                        };
                    }
                }

                let failure_reason = "cloud polish configuration incomplete";
                event_target.emit_polish_error_tooltip(task_id, Some(failure_reason));
                warn!(
                    task_id,
                    provider = %provider_type,
                    failure_reason,
                    "polish_cloud_config_incomplete-using_raw"
                );
                return PolishProcessingResult::using_original(
                    accumulated_text,
                    0,
                    elapsed_ms(polish_decision_started),
                    failure_reason,
                );
            }

            run_local_polish(
                event_target,
                state,
                task_id,
                accumulated_text,
                LocalPolishContext {
                    system_prompt,
                    window_context,
                    language,
                    model_id: polish_model_id,
                    log_context: "local",
                },
                polish_decision_started,
            )
            .await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        classify_polish_failure_reason, local_polish_timeout, should_reject_question_answer_polish,
        LOCAL_POLISH_BASE_TIMEOUT, LOCAL_POLISH_MAX_TIMEOUT,
    };

    #[test]
    fn rejects_polish_output_that_answers_a_dictated_question() {
        let input =
            "哎，你觉得这个功能现在完整了吗？咱们达到可以发布0.1版本的时候了吗？所有东西都就绪了吗？";
        let output =
            "我觉得这个功能现在还不够完整，还没到可以发布 0.1 版本的时候，还不是所有东西都就绪了。";

        assert!(should_reject_question_answer_polish(input, output));
    }

    #[test]
    fn accepts_polish_output_that_preserves_a_question() {
        let input = "看一下最终的结果，我们当前是不是可以发布0.1版本了？还差哪些东西？";
        let output = "看一下最终的结果，我们当前是不是可以发布 0.1 版本了？还差哪些东西？";

        assert!(!should_reject_question_answer_polish(input, output));
    }

    #[test]
    fn accepts_non_question_polish_output() {
        let input = "嗯，我觉得这个功能现在已经完整了";
        let output = "我觉得这个功能现在已经完整了。";

        assert!(!should_reject_question_answer_polish(input, output));
    }

    #[test]
    fn local_polish_timeout_stays_short_for_short_text() {
        assert_eq!(
            local_polish_timeout("short text"),
            LOCAL_POLISH_BASE_TIMEOUT
        );
    }

    #[test]
    fn local_polish_timeout_expands_for_long_text() {
        let text = "a".repeat(1_400);
        let timeout = local_polish_timeout(&text);

        assert!(timeout > LOCAL_POLISH_BASE_TIMEOUT);
        assert!(timeout <= LOCAL_POLISH_MAX_TIMEOUT);
    }

    #[test]
    fn local_polish_timeout_is_capped() {
        let text = "a".repeat(20_000);

        assert_eq!(local_polish_timeout(&text), LOCAL_POLISH_MAX_TIMEOUT);
    }

    #[test]
    fn classifies_model_load_failure_as_memory_likely() {
        assert_eq!(
            classify_polish_failure_reason("Model load: null reference from llama.cpp"),
            "model could not be loaded, likely low memory"
        );
    }

    #[test]
    fn classifies_incomplete_download_failure() {
        assert_eq!(
            classify_polish_failure_reason(
                "Model file appears incomplete: 18MB (expected at least 400MB)"
            ),
            "model download looks incomplete"
        );
    }

    #[test]
    fn classifies_cloud_auth_and_rate_limit_failures() {
        assert_eq!(
            classify_polish_failure_reason("HTTP 401 unauthorized: bad API key"),
            "cloud polish authentication failed"
        );
        assert_eq!(
            classify_polish_failure_reason("HTTP 429 rate limit exceeded"),
            "cloud polish was rate limited"
        );
    }

    #[test]
    fn classifies_local_timeout_without_cloud_reason() {
        assert_eq!(
            classify_polish_failure_reason("Local polish timed out"),
            "local polish timed out"
        );
    }

    #[test]
    fn classifies_local_server_unavailable_without_cloud_reason() {
        assert_eq!(
            classify_polish_failure_reason("Local polish server unavailable: connection refused"),
            "local polish server unavailable"
        );
    }
}
