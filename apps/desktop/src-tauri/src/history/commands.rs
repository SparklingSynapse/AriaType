use tauri::{AppHandle, State};

use super::models::{HistoryFilter, NewTranscriptionEntry, TranscriptionEntry};
use super::store::{EntryUpdates, HistoryStore};
use crate::state::app_state::AppState;

#[tauri::command]
pub fn get_transcription_history(
    state: State<'_, AppState>,
    filter: HistoryFilter,
) -> Result<Vec<TranscriptionEntry>, String> {
    let store = state.history_store.lock();
    store.get_history(&filter)
}

#[tauri::command]
pub fn get_transcription_entry(
    state: State<'_, AppState>,
    id: String,
) -> Result<Option<TranscriptionEntry>, String> {
    let store = state.history_store.lock();
    store.get_entry(&id)
}

#[tauri::command]
pub fn get_dashboard_stats(
    state: State<'_, AppState>,
) -> Result<super::models::DashboardStats, String> {
    let store = state.history_store.lock();
    store.get_dashboard_stats()
}

#[tauri::command]
pub fn get_daily_usage(
    state: State<'_, AppState>,
    days: u32,
) -> Result<Vec<super::models::DailyUsage>, String> {
    let store = state.history_store.lock();
    store.get_daily_usage(days)
}

#[tauri::command]
pub fn get_engine_usage(
    state: State<'_, AppState>,
) -> Result<Vec<super::models::EngineUsage>, String> {
    let store = state.history_store.lock();
    store.get_engine_usage()
}

#[tauri::command]
pub fn delete_transcription_entry(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let store = state.history_store.lock();
    store.delete_entry(&id)
}

#[tauri::command]
pub fn clear_transcription_history(state: State<'_, AppState>) -> Result<(), String> {
    let store = state.history_store.lock();
    store.clear_all()
}

/// Insert a history entry. Called internally from the transcription pipeline — not exposed to frontend.
pub fn save_history_entry(
    store: &HistoryStore,
    entry: NewTranscriptionEntry,
) -> Result<String, String> {
    store.insert(entry)
}

/// Update a history entry after retry. Called internally.
pub fn update_history_entry(
    store: &HistoryStore,
    id: &str,
    updates: EntryUpdates,
) -> Result<(), String> {
    store.update_entry(id, updates)
}

/// Mark a history entry as failed. Called internally.
pub fn mark_history_error(store: &HistoryStore, id: &str, error: &str) -> Result<(), String> {
    store.mark_error(id, error)
}

/// Retry transcription for a failed entry.
/// This is a Tauri command exposed to frontend.
#[tauri::command]
pub async fn retry_transcription(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<String, String> {
    crate::commands::audio::retry_transcription_internal(app, state, id).await
}

/// Save a successful recording entry to history.
pub fn save_to_history(
    state: &AppState,
    raw_text: &str,
    final_text: &str,
    stt_duration_ms: Option<i64>,
    polish_duration_ms: Option<i64>,
    polish_applied: bool,
    audio_path: Option<String>,
) {
    let (stt_engine, stt_model, language, is_cloud) = {
        let settings = state.settings.lock();
        let cloud_config = settings.get_active_cloud_stt_config();
        let is_cloud = cloud_config.enabled;
        let engine_str = if is_cloud {
            format!("cloud-{}", cloud_config.provider_type)
        } else {
            crate::stt_engine::UnifiedEngineManager::get_engine_by_model_name(&settings.model)
                .map(|et| et.as_str().to_string())
                .unwrap_or_else(|| "unknown".to_string())
        };
        (
            engine_str,
            if is_cloud {
                Some(cloud_config.model.clone())
            } else {
                Some(settings.model.clone())
            },
            if settings.stt_engine_language.is_empty() {
                None
            } else {
                Some(settings.stt_engine_language.clone())
            },
            is_cloud,
        )
    };

    let recording_duration_ms = {
        let start = state
            .recording_start_time
            .load(std::sync::atomic::Ordering::SeqCst);
        if start > 0 {
            Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as i64
                    - start as i64,
            )
        } else {
            None
        }
    };

    let total_ms = match (stt_duration_ms, polish_duration_ms) {
        (Some(stt), Some(pol)) => Some(stt + pol),
        (Some(stt), None) => Some(stt),
        _ => None,
    };

    let entry = NewTranscriptionEntry {
        raw_text: raw_text.to_string(),
        final_text: final_text.to_string(),
        stt_engine,
        stt_model,
        language,
        audio_duration_ms: recording_duration_ms,
        stt_duration_ms,
        polish_duration_ms,
        total_duration_ms: total_ms,
        polish_applied,
        polish_engine: None,
        is_cloud,
        audio_path,
        status: "success".to_string(),
        error: None,
    };

    let store = state.history_store.lock();
    if let Err(e) = save_history_entry(&store, entry) {
        tracing::warn!(error = %e, "failed_to_save_history");
    }
}

/// Save a failed recording entry to history.
pub fn save_failed_history(state: &AppState, audio_path: Option<String>, error: &str) {
    let (stt_engine, stt_model, language, is_cloud) = {
        let settings = state.settings.lock();
        let cloud_config = settings.get_active_cloud_stt_config();
        let is_cloud = cloud_config.enabled;
        let engine_str = if is_cloud {
            format!("cloud-{}", cloud_config.provider_type)
        } else {
            crate::stt_engine::UnifiedEngineManager::get_engine_by_model_name(&settings.model)
                .map(|et| et.as_str().to_string())
                .unwrap_or_else(|| "unknown".to_string())
        };
        (
            engine_str,
            if is_cloud {
                Some(cloud_config.model.clone())
            } else {
                Some(settings.model.clone())
            },
            if settings.stt_engine_language.is_empty() {
                None
            } else {
                Some(settings.stt_engine_language.clone())
            },
            is_cloud,
        )
    };

    let recording_duration_ms = {
        let start = state
            .recording_start_time
            .load(std::sync::atomic::Ordering::SeqCst);
        if start > 0 {
            Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as i64
                    - start as i64,
            )
        } else {
            None
        }
    };

    let entry = NewTranscriptionEntry {
        raw_text: String::new(),
        final_text: String::new(),
        stt_engine,
        stt_model,
        language,
        audio_duration_ms: recording_duration_ms,
        stt_duration_ms: None,
        polish_duration_ms: None,
        total_duration_ms: None,
        polish_applied: false,
        polish_engine: None,
        is_cloud,
        audio_path,
        status: "error".to_string(),
        error: Some(error.to_string()),
    };

    let store = state.history_store.lock();
    if let Err(e) = save_history_entry(&store, entry) {
        tracing::warn!(error = %e, "failed_to_save_failed_history");
    }
}
