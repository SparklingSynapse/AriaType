use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use tracing::info;

use crate::commands::window::position_pill_window;
use crate::events::EventName;
use crate::state::app_state::AppState;
use crate::utils::AppPaths;

/// Cloud provider configuration for polish
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CloudProviderConfig {
    pub enabled: bool,
    pub provider_type: String,
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub enable_thinking: bool,
}

/// Cloud provider configuration for STT
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CloudSttConfig {
    pub enabled: bool,
    pub provider_type: String,
    pub api_key: String,
    pub app_id: String,
    pub base_url: String,
    pub model: String,
    pub language: String,
}

/// User-defined custom polish template
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CustomPolishTemplate {
    pub id: String,
    pub name: String,
    pub system_prompt: String,
}

// Legacy config structs for migration from old format
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
struct LegacyCloudProviderConfig {
    pub enabled: bool,
    pub provider_type: String,
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub enable_thinking: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
struct LegacyCloudSttConfig {
    pub enabled: bool,
    pub provider_type: String,
    pub api_key: String,
    pub app_id: String,
    pub base_url: String,
    pub model: String,
    pub language: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    pub hotkey: String,
    pub recording_mode: String,
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
    /// Whether cloud STT is enabled globally
    pub cloud_stt_enabled: bool,
    /// Currently active cloud STT provider (e.g., "volcengine-streaming", "openai")
    pub active_cloud_stt_provider: String,
    /// Per-provider cloud STT configurations, keyed by provider_type
    pub cloud_stt_configs: HashMap<String, CloudSttConfig>,
    /// Whether cloud polish is enabled globally
    pub cloud_polish_enabled: bool,
    /// Currently active cloud polish provider (e.g., "anthropic", "openai")
    pub active_cloud_polish_provider: String,
    /// Per-provider cloud polish configurations, keyed by provider_type
    pub cloud_polish_configs: HashMap<String, CloudProviderConfig>,
    /// Whether to enable Voice Activity Detection (VAD) for silence trimming
    pub vad_enabled: bool,
    /// Whether app should stay in system tray when hidden (macOS only)
    pub stay_in_tray: bool,
    /// Currently selected polish template ID (built-in or user-defined)
    pub polish_selected_template: String,
    /// User-defined custom polish templates
    pub polish_custom_templates: Vec<CustomPolishTemplate>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            hotkey: "shift+space".to_string(),
            recording_mode: "hold".to_string(),
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
            denoise_mode: "off".to_string(),
            stt_engine_work_domain: "general".to_string(),
            stt_engine_work_domain_prompt: String::new(),
            stt_engine_work_subdomain: String::new(),
            stt_engine_user_glossary: String::new(),
            analytics_opt_in: false,
            cloud_stt_enabled: false,
            active_cloud_stt_provider: "volcengine-streaming".to_string(),
            cloud_stt_configs: HashMap::new(),
            cloud_polish_enabled: false,
            active_cloud_polish_provider: "anthropic".to_string(),
            cloud_polish_configs: HashMap::new(),
            vad_enabled: false,
            stay_in_tray: false,
            polish_selected_template: "filler".to_string(),
            polish_custom_templates: Vec::new(),
        }
    }
}

impl AppSettings {
    pub fn get_active_cloud_stt_config(&self) -> CloudSttConfig {
        let mut config = self
            .cloud_stt_configs
            .get(&self.active_cloud_stt_provider)
            .cloned()
            .unwrap_or_default();
        config.enabled = self.cloud_stt_enabled;
        config.provider_type = self.active_cloud_stt_provider.clone();
        config
    }

    pub fn get_active_cloud_polish_config(&self) -> CloudProviderConfig {
        let mut config = self
            .cloud_polish_configs
            .get(&self.active_cloud_polish_provider)
            .cloned()
            .unwrap_or_default();
        config.enabled = self.cloud_polish_enabled;
        config.provider_type = self.active_cloud_polish_provider.clone();
        config
    }

    /// Check if any streaming cloud STT provider is active
    pub fn is_streaming_stt_active(&self) -> bool {
        self.cloud_stt_enabled
            && matches!(
                self.active_cloud_stt_provider.as_str(),
                "volcengine-streaming" | "qwen-omni-realtime" | "elevenlabs"
            )
    }

    /// Legacy method - use is_streaming_stt_active instead
    #[deprecated(note = "Use is_streaming_stt_active instead")]
    pub fn is_volcengine_streaming_active(&self) -> bool {
        self.is_streaming_stt_active()
    }
}

fn get_settings_path() -> PathBuf {
    AppPaths::data_dir().join("settings.json")
}

/// Migrate old cloud settings format to new per-provider format.
/// Old: single cloud_stt/cloud_polish objects with enabled/provider_type inside.
/// New: cloud_stt_enabled, active_cloud_stt_provider, cloud_stt_configs HashMap.
fn migrate_cloud_settings(json: &mut serde_json::Value) -> bool {
    let mut migrated = false;

    if let Some(old_stt) = json.get("cloud_stt").cloned() {
        if let Ok(legacy_config) = serde_json::from_value::<LegacyCloudSttConfig>(old_stt.clone()) {
            let provider_type = resolve_stt_provider_type(&legacy_config, &old_stt);

            let new_config = CloudSttConfig {
                enabled: legacy_config.enabled,
                provider_type: provider_type.clone(),
                api_key: legacy_config.api_key,
                app_id: legacy_config.app_id,
                base_url: legacy_config.base_url,
                model: legacy_config.model,
                language: legacy_config.language,
            };

            let mut configs = HashMap::new();
            configs.insert(provider_type.clone(), new_config);

            json["cloud_stt_enabled"] = serde_json::json!(legacy_config.enabled);
            json["active_cloud_stt_provider"] = serde_json::json!(provider_type);
            json["cloud_stt_configs"] =
                serde_json::to_value(&configs).unwrap_or(serde_json::json!({}));
            json.as_object_mut().map(|obj| obj.remove("cloud_stt"));

            tracing::info!(
                enabled = legacy_config.enabled,
                provider = %provider_type,
                "cloud_stt_migrated-per_provider_format"
            );
            migrated = true;
        }
    }

    if let Some(old_polish) = json.get("cloud_polish").cloned() {
        if let Ok(legacy_config) = serde_json::from_value::<LegacyCloudProviderConfig>(old_polish) {
            let provider_type = if legacy_config.provider_type.is_empty() {
                "anthropic".to_string()
            } else {
                legacy_config.provider_type.clone()
            };

            let new_config = CloudProviderConfig {
                enabled: legacy_config.enabled,
                provider_type: provider_type.clone(),
                api_key: legacy_config.api_key,
                base_url: legacy_config.base_url,
                model: legacy_config.model,
                enable_thinking: legacy_config.enable_thinking,
            };

            let mut configs = HashMap::new();
            configs.insert(provider_type.clone(), new_config);

            json["cloud_polish_enabled"] = serde_json::json!(legacy_config.enabled);
            json["active_cloud_polish_provider"] = serde_json::json!(provider_type);
            json["cloud_polish_configs"] =
                serde_json::to_value(&configs).unwrap_or(serde_json::json!({}));
            json.as_object_mut().map(|obj| obj.remove("cloud_polish"));

            tracing::info!(
                enabled = legacy_config.enabled,
                provider = %provider_type,
                "cloud_polish_migrated-per_provider_format"
            );
            migrated = true;
        }
    }

    migrated
}

fn resolve_stt_provider_type(
    legacy_config: &LegacyCloudSttConfig,
    _old_stt: &serde_json::Value,
) -> String {
    if legacy_config.provider_type == "volcengine" || legacy_config.provider_type.is_empty() {
        "volcengine-streaming".to_string()
    } else {
        legacy_config.provider_type.clone()
    }
}

pub fn load_settings_from_disk() -> AppSettings {
    let path = get_settings_path();
    if path.exists() {
        if let Ok(json) = fs::read_to_string(&path) {
            // Try to parse and migrate if needed
            let mut json_value: serde_json::Value = match serde_json::from_str(&json) {
                Ok(v) => v,
                Err(_) => {
                    // Fall back to direct parsing if JSON is invalid
                    match serde_json::from_str::<AppSettings>(&json) {
                        Ok(settings) => {
                            tracing::info!(path = %path.display(), "settings_loaded");
                            return settings;
                        }
                        Err(e) => {
                            tracing::warn!(error = %e, "settings_parse_failed");
                            return AppSettings::default();
                        }
                    }
                }
            };

            // Run migration
            let migrated = migrate_cloud_settings(&mut json_value);

            // Parse into AppSettings
            match serde_json::from_value::<AppSettings>(json_value.clone()) {
                Ok(settings) => {
                    tracing::info!(path = %path.display(), migrated = migrated, "settings_loaded-migrated");

                    // Save migrated settings back to disk
                    if migrated {
                        if let Ok(pretty_json) = serde_json::to_string_pretty(&settings) {
                            let _ = fs::write(&path, pretty_json);
                        }
                    }

                    return settings;
                }
                Err(e) => {
                    tracing::warn!(error = %e, "settings_parse_failed")
                }
            }
        }
    } else {
        tracing::info!(path = %path.display(), "settings_not_found");
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
            "recording_mode" => {
                if let Some(v) = value.as_str() {
                    settings.recording_mode = v.to_string();
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
                    if settings.beep_on_record != v {
                        if v {
                            crate::audio::beep::enable_beep();
                        } else {
                            crate::audio::beep::disable_beep();
                        }
                    }
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
            "vad_enabled" => {
                if let Some(v) = value.as_bool() {
                    settings.vad_enabled = v;
                }
            }
            "stay_in_tray" => {
                if let Some(v) = value.as_bool() {
                    settings.stay_in_tray = v;
                    #[cfg(target_os = "macos")]
                    {
                        if let Err(e) =
                            crate::commands::settings::set_activation_policy_for_app(&app, v)
                        {
                            tracing::error!(error = %e, "activation_policy_set_failed");
                        }
                        if v {
                            if let Err(e) = crate::tray::show_tray(&app) {
                                tracing::error!(error = %e, "tray_show_failed");
                            }
                        } else {
                            crate::tray::remove_tray(&app);
                        }
                    }
                }
            }
            "cloud_stt_enabled" => {
                if let Some(v) = value.as_bool() {
                    settings.cloud_stt_enabled = v;
                }
            }
            "active_cloud_stt_provider" => {
                if let Some(v) = value.as_str() {
                    settings.active_cloud_stt_provider = v.to_string();
                }
            }
            "cloud_stt_configs" => {
                match serde_json::from_value::<HashMap<String, CloudSttConfig>>(value.clone()) {
                    Ok(v) => {
                        settings.cloud_stt_configs = v;
                    }
                    Err(e) => {
                        tracing::error!(error = %e, value = ?value, "cloud_stt_configs_parse_failed");
                    }
                }
            }
            "cloud_polish_enabled" => {
                if let Some(v) = value.as_bool() {
                    settings.cloud_polish_enabled = v;
                }
            }
            "active_cloud_polish_provider" => {
                if let Some(v) = value.as_str() {
                    settings.active_cloud_polish_provider = v.to_string();
                }
            }
            "cloud_polish_configs" => {
                match serde_json::from_value::<HashMap<String, CloudProviderConfig>>(value.clone())
                {
                    Ok(v) => {
                        settings.cloud_polish_configs = v;
                    }
                    Err(e) => {
                        tracing::error!(error = %e, value = ?value, "cloud_polish_configs_parse_failed");
                    }
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

        info!(key = %key, "settings_updated");
        let _ = app.emit(EventName::SETTINGS_CHANGED, settings.clone());
    } // lock released here

    if let Some(preset) = preset_to_apply {
        position_pill_window(&app, &preset);
    }

    if let Some(hotkey) = hotkey_to_register {
        let _ = app.global_shortcut().unregister_all();
        match register_global_shortcut(&app, &hotkey) {
            Ok(_) => info!(hotkey = %hotkey, "shortcut_registered"),
            Err(e) => tracing::error!(error = %e, "shortcut_registration_failed"),
        }
    }

    if indicator_mode_to_apply.is_some() {
        crate::commands::window::update_pill_visibility(&app);
    }

    if should_unload_model {
        // Model unloading is now handled by UnifiedEngineManager's engine cache
        tracing::info!(
            mem_mb = get_process_rss_mb(),
            "model_unload_requested-resident_disabled"
        );
    }

    if let Some(model_name) = should_load_model {
        // Model preloading is now handled by UnifiedEngineManager's engine cache
        tracing::info!(model = %model_name, mem_mb = get_process_rss_mb(), "model_preload_requested-resident_enabled");
    }

    if let Some(model_name) = should_reload_for_gpu {
        // Model reloading is now handled by UnifiedEngineManager's engine cache
        tracing::info!(model = %model_name, mem_mb = get_process_rss_mb(), "model_reload_requested-gpu_changed");
    }

    if let Some(model_name) = model_to_reload {
        // Model hot-reloading is now handled by UnifiedEngineManager's engine cache
        tracing::info!(model = %model_name, mem_mb = get_process_rss_mb(), "model_reload_requested-hot_swap");
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
        info!("shortcuts_unregistered-capture_mode");
    } else {
        let hotkey = state.settings.lock().hotkey.clone();
        match register_global_shortcut(&app, &hotkey) {
            Ok(_) => info!(hotkey = %hotkey, "shortcut_registered"),
            Err(e) => {
                tracing::error!(error = %e, "shortcut_registration_failed");
            }
        }
    }

    Ok(())
}

pub fn parse_hotkey(hotkey: &str) -> Result<(Option<Modifiers>, Code), String> {
    if hotkey.is_empty() {
        return Err("Invalid hotkey '': must have 1-5 keys".to_string());
    }

    let parts: Vec<&str> = hotkey.split('+').map(str::trim).collect();

    if parts.iter().any(|part| part.is_empty()) {
        return Err(format!("Invalid hotkey '{}': invalid format", hotkey));
    }

    if parts.len() > 5 {
        return Err(format!("Invalid hotkey '{}': must have 1-5 keys", hotkey));
    }

    let mut modifier_set = Modifiers::empty();
    let mut seen_modifiers: std::collections::HashSet<String> = std::collections::HashSet::new();

    for part in &parts[..parts.len() - 1] {
        let normalized = part.to_lowercase();
        if !seen_modifiers.insert(normalized.clone()) {
            return Err(format!(
                "Invalid hotkey '{}': duplicate modifier '{}'",
                hotkey, part
            ));
        }
        match normalized.as_str() {
            "ctrl" => modifier_set |= Modifiers::CONTROL,
            "shift" => modifier_set |= Modifiers::SHIFT,
            "alt" => modifier_set |= Modifiers::ALT,
            "cmd" | "command" | "meta" => modifier_set |= Modifiers::SUPER,
            _ => {
                return Err(format!(
                    "Invalid hotkey '{}': unknown modifier '{}'",
                    hotkey, part
                ))
            }
        }
    }

    let modifiers = if modifier_set.is_empty() {
        None
    } else {
        Some(modifier_set)
    };

    let key = parts
        .last()
        .ok_or_else(|| format!("Invalid hotkey '{}': missing key", hotkey))?;

    let key_lower = key.to_lowercase();

    let modifier_keys = [
        "cmd",
        "command",
        "meta",
        "ctrl",
        "control",
        "shift",
        "alt",
        "cmdleft",
        "cmdright",
        "metaleft",
        "metaright",
        "ctrlleft",
        "ctrlright",
        "controlleft",
        "controlright",
        "shiftleft",
        "shiftright",
        "altleft",
        "altright",
    ];
    if modifier_keys.contains(&key_lower.as_str()) {
        return Err(format!(
            "Invalid hotkey '{}': Global shortcuts require a key (Space, A-Z, F1-F12). '{}' alone is not supported by the system.",
            hotkey, key
        ));
    }

    let code_name = match key_lower.as_str() {
        "space" => "Space".to_string(),
        "enter" => "Enter".to_string(),
        "backspace" => "Backspace".to_string(),
        "tab" => "Tab".to_string(),
        "escape" => "Escape".to_string(),
        "arrowup" => "ArrowUp".to_string(),
        "arrowdown" => "ArrowDown".to_string(),
        "arrowleft" => "ArrowLeft".to_string(),
        "arrowright" => "ArrowRight".to_string(),
        "delete" => "Delete".to_string(),
        "home" => "Home".to_string(),
        "end" => "End".to_string(),
        "pageup" => "PageUp".to_string(),
        "pagedown" => "PageDown".to_string(),
        "insert" => "Insert".to_string(),
        "capslock" => "CapsLock".to_string(),
        "printscreen" => "PrintScreen".to_string(),
        "scrolllock" => "ScrollLock".to_string(),
        "pause" => "Pause".to_string(),
        "minus" => "Minus".to_string(),
        "equal" => "Equal".to_string(),
        "bracketleft" => "BracketLeft".to_string(),
        "bracketright" => "BracketRight".to_string(),
        "backslash" => "Backslash".to_string(),
        "semicolon" => "Semicolon".to_string(),
        "quote" => "Quote".to_string(),
        "backquote" => "Backquote".to_string(),
        "comma" => "Comma".to_string(),
        "period" => "Period".to_string(),
        "slash" => "Slash".to_string(),
        "numlock" => "NumLock".to_string(),
        "numpadadd" => "NumpadAdd".to_string(),
        "numpaddecimal" => "NumpadDecimal".to_string(),
        "numpaddivide" => "NumpadDivide".to_string(),
        "numpadenter" => "NumpadEnter".to_string(),
        "numpadequal" => "NumpadEqual".to_string(),
        "numpadmultiply" => "NumpadMultiply".to_string(),
        "numpadsubtract" => "NumpadSubtract".to_string(),
        "audiovolumedown" => "AudioVolumeDown".to_string(),
        "audiovolumeup" => "AudioVolumeUp".to_string(),
        "audiovolumemute" => "AudioVolumeMute".to_string(),
        "mediaplay" => "MediaPlay".to_string(),
        "mediapause" => "MediaPause".to_string(),
        "mediaplaypause" => "MediaPlayPause".to_string(),
        "mediastop" => "MediaStop".to_string(),
        "mediatracknext" => "MediaTrackNext".to_string(),
        "mediatrackprev" => "MediaTrackPrev".to_string(),
        _ if key.len() == 1 && key.chars().next().unwrap().is_ascii_alphabetic() => {
            format!("Key{}", key.to_uppercase())
        }
        _ if key.len() == 1 && key.chars().next().unwrap().is_ascii_digit() => {
            format!("Digit{}", key)
        }
        _ if key_lower.starts_with("f") && key_lower[1..].chars().all(|ch| ch.is_ascii_digit()) => {
            key_lower.to_uppercase()
        }
        _ if key_lower.starts_with("numpad")
            && key_lower[6..].chars().all(|ch| ch.is_ascii_digit()) =>
        {
            format!("Numpad{}", &key_lower[6..])
        }
        _ => {
            return Err(format!(
                "Invalid hotkey '{}': unknown key '{}'",
                hotkey, key
            ))
        }
    };

    let code = Code::from_str(&code_name)
        .map_err(|_| format!("Invalid hotkey '{}': unknown key '{}'", hotkey, key))?;
    Ok((modifiers, code))
}

pub fn register_global_shortcut(app: &AppHandle, hotkey: &str) -> Result<(), String> {
    let (modifiers, code) = parse_hotkey(hotkey)?;

    let shortcut = Shortcut::new(modifiers, code);
    app.global_shortcut()
        .on_shortcut(shortcut, |app, _shortcut, event| {
            tracing::debug!("shortcut-triggered");

            let state_result = app.try_state::<AppState>();
            match state_result {
                Some(state) => {
                    if state.hotkey_capture_mode.load(Ordering::SeqCst) {
                        tracing::debug!("shortcut_ignored-capture_mode");
                        return;
                    }

                    let is_recording = state.is_recording.load(Ordering::SeqCst);
                    let recording_mode = state.settings.lock().recording_mode.clone();

                    match recording_mode.as_str() {
                        "hold" => {
                            // Hold mode: Press to start, Release to stop
                            if event.state == ShortcutState::Pressed && !is_recording {
                                match crate::commands::audio::start_recording_sync(app.clone()) {
                                    Ok(_) => {}
                                    Err(e) => {
                                        tracing::error!(error = %e, "recording_start_failed")
                                    }
                                }
                            } else if event.state == ShortcutState::Released && is_recording {
                                match crate::commands::audio::stop_recording_sync(app.clone()) {
                                    Ok(_) => {}
                                    Err(e) => {
                                        tracing::error!(error = %e, "recording_stop_failed")
                                    }
                                }
                            }
                        }
                        _ => {
                            // Toggle mode (default): Press to toggle
                            if event.state == ShortcutState::Pressed {
                                if is_recording {
                                    match crate::commands::audio::stop_recording_sync(app.clone()) {
                                        Ok(_) => {}
                                        Err(e) => {
                                            tracing::error!(error = %e, "recording_stop_failed")
                                        }
                                    }
                                } else {
                                    match crate::commands::audio::start_recording_sync(app.clone())
                                    {
                                        Ok(_) => {}
                                        Err(e) => {
                                            tracing::error!(error = %e, "recording_start_failed")
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                None => {
                    tracing::error!("app_state_unavailable");
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

#[cfg(target_os = "macos")]
pub fn set_activation_policy_for_app(app: &AppHandle, stay_in_tray: bool) -> Result<(), String> {
    // Save the main window's visibility state before changing policy
    let main_window_was_visible = app
        .get_webview_window("main")
        .map(|w| w.is_visible().unwrap_or(false))
        .unwrap_or(false);

    let policy = if stay_in_tray {
        tauri::ActivationPolicy::Accessory
    } else {
        tauri::ActivationPolicy::Regular
    };
    app.set_activation_policy(policy)
        .map_err(|e| format!("Failed to set activation policy: {}", e))?;

    // When switching to Accessory mode, macOS hides the app's windows.
    // Restore the main window's visibility if it was visible before.
    if stay_in_tray && main_window_was_visible {
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.show();
            let _ = window.set_focus();
        }
    }

    info!(stay_in_tray = stay_in_tray, "activation_policy_updated");
    Ok(())
}

#[cfg(test)]
mod __test__;
