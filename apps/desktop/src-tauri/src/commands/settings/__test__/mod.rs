use super::{
    classify_cloud_check_error, migrate_platform_shortcut_defaults_for_test,
    migrate_to_profiles_map_for_test, normalize_pill_background_color,
    normalize_pill_background_opacity, polish_runtime_action_for_setting_update,
    selected_local_runtime_check_mode, validate_cloud_polish_config_for_check,
    validate_cloud_stt_config_for_check, AppSettings, CloudProviderConfig, CloudSttConfig,
    LocalPolishRuntimeSettingAction, LocalRuntimeCheckMode,
};
use crate::polish_engine::PolishEngineType;
use serde_json::json;

#[test]
fn test_is_streaming_stt_active_accepts_aliyun_stream_provider_id() {
    let mut settings = AppSettings::default();
    settings.cloud_stt_enabled = true;
    settings.active_cloud_stt_provider = "aliyun-stream".to_string();

    assert!(settings.is_streaming_stt_active());
}

#[test]
fn cloud_stt_check_validation_requires_schema_fields() {
    let mut config = CloudSttConfig {
        provider_type: "volcengine-streaming".to_string(),
        api_key: "token".to_string(),
        app_id: String::new(),
        base_url: String::new(),
        model: String::new(),
        language: String::new(),
        enabled: true,
    };

    let err = validate_cloud_stt_config_for_check(&config).unwrap_err();
    assert_eq!(err.kind, "missing_required");
    assert!(err.message.contains("App ID"));

    config.app_id = "app-id".to_string();
    assert!(validate_cloud_stt_config_for_check(&config).is_ok());
}

#[test]
fn cloud_polish_check_validation_requires_model() {
    let config = CloudProviderConfig {
        provider_type: "openai".to_string(),
        api_key: "sk-test".to_string(),
        base_url: String::new(),
        model: String::new(),
        enable_thinking: false,
        enabled: true,
    };

    let err = validate_cloud_polish_config_for_check(&config).unwrap_err();
    assert_eq!(err.kind, "missing_required");
    assert!(err.message.contains("Model"));
}

#[test]
fn cloud_check_validation_rejects_invalid_base_url() {
    let config = CloudProviderConfig {
        provider_type: "anthropic".to_string(),
        api_key: "sk-test".to_string(),
        base_url: "not a url".to_string(),
        model: "claude-sonnet-4-20250514".to_string(),
        enable_thinking: false,
        enabled: true,
    };

    let err = validate_cloud_polish_config_for_check(&config).unwrap_err();
    assert_eq!(err.kind, "invalid_url");
}

#[test]
fn cloud_check_error_classifier_maps_auth_and_timeout() {
    assert_eq!(
        classify_cloud_check_error("API error (401 Unauthorized): invalid_api_key"),
        "auth_failed"
    );
    assert_eq!(
        classify_cloud_check_error("connection check timed out after 10s"),
        "timeout"
    );
    assert_eq!(
        classify_cloud_check_error("API error (404): model not found"),
        "model_failed"
    );
}

#[test]
fn enabling_cloud_polish_stops_managed_local_runtime() {
    assert_eq!(
        polish_runtime_action_for_setting_update("cloud_polish_enabled", &json!(true)),
        LocalPolishRuntimeSettingAction::StopManagedRuntime
    );
}

#[test]
fn disabling_cloud_polish_keeps_local_runtime_available() {
    assert_eq!(
        polish_runtime_action_for_setting_update("cloud_polish_enabled", &json!(false)),
        LocalPolishRuntimeSettingAction::None
    );
}

#[test]
fn migrate_from_legacy_hotkey_copies_global_recording_mode_into_profiles() {
    let mut json = json!({
        "hotkey": "Shift+Space",
        "recording_mode": "toggle",
    });

    migrate_to_profiles_map_for_test(&mut json);

    assert_eq!(
        json["shortcut_profiles"]["dictate"]["trigger_mode"],
        "toggle"
    );
    assert_eq!(json["shortcut_profiles"]["riff"]["trigger_mode"], "toggle");
}

#[test]
fn migrate_array_profiles_copies_global_recording_mode_into_existing_profiles() {
    let mut json = json!({
        "recording_mode": "hold",
        "shortcut_profiles": [
            {
                "hotkey": "Cmd+Slash",
                "action": { "Record": { "polish_template_id": null } }
            },
            {
                "hotkey": "Opt+Slash",
                "action": { "Record": { "polish_template_id": "filler" } }
            },
            {
                "hotkey": "Cmd+Alt+Slash",
                "action": { "Record": { "polish_template_id": "formal" } }
            }
        ]
    });

    migrate_to_profiles_map_for_test(&mut json);

    assert_eq!(json["shortcut_profiles"]["dictate"]["trigger_mode"], "hold");
    assert_eq!(json["shortcut_profiles"]["riff"]["trigger_mode"], "hold");
    assert_eq!(json["shortcut_profiles"]["custom"]["trigger_mode"], "hold");
}

#[test]
fn migrate_platform_shortcut_defaults_rewrites_untouched_mac_defaults_on_windows() {
    let mut json = json!({
        "shortcut_profiles": {
            "dictate": {
                "hotkey": "Cmd+Slash",
                "trigger_mode": "hold",
                "action": { "Record": { "polish_template_id": null } }
            },
            "riff": {
                "hotkey": "Opt+Slash",
                "trigger_mode": "toggle",
                "action": { "Record": { "polish_template_id": "filler" } }
            },
            "custom": {
                "hotkey": "Cmd+Alt+Slash",
                "trigger_mode": "toggle",
                "action": { "Record": { "polish_template_id": null } }
            }
        }
    });

    let migrated = migrate_platform_shortcut_defaults_for_test(&mut json, false);

    assert!(migrated);
    assert_eq!(json["shortcut_profiles"]["dictate"]["hotkey"], "Ctrl+Slash");
    assert_eq!(json["shortcut_profiles"]["riff"]["hotkey"], "Alt+Slash");
    assert_eq!(
        json["shortcut_profiles"]["custom"]["hotkey"],
        "Cmd+Alt+Slash"
    );
}

#[test]
fn migrate_platform_shortcut_defaults_keeps_macos_and_customized_values() {
    let mut mac_json = json!({
        "shortcut_profiles": {
            "dictate": { "hotkey": "Cmd+Slash" },
            "riff": { "hotkey": "Opt+Slash" }
        }
    });
    let mut customized_json = json!({
        "shortcut_profiles": {
            "dictate": { "hotkey": "Shift+Space" },
            "riff": { "hotkey": "Ctrl+Space" }
        }
    });

    assert!(!migrate_platform_shortcut_defaults_for_test(
        &mut mac_json,
        true
    ));
    assert_eq!(
        mac_json["shortcut_profiles"]["dictate"]["hotkey"],
        "Cmd+Slash"
    );
    assert_eq!(mac_json["shortcut_profiles"]["riff"]["hotkey"], "Opt+Slash");

    assert!(!migrate_platform_shortcut_defaults_for_test(
        &mut customized_json,
        false
    ));
    assert_eq!(
        customized_json["shortcut_profiles"]["dictate"]["hotkey"],
        "Shift+Space"
    );
    assert_eq!(
        customized_json["shortcut_profiles"]["riff"]["hotkey"],
        "Ctrl+Space"
    );
}

#[test]
fn missing_pill_background_color_uses_default() {
    let settings: AppSettings = serde_json::from_value(json!({})).unwrap();

    assert_eq!(settings.pill_background_color, "#1d1d1d");
    assert_eq!(settings.pill_background_opacity, 1.0);
}

#[test]
fn correction_memory_defaults_enabled() {
    let settings: AppSettings = serde_json::from_value(json!({})).unwrap();

    assert!(settings.correction_memory_enabled);
}

#[test]
fn stay_in_tray_defaults_enabled() {
    let settings: AppSettings = serde_json::from_value(json!({})).unwrap();

    assert!(settings.stay_in_tray);
}

#[test]
fn local_polish_runtime_uses_default_when_missing() {
    let settings: AppSettings = serde_json::from_value(json!({})).unwrap();

    assert_eq!(settings.local_polish_runtime.provider_type, "llama-server");
    assert_eq!(
        settings.local_polish_runtime.base_url,
        "http://127.0.0.1:8000/v1"
    );
    assert_eq!(settings.local_polish_runtime.ready_timeout_secs, 20);
}

#[test]
fn local_runtime_check_uses_health_only_without_selected_model() {
    let mode = selected_local_runtime_check_mode("", None, |_, _| true);

    assert_eq!(mode, LocalRuntimeCheckMode::HealthOnly);
}

#[test]
fn local_runtime_check_uses_health_only_when_selected_model_is_not_downloaded() {
    let mode =
        selected_local_runtime_check_mode("qwen3.5-0.8b", Some(PolishEngineType::Qwen), |_, _| {
            false
        });

    assert_eq!(mode, LocalRuntimeCheckMode::HealthOnly);
}

#[test]
fn local_runtime_check_prepares_selected_downloaded_model() {
    let mode = selected_local_runtime_check_mode(
        " qwen3.5-0.8b ",
        Some(PolishEngineType::Qwen),
        |engine_type, model_id| {
            assert_eq!(engine_type, PolishEngineType::Qwen);
            assert_eq!(model_id, "qwen3.5-0.8b");
            true
        },
    );

    assert_eq!(
        mode,
        LocalRuntimeCheckMode::SelectedModelReady {
            engine_type: PolishEngineType::Qwen,
            model_id: "qwen3.5-0.8b".to_string(),
        }
    );
}

#[test]
fn direct_stream_typing_defaults_disabled() {
    let settings: AppSettings = serde_json::from_value(json!({})).unwrap();

    assert!(!settings.polish_stream_direct_typing_enabled);
}

#[test]
fn normalize_pill_background_color_accepts_only_hex_rgb_values() {
    assert_eq!(
        normalize_pill_background_color(" #AABBCC "),
        Some("#aabbcc".to_string())
    );
    assert_eq!(normalize_pill_background_color("#abc"), None);
    assert_eq!(normalize_pill_background_color("red"), None);
    assert_eq!(normalize_pill_background_color("#zzzzzz"), None);
}

#[test]
fn normalize_pill_background_opacity_clamps_to_visible_range() {
    assert_eq!(normalize_pill_background_opacity(0.65), Some(0.65));
    assert_eq!(normalize_pill_background_opacity(0.0), Some(0.2));
    assert_eq!(normalize_pill_background_opacity(1.5), Some(1.0));
    assert_eq!(normalize_pill_background_opacity(f64::NAN), None);
}
