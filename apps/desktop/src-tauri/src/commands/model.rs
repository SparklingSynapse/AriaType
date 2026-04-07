use crate::events::EventName;
use crate::polish_engine::{self as polish, PolishModel};
use crate::state::app_state::AppState;
use crate::stt_engine::{EngineType, ModelInfo};
use crate::utils::downloader::{download, DownloadOptions};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Instant;
use tauri::{AppHandle, Emitter, State};
use tracing::{error, info, instrument, warn};

// Legacy command for backward compatibility - returns all engines
#[tauri::command]
pub fn get_models(state: State<'_, AppState>) -> Vec<ModelInfo> {
    state.engine_manager.get_all_models()
}

// New command supporting multiple engines
#[tauri::command]
pub fn get_models_for_engine(
    engine: String,
    state: State<'_, AppState>,
) -> Result<Vec<ModelInfo>, String> {
    let engine_type: EngineType = engine.parse()?;
    Ok(state.engine_manager.get_models(engine_type))
}

// Legacy command for backward compatibility (Whisper only)
#[tauri::command]
pub fn is_model_downloaded(model_name: String, state: State<'_, AppState>) -> bool {
    let engine_type =
        crate::stt_engine::UnifiedEngineManager::get_engine_by_model_name(&model_name)
            .unwrap_or(EngineType::Whisper); // fallback to Whisper if unknown

    state
        .engine_manager
        .is_model_downloaded(engine_type, &model_name)
}

// New command supporting multiple engines
#[tauri::command]
pub fn is_model_downloaded_for_engine(
    engine: String,
    model_name: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let engine_type: EngineType = engine.parse()?;
    Ok(state
        .engine_manager
        .is_model_downloaded(engine_type, &model_name))
}

// Get recommended models for a language
#[tauri::command]
pub fn recommend_models_by_language(
    language: String,
    state: State<'_, AppState>,
) -> Vec<serde_json::Value> {
    state
        .engine_manager
        .recommend_by_language(&language)
        .into_iter()
        .map(|rec| {
            serde_json::json!({
                "engine_type": rec.engine_type.to_string(),
                "model_name": rec.model_name,
                "display_name": rec.display_name,
                "size_mb": rec.size_mb,
                "speed_score": rec.speed_score,
                "accuracy_score": rec.accuracy_score,
                "downloaded": rec.downloaded,
            })
        })
        .collect()
}

#[tauri::command]
#[instrument(skip(app, state), fields(model_name = %model_name), err)]
pub async fn download_model(
    app: AppHandle,
    model_name: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let start_time = Instant::now();

    // Auto-detect engine type from model name
    let engine_type =
        crate::stt_engine::UnifiedEngineManager::get_engine_by_model_name(&model_name)
            .ok_or_else(|| format!("Unknown model: {}", model_name))?;

    let cancel_flag = {
        let mut downloading = state.downloading_models.lock();
        if downloading.contains(&model_name) {
            warn!(model = %model_name, "download_rejected-duplicate");
            return Err(format!("Model {} is already downloading", model_name));
        }
        downloading.insert(model_name.clone());
        let flag = Arc::new(AtomicBool::new(false));
        state
            .download_cancellations
            .lock()
            .insert(model_name.clone(), flag.clone());
        info!(model = %model_name, engine = ?engine_type, "download_initiated");
        flag
    };

    let app_clone = app.clone();
    let model_name_clone = model_name.clone();

    let result = state
        .engine_manager
        .download_model(
            engine_type,
            &model_name,
            cancel_flag,
            move |downloaded, total| {
                let progress = if total > 0 {
                    (downloaded as f64 / total as f64 * 100.0) as u32
                } else {
                    0
                };
                if let Err(e) = app_clone.emit(
                    EventName::MODEL_DOWNLOAD_PROGRESS,
                    serde_json::json!({
                        "model": model_name_clone,
                        "downloaded": downloaded,
                        "total": total,
                        "progress": progress,
                    }),
                ) {
                    warn!(error = %e, model = %model_name_clone, "model_download_progress_emit_failed");
                }
            },
        )
        .await;

    let elapsed = start_time.elapsed();
    state.downloading_models.lock().remove(&model_name);
    state.download_cancellations.lock().remove(&model_name);

    match result {
        Err(ref e) if e == "cancelled" => {
            let path = state
                .engine_manager
                .get_model_path(engine_type, &model_name);
            if path.exists() {
                if let Err(e) = std::fs::remove_file(&path) {
                    warn!(error = %e, path = ?path, model = %model_name, "model_file_cleanup_failed-cancellation");
                }
            }
            let tmp_path = path.with_extension("bin.tmp");
            if tmp_path.exists() {
                if let Err(e) = std::fs::remove_file(&tmp_path) {
                    warn!(error = %e, path = ?tmp_path, model = %model_name, "temp_file_cleanup_failed-cancellation");
                }
            }
            warn!(
                model = %model_name,
                elapsed_secs = elapsed.as_secs(),
                "model_download_cancelled-temp_cleaned"
            );
            if let Err(e) = app.emit(
                EventName::MODEL_DOWNLOAD_CANCELLED,
                serde_json::json!({ "model": model_name }),
            ) {
                error!(error = %e, model = %model_name, "model_download_cancelled_emit_failed");
            }
            Ok(())
        }
        Err(e) => {
            error!(
                model = %model_name,
                elapsed_secs = elapsed.as_secs(),
                error = %e,
                "model_download_failed"
            );
            Err(e)
        }
        Ok(_) => {
            info!(
                model = %model_name,
                elapsed_secs = elapsed.as_secs(),
                elapsed_ms = elapsed.as_millis(),
                "model_download_completed"
            );
            if let Err(e) = app.emit(
                EventName::MODEL_DOWNLOAD_COMPLETE,
                serde_json::json!({ "model": model_name }),
            ) {
                error!(error = %e, model = %model_name, "model_download_complete_emit_failed");
            }
            Ok(())
        }
    }
}

#[tauri::command]
pub fn cancel_download(model_name: String, state: State<'_, AppState>) -> Result<(), String> {
    let cancellations = state.download_cancellations.lock();
    if let Some(flag) = cancellations.get(&model_name) {
        flag.store(true, Ordering::Relaxed);
        Ok(())
    } else {
        Err(format!("No active download for model {}", model_name))
    }
}

#[tauri::command]
#[instrument(skip(app, state), fields(model_name = %model_name), err)]
pub fn delete_model(
    app: AppHandle,
    model_name: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Auto-detect engine type from model name
    let engine_type =
        crate::stt_engine::UnifiedEngineManager::get_engine_by_model_name(&model_name)
            .ok_or_else(|| format!("Unknown model: {}", model_name))?;

    // Delete using the manager (which also clears cache)
    state
        .engine_manager
        .delete_model(engine_type, &model_name)?;

    // Clean up temp file if exists
    let path = state
        .engine_manager
        .get_model_path(engine_type, &model_name);
    let tmp_path = path.with_extension("bin.tmp");
    if tmp_path.exists() {
        if let Err(e) = std::fs::remove_file(&tmp_path) {
            warn!(error = %e, path = ?tmp_path, model = %model_name, "temp_file_cleanup_failed-deeletion");
        }
    }

    info!(model = %model_name, "model_deleted");
    if let Err(e) = app.emit(
        EventName::MODEL_DELETED,
        serde_json::json!({ "model": model_name }),
    ) {
        error!(error = %e, model = %model_name, "model_deleted_emit_failed");
    }
    Ok(())
}

#[tauri::command]
pub fn get_polish_models(_state: State<'_, AppState>) -> Vec<serde_json::Value> {
    polish::get_all_models()
        .into_iter()
        .map(|(id, name, size)| {
            let downloaded = PolishModel::from_id(&id)
                .map(polish::is_polish_model_downloaded_for)
                .unwrap_or(false);
            serde_json::json!({
                "id": id,
                "name": name,
                "size": size,
                "downloaded": downloaded
            })
        })
        .collect()
}

#[tauri::command]
pub fn get_current_polish_model(state: State<'_, AppState>) -> String {
    state.settings.lock().polish_model.clone()
}

#[tauri::command]
pub fn is_polish_model_downloaded(_state: State<'_, AppState>) -> bool {
    polish::is_polish_model_downloaded()
}

#[tauri::command]
pub fn is_polish_model_downloaded_for_model(model_id: String, _state: State<'_, AppState>) -> bool {
    PolishModel::from_id(&model_id)
        .map(polish::is_polish_model_downloaded_for)
        .unwrap_or(false)
}

#[tauri::command]
pub async fn download_polish_model(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    download_polish_model_internal(app, polish::get_current_model(), state).await
}

#[tauri::command]
pub async fn download_polish_model_by_id(
    app: AppHandle,
    model_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let model =
        PolishModel::from_id(&model_id).ok_or_else(|| format!("Unknown model: {}", model_id))?;
    download_polish_model_internal(app, model, state).await
}

#[tauri::command]
pub fn cancel_polish_download(model_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let cancellations = state.polish_download_cancellations.lock();
    if let Some(flag) = cancellations.get(&model_id) {
        flag.store(true, Ordering::Relaxed);
        Ok(())
    } else {
        Err(format!("No active download for model {}", model_id))
    }
}

async fn download_polish_model_internal(
    app: AppHandle,
    model: PolishModel,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let start_time = Instant::now();
    let model_id = model.id().to_string();

    let cancel_flag = {
        let mut cancellations = state.polish_download_cancellations.lock();
        if cancellations.contains_key(&model_id) {
            warn!(model_id = %model_id, "polish_download_rejected-duplicate");
            return Err(format!("Model {} is already downloading", model_id));
        }
        let flag = Arc::new(AtomicBool::new(false));
        cancellations.insert(model_id.clone(), flag.clone());
        info!(model_id = %model_id, "polish_model_download_initiated");
        flag
    };

    let model_path = polish::get_polish_model_path_for(model);
    let urls = model.urls();
    let app_clone = app.clone();
    let model_id_clone = model_id.clone();

    let progress_callback = Arc::new(move |downloaded: u64, total: u64| {
        let progress = if total > 0 {
            (downloaded as f64 / total as f64 * 100.0) as u32
        } else {
            0
        };
        if let Err(e) = app_clone.emit(
            EventName::POLISH_MODEL_DOWNLOAD_PROGRESS,
            serde_json::json!({
                "model_id": model_id_clone,
                "downloaded": downloaded,
                "total": total,
                "progress": progress
            }),
        ) {
            error!(error = %e, model_id = %model_id_clone, "polish_model_download_progress_emit_failed");
        }
    });

    let options = DownloadOptions::new(&urls[0], &model_path)
        .with_fallbacks(urls[1..].to_vec())
        .with_cancel_flag(cancel_flag)
        .with_progress_callback(progress_callback)
        .with_model_name(&model_id);

    let result = download(options).await;
    let elapsed = start_time.elapsed();
    state.polish_download_cancellations.lock().remove(&model_id);

    match result {
        Ok(_) => {
            info!(
                model_id = %model_id,
                elapsed_secs = elapsed.as_secs(),
                elapsed_ms = elapsed.as_millis(),
                "polish_model_download_completed"
            );
            if let Err(e) = app.emit(
                EventName::POLISH_MODEL_DOWNLOAD_COMPLETE,
                serde_json::json!({ "model_id": model_id }),
            ) {
                error!(error = %e, model_id = %model_id, "polish_model_download_complete_emit_failed");
            }
            Ok(())
        }
        Err(e) => {
            if e == "cancelled" {
                warn!(
                    model_id = %model_id,
                    elapsed_secs = elapsed.as_secs(),
                    "polish_model_download_cancelled"
                );
                if let Err(e) = app.emit(
                    EventName::POLISH_MODEL_DOWNLOAD_CANCELLED,
                    serde_json::json!({ "model_id": model_id }),
                ) {
                    error!(error = %e, model_id = %model_id, "polish_model_download_cancelled_emit_failed");
                }
                Ok(())
            } else {
                error!(
                    model_id = %model_id,
                    elapsed_secs = elapsed.as_secs(),
                    error = %e,
                    "polish_model_download_failed"
                );
                Err(e)
            }
        }
    }
}

#[tauri::command]
pub fn delete_polish_model(app: AppHandle, _state: State<'_, AppState>) -> Result<(), String> {
    delete_polish_model_internal(app, polish::get_current_model())
}

#[tauri::command]
pub fn delete_polish_model_by_id(
    app: AppHandle,
    model_id: String,
    _state: State<'_, AppState>,
) -> Result<(), String> {
    let model =
        PolishModel::from_id(&model_id).ok_or_else(|| format!("Unknown model: {}", model_id))?;
    delete_polish_model_internal(app, model)
}

fn delete_polish_model_internal(app: AppHandle, model: PolishModel) -> Result<(), String> {
    let path = polish::get_polish_model_path_for(model);
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| format!("polish_model_delete_failed: {e}"))?;
    }
    info!(model_id = ?model, "polish_model_deleted");

    if let Err(e) = app.emit(
        EventName::POLISH_MODEL_DELETED,
        serde_json::json!({ "model_id": model.id() }),
    ) {
        error!(error = %e, model_id = ?model, "polish_model_deleted_emit_failed");
    }
    Ok(())
}

#[tauri::command]
pub fn get_polish_templates() -> Vec<serde_json::Value> {
    polish::get_all_templates()
        .iter()
        .map(|(id, name, description)| {
            serde_json::json!({
                "id": id,
                "name": name,
                "description": description
            })
        })
        .collect()
}

#[tauri::command]
pub fn get_polish_template_prompt(
    template_id: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    if template_id.starts_with("user_") {
        let settings = state.settings.lock();
        settings
            .polish_custom_templates
            .iter()
            .find(|t| t.id == template_id)
            .map(|t| t.system_prompt.clone())
            .ok_or_else(|| format!("template not found: {}", template_id))
    } else {
        polish::get_template_by_id(&template_id)
            .map(|t| t.system_prompt.to_string())
            .ok_or_else(|| format!("unknown template: {}", template_id))
    }
}

fn generate_template_id() -> String {
    format!("user_{}", uuid::Uuid::new_v4())
}

#[tauri::command]
pub fn create_polish_custom_template(
    name: String,
    system_prompt: String,
    state: State<'_, AppState>,
) -> Result<crate::commands::settings::CustomPolishTemplate, String> {
    let id = generate_template_id();
    let template = crate::commands::settings::CustomPolishTemplate {
        id: id.clone(),
        name,
        system_prompt,
    };

    {
        let mut settings = state.settings.lock();
        settings.polish_custom_templates.push(template.clone());

        let path = crate::utils::AppPaths::data_dir().join("settings.json");
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let json = serde_json::to_string_pretty(&*settings).map_err(|e| e.to_string())?;
        std::fs::write(&path, json).map_err(|e| e.to_string())?;
    }

    info!(template_id = %id, "polish_custom_template_created");
    Ok(template)
}

#[tauri::command]
pub fn update_polish_custom_template(
    id: String,
    name: String,
    system_prompt: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    {
        let mut settings = state.settings.lock();
        let template = settings
            .polish_custom_templates
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or_else(|| format!("template not found: {}", id))?;

        template.name = name;
        template.system_prompt = system_prompt;

        let path = crate::utils::AppPaths::data_dir().join("settings.json");
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let json = serde_json::to_string_pretty(&*settings).map_err(|e| e.to_string())?;
        std::fs::write(&path, json).map_err(|e| e.to_string())?;
    }

    info!(template_id = %id, "polish_custom_template_updated");
    Ok(())
}

#[tauri::command]
pub fn delete_polish_custom_template(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let selected_template: String;
    let custom_templates_count: usize;

    {
        let mut settings = state.settings.lock();
        let original_len = settings.polish_custom_templates.len();
        settings.polish_custom_templates.retain(|t| t.id != id);

        if settings.polish_custom_templates.len() == original_len {
            return Err(format!("template not found: {}", id));
        }

        selected_template = settings.polish_selected_template.clone();
        custom_templates_count = settings.polish_custom_templates.len();

        if settings.polish_selected_template == id {
            settings.polish_selected_template = polish::POLISH_TEMPLATES[0].id.to_string();
            settings.polish_system_prompt = polish::POLISH_TEMPLATES[0].system_prompt.to_string();
        }

        let path = crate::utils::AppPaths::data_dir().join("settings.json");
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let json = serde_json::to_string_pretty(&*settings).map_err(|e| e.to_string())?;
        std::fs::write(&path, json).map_err(|e| e.to_string())?;
    }

    info!(template_id = %id, was_selected = %selected_template, remaining = custom_templates_count, "polish_custom_template_deleted");
    Ok(())
}

#[tauri::command]
pub fn select_polish_template(
    template_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    {
        let mut settings = state.settings.lock();

        let prompt = if template_id.starts_with("user_") {
            settings
                .polish_custom_templates
                .iter()
                .find(|t| t.id == template_id)
                .map(|t| t.system_prompt.clone())
                .ok_or_else(|| format!("template not found: {}", template_id))?
        } else if let Some(t) = polish::get_template_by_id(&template_id) {
            t.system_prompt.to_string()
        } else {
            return Err(format!("unknown template: {}", template_id));
        };

        settings.polish_selected_template = template_id.clone();
        settings.polish_system_prompt = prompt;

        let path = crate::utils::AppPaths::data_dir().join("settings.json");
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let json = serde_json::to_string_pretty(&*settings).map_err(|e| e.to_string())?;
        std::fs::write(&path, json).map_err(|e| e.to_string())?;
    }

    info!(template_id = %template_id, "polish_template_selected");
    Ok(())
}

#[tauri::command]
pub fn get_polish_selected_template(state: State<'_, AppState>) -> Result<String, String> {
    let settings = state.settings.lock();
    let selected = settings.polish_selected_template.clone();

    let valid = if selected.starts_with("user_") {
        settings
            .polish_custom_templates
            .iter()
            .any(|t| t.id == selected)
    } else {
        polish::get_template_by_id(&selected).is_some()
    };

    if valid {
        Ok(selected)
    } else {
        Ok(polish::POLISH_TEMPLATES[0].id.to_string())
    }
}

#[tauri::command]
pub fn get_polish_custom_templates(
    state: State<'_, AppState>,
) -> Vec<crate::commands::settings::CustomPolishTemplate> {
    let settings = state.settings.lock();
    settings.polish_custom_templates.clone()
}
