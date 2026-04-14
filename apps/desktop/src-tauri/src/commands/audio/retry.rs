use tauri::{AppHandle, Manager, State};
use tracing::info;

use crate::events::{emit_retry_state, RetryStatus};
use crate::services::retry_transcription::{
    build_retry_entry_updates, cleanup_retry_audio_file, mark_retry_entry_error,
    prepare_retry_transcription, transcribe_retry_audio_file, update_retry_entry_success,
};
use crate::state::app_state::AppState;

use super::polish::maybe_polish_transcription_text;
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
            let (final_text, polish_time_ms) = if output.raw_text.is_empty() {
                (String::new(), 0)
            } else {
                maybe_polish_transcription_text(
                    &ProcessingEventTarget::Retry {
                        app: &app,
                        entry_id: &entry_id,
                    },
                    &state,
                    retry_task_id,
                    output.raw_text.clone(),
                )
                .await
            };

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
