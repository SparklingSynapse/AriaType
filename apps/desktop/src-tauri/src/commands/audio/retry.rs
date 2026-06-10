use tauri::{AppHandle, Manager, State};
use tracing::info;

use crate::events::{emit_retry_state, RetryStatus};
use crate::services::retry_transcription::{
    build_retry_entry_updates, cleanup_retry_audio_file, mark_retry_entry_error,
    prepare_retry_transcription, transcribe_retry_audio_file, update_retry_entry_success,
};
use crate::state::app_state::AppState;

use super::polish::{maybe_polish_transcription_text, PolishProcessingResult};
use super::postprocess::apply_post_stt_processing;
use super::shared::{apply_retry_error, apply_retry_success, ProcessingEventTarget};

pub async fn retry_transcription_internal(
    app: AppHandle,
    _state: State<'_, AppState>,
    id: String,
) -> Result<String, String> {
    let state = app
        .try_state::<AppState>()
        .ok_or_else(|| "AppState not available".to_string())?;

    let entry = {
        let store = state.history_store.lock();
        store
            .get_entry(&id)
            .map_err(|e| format!("Failed to get entry: {e}"))?
    };

    let entry = entry.ok_or_else(|| "Entry not found".to_string())?;
    let prepared_retry = prepare_retry_transcription(&state, id, entry)?;
    let entry_id = prepared_retry.entry_id.clone();
    let audio_path = prepared_retry.audio_path.clone();
    let retry_task_id = prepared_retry.task_id;

    info!(
        entry_id = %entry_id,
        audio_path = %audio_path,
        task_id = retry_task_id,
        "retry_transcription_started"
    );

    emit_retry_state(&app, &entry_id, RetryStatus::Transcribing, retry_task_id);

    let text_result = transcribe_retry_audio_file(&state, &prepared_retry).await;

    match text_result {
        Ok(output) => {
            let app_clone = app.clone();
            let (correction_memory_enabled, user_glossary, custom_dictionary) = {
                let settings = state.settings.lock();
                (
                    settings.correction_memory_enabled,
                    settings.stt_engine_user_glossary.clone(),
                    settings.custom_dictionary.clone(),
                )
            };
            let postprocess = apply_post_stt_processing(
                &output.raw_text,
                correction_memory_enabled,
                &user_glossary,
                &custom_dictionary,
                retry_task_id,
                "retry",
            );
            let polish_result = if postprocess.text.is_empty() {
                PolishProcessingResult::skipped(String::new(), "empty postprocess text")
            } else {
                maybe_polish_transcription_text(
                    &ProcessingEventTarget::Retry {
                        app: &app,
                        entry_id: &entry_id,
                    },
                    &state,
                    retry_task_id,
                    postprocess.text,
                    None,
                )
                .await
            };
            let polish_time_ms = polish_result.polish_ms;
            let final_text = polish_result.text;

            if final_text.is_empty() {
                mark_retry_entry_error(&state, &entry_id, "Retry produced empty transcription")?;
                apply_retry_error(
                    &app_clone,
                    &entry_id,
                    retry_task_id,
                    "Retry produced empty transcription",
                );

                return Err("Retry produced empty transcription".to_string());
            }

            let updates = build_retry_entry_updates(&output, &final_text, polish_time_ms);
            update_retry_entry_success(&state, &entry_id, updates)?;
            cleanup_retry_audio_file(&audio_path);

            info!(
                entry_id = %entry_id,
                text_len = final_text.len(),
                postprocess_ms = postprocess.postprocess_ms,
                normalization_applied = postprocess.normalization_applied,
                corrections_applied = postprocess.corrections_applied,
                hotwords_applied = postprocess.hotwords_applied,
                glossary_applied = postprocess.glossary_applied,
                polish_ms = polish_time_ms,
                polish_wall_ms = polish_result.polish_wall_ms,
                polish_queue_ms = polish_result.polish_queue_ms,
                model_load_ms = polish_result.model_load_ms,
                context_create_ms = polish_result.context_create_ms,
                prefill_ms = polish_result.prefill_ms,
                inference_ms = polish_result.inference_ms,
                time_to_first_token_ms = polish_result.time_to_first_token_ms,
                generation_ms = polish_result.generation_ms,
                fallback_reason = polish_result.fallback_reason.unwrap_or(""),
                "retry_transcription_completed"
            );

            apply_retry_success(&app_clone, &entry_id, retry_task_id, &final_text).await;

            Ok(final_text)
        }
        Err(e) => {
            mark_retry_entry_error(&state, &entry_id, &e)?;
            apply_retry_error(&app, &entry_id, retry_task_id, &e);

            Err(format!("Transcription failed: {}", e))
        }
    }
}
