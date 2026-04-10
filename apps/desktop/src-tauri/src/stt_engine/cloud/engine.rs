use crate::stt_engine::traits::EngineType;

#[derive(Clone)]
pub struct CloudSttEngine {}

impl CloudSttEngine {
    pub fn new() -> Result<Self, String> {
        Ok(Self {})
    }

    pub fn engine_type(&self) -> EngineType {
        EngineType::Cloud
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
}
