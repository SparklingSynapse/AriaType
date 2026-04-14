use tracing::{info, instrument, warn};

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
) -> (String, u64) {
    let (polish_enabled, cloud_polish_enabled) = {
        let settings = state.settings.lock();
        (settings.polish_enabled, settings.cloud_polish_enabled)
    };

    if !polish_enabled && !cloud_polish_enabled {
        return (accumulated_text, 0);
    }

    let (polish_system_prompt, polish_language, polish_model_id, cloud_polish_config) = {
        let settings = state.settings.lock();
        let prompt = settings.polish_system_prompt.clone();
        (
            if prompt.is_empty() {
                crate::polish_engine::DEFAULT_POLISH_PROMPT.to_string()
            } else {
                prompt
            },
            settings.stt_engine_language.clone(),
            settings.polish_model.clone(),
            settings.get_active_cloud_polish_config(),
        )
    };

    if cloud_polish_config.enabled {
        if cloud_polish_config.api_key.is_empty() || cloud_polish_config.model.is_empty() {
            warn!(task_id, provider = %cloud_polish_config.provider_type, api_key_empty = cloud_polish_config.api_key.is_empty(), model_empty = cloud_polish_config.model.is_empty(), "cloud_polish_config_incomplete-fallback_local");

            return run_local_polish(
                event_target,
                state,
                task_id,
                accumulated_text,
                LocalPolishContext {
                    system_prompt: polish_system_prompt,
                    language: polish_language,
                    model_id: polish_model_id,
                    log_context: "cloud_fallback",
                },
            )
            .await;
        }

        info!(task_id, provider = %cloud_polish_config.provider_type, model = %cloud_polish_config.model, "polish_started-cloud");

        let request = crate::polish_engine::PolishRequest::new(
            accumulated_text.clone(),
            polish_system_prompt,
            polish_language,
        );

        event_target.emit_polishing(task_id);

        return match state
            .polish_manager
            .polish_cloud(
                request,
                &cloud_polish_config.provider_type,
                &cloud_polish_config.api_key,
                &cloud_polish_config.base_url,
                &cloud_polish_config.model,
                cloud_polish_config.enable_thinking,
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
                warn!(task_id, provider = %cloud_polish_config.provider_type, "polish_empty_result-cloud_using_raw");
                (accumulated_text, 0)
            }
            Err(e) => {
                warn!(task_id, provider = %cloud_polish_config.provider_type, error = %e, "polish_failed-cloud_using_raw");
                (accumulated_text, 0)
            }
        };
    }

    run_local_polish(
        event_target,
        state,
        task_id,
        accumulated_text,
        LocalPolishContext {
            system_prompt: polish_system_prompt,
            language: polish_language,
            model_id: polish_model_id,
            log_context: "local",
        },
    )
    .await
}
