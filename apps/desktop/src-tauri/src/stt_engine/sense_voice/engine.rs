use std::sync::Arc;

use super::transcriber::SenseVoiceTranscriber;
use crate::stt_engine::traits::{EngineType, SttEngine, TranscriptionRequest, TranscriptionResult};

#[derive(Clone)]
pub struct SenseVoiceEngine {
    transcriber: Arc<SenseVoiceTranscriber>,
}

impl SenseVoiceEngine {
    pub fn new(models_dir: &std::path::Path, version: &str) -> Result<Self, String> {
        let model_path = models_dir.join(format!("{}.gguf", version));

        if !model_path.exists() {
            return Err(format!(
                "SenseVoice model '{}' not found at {:?}",
                version, model_path
            ));
        }

        let transcriber = SenseVoiceTranscriber::new(&model_path)?;

        Ok(Self {
            transcriber: Arc::new(transcriber),
        })
    }
}

impl SttEngine for SenseVoiceEngine {
    fn engine_type(&self) -> EngineType {
        EngineType::SenseVoice
    }

    async fn transcribe(
        &self,
        request: TranscriptionRequest,
    ) -> Result<TranscriptionResult, String> {
        let engine_type = self.engine_type();
        let start = std::time::Instant::now();

        let audio_path = request.audio_path.clone();
        let language = request.language.clone();
        let transcriber = self.transcriber.clone();

        let text = tokio::task::spawn_blocking(move || {
            transcriber.transcribe(&audio_path, language.as_deref())
        })
        .await
        .map_err(|e| format!("Transcription task failed: {}", e))??;

        let total_ms = start.elapsed().as_millis() as u64;

        Ok(TranscriptionResult::with_metrics(
            text,
            engine_type,
            total_ms,
            None,
            None,
            Some(total_ms),
        ))
    }
}
