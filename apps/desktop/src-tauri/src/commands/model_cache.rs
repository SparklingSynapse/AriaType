use crate::state::app_state::AppState;
use crate::stt_engine::EngineType;
use tauri::State;
use tracing::instrument;

#[tauri::command]
pub fn get_model_status(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    // Query the current settings to determine which model should be loaded
    let (engine_str, model_name) = {
        let settings = state.settings.lock();
        (settings.stt_engine.clone(), settings.model.clone())
    };

    let engine_type: EngineType = engine_str
        .parse()
        .map_err(|e| format!("Invalid engine type: {}", e))?;

    // Check if the model is downloaded
    let is_downloaded = state
        .engine_manager
        .is_model_downloaded(engine_type, &model_name);

    Ok(serde_json::json!({
        "is_loaded": is_downloaded,
        "current_model": model_name,
        "engine_type": engine_str,
    }))
}

#[tauri::command]
#[instrument(skip(state), err)]
pub fn preload_model(state: State<'_, AppState>) -> Result<(), String> {
    let (engine_str, model_name) = {
        let settings = state.settings.lock();
        (settings.stt_engine.clone(), settings.model.clone())
    };

    let engine_type: EngineType = engine_str
        .parse()
        .map_err(|e| format!("Invalid engine type: {}", e))?;

    state.engine_manager.load_model(engine_type, &model_name)
}

#[tauri::command]
#[instrument(skip(state), err)]
pub fn unload_model(state: State<'_, AppState>) -> Result<(), String> {
    // Clear the engine cache to free memory
    state.engine_manager.clear_cache();
    Ok(())
}

// ==================== Polish Model Cache Commands ====================

#[tauri::command]
pub fn get_polish_model_status(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let (polish_enabled, polish_model_id) = {
        let settings = state.settings.lock();
        (settings.polish_enabled, settings.polish_model.clone())
    };

    if !polish_enabled || polish_model_id.is_empty() {
        return Ok(serde_json::json!({
            "is_loaded": false,
            "current_model": "",
            "engine_type": "",
        }));
    }

    // Auto-detect engine type
    let engine_type =
        crate::polish_engine::UnifiedPolishManager::get_engine_by_model_id(&polish_model_id);

    // Check if the model is downloaded
    let is_downloaded = if let Some(et) = engine_type {
        state
            .polish_manager
            .is_model_downloaded(et, &polish_model_id)
    } else {
        false
    };

    Ok(serde_json::json!({
        "is_loaded": is_downloaded,
        "current_model": polish_model_id,
        "engine_type": engine_type.map(|et| et.as_str()).unwrap_or(""),
    }))
}

#[tauri::command]
#[instrument(skip(state), err)]
pub fn preload_polish_model(state: State<'_, AppState>) -> Result<(), String> {
    let (polish_enabled, polish_model_id) = {
        let settings = state.settings.lock();
        (settings.polish_enabled, settings.polish_model.clone())
    };

    if !polish_enabled || polish_model_id.is_empty() {
        return Err("Polish is not enabled or no model selected".to_string());
    }

    // Auto-detect engine type
    let engine_type =
        crate::polish_engine::UnifiedPolishManager::get_engine_by_model_id(&polish_model_id)
            .ok_or_else(|| format!("Unknown polish model: {}", polish_model_id))?;

    state
        .polish_manager
        .load_model(engine_type, &polish_model_id)
}

#[tauri::command]
#[instrument(skip(state), err)]
pub fn unload_polish_model(state: State<'_, AppState>) -> Result<(), String> {
    // Clear the polish engine cache to free memory
    state.polish_manager.clear_cache();
    Ok(())
}
