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