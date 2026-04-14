use tauri::{AppHandle, Manager};

use crate::events::{emit_recording_state, RecordingStatus};
use crate::services::recording_lifecycle::prepare_recording_cancellation;
use crate::state::app_state::AppState;

use super::shared::await_streaming_task_in_background;

#[tauri::command]
pub async fn cancel_recording(app: AppHandle) -> Result<(), String> {
    cancel_recording_sync(app)
}

pub fn cancel_recording_sync(app: AppHandle) -> Result<(), String> {
    cancel_recording_internal(app, true)
}

pub fn cancel_recording_from_hotkey_sync(app: AppHandle) -> Result<(), String> {
    cancel_recording_internal(app, false)
}

fn cancel_recording_internal(
    app: AppHandle,
    unregister_cancel_hotkey_immediately: bool,
) -> Result<(), String> {
    tracing::info!("cancel_recording_sync_entered");

    let state = app
        .try_state::<AppState>()
        .ok_or_else(|| "AppState not available".to_string())?;

    let active_task_id = state.task_counter.load(std::sync::atomic::Ordering::SeqCst);

    if unregister_cancel_hotkey_immediately {
        if let Some(shortcut_manager) = app.try_state::<crate::shortcut::ShortcutManager>() {
            let _ = shortcut_manager.unregister_cancel_for_task(active_task_id);
        }
    }

    let Some(prepared) = prepare_recording_cancellation(&state) else {
        return Ok(());
    };

    if prepared.should_stop_recorder {
        let recorder = state.recorder.lock();
        let _ = recorder.stop();
    }

    if let Some(stt) = state.streaming_stt.lock().take() {
        if let Some(handle) = stt.streaming_task.lock().take() {
            await_streaming_task_in_background(prepared.task_id, handle);
        }
    }

    crate::commands::window::update_pill_visibility(&app);
    emit_recording_state(&app, RecordingStatus::Idle, prepared.task_id);

    tracing::info!(task_id = prepared.task_id, "recording_canceled");
    Ok(())
}
