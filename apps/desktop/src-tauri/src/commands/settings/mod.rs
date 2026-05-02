use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager, State};
use tracing::{info, warn};

use crate::commands::window::position_pill_window;
use crate::events::EventName;
use crate::shortcut::ShortcutProfilesMap;
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
    /// Shortcut profiles map with fixed keys: { dictate, riff, custom? }
    pub shortcut_profiles: ShortcutProfilesMap,
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
    pub polish_system_prompt: String,
    pub polish_model: String,
    pub theme_mode: String,
    pub stt_engine_initial_prompt: String,
    pub model_resident: bool,
    pub idle_unload_minutes: u32,
    pub denoise_mode: String,
    pub stt_engine_work_domain: String,
    pub stt_engine_work_domain_prompt: String,
    pub stt_engine_work_subdomain: String,
    pub stt_engine_user_glossary: String,
    pub analytics_opt_in: bool,
    pub cloud_stt_enabled: bool,
    pub active_cloud_stt_provider: String,
    pub cloud_stt_configs: HashMap<String, CloudSttConfig>,
    pub cloud_polish_enabled: bool,
    pub active_cloud_polish_provider: String,
    pub cloud_polish_configs: HashMap<String, CloudProviderConfig>,
    pub vad_enabled: bool,
    pub stay_in_tray: bool,
    pub polish_custom_templates: Vec<CustomPolishTemplate>,
    /// Enable window context capture via screenshot + OCR at recording start.
    /// When enabled, the focused window content is injected into polish prompts.
    pub window_context_enabled: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            shortcut_profiles: ShortcutProfilesMap::default(),
            recording_mode: "hold".to_string(),
            model: "whisper-base".to_string(),
            stt_engine: "whisper".to_string(),
            pill_position: "bottom-center".to_string(),
            pill_indicator_mode: "when_recording".to_string(),
            auto_start: false,
            gpu_acceleration: true,
            language: "auto".to_string(),
            stt_engine_language: "auto".to_string(),
            beep_on_record: true,
            audio_device: "default".to_string(),
            polish_system_prompt: crate::polish_engine::DEFAULT_POLISH_PROMPT.to_string(),
            polish_model: String::new(),
            theme_mode: "light".to_string(),
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
            polish_custom_templates: Vec::new(),
            window_context_enabled: false,
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
        self.cloud_polish_configs
            .get(&self.active_cloud_polish_provider)
            .cloned()
            .unwrap_or_default()
    }

    /// Check if any streaming cloud STT provider is active
    pub fn is_streaming_stt_active(&self) -> bool {
        self.cloud_stt_enabled
            && matches!(
                self.active_cloud_stt_provider.as_str(),
                "volcengine-streaming" | "aliyun-stream" | "elevenlabs"
            )
    }

    #[deprecated(note = "Use is_streaming_stt_active instead")]
    pub fn is_volcengine_streaming_active(&self) -> bool {
        self.is_streaming_stt_active()
    }

    pub fn get_dictate_hotkey(&self) -> String {
        self.shortcut_profiles.dictate.hotkey.clone()
    }

    pub fn set_dictate_hotkey(&mut self, hotkey: &str) {
        self.shortcut_profiles.dictate.hotkey = hotkey.to_string();
    }

    pub fn get_riff_hotkey(&self) -> String {
        self.shortcut_profiles.riff.hotkey.clone()
    }

    pub fn set_riff_hotkey(&mut self, hotkey: &str) {
        self.shortcut_profiles.riff.hotkey = hotkey.to_string();
    }

    pub fn get_custom_hotkey(&self) -> Option<String> {
        self.shortcut_profiles
            .custom
            .as_ref()
            .map(|p| p.hotkey.clone())
    }

    /// Resolve polish provider config.
    ///
    /// Provider resolution order:
    /// 1. Check active_cloud_polish_provider in cloud_polish_configs
    /// 2. If valid (api_key + model non-empty) → use cloud
    /// 3. Otherwise → local fallback
    pub fn resolve_polish_config(
        &self,
        provider_override: Option<&str>,
        model_override: Option<&str>,
    ) -> (Option<String>, CloudProviderConfig) {
        match provider_override {
            Some(provider_key) => match self.cloud_polish_configs.get(provider_key) {
                Some(cfg) if !cfg.api_key.is_empty() && !cfg.model.is_empty() => {
                    let mut resolved = cfg.clone();
                    resolved.enabled = true;
                    resolved.provider_type = provider_key.to_string();
                    if let Some(m) = model_override.filter(|m| !m.is_empty()) {
                        resolved.model = m.to_string();
                    }
                    (Some(provider_key.to_string()), resolved)
                }
                _ => {
                    tracing::warn!(
                        provider = %provider_key,
                        "polish_override_provider_invalid_fallback_to_global"
                    );
                    self.resolve_global_polish_config()
                }
            },
            None => self.resolve_global_polish_config(),
        }
    }

    fn resolve_global_polish_config(&self) -> (Option<String>, CloudProviderConfig) {
        let provider_type = &self.active_cloud_polish_provider;

        if let Some(cfg) = self.cloud_polish_configs.get(provider_type) {
            if !cfg.api_key.is_empty() && !cfg.model.is_empty() {
                return (Some(provider_type.clone()), cfg.clone());
            }
        }

        (None, CloudProviderConfig::default())
    }
}

fn get_settings_path() -> PathBuf {
    AppPaths::data_dir().join("settings.json")
}

/// Save settings to disk without requiring a specific key update.
/// Used by hotkey recording to persist the new hotkey.
pub fn save_settings_internal(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let settings = state.settings.lock();

    let path = get_settings_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(&*settings).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())?;

    info!("settings_saved_to_disk");
    Ok(())
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

fn validate_model_name(json: &mut serde_json::Value) -> bool {
    let model_value = match json.get("model").and_then(|v| v.as_str()) {
        Some(m) => m,
        None => return false,
    };

    if crate::stt_engine::models::find_by_name(model_value).is_some() {
        return false;
    }

    tracing::info!(old = %model_value, new = "whisper-base", "model_name_reset_to_default");
    json["model"] = serde_json::Value::String("whisper-base".to_string());
    json["stt_engine"] = serde_json::Value::String("whisper".to_string());

    true
}

pub fn migrate_to_profiles_map_for_test(json: &mut serde_json::Value) {
    migrate_to_profiles_map(json);
}

fn migrate_to_profiles_map(json: &mut serde_json::Value) -> bool {
    let legacy_recording_mode = json
        .get("recording_mode")
        .and_then(|value| value.as_str())
        .map(str::to_string);

    if let Some(obj) = json.as_object_mut() {
        obj.remove("hotkey");
        obj.remove("polish_enabled");
        obj.remove("polish_selected_template");
    }

    // Check if shortcut_profiles is already a map
    if let Some(profiles) = json.get_mut("shortcut_profiles") {
        if profiles.is_object() {
            return ensure_profile_trigger_modes(profiles, legacy_recording_mode.as_deref());
        }

        // If array, convert to map
        if let Some(arr) = profiles.as_array() {
            let mut map = serde_json::Map::new();

            // First element → dictate
            if let Some(first) = arr.first() {
                let dictate = convert_array_item_to_profile(
                    first,
                    None,
                    profile_trigger_mode("dictate", legacy_recording_mode.as_deref()),
                );
                map.insert("dictate".to_string(), dictate);
            } else {
                map.insert(
                    "dictate".to_string(),
                    serde_json::json!({
                        "hotkey": "Shift+Space",
                        "trigger_mode": "hold",
                        "action": { "Record": { "polish_template_id": null } }
                    }),
                );
            }

            // Second element → riff
            if let Some(second) = arr.get(1) {
                let riff = convert_array_item_to_profile(
                    second,
                    Some("filler"),
                    profile_trigger_mode("riff", legacy_recording_mode.as_deref()),
                );
                map.insert("riff".to_string(), riff);
            } else {
                map.insert(
                    "riff".to_string(),
                    serde_json::json!({
                        "hotkey": "",
                        "trigger_mode": "toggle",
                        "action": { "Record": { "polish_template_id": "filler" } }
                    }),
                );
            }

            // Third element → custom (if exists)
            if let Some(third) = arr.get(2) {
                let custom = convert_array_item_to_profile(
                    third,
                    None,
                    profile_trigger_mode("custom", legacy_recording_mode.as_deref()),
                );
                map.insert("custom".to_string(), custom);
            }

            json["shortcut_profiles"] = serde_json::Value::Object(map);
            tracing::info!("shortcut_profiles_migrated-array_to_map");
            return true;
        }
    }

    // No shortcut_profiles, create from old hotkey
    let old_hotkey = json
        .get("hotkey")
        .and_then(|v| v.as_str())
        .unwrap_or("Shift+Space")
        .to_string();

    json["shortcut_profiles"] = serde_json::json!({
        "dictate": {
            "hotkey": old_hotkey,
            "trigger_mode": profile_trigger_mode("dictate", legacy_recording_mode.as_deref()),
            "action": { "Record": { "polish_template_id": null } }
        },
        "riff": {
            "hotkey": "",
            "trigger_mode": profile_trigger_mode("riff", legacy_recording_mode.as_deref()),
            "action": { "Record": { "polish_template_id": "filler" } }
        }
    });

    tracing::info!(hotkey = %old_hotkey, "shortcut_profiles_migrated-from_hotkey");
    true
}

fn convert_array_item_to_profile(
    item: &serde_json::Value,
    default_template: Option<&str>,
    trigger_mode: &str,
) -> serde_json::Value {
    let hotkey = item
        .get("hotkey")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let template_id = item
        .get("action")
        .and_then(|a| a.get("Record"))
        .and_then(|r| r.get("polish_template_id"))
        .and_then(|t| t.as_str())
        .map(|s| s.to_string())
        .or_else(|| default_template.map(|s| s.to_string()));

    serde_json::json!({
        "hotkey": hotkey,
        "trigger_mode": trigger_mode,
        "action": { "Record": { "polish_template_id": template_id } }
    })
}

fn ensure_profile_trigger_modes(
    profiles: &mut serde_json::Value,
    legacy_recording_mode: Option<&str>,
) -> bool {
    let Some(map) = profiles.as_object_mut() else {
        return false;
    };

    let mut migrated = false;
    for key in ["dictate", "riff", "custom"] {
        let Some(profile) = map.get_mut(key) else {
            continue;
        };
        let Some(profile_object) = profile.as_object_mut() else {
            continue;
        };
        if profile_object.contains_key("trigger_mode") {
            continue;
        }

        profile_object.insert(
            "trigger_mode".to_string(),
            serde_json::Value::String(profile_trigger_mode(key, legacy_recording_mode).to_string()),
        );
        migrated = true;
    }

    migrated
}

fn profile_trigger_mode(profile_key: &str, legacy_recording_mode: Option<&str>) -> &'static str {
    if let Some(recording_mode) = legacy_recording_mode {
        if recording_mode.eq_ignore_ascii_case("hold") {
            return "hold";
        }
        if recording_mode.eq_ignore_ascii_case("toggle") {
            return "toggle";
        }
    }

    match profile_key {
        "dictate" => "hold",
        "riff" | "custom" => "toggle",
        _ => "hold",
    }
}

pub fn load_settings_from_disk() -> AppSettings {
    let path = get_settings_path();
    if path.exists() {
        if let Ok(json) = fs::read_to_string(&path) {
            let mut json_value: serde_json::Value = match serde_json::from_str(&json) {
                Ok(v) => v,
                Err(_) => match serde_json::from_str::<AppSettings>(&json) {
                    Ok(settings) => {
                        tracing::info!(path = %path.display(), "settings_loaded");
                        return settings;
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "settings_parse_failed");
                        return AppSettings::default();
                    }
                },
            };

            let migrated_cloud = migrate_cloud_settings(&mut json_value);
            let migrated_model = validate_model_name(&mut json_value);
            let migrated_profiles = migrate_to_profiles_map(&mut json_value);
            let migrated = migrated_cloud || migrated_model || migrated_profiles;

            match serde_json::from_value::<AppSettings>(json_value.clone()) {
                Ok(settings) => {
                    tracing::info!(path = %path.display(), migrated = migrated, "settings_loaded-migrated");

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
    let mut should_clear_cache = false;
    let mut model_to_preload: Option<String> = None;
    let mut hotkey_to_register: Option<String> = None;
    let preset_to_apply: Option<String>;
    let indicator_mode_to_apply: Option<String>;

    {
        let mut settings = state.settings.lock();

        match key.as_str() {
            "hotkey" => {
                if let Some(v) = value.as_str() {
                    settings.set_dictate_hotkey(v);
                    hotkey_to_register = Some(v.to_string());
                }
            }
            "shortcut_profiles" => {
                if let Ok(profiles) = serde_json::from_value::<ShortcutProfilesMap>(value.clone()) {
                    settings.shortcut_profiles = profiles;
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
                        should_clear_cache = true;
                        model_to_preload = Some(v.to_string());
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
                    if v != settings.gpu_acceleration {
                        should_clear_cache = true;
                        state.engine_manager.set_provider(v);
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
                    if settings.stt_engine_language != v {
                        state.engine_manager.clear_cache();
                    }
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
                    if v != settings.model_resident {
                        should_clear_cache = true;
                        if v {
                            model_to_preload = Some(settings.model.clone());
                        }
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
            "window_context_enabled" => {
                if let Some(v) = value.as_bool() {
                    settings.window_context_enabled = v;
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
        if let Some(manager) = app.try_state::<crate::shortcut::ShortcutManager>() {
            tracing::info!("unregistering_old_hotkey");
            if let Err(e) = manager.unregister_profile("dictate") {
                tracing::warn!(error = %e, "old_hotkey_unregister_failed");
            }

            let profile = crate::shortcut::ShortcutProfile {
                hotkey: hotkey.clone(),
                trigger_mode: state.settings.lock().shortcut_profiles.dictate.trigger_mode,
                action: crate::shortcut::ShortcutAction::Record {
                    polish_template_id: None,
                },
            };
            match manager.register_profile("dictate", &profile) {
                Ok(_) => info!(hotkey = %hotkey, "shortcut_registered"),
                Err(e) => tracing::error!(error = %e, "shortcut_registration_failed"),
            }
        } else {
            tracing::error!("shortcut_manager_not_available");
        }
    }

    if indicator_mode_to_apply.is_some() {
        crate::commands::window::update_pill_visibility(&app);
    }

    if should_clear_cache {
        state.engine_manager.clear_cache();
    }

    if let Some(model_name) = model_to_preload {
        let engine_type =
            crate::stt_engine::UnifiedEngineManager::get_engine_by_model_name(&model_name)
                .unwrap_or(crate::stt_engine::traits::EngineType::Whisper);
        let engine_manager = state.engine_manager.clone();
        let app_clone = app.clone();
        tauri::async_runtime::spawn_blocking(move || {
            if let Err(e) = engine_manager.load_model(engine_type, &model_name) {
                tracing::warn!(model = %model_name, error = %e, "model_preload_failed");
            } else {
                tracing::info!(model = %model_name, mem_mb = get_process_rss_mb(), "model_preloaded");
                let _ = app_clone.emit(
                    EventName::MODEL_LOADED,
                    crate::events::ModelLoadedEvent { model: model_name },
                );
            }
        });
    }

    Ok(())
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

#[tauri::command]
pub fn get_cloud_provider_schemas() -> crate::provider_schema::CloudProviderSchemas {
    crate::provider_schema::get_schemas()
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

/// Scans the models directory for legacy model files (ggml/gguf format)
/// and deletes them. These are from the old whisper.cpp format that is no longer used.
/// Current models use sherpa-onnx ONNX format (.onnx, .int8.onnx).
///
/// Returns the number of legacy files deleted.
pub fn cleanup_legacy_models() -> Result<usize, String> {
    let models_dir = AppPaths::models_dir();

    if !models_dir.exists() {
        info!(path = ?models_dir, "cleanup_legacy_models_skip-no_models_dir");
        return Ok(0);
    }

    let mut deleted_count = 0;
    let legacy_extensions = [".ggml", ".gguf"];

    let entries = fs::read_dir(&models_dir).map_err(|e| {
        format!(
            "Failed to read models directory '{}': {}",
            models_dir.display(),
            e
        )
    })?;

    for entry in entries.flatten() {
        let path = entry.path();

        // Check if it's a file with a legacy extension
        if path.is_file() {
            if let Some(ext) = path.extension() {
                let ext_lower = ext.to_string_lossy().to_lowercase();
                if legacy_extensions
                    .iter()
                    .any(|&e| e == format!(".{}", ext_lower))
                {
                    match fs::remove_file(&path) {
                        Ok(_) => {
                            info!(file = %path.display(), "legacy_model_file_deleted");
                            deleted_count += 1;
                        }
                        Err(e) => {
                            warn!(file = %path.display(), error = %e, "legacy_model_file_deletion_failed");
                        }
                    }
                }
            }
        }

        // Also check for legacy model subdirectories (e.g., "model.ggml" folders)
        if path.is_dir() {
            let dir_name = path.file_name().unwrap_or_default().to_string_lossy();
            if dir_name.ends_with(".ggml") || dir_name.ends_with(".gguf") {
                match fs::remove_dir_all(&path) {
                    Ok(_) => {
                        info!(dir = %path.display(), "legacy_model_dir_deleted");
                        deleted_count += 1;
                    }
                    Err(e) => {
                        warn!(dir = %path.display(), error = %e, "legacy_model_dir_deletion_failed");
                    }
                }
            }
        }
    }

    info!(deleted = deleted_count, "cleanup_legacy_models_complete");
    Ok(deleted_count)
}

#[tauri::command]
pub async fn cleanup_legacy_models_cmd() -> Result<usize, String> {
    cleanup_legacy_models()
}

#[cfg(test)]
mod __test__;
