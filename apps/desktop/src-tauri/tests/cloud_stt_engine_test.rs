use std::io::Cursor;
use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests {
    use super::*;
    use ariatype_lib::commands::settings::CloudSttConfig;
    use ariatype_lib::stt_engine::cloud::CloudSttEngine;
    use ariatype_lib::stt_engine::traits::{SttEngine, TranscriptionRequest, TranscriptionResult};
    use std::fs;

    fn create_test_wav(sample_rate: u32, channels: u16, duration_secs: f32) -> Vec<u8> {
        let samples_per_channel = (sample_rate as f32 * duration_secs) as usize;
        let total_samples = samples_per_channel * channels as usize;

        let spec = hound::WavSpec {
            channels,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut cursor = Cursor::new(Vec::new());
        {
            let mut writer = hound::WavWriter::new(&mut cursor, spec).unwrap();

            for i in 0..total_samples {
                let t = i as f32 / sample_rate as f32;
                let freq = 440.0;
                let amplitude = 16000.0;
                let sample = (amplitude * (2.0 * std::f32::consts::PI * freq * t).sin()) as i16;
                writer.write_sample(sample).unwrap();
            }
            writer.finalize().unwrap();
        }

        cursor.into_inner()
    }

    #[cfg(not(target_os = "windows"))]
    fn get_default_settings_path() -> PathBuf {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("ariatype")
            .join("settings.json")
    }

    #[cfg(target_os = "windows")]
    fn get_default_settings_path() -> PathBuf {
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("ariatype")
            .join("settings.json")
    }

    fn load_settings_from_env_or_default() -> Option<ariatype_lib::commands::settings::AppSettings>
    {
        if let Ok(settings_path) = std::env::var("ARIATYPE_SETTINGS_PATH") {
            let path = Path::new(&settings_path);
            if path.exists() {
                if let Ok(content) = fs::read_to_string(path) {
                    if let Ok(settings) = serde_json::from_str::<
                        ariatype_lib::commands::settings::AppSettings,
                    >(&content)
                    {
                        return Some(settings);
                    }
                }
            }
        }

        let default_path = get_default_settings_path();
        if default_path.exists() {
            if let Ok(content) = fs::read_to_string(&default_path) {
                if let Ok(settings) =
                    serde_json::from_str::<ariatype_lib::commands::settings::AppSettings>(&content)
                {
                    return Some(settings);
                }
            }
        }

        None
    }

    fn skip_if_no_volcengine_config(settings: &ariatype_lib::commands::settings::AppSettings) {
        let cloud_stt = settings.get_active_cloud_stt_config();
        if !cloud_stt.enabled
            || !cloud_stt.provider_type.starts_with("volcengine")
            || cloud_stt.app_id.is_empty()
            || cloud_stt.api_key.is_empty()
        {
            eprintln!("Skipping Volcengine integration test: not properly configured");
            return;
        }
    }

    fn skip_if_no_openai_config(settings: &ariatype_lib::commands::settings::AppSettings) {
        let cloud_stt = settings.get_active_cloud_stt_config();
        if !cloud_stt.enabled
            || (cloud_stt.provider_type != "openai" && cloud_stt.provider_type != "custom")
            || cloud_stt.api_key.is_empty()
        {
            eprintln!("Skipping OpenAI integration test: not properly configured");
            return;
        }
    }

    #[test]
    #[ignore]
    fn test_volcengine_stt_basic() {
        let settings = match load_settings_from_env_or_default() {
            Some(s) => s,
            None => {
                eprintln!("Skipping integration test: settings.json not found");
                return;
            }
        };

        skip_if_no_volcengine_config(&settings);

        let wav_data = create_test_wav(16000, 1, 2.0);
        let temp_path = tempfile::NamedTempFile::new().expect("Failed to create temp file");
        fs::write(temp_path.path(), &wav_data).expect("Failed to write temp WAV");

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            let engine = CloudSttEngine::new().expect("Failed to create CloudSttEngine");
            let request = TranscriptionRequest {
                audio_path: temp_path.path().to_path_buf(),
                language: Some("en-US".to_string()),
                model_name: None,
                initial_prompt: None,
                denoise_mode: "off".to_string(),
                vad_enabled: false,
                cloud_config: Some(settings.get_active_cloud_stt_config()),
            };

            engine.transcribe(request).await
        });

        match result {
            Ok(transcription) => {
                println!(
                    "[Volcengine Basic Test] Transcription: \"{}\"",
                    transcription.text
                );
                assert!(
                    !transcription.text.is_empty(),
                    "Transcription should not be empty"
                );
            }
            Err(e) => {
                eprintln!("[Volcengine Basic Test] Failed: {}", e);
                panic!("Volcengine basic transcription failed: {}", e);
            }
        }
    }

    #[test]
    #[ignore]
    fn test_volcengine_stt_chinese() {
        let settings = match load_settings_from_env_or_default() {
            Some(s) => s,
            None => {
                eprintln!("Skipping integration test: settings.json not found");
                return;
            }
        };

        skip_if_no_volcengine_config(&settings);

        let wav_data = create_test_wav(16000, 1, 3.0);
        let temp_path = tempfile::NamedTempFile::new().expect("Failed to create temp file");
        fs::write(temp_path.path(), &wav_data).expect("Failed to write temp WAV");

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            let engine = CloudSttEngine::new().expect("Failed to create CloudSttEngine");
            let request = TranscriptionRequest {
                audio_path: temp_path.path().to_path_buf(),
                language: Some("zh-CN".to_string()),
                model_name: None,
                initial_prompt: None,
                denoise_mode: "off".to_string(),
                vad_enabled: false,
                cloud_config: Some(settings.get_active_cloud_stt_config()),
            };

            engine.transcribe(request).await
        });

        match result {
            Ok(transcription) => {
                println!(
                    "[Volcengine Chinese Test] Transcription: \"{}\"",
                    transcription.text
                );
                assert!(
                    !transcription.text.is_empty(),
                    "Chinese transcription should not be empty"
                );
            }
            Err(e) => {
                eprintln!("[Volcengine Chinese Test] Failed: {}", e);
                panic!("Volcengine Chinese transcription failed: {}", e);
            }
        }
    }

    #[test]
    fn test_volcengine_stt_empty_audio() {
        let empty_wav_data = create_test_wav(16000, 1, 0.1);
        let temp_path = tempfile::NamedTempFile::new().expect("Failed to create temp file");
        fs::write(temp_path.path(), &empty_wav_data).expect("Failed to write temp WAV");

        let config = CloudSttConfig {
            enabled: true,
            provider_type: "volcengine-streaming".to_string(),
            api_key: "dummy-key".to_string(),
            app_id: "dummy-app".to_string(),
            base_url: "".to_string(),
            model: "".to_string(),
            language: "".to_string(),
        };

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            let engine = CloudSttEngine::new().expect("Failed to create CloudSttEngine");
            let request = TranscriptionRequest {
                audio_path: temp_path.path().to_path_buf(),
                language: Some("en-US".to_string()),
                model_name: None,
                initial_prompt: None,
                denoise_mode: "off".to_string(),
                vad_enabled: false,
                cloud_config: Some(config),
            };

            engine.transcribe(request).await
        });

        match result {
            Ok(transcription) => {
                assert!(
                    transcription.text.len() <= 100,
                    "Empty audio shouldn't produce long transcription"
                );
            }
            Err(e) => {
                assert!(
                    e.contains("streaming lifecycle")
                        || e.contains("StreamingSttEngine")
                        || e.contains("authentication")
                        || e.contains("connection")
                        || e.contains("401")
                        || e.contains("403"),
                    "Should be streaming lifecycle guidance or auth/connection error, got: {}",
                    e
                );
            }
        }
    }

    #[test]
    #[ignore]
    fn test_volcengine_stt_error_handling() {
        let settings = match load_settings_from_env_or_default() {
            Some(s) => s,
            None => {
                eprintln!("Skipping integration test: settings.json not found");
                return;
            }
        };

        skip_if_no_volcengine_config(&settings);

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            let engine = CloudSttEngine::new().expect("Failed to create CloudSttEngine");
            let request = TranscriptionRequest {
                audio_path: PathBuf::from("/non/existent/file.wav"),
                language: Some("en-US".to_string()),
                model_name: None,
                initial_prompt: None,
                denoise_mode: "off".to_string(),
                vad_enabled: false,
                cloud_config: Some(settings.get_active_cloud_stt_config()),
            };

            engine.transcribe(request).await
        });

        match result {
            Ok(_) => {
                panic!("Should have failed with invalid audio path");
            }
            Err(e) => {
                assert!(
                    e.contains("file") || e.contains("audio") || e.contains("read"),
                    "Error should mention file/audio reading issue"
                );
            }
        }
    }

    #[test]
    #[ignore]
    fn test_openai_stt_basic() {
        let settings = match load_settings_from_env_or_default() {
            Some(s) => s,
            None => {
                eprintln!("Skipping integration test: settings.json not found");
                return;
            }
        };

        skip_if_no_openai_config(&settings);

        let wav_data = create_test_wav(16000, 1, 2.0);
        let temp_path = tempfile::NamedTempFile::new().expect("Failed to create temp file");
        fs::write(temp_path.path(), &wav_data).expect("Failed to write temp WAV");

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            let engine = CloudSttEngine::new().expect("Failed to create CloudSttEngine");
            let request = TranscriptionRequest {
                audio_path: temp_path.path().to_path_buf(),
                language: Some("en".to_string()),
                model_name: None,
                initial_prompt: None,
                denoise_mode: "off".to_string(),
                vad_enabled: false,
                cloud_config: Some(settings.get_active_cloud_stt_config()),
            };

            engine.transcribe(request).await
        });

        match result {
            Ok(transcription) => {
                println!(
                    "[OpenAI Basic Test] Transcription: \"{}\"",
                    transcription.text
                );
                assert!(
                    !transcription.text.is_empty(),
                    "OpenAI transcription should not be empty"
                );
            }
            Err(e) => {
                eprintln!("[OpenAI Basic Test] Failed: {}", e);
                panic!("OpenAI basic transcription failed: {}", e);
            }
        }
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
