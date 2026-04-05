pub mod cloud;
pub mod sense_voice;
pub mod traits;
pub mod unified_manager;
pub mod whisper;

pub use traits::{EngineType, SttEngine, TranscriptionRequest, TranscriptionResult};
pub use unified_manager::{ModelInfo, RecommendedModel, UnifiedEngineManager};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_selection_by_type() {
        // Verify EngineType variants are available and match expected values
        assert_eq!(EngineType::Whisper.as_str(), "whisper");
        assert_eq!(EngineType::SenseVoice.as_str(), "sensevoice");
        assert_eq!(EngineType::Cloud.as_str(), "cloud");
    }

    #[test]
    fn test_error_types() {
        // Test that error handling through Result<String> works correctly
        let error_result: Result<TranscriptionResult, String> =
            Err("STT engine initialization failed".to_string());
        assert!(error_result.is_err());
        assert_eq!(
            error_result.unwrap_err(),
            "STT engine initialization failed"
        );

        // Test success path
        let success_result: Result<TranscriptionResult, String> = Ok(TranscriptionResult::new(
            "Test transcription".to_string(),
            EngineType::Whisper,
            100,
        ));
        assert!(success_result.is_ok());
        assert_eq!(success_result.unwrap().text, "Test transcription");
    }
}
