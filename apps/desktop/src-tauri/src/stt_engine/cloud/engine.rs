use crate::stt_engine::traits::{EngineType, SttEngine, TranscriptionRequest, TranscriptionResult};

#[derive(Clone)]
pub struct CloudSttEngine {}

impl CloudSttEngine {
    pub fn new() -> Result<Self, String> {
        Ok(Self {})
    }
}

impl SttEngine for CloudSttEngine {
    fn engine_type(&self) -> EngineType {
        EngineType::Cloud
    }

    async fn transcribe(
        &self,
        _request: TranscriptionRequest,
    ) -> Result<TranscriptionResult, String> {
        Err(
            "Cloud STT requires streaming lifecycle. Use StreamingSttEngine trait instead of transcribe()."
                .to_string(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cloud_stt_engine_new() {
        let engine = CloudSttEngine::new();
        assert!(engine.is_ok());
    }

    #[test]
    fn test_cloud_stt_engine_engine_type() {
        let engine = CloudSttEngine::new().unwrap();
        assert_eq!(engine.engine_type(), EngineType::Cloud);
    }

    #[test]
    fn test_cloud_stt_engine_transcribe_error() {
        let engine = CloudSttEngine::new().unwrap();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            engine
                .transcribe(TranscriptionRequest::new("/tmp/test.wav"))
                .await
        });
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("streaming lifecycle"));
    }
}
