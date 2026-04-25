use std::sync::atomic::Ordering;
use tauri::{AppHandle, Manager, State};
use tracing::{debug, info, instrument};

use crate::events::{emit_recording_state, RecordingStatus};
use crate::services::recording_lifecycle::{prepare_recording_start, RecordingStartGuard};
use crate::shortcut::ShortcutProfile;
use crate::state::app_state::AppState;

use super::capture::start_unified_recording;

#[tauri::command]
#[instrument(skip(app, state), ret, err)]
pub async fn start_recording(app: AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    start_recording_sync(&app, None)?;
    let path = state.output_path.lock().clone().unwrap_or_default();
    Ok(path)
}

pub fn start_recording_sync(
    app: &AppHandle,
    profile: Option<&ShortcutProfile>,
) -> Result<(), String> {
    start_recording_sync_internal(app, profile, true)
}

pub(crate) fn start_recording_sync_internal(
    app: &AppHandle,
    profile: Option<&ShortcutProfile>,
    register_cancel_hotkey: bool,
) -> Result<(), String> {
    tracing::info!("start_recording_sync_entered");

    let state = app
        .try_state::<AppState>()
        .ok_or_else(|| "AppState not available".to_string())?;

    tracing::info!("start_recording_sync_state_acquired");

    if state.is_recording.load(Ordering::SeqCst) {
        tracing::warn!("start_recording_sync_already_recording");
        return Err("Already recording".to_string());
    }

    tracing::info!("start_recording_sync_positioning_pill");
    {
        let settings = state.settings.lock();
        let preset = settings.pill_position.clone();
        drop(settings);
        crate::commands::window::position_pill_window(app, &preset);
    }

    tracing::info!("start_recording_sync_updating_visibility");
    state.is_recording.store(true, Ordering::SeqCst);
    state.is_transcribing.store(false, Ordering::SeqCst);
    crate::commands::window::update_pill_visibility(app);

    tracing::info!("start_recording_sync_playing_beep");
    {
        let settings = state.settings.lock();
        let beep_enabled = settings.beep_on_record;
        drop(settings);

        debug!(beep_enabled, "beep_check-start_recording");
        if beep_enabled {
            debug!("beep_play-start");
            crate::audio::beep::play_start_beep();
        }
    }

    tracing::info!("start_recording_sync_reading_settings");
    let prepared = prepare_recording_start(&state, profile);
    tracing::info!(
        cloud_stt_enabled = prepared.cloud_stt_enabled,
        language = %prepared.language,
        polish_template_id = ?prepared.resolved_polish_template_id,
        "start_recording_sync_config"
    );
    tracing::info!(
        task_id = prepared.task_id,
        "start_recording_sync_starting_session"
    );

    let mut start_guard = RecordingStartGuard::new(&state, prepared.task_id);
    if let Err(err) = start_unified_recording(
        app,
        prepared.task_id,
        prepared.cloud_stt_enabled,
        prepared.cloud_stt_config,
        prepared.language,
        prepared.resolved_polish_template_id,
    ) {
        crate::commands::window::update_pill_visibility(app);
        return Err(err);
    }
    start_guard.commit();

    if let Some(tx) = state.level_monitor_tx.lock().as_ref() {
        let _ = tx.send(true);
    }

    info!(
        task_id = prepared.task_id,
        streaming = prepared.cloud_stt_enabled,
        "recording_started"
    );
    emit_recording_state(app, RecordingStatus::Recording, prepared.task_id);

    if register_cancel_hotkey {
        if let Some(shortcut_manager) = app.try_state::<crate::shortcut::ShortcutManager>() {
            let _ = shortcut_manager.register_cancel(prepared.task_id);
        }
    }

    Ok(())
}
