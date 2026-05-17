use super::{
    classify_cloud_check_error, migrate_to_profiles_map_for_test, normalize_pill_background_color,
    normalize_pill_background_opacity, validate_cloud_polish_config_for_check,
    validate_cloud_stt_config_for_check, AppSettings, CloudProviderConfig, CloudSttConfig,
};
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
fn missing_pill_background_color_uses_default() {
    let settings: AppSettings = serde_json::from_value(json!({})).unwrap();

    assert_eq!(settings.pill_background_color, "#1d1d1d");
    assert_eq!(settings.pill_background_opacity, 1.0);
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
