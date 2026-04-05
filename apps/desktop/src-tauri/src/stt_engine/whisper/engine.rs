use std::path::Path;
use std::sync::Arc;
use tracing::{info, warn};

use super::transcriber::Transcriber;
use crate::stt_engine::traits::{EngineType, SttEngine, TranscriptionRequest, TranscriptionResult};
use whisper_rs::WhisperContext;

#[derive(Clone)]
pub struct WhisperEngine {
    context: Arc<WhisperContext>,
}

impl WhisperEngine {
    pub fn new(models_dir: &Path, version: &str) -> Result<Self, String> {
        let model_path = models_dir.join(format!("ggml-{}.bin", version));

        if !model_path.exists() {
            return Err(format!(
                "Whisper model '{}' not found at {:?}",
                version, model_path
            ));
        }

        let context = Self::load_model(&model_path)?;

        Ok(Self {
            context: Arc::new(context),
        })
    }

    fn load_model(model_path: &Path) -> Result<WhisperContext, String> {
        let model_path_str = model_path
            .to_str()
            .ok_or_else(|| format!("Invalid model path: {:?}", model_path))?;

        info!(engine = "whisper", model = %model_path_str, "model_loading");

        let mut ctx_params = whisper_rs::WhisperContextParameters::default();

        #[cfg(target_os = "macos")]
        {
            let is_apple_silicon = std::env::consts::ARCH == "aarch64";
            ctx_params.use_gpu(is_apple_silicon);
            info!(
                engine = "whisper",
                gpu = is_apple_silicon,
                "gpu_acceleration_metal"
            );
        }

        #[cfg(target_os = "windows")]
        {
            ctx_params.use_gpu(false);
            info!(engine = "whisper", "inference_mode_cpu_windows");
        }

        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            ctx_params.use_gpu(false);
            info!(engine = "whisper", "inference_mode_cpu_unsupported");
        }

        let ctx = WhisperContext::new_with_params(model_path_str, ctx_params)
            .or_else(|e| {
                warn!(engine = "whisper", error = %e, "gpu_init_failed_fallback_cpu");
                let mut cpu_params = whisper_rs::WhisperContextParameters::default();
                cpu_params.use_gpu(false);
                WhisperContext::new_with_params(model_path_str, cpu_params)
            })
            .map_err(|e| format!("Failed to load Whisper model: {}", e))?;

        info!(engine = "whisper", "model_loaded");
        Ok(ctx)
    }
}

impl SttEngine for WhisperEngine {
    fn engine_type(&self) -> EngineType {
        EngineType::Whisper
    }

    async fn transcribe(
        &self,
        request: TranscriptionRequest,
    ) -> Result<TranscriptionResult, String> {
        let engine_type = self.engine_type();
        let start = std::time::Instant::now();

        let audio_path = request.audio_path.clone();
        let language = request.language.clone();
        let initial_prompt = request.initial_prompt.clone();
        let denoise_mode = request.denoise_mode.clone();
        let vad_enabled = request.vad_enabled;

        let context = self.context.clone();

        let (text, metrics) = tokio::task::spawn_blocking(move || {
            let transcriber = Transcriber::from_context(context);
            transcriber.transcribe_with_metrics(
                &audio_path,
                language.as_deref(),
                initial_prompt.as_deref(),
                &denoise_mode,
                vad_enabled,
            )
        })
        .await
        .map_err(|e| format!("Transcription task failed: {}", e))??;

        let total_ms = start.elapsed().as_millis() as u64;

        Ok(TranscriptionResult::with_metrics(
            text,
            engine_type,
            total_ms,
            Some(0),
            Some(metrics.preprocess_time_ms),
            Some(metrics.inference_time_ms),
        ))
    }
}
