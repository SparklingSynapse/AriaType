use std::path::Path;
use std::sync::{Arc, Mutex};

use sherpa_onnx::{
    OfflineRecognizer, OfflineRecognizerConfig, OfflineSenseVoiceModelConfig,
    OfflineWhisperModelConfig,
};
use tracing::{debug, info};

use crate::stt_engine::models::ModelDefinition;
use crate::stt_engine::traits::{EngineType, TranscriptionRequest, TranscriptionResult};
use crate::stt_engine::unified_manager::InferenceProvider;

struct ThreadSafeRecognizer(OfflineRecognizer);

// SAFETY: OfflineRecognizer wraps a C++ pointer (*const) that lacks
// auto-derived Send/Sync. We gate all access through a Mutex, so no
// concurrent mutation is possible. The underlying C++ recognizer is
// safe to use from any thread when accessed exclusively.
unsafe impl Send for ThreadSafeRecognizer {}
unsafe impl Sync for ThreadSafeRecognizer {}

#[derive(Clone)]
pub struct SherpaOnnxEngine {
    recognizer: Arc<Mutex<ThreadSafeRecognizer>>,
    engine_type: EngineType,
}

impl SherpaOnnxEngine {
    /// Create a new engine from a model directory and model definition.
    ///
    /// The model directory should contain the model files as specified in the
    /// model definition's `files` array (e.g. `model.int8.onnx`, `tokens.txt`).
    pub fn new(
        model_dir: &Path,
        model_def: &ModelDefinition,
        language: Option<&str>,
        provider: InferenceProvider,
    ) -> Result<Self, String> {
        let model_subdir = model_dir.join(model_def.name);
        let num_threads = std::thread::available_parallelism()
            .map(|n| n.get().min(4) as i32)
            .unwrap_or(2);

        let mut config = OfflineRecognizerConfig::default();

        match model_def.engine_type {
            EngineType::SenseVoice => {
                let model_path = model_subdir.join("model.int8.onnx");
                let tokens_path = model_subdir.join("tokens.txt");

                if !model_path.exists() {
                    return Err(format!(
                        "SenseVoice model not found at: {}",
                        model_path.display()
                    ));
                }

                info!(
                    engine = "sensevoice",
                    model = %model_path.display(),
                    threads = num_threads,
                    "loading_model"
                );

                config.model_config.sense_voice = OfflineSenseVoiceModelConfig {
                    model: Some(
                        model_path
                            .to_str()
                            .ok_or("Invalid model path encoding")?
                            .to_string(),
                    ),
                    language: Some("auto".to_string()),
                    use_itn: true,
                };
                config.model_config.tokens = Some(
                    tokens_path
                        .to_str()
                        .ok_or("Invalid tokens path encoding")?
                        .to_string(),
                );
            }
            EngineType::Whisper => {
                let prefix = match model_def.name {
                    "whisper-base" => "base",
                    "whisper-small" => "small",
                    other => {
                        return Err(format!("Unknown whisper variant: {}", other));
                    }
                };
                let encoder_path = model_subdir.join(format!("{}-encoder.onnx", prefix));
                let decoder_path = model_subdir.join(format!("{}-decoder.onnx", prefix));
                let tokens_path = model_subdir.join(format!("{}-tokens.txt", prefix));

                if !encoder_path.exists() {
                    return Err(format!(
                        "Whisper encoder not found at: {}",
                        encoder_path.display()
                    ));
                }
                if !decoder_path.exists() {
                    return Err(format!(
                        "Whisper decoder not found at: {}",
                        decoder_path.display()
                    ));
                }

                let whisper_lang = match language {
                    Some(lang) if lang != "auto" => {
                        let base = lang.split('-').next().unwrap_or(lang);
                        Some(base.to_string())
                    }
                    _ => None,
                };

                info!(
                    engine = "whisper",
                    encoder = %encoder_path.display(),
                    decoder = %decoder_path.display(),
                    language = ?whisper_lang,
                    threads = num_threads,
                    "loading_model"
                );

                config.model_config.whisper = OfflineWhisperModelConfig {
                    encoder: Some(
                        encoder_path
                            .to_str()
                            .ok_or("Invalid encoder path encoding")?
                            .to_string(),
                    ),
                    decoder: Some(
                        decoder_path
                            .to_str()
                            .ok_or("Invalid decoder path encoding")?
                            .to_string(),
                    ),
                    language: whisper_lang,
                    task: Some("transcribe".to_string()),
                    tail_paddings: -1,
                    enable_token_timestamps: false,
                    enable_segment_timestamps: false,
                };
                config.model_config.tokens = Some(
                    tokens_path
                        .to_str()
                        .ok_or("Invalid tokens path encoding")?
                        .to_string(),
                );
            }
            EngineType::Cloud => {
                return Err("Cloud engine not supported by SherpaOnnxEngine".to_string());
            }
        }

        config.model_config.num_threads = num_threads;
        config.model_config.provider = Some(provider.as_str().to_string());
        info!(
            engine = model_def.engine_type.as_str(),
            provider = %provider,
            "provider_configured"
        );
        config.model_config.debug = false;

        let recognizer = OfflineRecognizer::create(&config).ok_or_else(|| {
            format!(
                "Failed to create {} recognizer",
                model_def.engine_type.as_str()
            )
        })?;

        info!(
            engine = model_def.engine_type.as_str(),
            model = %model_def.name,
            "model_loaded"
        );

        Ok(Self {
            recognizer: Arc::new(Mutex::new(ThreadSafeRecognizer(recognizer))),
            engine_type: model_def.engine_type,
        })
    }
}

impl SherpaOnnxEngine {
    pub fn engine_type(&self) -> EngineType {
        self.engine_type
    }

    pub async fn transcribe(
        &self,
        request: TranscriptionRequest,
    ) -> Result<TranscriptionResult, String> {
        let engine_type = self.engine_type();
        let start = std::time::Instant::now();

        let samples = request.samples.clone();
        let engine = self.clone();

        let (text, preprocess_ms, inference_ms) =
            tokio::task::spawn_blocking(move || -> Result<(String, u64, u64), String> {
                let preprocess_start = std::time::Instant::now();

                let duration = samples.len() as f32 / 16_000.0;
                if duration < 0.35 {
                    debug!(
                        engine = engine_type.as_str(),
                        duration_secs = format!("{:.2}", duration),
                        "audio_too_short"
                    );
                    return Ok((String::new(), 0, 0));
                }

                let audio = samples;

                let duration = audio.len() as f32 / 16_000.0;

                let preprocess_ms = preprocess_start.elapsed().as_millis() as u64;
                let inference_start = std::time::Instant::now();

                let guard = engine.recognizer.lock().unwrap();
                let recognizer = &guard.0;
                let stream = recognizer.create_stream();
                stream.accept_waveform(16000, &audio);
                recognizer.decode(&stream);

                let text = stream
                    .get_result()
                    .map(|r| r.text.trim().to_string())
                    .unwrap_or_default();
                drop(guard);

                let inference_ms = inference_start.elapsed().as_millis() as u64;

                info!(
                    engine = engine_type.as_str(),
                    chars = text.len(),
                    duration_secs = format!("{:.2}", duration),
                    "transcription_completed"
                );

                Ok((text, preprocess_ms, inference_ms))
            })
            .await
            .map_err(|e| format!("Transcription task failed: {}", e))??;

        let total_ms = start.elapsed().as_millis() as u64;

        Ok(TranscriptionResult::with_metrics(
            text,
            engine_type,
            total_ms,
            Some(0),
            Some(preprocess_ms),
            Some(inference_ms),
        ))
    }
}
