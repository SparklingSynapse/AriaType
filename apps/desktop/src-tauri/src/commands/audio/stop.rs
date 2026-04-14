use tauri::{AppHandle, Manager, State};
use tracing::{debug, info, instrument, warn};

use crate::events::{emit_recording_state, RecordingStatus};
use crate::services::recording_lifecycle::prepare_recording_stop;
use crate::state::app_state::AppState;

use super::shared::{
    await_streaming_task_in_background, flush_pending_chunk_for_stop, send_flushed_chunk_for_stop,
};

#[tauri::command]
#[instrument(skip(app, _state), ret, err)]
pub async fn stop_recording(
    app: AppHandle,
    _state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let output_path = stop_recording_sync(app.clone())?;
    Ok(output_path)
}

pub fn stop_recording_sync(app: AppHandle) -> Result<Option<String>, String> {
    let state = app
        .try_state::<AppState>()
        .ok_or_else(|| "AppState not available".to_string())?;

    let Some(prepared) = prepare_recording_stop(&state) else {
        return Ok(None);
    };

    {
        let recorder = state.recorder.lock();
        recorder.stop().map_err(|e| e.to_string())?;
    }

    {
        let settings = state.settings.lock();
        let beep_enabled = settings.beep_on_record;
        drop(settings);

        debug!(beep_enabled, "beep_check-stop_recording");
        if beep_enabled {
            debug!("beep_play-stop");
            crate::audio::beep::play_stop_beep();
        }
    }

    let streaming_state = state.streaming_stt.lock().take();
    if let Some(stt) = streaming_state {
        if let Some(flushed_chunk) = flush_pending_chunk_for_stop(
            &stt.chunk_buffer,
            &stt.processor,
            stt.sample_rate,
            stt.channels,
        ) {
            if let Err(e) = tauri::async_runtime::block_on(send_flushed_chunk_for_stop(
                &stt.audio_tx,
                flushed_chunk,
            )) {
                warn!(task_id = prepared.task_id, error = %e, "audio_tail_flush_enqueue_failed");
            } else {
                info!(
                    task_id = prepared.task_id,
                    "audio_tail_flushed_before_finish"
                );
            }
        }

        info!(task_id = prepared.task_id, "stt_stopping-awaiting_final");
        if let Some(handle) = stt.streaming_task.lock().take() {
            await_streaming_task_in_background(prepared.task_id, handle);
        }
    } else {
        warn!(
            task_id = prepared.task_id,
            "streaming_state_missing-recording_interrupted"
        );
        emit_recording_state(&app, RecordingStatus::Idle, prepared.task_id);
    }

    Ok(None)
}
