use crate::stt_engine::traits::{PartialResultCallback, RecordingConsumer};
use crate::stt_engine::unified_manager::UnifiedEngineManager;
use async_trait::async_trait;
use parking_lot::Mutex;
use std::sync::Arc;
use tracing::{info, instrument, warn};

/// Buffering consumer for local STT models (Whisper, SenseVoice via sherpa-onnx).
///
/// Accumulates PCM chunks in memory during recording, then batch-transcribes
/// when `finish()` is called. No partial results are produced — local engines
/// are inherently batch, not streaming.
pub struct BufferingConsumer {
    chunks: Mutex<Vec<Vec<i16>>>,
    engine_manager: Arc<UnifiedEngineManager>,
    model_name: String,
    language: String,
    initial_prompt: Option<String>,
    stt_context: crate::stt_engine::traits::SttContext,
}

impl BufferingConsumer {
    pub fn new(
        engine_manager: Arc<UnifiedEngineManager>,
        model_name: String,
        language: String,
        initial_prompt: Option<String>,
        stt_context: crate::stt_engine::traits::SttContext,
    ) -> Self {
        Self {
            chunks: Mutex::new(Vec::new()),
            engine_manager,
            model_name,
            language,
            initial_prompt,
            stt_context,
        }
    }
}

#[async_trait]
impl RecordingConsumer for BufferingConsumer {
    async fn send_chunk(&self, pcm_data: Vec<i16>) -> Result<(), String> {
        self.chunks.lock().push(pcm_data);
        Ok(())
    }

    #[instrument(skip(self), fields(model = %self.model_name, language = %self.language))]
    async fn finish(&self) -> Result<String, String> {
        let all_pcm: Vec<i16> = self.chunks.lock().drain(..).flatten().collect();

        if all_pcm.is_empty() {
            warn!("no_audio_chunks_buffered");
            return Ok(String::new());
        }

        let duration_secs = all_pcm.len() as f64 / 16000.0;
        if duration_secs < 0.35 {
            info!(duration_secs, "recording_too_short-skipping");
            return Ok(String::new());
        }

        let samples: Vec<f32> = all_pcm.iter().map(|&s| s as f32 / 32768.0).collect();

        let engine_type =
            crate::stt_engine::UnifiedEngineManager::get_engine_by_model_name(&self.model_name)
                .unwrap_or(crate::stt_engine::traits::EngineType::Whisper);

        let mut request = crate::stt_engine::traits::TranscriptionRequest::new(samples)
            .with_model(&self.model_name)
            .with_language(&self.language);

        let prompt_parts: Vec<&str> = [
            self.initial_prompt.as_deref(),
            self.stt_context.domain.as_deref(),
            self.stt_context.subdomain.as_deref(),
            self.stt_context.glossary.as_deref(),
        ]
        .iter()
        .filter_map(|&s| s.filter(|v| !v.is_empty()))
        .collect();

        if !prompt_parts.is_empty() {
            request = request.with_prompt(prompt_parts.join(" "));
        }

        let result = self.engine_manager.transcribe(engine_type, request).await?;
        Ok(result.text)
    }

    fn set_partial_callback(&mut self, _callback: PartialResultCallback) {
        // No-op: batch engines don't produce partial results
    }
}
