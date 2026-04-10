#[cfg(test)]
mod tests {
    use ariatype_lib::commands::settings::CloudSttConfig;
    use ariatype_lib::stt_engine::traits::TranscriptionResult;

    #[test]
    #[ignore = "TODO: Rewrite to use RecordingConsumer streaming API (send_chunk + finish)"]
    fn test_volcengine_stt_basic() {
        return;
    }

    #[test]
    #[ignore = "TODO: Rewrite to use RecordingConsumer streaming API (send_chunk + finish)"]
    fn test_volcengine_stt_chinese() {
        return;
    }

    #[test]
    #[ignore = "TODO: Rewrite to use RecordingConsumer streaming API (send_chunk + finish)"]
    fn test_volcengine_stt_empty_audio() {
        return;
    }

    #[test]
    #[ignore = "TODO: Rewrite to use RecordingConsumer streaming API (send_chunk + finish)"]
    fn test_volcengine_stt_error_handling() {
        return;
    }

    #[test]
    #[ignore = "TODO: Rewrite to use RecordingConsumer streaming API (send_chunk + finish)"]
    fn test_openai_stt_basic() {
        return;
    }

    #[test]
    fn test_cloud_stt_config_validation() {
        let config = CloudSttConfig {
            enabled: true,
            provider_type: "volcengine-streaming".to_string(),
            api_key: "test-key-123".to_string(),
            app_id: "test-app-456".to_string(),
            base_url: "https://api.custom.com".to_string(),
            model: "custom-model".to_string(),
            language: "en-US".to_string(),
        };

        let json = serde_json::to_value(&config).expect("Failed to serialize config");
        let deserialized: CloudSttConfig =
            serde_json::from_value(json).expect("Failed to deserialize config");

        assert_eq!(deserialized.enabled, config.enabled);
        assert_eq!(deserialized.provider_type, config.provider_type);
        assert_eq!(deserialized.api_key, config.api_key);
        assert_eq!(deserialized.app_id, config.app_id);
        assert_eq!(deserialized.base_url, config.base_url);
        assert_eq!(deserialized.model, config.model);
        assert_eq!(deserialized.language, config.language);
    }

    #[test]
    fn test_transcription_result_metrics() {
        let result = TranscriptionResult::with_metrics(
            "Hello world".to_string(),
            ariatype_lib::stt_engine::traits::EngineType::Cloud,
            1500,
            Some(200),
            Some(100),
            Some(1200),
        );

        assert_eq!(result.text, "Hello world");
        assert_eq!(
            result.engine,
            ariatype_lib::stt_engine::traits::EngineType::Cloud
        );
        assert_eq!(result.total_ms, 1500);
        assert_eq!(result.model_load_ms, Some(200));
        assert_eq!(result.preprocess_ms, Some(100));
        assert_eq!(result.inference_ms, Some(1200));
    }
}
