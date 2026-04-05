use ariatype_lib::commands::settings::CloudSttConfig;

#[test]
fn test_migration_from_volcengine_streaming() {
    let old_settings = serde_json::json!({
        "cloud_stt": {
            "enabled": true,
            "provider_type": "volcengine",
            "volcengine_mode": "streaming",
            "api_key": "test_key",
            "app_id": "test_app",
            "base_url": "",
            "model": ""
        }
    });

    let mut json_value = old_settings.clone();
    let migrated = migrate_cloud_stt_config(&mut json_value);

    assert!(migrated, "Migration should return true");

    let new_config: CloudSttConfig =
        serde_json::from_value(json_value["cloud_stt"].clone()).unwrap();
    assert_eq!(new_config.provider_type, "volcengine-streaming");
}

#[test]
fn test_migration_from_volcengine_flash() {
    let old_settings = serde_json::json!({
        "cloud_stt": {
            "enabled": true,
            "provider_type": "volcengine",
            "volcengine_mode": "flash",
            "api_key": "test_key",
            "app_id": "test_app",
            "base_url": "",
            "model": ""
        }
    });

    let mut json_value = old_settings.clone();
    let migrated = migrate_cloud_stt_config(&mut json_value);

    assert!(migrated, "Migration should return true");

    let new_config: CloudSttConfig =
        serde_json::from_value(json_value["cloud_stt"].clone()).unwrap();
    assert_eq!(new_config.provider_type, "volcengine-flash");
}

#[test]
fn test_no_migration_for_openai() {
    let settings = serde_json::json!({
        "cloud_stt": {
            "enabled": true,
            "provider_type": "openai",
            "api_key": "test_key",
            "app_id": "",
            "base_url": "",
            "model": "",
            "language": ""
        }
    });

    let mut json_value = settings.clone();
    let migrated = migrate_cloud_stt_config(&mut json_value);

    assert!(!migrated, "OpenAI config should not need migration");

    let parsed: CloudSttConfig = serde_json::from_value(json_value["cloud_stt"].clone()).unwrap();
    assert_eq!(parsed.provider_type, "openai");
}

#[test]
fn test_no_migration_for_new_format() {
    let settings = serde_json::json!({
        "cloud_stt": {
            "enabled": true,
            "provider_type": "volcengine-streaming",
            "api_key": "test_key",
            "app_id": "test_app",
            "base_url": "",
            "model": "",
            "language": ""
        }
    });

    let mut json_value = settings.clone();
    let migrated = migrate_cloud_stt_config(&mut json_value);

    assert!(!migrated, "New format should not need migration");

    let parsed: CloudSttConfig = serde_json::from_value(json_value["cloud_stt"].clone()).unwrap();
    assert_eq!(parsed.provider_type, "volcengine-streaming");
}

#[test]
fn test_migration_preserves_other_fields() {
    let old_settings = serde_json::json!({
        "cloud_stt": {
            "enabled": true,
            "provider_type": "volcengine",
            "volcengine_mode": "streaming",
            "api_key": "secret_key_123",
            "app_id": "app_456",
            "base_url": "https://custom.url",
            "model": "custom_model"
        }
    });

    let mut json_value = old_settings.clone();
    migrate_cloud_stt_config(&mut json_value);

    let new_config: CloudSttConfig =
        serde_json::from_value(json_value["cloud_stt"].clone()).unwrap();

    assert_eq!(new_config.enabled, true);
    assert_eq!(new_config.api_key, "secret_key_123");
    assert_eq!(new_config.app_id, "app_456");
    assert_eq!(new_config.base_url, "https://custom.url");
    assert_eq!(new_config.model, "custom_model");
}

#[test]
fn test_migration_missing_cloud_stt() {
    let settings = serde_json::json!({
        "enabled": true
    });

    let mut json_value = settings.clone();
    let migrated = migrate_cloud_stt_config(&mut json_value);

    assert!(!migrated, "Missing cloud_stt should not migrate");
}

#[test]
fn test_provider_type_validation() {
    let valid_providers = vec!["volcengine-streaming", "qwen-omni-realtime", "elevenlabs"];

    for provider in valid_providers {
        let config = CloudSttConfig {
            enabled: true,
            provider_type: provider.to_string(),
            api_key: "test".to_string(),
            app_id: String::new(),
            base_url: String::new(),
            model: String::new(),
            language: String::new(),
        };

        let json = serde_json::to_string(&config).unwrap();
        let parsed: CloudSttConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.provider_type, provider);
    }
}

fn migrate_cloud_stt_config(json: &mut serde_json::Value) -> bool {
    let cloud_stt = match json.get_mut("cloud_stt") {
        Some(v) => v,
        None => return false,
    };

    let provider_type = cloud_stt
        .get("provider_type")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if provider_type == "volcengine" {
        let volcengine_mode = cloud_stt
            .get("volcengine_mode")
            .and_then(|v| v.as_str())
            .unwrap_or("streaming")
            .to_string();

        let new_provider_type = match volcengine_mode.as_str() {
            "flash" => "volcengine-flash",
            _ => "volcengine-streaming",
        };

        cloud_stt["provider_type"] = serde_json::json!(new_provider_type);
        cloud_stt
            .as_object_mut()
            .map(|obj| obj.remove("volcengine_mode"));

        return true;
    }

    false
}
