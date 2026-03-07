use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};
use tracing::info;

use crate::commands::window::position_pill_window;
use crate::events::EventName;
use crate::state::app_state::AppState;
use crate::utils::AppPaths;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    pub hotkey: String,
    pub model: String,
    pub stt_engine: String,
    pub pill_position: String,
    pub pill_indicator_mode: String,
    pub auto_start: bool,
    pub gpu_acceleration: bool,
    pub language: String,
    pub stt_engine_language: String,
    pub beep_on_record: bool,
    pub audio_device: String,
    pub polish_enabled: bool,
    pub polish_system_prompt: String,
    pub polish_model: String,
    pub theme_mode: String,
    /// Script-bias prompt passed directly to Whisper's initial_prompt field.
    /// Set by the frontend when the user picks a language; backend is unaware of specifics.
    pub stt_engine_initial_prompt: String,
    pub model_resident: bool,
    pub idle_unload_minutes: u32,
    pub denoise_mode: String,
    /// Domain for transcription (general, it, legal, medical)
    pub stt_engine_work_domain: String,
    /// Domain-specific prompt template
    pub stt_engine_work_domain_prompt: String,
    /// Glossary subdomain (e.g., it_general, legal_civil)
    pub stt_engine_work_subdomain: String,
    /// Glossary terms (comma or newline separated)
    pub stt_engine_user_glossary: String,
    pub analytics_opt_in: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            hotkey: "shift+space".to_string(),
            model: "base".to_string(),
            stt_engine: "whisper".to_string(),
            pill_position: "bottom-center".to_string(),
            pill_indicator_mode: "always".to_string(),
            auto_start: false,
            gpu_acceleration: true,
            language: "auto".to_string(),
            stt_engine_language: "auto".to_string(),
            beep_on_record: true,
            audio_device: "default".to_string(),
            polish_enabled: false,
            polish_system_prompt: crate::polish_engine::DEFAULT_POLISH_PROMPT.to_string(),
            polish_model: String::new(),
            theme_mode: "system".to_string(),
            stt_engine_initial_prompt: String::new(),
            model_resident: true,
            idle_unload_minutes: 5,
            denoise_mode: "auto".to_string(),
            stt_engine_work_domain: "general".to_string(),
            stt_engine_work_domain_prompt: String::new(),
            stt_engine_work_subdomain: String::new(),
            stt_engine_user_glossary: String::new(),
            analytics_opt_in: false,
        }
    }
}

fn get_settings_path() -> PathBuf {
    AppPaths::data_dir().join("settings.json")
}

pub fn load_settings_from_disk() -> AppSettings {
    let path = get_settings_path();
    if path.exists() {
        if let Ok(json) = fs::read_to_string(&path) {
            match serde_json::from_str::<AppSettings>(&json) {
                Ok(settings) => {
                    tracing::info!(path = %path.display(), settings = %json.trim(), "loaded settings from disk");
                    return settings;
                }
                Err(e) => {
                    tracing::warn!(error = %e, "failed to parse settings.json, using defaults")
                }
            }
        }
    } else {
        tracing::info!(path = %path.display(), "settings.json not found, using defaults");
    }
    AppSettings::default()
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    let settings = state.settings.lock();
    Ok(settings.clone())
}

#[tauri::command]
pub fn update_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    key: String,
    value: serde_json::Value,
) -> Result<(), String> {
    let mut model_to_reload: Option<String> = None;
    let mut should_unload_model = false;
    let mut should_load_model: Option<String> = None;
    let mut should_reload_for_gpu: Option<String> = None;
    let mut hotkey_to_register: Option<String> = None;
    let preset_to_apply: Option<String>;
    let indicator_mode_to_apply: Option<String>;

    {
        let mut settings = state.settings.lock();

        match key.as_str() {
            "hotkey" => {
                if let Some(v) = value.as_str() {
                    settings.hotkey = v.to_string();
                    hotkey_to_register = Some(v.to_string());
                }
            }
            "model" => {
                if let Some(v) = value.as_str() {
                    if settings.model != v {
                        model_to_reload = Some(v.to_string());
                        // Auto-update stt_engine based on model name
                        if let Some(engine_type) =
                            crate::stt_engine::UnifiedEngineManager::get_engine_by_model_name(v)
                        {
                            settings.stt_engine = engine_type.to_string();
                        }
                    }
                    settings.model = v.to_string();
                }
            }
            "stt_engine" => {
                if let Some(v) = value.as_str() {
                    settings.stt_engine = v.to_string();
                }
            }
            "pill_position" => {
                if let Some(v) = value.as_str() {
                    settings.pill_position = v.to_string();
                }
            }
            "pill_indicator_mode" => {
                if let Some(v) = value.as_str() {
                    settings.pill_indicator_mode = v.to_string();
                }
            }
            "auto_start" => {
                if let Some(v) = value.as_bool() {
                    settings.auto_start = v;
                }
            }
            "gpu_acceleration" => {
                if let Some(v) = value.as_bool() {
                    if v != settings.gpu_acceleration && settings.model_resident {
                        // GPU setting changed while model is resident — reload with new setting
                        should_reload_for_gpu = Some(settings.model.clone());
                    }
                    settings.gpu_acceleration = v;
                }
            }
            "language" => {
                if let Some(v) = value.as_str() {
                    settings.language = v.to_string();
                }
            }
            "stt_engine_language" => {
                if let Some(v) = value.as_str() {
                    settings.stt_engine_language = v.to_string();
                }
            }
            "beep_on_record" => {
                if let Some(v) = value.as_bool() {
                    settings.beep_on_record = v;
                }
            }
            "audio_device" => {
                if let Some(v) = value.as_str() {
                    settings.audio_device = v.to_string();
                }
            }
            "polish_enabled" => {
                if let Some(v) = value.as_bool() {
                    settings.polish_enabled = v;
                }
            }
            "polish_system_prompt" => {
                if let Some(v) = value.as_str() {
                    settings.polish_system_prompt = v.to_string();
                }
            }
            "polish_model" => {
                if let Some(v) = value.as_str() {
                    settings.polish_model = v.to_string();
                }
            }
            "theme_mode" => {
                if let Some(v) = value.as_str() {
                    settings.theme_mode = v.to_string();
                }
            }
            "stt_engine_initial_prompt" => {
                if let Some(v) = value.as_str() {
                    settings.stt_engine_initial_prompt = v.to_string();
                }
            }
            "model_resident" => {
                if let Some(v) = value.as_bool() {
                    if !v && settings.model_resident {
                        // switching off: schedule model unload after lock is released
                        should_unload_model = true;
                    } else if v && !settings.model_resident {
                        // switching on: schedule model preload after lock is released
                        should_load_model = Some(settings.model.clone());
                    }
                    settings.model_resident = v;
                }
            }
            "idle_unload_minutes" => {
                if let Some(v) = value.as_u64() {
                    settings.idle_unload_minutes = v as u32;
                }
            }
            "denoise_mode" => {
                if let Some(v) = value.as_str() {
                    settings.denoise_mode = v.to_string();
                }
            }
            "stt_engine_work_domain" => {
                if let Some(v) = value.as_str() {
                    settings.stt_engine_work_domain = v.to_string();
                }
            }
            "stt_engine_work_domain_prompt" => {
                if let Some(v) = value.as_str() {
                    settings.stt_engine_work_domain_prompt = v.to_string();
                }
            }
            "stt_engine_work_subdomain" => {
                if let Some(v) = value.as_str() {
                    settings.stt_engine_work_subdomain = v.to_string();
                }
            }
            "stt_engine_user_glossary" => {
                if let Some(v) = value.as_str() {
                    settings.stt_engine_user_glossary = v.to_string();
                }
            }
            "analytics_opt_in" => {
                if let Some(v) = value.as_bool() {
                    settings.analytics_opt_in = v;
                }
            }
            _ => return Err(format!("Unknown setting key: {}", key)),
        }

        let path = get_settings_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let json = serde_json::to_string_pretty(&*settings).map_err(|e| e.to_string())?;
        fs::write(&path, json).map_err(|e| e.to_string())?;

        preset_to_apply = if key == "pill_position" {
            Some(settings.pill_position.clone())
        } else {
            None
        };

        // Check if pill_indicator_mode changed
        indicator_mode_to_apply = if key == "pill_indicator_mode" {
            Some(settings.pill_indicator_mode.clone())
        } else {
            None
        };

        info!(key = %key, "settings updated");
        let _ = app.emit(EventName::SETTINGS_CHANGED, settings.clone());
    } // lock released here

    if let Some(preset) = preset_to_apply {
        position_pill_window(&app, &preset);
    }

    if let Some(hotkey) = hotkey_to_register {
        let _ = app.global_shortcut().unregister_all();
        match register_global_shortcut(&app, &hotkey) {
            Ok(_) => info!(hotkey = %hotkey, "global shortcut re-registered"),
            Err(e) => tracing::error!(error = %e, "failed to re-register global shortcut"),
        }
    }

    if indicator_mode_to_apply.is_some() {
        crate::commands::window::update_pill_visibility(&app);
    }

    if should_unload_model {
        // Model unloading is now handled by UnifiedEngineManager's engine cache
        tracing::info!(
            mem_mb = get_process_rss_mb(),
            "model unload requested: model_resident turned off"
        );
    }

    if let Some(model_name) = should_load_model {
        // Model preloading is now handled by UnifiedEngineManager's engine cache
        tracing::info!(model = %model_name, mem_mb = get_process_rss_mb(), "model preload requested: model_resident turned on");
    }

    if let Some(model_name) = should_reload_for_gpu {
        // Model reloading is now handled by UnifiedEngineManager's engine cache
        tracing::info!(model = %model_name, mem_mb = get_process_rss_mb(), "model reload requested: gpu_acceleration changed");
    }

    if let Some(model_name) = model_to_reload {
        // Model hot-reloading is now handled by UnifiedEngineManager's engine cache
        tracing::info!(model = %model_name, mem_mb = get_process_rss_mb(), "model hot-reload requested");
    }

    Ok(())
}

#[tauri::command]
pub fn set_hotkey_capture_mode(
    app: AppHandle,
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<(), String> {
    state.hotkey_capture_mode.store(enabled, Ordering::SeqCst);

    if enabled {
        let _ = app.global_shortcut().unregister_all();
        info!("Unregistered all global shortcuts for hotkey capture mode");
    } else {
        let hotkey = state.settings.lock().hotkey.clone();
        match register_global_shortcut(&app, &hotkey) {
            Ok(_) => info!(hotkey = %hotkey, "Re-registered global shortcut"),
            Err(e) => {
                tracing::error!(error = %e, "Failed to re-register global shortcut");
            }
        }
    }

    Ok(())
}

fn parse_hotkey(hotkey: &str) -> Option<(Option<Modifiers>, Code)> {
    let parts: Vec<&str> = hotkey.split('+').collect();
    if parts.is_empty() {
        return None;
    }

    let mut modifier_set = Modifiers::empty();
    for part in &parts[..parts.len() - 1] {
        match part.to_lowercase().as_str() {
            "ctrl" => modifier_set |= Modifiers::CONTROL,
            "shift" => modifier_set |= Modifiers::SHIFT,
            "alt" => modifier_set |= Modifiers::ALT,
            "cmd" | "command" | "meta" => modifier_set |= Modifiers::SUPER,
            _ => {}
        }
    }
    let modifiers = if modifier_set.is_empty() {
        None
    } else {
        Some(modifier_set)
    };

    let key = parts.last()?;

    let code_name = if key.to_lowercase() == "space" {
        "Space".to_string()
    } else if key.len() == 1 && key.chars().next().unwrap().is_ascii_alphabetic() {
        format!("Key{}", key.to_uppercase())
    } else if key.len() == 1 && key.chars().next().unwrap().is_ascii_digit() {
        format!("Digit{}", key)
    } else {
        let mut chars = key.chars();
        chars
            .next()
            .map(|c| c.to_uppercase().to_string())
            .unwrap_or_default()
            + chars.as_str()
    };

    let code = Code::from_str(&code_name).ok()?;
    Some((modifiers, code))
}

pub fn register_global_shortcut(app: &AppHandle, hotkey: &str) -> Result<(), String> {
    let (modifiers, code) =
        parse_hotkey(hotkey).ok_or_else(|| format!("Invalid hotkey format: {}", hotkey))?;

    let shortcut = Shortcut::new(modifiers, code);
    app.global_shortcut()
        .on_shortcut(shortcut, |app, _shortcut, _event| {
            tracing::debug!("global shortcut triggered");

            let state_result = app.try_state::<AppState>();
            match state_result {
                Some(state) => {
                    if state.hotkey_capture_mode.load(Ordering::SeqCst) {
                        tracing::debug!("hotkey capture mode active, ignoring shortcut");
                        return;
                    }

                    let is_recording = state.is_recording.load(Ordering::SeqCst);

                    if is_recording {
                        match crate::commands::audio::stop_recording_sync(app.clone()) {
                            Ok(_) => {}
                            Err(e) => tracing::error!(error = %e, "failed to stop recording"),
                        }
                    } else {
                        match crate::commands::audio::start_recording_sync(app.clone()) {
                            Ok(_) => {}
                            Err(e) => tracing::error!(error = %e, "failed to start recording"),
                        }
                    }
                }
                None => {
                    tracing::error!("could not get AppState from app handle");
                }
            }
        })
        .map_err(|e| e.to_string())
}

/// Returns the current process RSS memory in MB, or 0 if unavailable.
fn get_process_rss_mb() -> u64 {
    let pid = std::process::id();
    std::process::Command::new("ps")
        .args(["-o", "rss=", "-p", &pid.to_string()])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.trim().parse::<u64>().ok())
        .map(|kb| kb / 1024)
        .unwrap_or(0)
}

fn get_subdomains_for_domain(domain: &str) -> Vec<String> {
    match domain {
        "it" => vec![
            "general".to_string(),
            "security".to_string(),
            "hardware".to_string(),
            "software".to_string(),
            "web".to_string(),
            "ai".to_string(),
        ],
        "legal" => vec![
            "general".to_string(),
            "civil".to_string(),
            "criminal".to_string(),
            "corporate".to_string(),
            "international".to_string(),
        ],
        "medical" => vec![
            "general".to_string(),
            "pharmacy".to_string(),
            "diagnostics".to_string(),
            "cardiology".to_string(),
            "neurology".to_string(),
        ],
        _ => vec![],
    }
}

#[tauri::command]
pub fn get_glossary_content(_subdomain: String) -> Result<String, String> {
    // User maintains their own glossary - no default content
    Ok(String::new())
}

#[tauri::command]
pub fn get_available_subdomains(domain: String) -> Result<Vec<String>, String> {
    Ok(get_subdomains_for_domain(&domain))
}
