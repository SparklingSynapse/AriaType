use tracing::{info, instrument, warn};

use crate::polish_engine::{get_template_by_id, DEFAULT_POLISH_PROMPT};
use crate::state::app_state::AppState;

use super::shared::ProcessingEventTarget;

struct LocalPolishContext {
    system_prompt: String,
    language: String,
    model_id: String,
    log_context: &'static str,
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
                        (result.text, result.total_ms)
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

#[instrument(skip(state, accumulated_text), fields(task_id))]
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

            if cloud_polish_enabled {
                if let Some(cfg) = cloud_config {
                    if !cfg.api_key.is_empty() && !cfg.model.is_empty() {
                        info!(task_id, provider = %provider_type, model = %cfg.model, "polish_started-cloud");

                        let request = crate::polish_engine::PolishRequest::new(
                            accumulated_text.clone(),
                            system_prompt,
                            language,
                        );

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
                                (result.text, result.total_ms)
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
                    language,
                    model_id: polish_model_id,
                    log_context: "local",
                },
            )
            .await
        }
    }
}
