use tracing::{info, instrument, warn};

use crate::polish_engine::{get_template_by_id, DEFAULT_POLISH_PROMPT};
use crate::state::app_state::AppState;

use super::shared::ProcessingEventTarget;

struct LocalPolishContext {
    system_prompt: String,
    window_context: Option<String>,
    language: String,
    model_id: String,
    log_context: &'static str,
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
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

fn accept_polish_result(
    task_id: u64,
    accumulated_text: String,
    result_text: String,
    polish_ms: u64,
    context: &'static str,
) -> (String, u64) {
    if should_reject_question_answer_polish(&accumulated_text, &result_text) {
        warn!(
            task_id,
            context,
            input_chars = accumulated_text.len(),
            output_chars = result_text.len(),
            "polish_rejected_answered_question-using_raw"
        );
        (accumulated_text, 0)
    } else {
        (result_text, polish_ms)
    }
}

async fn run_local_polish(
    event_target: &ProcessingEventTarget<'_>,
    state: &AppState,
    task_id: u64,
    accumulated_text: String,
    context: LocalPolishContext,
) -> (String, u64) {
    let LocalPolishContext {
        system_prompt,
        window_context,
        language,
        model_id,
        log_context,
    } = context;

    if model_id.is_empty() {
        warn!(
            task_id,
            context = log_context,
            "polish_model_not_configured"
        );
        return (accumulated_text, 0);
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
                info!(task_id, engine = ?engine_type, model_id = %model_id, context = log_context, "polish_started-local");

                let request = crate::polish_engine::PolishRequest::new(
                    accumulated_text.clone(),
                    system_prompt,
                    language,
                )
                .with_model(model_filename);
                let request = match window_context {
                    Some(ref ctx) => request.with_window_context(ctx),
                    None => request,
                };

                event_target.emit_polishing(task_id);

                match state.polish_manager.polish(engine_type, request).await {
                    Ok(result) if !result.text.is_empty() => {
                        info!(
                            task_id,
                            chars = result.text.len(),
                            polish_ms = result.total_ms,
                            context = log_context,
                            "polish_completed-local"
                        );
                        accept_polish_result(
                            task_id,
                            accumulated_text,
                            result.text,
                            result.total_ms,
                            log_context,
                        )
                    }
                    Ok(_) => {
                        warn!(
                            task_id,
                            context = log_context,
                            "polish_empty_result-local_using_raw"
                        );
                        (accumulated_text, 0)
                    }
                    Err(e) => {
                        warn!(task_id, error = %e, context = log_context, "polish_failed-local_using_raw");
                        (accumulated_text, 0)
                    }
                }
            } else {
                warn!(
                    task_id,
                    context = log_context,
                    "polish_model_not_downloaded-using_raw"
                );
                (accumulated_text, 0)
            }
        }
        None => {
            warn!(task_id, model_id = %model_id, context = log_context, "polish_model_unknown-engine_undetermined");
            (accumulated_text, 0)
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
) -> (String, u64) {
    match resolved_polish_template_id {
        None => {
            info!(task_id, "polish_skipped-no_template");
            (accumulated_text, 0)
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
                        info!(
                            task_id,
                            provider = %provider_type,
                            model = %cfg.model,
                            has_window_context = window_context_chars > 0,
                            window_context_chars,
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

                        event_target.emit_polishing(task_id);

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
                                info!(
                                    task_id,
                                    chars = result.text.len(),
                                    polish_ms = result.total_ms,
                                    "polish_completed-cloud"
                                );
                                accept_polish_result(
                                    task_id,
                                    accumulated_text,
                                    result.text,
                                    result.total_ms,
                                    "cloud",
                                )
                            }
                            Ok(_) => {
                                warn!(task_id, provider = %provider_type, "polish_empty_result-cloud_using_raw");
                                (accumulated_text, 0)
                            }
                            Err(e) => {
                                warn!(task_id, provider = %provider_type, error = %e, "polish_failed-cloud_using_raw");
                                (accumulated_text, 0)
                            }
                        };
                    }
                }
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
            )
            .await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::should_reject_question_answer_polish;

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
}
