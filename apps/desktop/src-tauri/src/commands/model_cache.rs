use crate::state::app_state::AppState;
use crate::stt_engine::EngineType;
use std::sync::Arc;
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
pub async fn get_polish_model_status(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let polish_model_id = {
        let settings = state.settings.lock();
        settings.polish_model.clone()
    };

    build_polish_model_status(state.polish_manager.clone(), polish_model_id).await
}

async fn build_polish_model_status(
    polish_manager: Arc<crate::polish_engine::UnifiedPolishManager>,
    polish_model_id: String,
) -> Result<serde_json::Value, String> {
    if polish_model_id.is_empty() {
        return Ok(serde_json::json!({
            "is_loaded": false,
            "is_downloaded": false,
            "runtime_ready": false,
            "current_model": "",
            "engine_type": "",
        }));
    }

    tokio::task::spawn_blocking(move || {
        // Auto-detect engine type
        let engine_type =
            crate::polish_engine::UnifiedPolishManager::get_engine_by_model_id(&polish_model_id);

        // Check if the model is downloaded
        let is_downloaded = if let Some(et) = engine_type {
            polish_manager.is_model_downloaded(et, &polish_model_id)
        } else {
            false
        };
        let runtime_ready = polish_manager.is_local_runtime_ready();

        Ok(serde_json::json!({
            "is_loaded": is_downloaded && runtime_ready,
            "is_downloaded": is_downloaded,
            "runtime_ready": runtime_ready,
            "current_model": polish_model_id,
            "engine_type": engine_type.map(|et| et.as_str()).unwrap_or(""),
        }))
    })
    .await
    .map_err(|e| format!("Polish model status task failed: {e}"))?
}

#[tauri::command]
#[instrument(skip(state), err)]
pub fn preload_polish_model(state: State<'_, AppState>) -> Result<(), String> {
    let polish_model_id = {
        let settings = state.settings.lock();
        settings.polish_model.clone()
    };

    if polish_model_id.is_empty() {
        return Err("No polish model selected".to_string());
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::settings::LocalPolishRuntimeSettings;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn polish_model_status_runtime_probe_does_not_panic_in_async_context() {
        let polish_manager = Arc::new(crate::polish_engine::UnifiedPolishManager::new());
        polish_manager
            .configure_local_runtime(&LocalPolishRuntimeSettings {
                provider_type: "llama-server".to_string(),
                base_url: "http://127.0.0.1:1/v1".to_string(),
                api_key: String::new(),
                server_command: String::new(),
                server_args_json: String::new(),
                ready_timeout_secs: 1,
            })
            .unwrap();

        let status = build_polish_model_status(polish_manager, "qwen3.5-0.8b".to_string())
            .await
            .unwrap();

        assert_eq!(status["current_model"], "qwen3.5-0.8b");
        assert_eq!(status["engine_type"], "qwen");
        assert_eq!(status["runtime_ready"], false);
    }
}
