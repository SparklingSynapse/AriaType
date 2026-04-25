//! End-to-End Tests for AriaType Application Flow
//!
//! These tests validate the core functionality of the application using public APIs
//! that don't require a Tauri runtime context.

mod common;
use common::audio_fixtures::{create_speech_like_wav, write_temp_wav};

#[test]
fn test_e2e_recording_state_transitions() {
    use ariatype_lib::state::unified_state::{RecordingState, UnifiedRecordingState};

    let state = UnifiedRecordingState::new();

    assert_eq!(state.current(), RecordingState::Idle);

    assert!(state.transition_to(RecordingState::Starting).is_ok());
    assert!(state.transition_to(RecordingState::Recording).is_ok());
    assert!(state.transition_to(RecordingState::Stopping).is_ok());
    assert!(state.transition_to(RecordingState::Idle).is_ok());

    assert!(state.transition_to(RecordingState::Recording).is_err());

    // Error can be reached from Starting state
    assert!(state.transition_to(RecordingState::Starting).is_ok());
    assert!(state.transition_to(RecordingState::Error).is_ok());
    assert!(state.transition_to(RecordingState::Idle).is_ok());
}

#[test]
fn test_e2e_audio_system_functions() {
    let devices = ariatype_lib::commands::system::get_audio_devices();
    println!("Audio devices count: {}", devices.len());

    let log_content = ariatype_lib::commands::system::get_log_content(100);
    println!("Log content length: {}", log_content.len());
}

#[test]
fn test_e2e_settings_default() {
    let settings = ariatype_lib::commands::settings::AppSettings::default();

    assert_eq!(settings.shortcut_profiles.dictate.hotkey, "Cmd+Slash");
    assert_eq!(settings.shortcut_profiles.chat.hotkey, "Opt+Slash");
    assert!(settings.shortcut_profiles.custom.is_none());
    assert_eq!(settings.model, "whisper-base");
    assert_eq!(settings.language, "auto");
    assert!(!settings.cloud_polish_enabled);
}

#[test]
fn test_e2e_cloud_stt_config() {
    let config = ariatype_lib::commands::settings::CloudSttConfig::default();
    assert!(!config.enabled);
    assert!(config.provider_type.is_empty());
    assert!(config.api_key.is_empty());
}

#[test]
fn test_e2e_cloud_polish_config() {
    let config = ariatype_lib::commands::settings::CloudProviderConfig::default();
    assert!(!config.enabled);
    assert!(config.provider_type.is_empty());
    assert!(config.api_key.is_empty());
}

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_e2e_very_short_audio_creation() {
        let wav_data = create_speech_like_wav(16000, 1, 0.1);
        assert!(!wav_data.is_empty());

        let temp_path = write_temp_wav(&wav_data);
        assert!(temp_path.exists());

        let _ = std::fs::remove_file(temp_path);
    }

    #[test]
    fn test_e2e_audio_resampling() {
        use std::io::Cursor;

        // Create test audio at 44.1kHz
        let samples: Vec<f32> = (0..44100)
            .map(|i| {
                let t = i as f32 / 44100.0;
                (2.0 * std::f32::consts::PI * 440.0 * t).sin() * 0.5
            })
            .collect();

        let resampled = ariatype_lib::audio::resampler::resample_to_16khz(&samples, 44100).unwrap();

        // 44100 samples at 44.1kHz = 1 second
        // At 16kHz, we expect ~16000 samples
        let expected = 16000.0;
        let tolerance = expected * 0.05;
        assert!(
            resampled.len() as f32 >= expected - tolerance
                && resampled.len() as f32 <= expected + tolerance,
            "Expected ~{} samples, got {}",
            expected,
            resampled.len()
        );
    }
}

// Pipeline tests using mock engines
mod mock_stt {
    use ariatype_lib::stt_engine::{EngineType, TranscriptionRequest, TranscriptionResult};

    pub struct MockSttEngine {
        pub result_text: String,
        pub latency_ms: u64,
        pub should_fail: bool,
    }

    impl MockSttEngine {
        pub fn new(text: impl Into<String>) -> Self {
            Self {
                result_text: text.into(),
                latency_ms: 100,
                should_fail: false,
            }
        }

        pub fn with_latency(mut self, latency_ms: u64) -> Self {
            self.latency_ms = latency_ms;
            self
        }

        pub fn with_failure(mut self) -> Self {
            self.should_fail = true;
            self
        }

        pub async fn transcribe(
            &self,
            _request: TranscriptionRequest,
        ) -> Result<TranscriptionResult, String> {
            tokio::time::sleep(tokio::time::Duration::from_millis(self.latency_ms)).await;

            if self.should_fail {
                return Err("Mock STT engine failed".to_string());
            }

            Ok(TranscriptionResult::with_metrics(
                self.result_text.clone(),
                EngineType::Whisper,
                self.latency_ms,
                Some(50),
                Some(20),
                Some(30),
            ))
        }
    }
}

mod mock_polish {
    use ariatype_lib::polish_engine::{PolishEngineType, PolishRequest, PolishResult};

    pub struct MockPolishEngine {
        pub result_text: String,
        pub latency_ms: u64,
        pub should_fail: bool,
    }

    impl MockPolishEngine {
        pub fn new(text: impl Into<String>) -> Self {
            Self {
                result_text: text.into(),
                latency_ms: 50,
                should_fail: false,
            }
        }

        pub fn with_latency(mut self, latency_ms: u64) -> Self {
            self.latency_ms = latency_ms;
            self
        }

        pub fn with_failure(mut self) -> Self {
            self.should_fail = true;
            self
        }

        pub async fn polish(&self, _request: PolishRequest) -> Result<PolishResult, String> {
            tokio::time::sleep(tokio::time::Duration::from_millis(self.latency_ms)).await;

            if self.should_fail {
                return Err("Mock polish engine failed".to_string());
            }

            Ok(PolishResult::with_metrics(
                self.result_text.clone(),
                PolishEngineType::Qwen,
                self.latency_ms,
                Some(10),
                Some(40),
            ))
        }
    }
}

#[tokio::test]
async fn test_e2e_mock_stt_engine_success() {
    let engine = mock_stt::MockSttEngine::new("Hello world");

    let request = ariatype_lib::stt_engine::TranscriptionRequest::new(vec![0.0f32; 16000]);
    let result = engine.transcribe(request).await.unwrap();

    assert_eq!(result.text, "Hello world");
    assert_eq!(result.engine, ariatype_lib::stt_engine::EngineType::Whisper);
    assert!(result.total_ms >= 100);
}

#[tokio::test]
async fn test_e2e_mock_stt_engine_failure() {
    let engine = mock_stt::MockSttEngine::new("Should not appear").with_failure();

    let request = ariatype_lib::stt_engine::TranscriptionRequest::new(vec![0.0f32; 16000]);
    let result = engine.transcribe(request).await;

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("failed"));
}

#[tokio::test]
async fn test_e2e_mock_polish_engine_success() {
    let engine = mock_polish::MockPolishEngine::new("Polished text");

    let request =
        ariatype_lib::polish_engine::PolishRequest::new("Raw text", "System prompt", "en");
    let result = engine.polish(request).await.unwrap();

    assert_eq!(result.text, "Polished text");
    assert_eq!(
        result.engine,
        ariatype_lib::polish_engine::PolishEngineType::Qwen
    );
}

#[tokio::test]
async fn test_e2e_mock_polish_engine_failure() {
    let engine = mock_polish::MockPolishEngine::new("Should not appear").with_failure();

    let request =
        ariatype_lib::polish_engine::PolishRequest::new("Raw text", "System prompt", "en");
    let result = engine.polish(request).await;

    assert!(result.is_err());
}

async fn run_pipeline(
    samples: Vec<f32>,
    stt_result: &str,
    polish_result: &str,
    polish_enabled: bool,
) -> Result<String, String> {
    let stt = mock_stt::MockSttEngine::new(stt_result);
    let polish = mock_polish::MockPolishEngine::new(polish_result);

    let stt_request = ariatype_lib::stt_engine::TranscriptionRequest::new(samples);
    let stt_result = stt.transcribe(stt_request).await?;

    if polish_enabled && !stt_result.text.is_empty() {
        let polish_request = ariatype_lib::polish_engine::PolishRequest::new(
            stt_result.text.clone(),
            "Polish this text",
            "en",
        );
        let polish_result = polish.polish(polish_request).await?;
        Ok(polish_result.text)
    } else {
        Ok(stt_result.text)
    }
}

#[tokio::test]
async fn test_e2e_pipeline_transcribe_only() {
    let result = run_pipeline(
        vec![0.0f32; 16000],
        "This is a test transcription",
        "This should not be used",
        false,
    )
    .await
    .unwrap();

    assert_eq!(result, "This is a test transcription");
}

#[tokio::test]
async fn test_e2e_pipeline_transcribe_and_polish() {
    let result = run_pipeline(
        vec![0.0f32; 16000],
        "um hello world uh",
        "hello world",
        true,
    )
    .await
    .unwrap();

    assert_eq!(result, "hello world");
}

#[tokio::test]
async fn test_e2e_pipeline_stt_fails_gracefully() {
    let stt = mock_stt::MockSttEngine::new("Should fail").with_failure();
    let request = ariatype_lib::stt_engine::TranscriptionRequest::new(vec![0.0f32; 16000]);
    let result = stt.transcribe(request).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_e2e_pipeline_empty_transcription() {
    let result = run_pipeline(vec![0.0f32; 16000], "", "Should not be called", true)
        .await
        .unwrap();

    assert_eq!(result, "");
}

#[tokio::test]
async fn test_e2e_pipeline_polish_failure_recovery() {
    let stt = mock_stt::MockSttEngine::new("um hello world uh");
    let polish = mock_polish::MockPolishEngine::new("Should fail").with_failure();

    let stt_request = ariatype_lib::stt_engine::TranscriptionRequest::new(vec![0.0f32; 16000]);
    let stt_result = stt.transcribe(stt_request).await.unwrap();

    assert_eq!(stt_result.text, "um hello world uh");

    let polish_request = ariatype_lib::polish_engine::PolishRequest::new(
        stt_result.text.clone(),
        "Polish this",
        "en",
    );
    let polish_result = polish.polish(polish_request).await;

    assert!(polish_result.is_err(), "Polish should fail");

    let pipeline_result: Result<String, String> = async {
        let stt_request = ariatype_lib::stt_engine::TranscriptionRequest::new(vec![0.0f32; 16000]);
        let stt_result = stt.transcribe(stt_request).await?;

        if !stt_result.text.is_empty() {
            let polish_request = ariatype_lib::polish_engine::PolishRequest::new(
                stt_result.text.clone(),
                "Polish this",
                "en",
            );
            match polish.polish(polish_request).await {
                Ok(polish_result) => Ok(polish_result.text),
                Err(_) => Ok(stt_result.text),
            }
        } else {
            Ok(stt_result.text)
        }
    }
    .await;

    assert!(
        pipeline_result.is_ok(),
        "Pipeline should handle polish failure gracefully"
    );
    let final_text = pipeline_result.unwrap();
    assert_eq!(
        final_text, "um hello world uh",
        "Should return original STT text on polish failure"
    );
}

mod shortcut_profiles {
    use ariatype_lib::commands::settings::AppSettings;
    use ariatype_lib::shortcut::{
        ShortcutAction, ShortcutProfile, ShortcutProfilesMap, ShortcutTriggerMode,
    };

    #[test]
    fn default_profiles_have_correct_hotkeys() {
        let settings = AppSettings::default();
        let profiles = &settings.shortcut_profiles;

        assert_eq!(profiles.dictate.hotkey, "Cmd+Slash");
        assert_eq!(profiles.chat.hotkey, "Opt+Slash");
        assert!(profiles.custom.is_none());
    }

    #[test]
    fn default_dictate_has_no_polish_template() {
        let profiles = ShortcutProfilesMap::default();
        let ShortcutAction::Record { polish_template_id } = &profiles.dictate.action;
        assert!(polish_template_id.is_none());
    }

    #[test]
    fn default_chat_has_polish_template() {
        let profiles = ShortcutProfilesMap::default();
        let ShortcutAction::Record { polish_template_id } = &profiles.chat.action;
        assert!(polish_template_id.is_some());
        assert_eq!(polish_template_id.as_deref(), Some("filler"));
    }

    #[test]
    fn profiles_map_serialization_roundtrip() {
        let profiles = ShortcutProfilesMap {
            dictate: ShortcutProfile {
                hotkey: "Cmd+Slash".to_string(),
                trigger_mode: ShortcutTriggerMode::Hold,
                action: ShortcutAction::Record {
                    polish_template_id: None,
                },
            },
            chat: ShortcutProfile {
                hotkey: "Opt+Slash".to_string(),
                trigger_mode: ShortcutTriggerMode::Toggle,
                action: ShortcutAction::Record {
                    polish_template_id: Some("filler".to_string()),
                },
            },
            custom: Some(ShortcutProfile {
                hotkey: "Cmd+Shift+Space".to_string(),
                trigger_mode: ShortcutTriggerMode::Toggle,
                action: ShortcutAction::Record {
                    polish_template_id: Some("formal".to_string()),
                },
            }),
        };

        let json = serde_json::to_string(&profiles).unwrap();
        let decoded: ShortcutProfilesMap = serde_json::from_str(&json).unwrap();
        assert_eq!(profiles, decoded);
    }

    #[test]
    fn custom_profile_serializes_skip_when_none() {
        let profiles = ShortcutProfilesMap::default();
        let json = serde_json::to_string(&profiles).unwrap();
        assert!(!json.contains("\"custom\""));
    }

    #[test]
    fn custom_profile_serializes_when_present() {
        let profiles = ShortcutProfilesMap {
            dictate: ShortcutProfile::default_dictate(),
            chat: ShortcutProfile::default_chat(),
            custom: Some(ShortcutProfile {
                hotkey: "Cmd+Alt+Space".to_string(),
                trigger_mode: ShortcutTriggerMode::Toggle,
                action: ShortcutAction::Record {
                    polish_template_id: None,
                },
            }),
        };
        let json = serde_json::to_string(&profiles).unwrap();
        assert!(json.contains("\"custom\""));
    }

    #[test]
    fn migration_from_old_hotkey_field() {
        let old_json = serde_json::json!({
            "hotkey": "Shift+Space",
            "model": "whisper-base",
            "language": "auto"
        });

        let mut json_value = old_json.clone();
        ariatype_lib::commands::settings::migrate_to_profiles_map_for_test(&mut json_value);

        assert!(json_value.get("hotkey").is_none());
        let profiles = json_value.get("shortcut_profiles").unwrap();
        let dictate_hotkey = profiles.get("dictate").unwrap().get("hotkey").unwrap();
        assert_eq!(dictate_hotkey.as_str(), Some("Shift+Space"));
        let chat_hotkey = profiles.get("chat").unwrap().get("hotkey").unwrap();
        assert_eq!(chat_hotkey.as_str(), Some(""));
    }

    #[test]
    fn settings_get_dictate_hotkey() {
        let settings = AppSettings::default();
        assert_eq!(settings.get_dictate_hotkey(), "Cmd+Slash");
    }

    #[test]
    fn settings_get_chat_hotkey() {
        let settings = AppSettings::default();
        assert_eq!(settings.get_chat_hotkey(), "Opt+Slash");
    }

    #[test]
    fn settings_set_dictate_hotkey() {
        let mut settings = AppSettings::default();
        settings.set_dictate_hotkey("Cmd+Shift+A");
        assert_eq!(settings.shortcut_profiles.dictate.hotkey, "Cmd+Shift+A");
    }

    #[test]
    fn settings_get_custom_hotkey_none_when_absent() {
        let settings = AppSettings::default();
        assert!(settings.get_custom_hotkey().is_none());
    }

    #[test]
    fn settings_get_custom_hotkey_when_present() {
        let mut settings = AppSettings::default();
        settings.shortcut_profiles.custom = Some(ShortcutProfile {
            hotkey: "Cmd+Alt+Space".to_string(),
            trigger_mode: ShortcutTriggerMode::Toggle,
            action: ShortcutAction::Record {
                polish_template_id: None,
            },
        });
        assert_eq!(
            settings.get_custom_hotkey(),
            Some("Cmd+Alt+Space".to_string())
        );
    }

    #[test]
    fn migration_from_profiles_array_to_map() {
        let old_json = serde_json::json!({
            "shortcut_profiles": [
                {"hotkey": "Shift+Space", "action": {"Record": {"polish_template_id": null}}},
                {"hotkey": "Cmd+Space", "action": {"Record": {"polish_template_id": "filler"}}}
            ]
        });

        let mut json_value = old_json;
        ariatype_lib::commands::settings::migrate_to_profiles_map_for_test(&mut json_value);

        let profiles = json_value.get("shortcut_profiles").unwrap();
        assert!(profiles.is_object());
        assert_eq!(
            profiles
                .get("dictate")
                .unwrap()
                .get("hotkey")
                .unwrap()
                .as_str(),
            Some("Shift+Space")
        );
        assert_eq!(
            profiles
                .get("chat")
                .unwrap()
                .get("hotkey")
                .unwrap()
                .as_str(),
            Some("Cmd+Space")
        );
    }
}
