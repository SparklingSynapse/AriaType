use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use tracing::{error, info, instrument};

use super::cloud::CloudSttEngine;
use super::sense_voice::SenseVoiceEngine;
use super::traits::{EngineType, SttEngine, TranscriptionRequest, TranscriptionResult};
use super::whisper::WhisperEngine;
use crate::utils::AppPaths;
use crate::utils::{download, DownloadOptions, HuggingFaceSource};

use super::sense_voice::models as sense_voice_models;
use super::whisper::models as whisper_models;
use sense_voice_models::versions as sense_voice_versions;
use whisper_models::versions as whisper_versions;

type EngineCacheKey = (EngineType, String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub display_name: String,
    pub size_mb: u64,
    pub filename: String,
    pub downloaded: bool,
    pub speed_score: u8,
    pub accuracy_score: u8,
    pub engine: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendedModel {
    pub engine_type: EngineType,
    pub model_name: String,
    pub display_name: String,
    pub size_mb: u64,
    pub speed_score: u8,
    pub accuracy_score: u8,
    pub downloaded: bool,
}

const WHISPER_REPO: &str = "ggerganov/whisper.cpp";
const WHISPER_REVISION: &str = "5359861c739e955e79d9a303bcbc70fb988958b1";
const SENSEVOICE_REPO: &str = "lovemefan/sense-voice-gguf";

pub struct UnifiedEngineManager {
    models_dir: PathBuf,
    engine_cache: Arc<Mutex<Option<(EngineCacheKey, EngineInstance)>>>,
}

impl UnifiedEngineManager {
    pub fn new(models_dir: PathBuf) -> Self {
        info!(models_dir = ?models_dir, "engine_manager_initialized");
        Self {
            models_dir,
            engine_cache: Arc::new(Mutex::new(None)),
        }
    }

    pub fn default_models_dir() -> PathBuf {
        AppPaths::models_dir()
    }

    fn create_engine_instance(
        &self,
        engine_type: EngineType,
        version: &str,
    ) -> Result<EngineInstance, String> {
        match engine_type {
            EngineType::Whisper => {
                let engine = WhisperEngine::new(&self.models_dir, version)?;
                Ok(EngineInstance::Whisper(engine))
            }
            EngineType::SenseVoice => {
                let engine = SenseVoiceEngine::new(&self.models_dir, version)?;
                Ok(EngineInstance::SenseVoice(engine))
            }
            EngineType::Cloud => {
                let engine = CloudSttEngine::new()?;
                Ok(EngineInstance::Cloud(engine))
            }
        }
    }

    pub(crate) fn get_or_create_engine(
        &self,
        engine_type: EngineType,
        version: &str,
    ) -> Result<EngineInstance, String> {
        let cache_key = (engine_type, version.to_string());
        let mut cache = self.engine_cache.lock().unwrap();

        if let Some((cached_key, cached_engine)) = cache.as_ref() {
            if *cached_key == cache_key {
                return Ok(cached_engine.clone());
            }
        }

        let engine = self.create_engine_instance(engine_type, version)?;
        *cache = Some((cache_key, engine.clone()));
        Ok(engine)
    }

    pub(crate) fn clear_cache(&self) {
        let mut cache = self.engine_cache.lock().unwrap();
        *cache = None;
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn clear_engine_cache(&self, engine_type: EngineType, version: Option<&str>) {
        let mut cache = self.engine_cache.lock().unwrap();
        if let Some((cached_key, _)) = cache.as_ref() {
            let should_clear = if let Some(v) = version {
                cached_key.0 == engine_type && cached_key.1 == v
            } else {
                cached_key.0 == engine_type
            };

            if should_clear {
                *cache = None;
                info!(
                    engine = ?engine_type,
                    version = ?version,
                    "engine_cache_cleared"
                );
            }
        }
    }

    #[instrument(
        skip(self, request),
        fields(
            engine = ?engine_type,
            model = request.model_name.as_deref().unwrap_or("default"),
            language = request.language.as_deref().unwrap_or("auto"),
        ),
        ret,
        err
    )]
    pub async fn transcribe(
        &self,
        engine_type: EngineType,
        request: TranscriptionRequest,
    ) -> Result<TranscriptionResult, String> {
        let model = request
            .model_name
            .clone()
            .unwrap_or_else(|| "default".to_string());
        let lang = request
            .language
            .clone()
            .unwrap_or_else(|| "auto".to_string());
        info!(
            engine = ?engine_type,
            model = %model,
            language = %lang,
            path = ?request.audio_path,
            "transcription_started"
        );

        // Get model version from request, use default if not specified
        let version = request.model_name.as_deref().unwrap_or({
            match engine_type {
                EngineType::Whisper => whisper_versions::DEFAULT,
                EngineType::SenseVoice => sense_voice_versions::DEFAULT,
                EngineType::Cloud => "cloud",
            }
        });

        let engine = self.get_or_create_engine(engine_type, version)?;

        let result = engine.transcribe(request).await;

        match &result {
            Ok(r) => {
                info!(engine = ?engine_type, model = %model, text_len = r.text.len(), duration_ms = r.total_ms, "transcription_completed")
            }
            Err(e) => {
                error!(engine = ?engine_type, model = %model, error = %e, "transcription_failed")
            }
        }

        result
    }

    /// Get all available engines
    pub fn available_engines() -> Vec<EngineType> {
        EngineType::all()
    }

    // ==================== Model Management Functions ====================

    /// Get all models for a specific engine
    pub fn get_models(&self, engine_type: EngineType) -> Vec<ModelInfo> {
        let models: Vec<ModelInfo> = match engine_type {
            EngineType::Whisper => whisper_models::ALL
                .iter()
                .map(|def| {
                    let path = self.models_dir.join(def.filename);
                    ModelInfo {
                        name: def.name.to_string(),
                        display_name: def.display_name.to_string(),
                        size_mb: def.size_mb as u64,
                        filename: def.filename.to_string(),
                        downloaded: path.exists(),
                        speed_score: def.speed_score,
                        accuracy_score: def.accuracy_score,
                        engine: "whisper".to_string(),
                    }
                })
                .collect(),
            EngineType::SenseVoice => sense_voice_models::ALL
                .iter()
                .map(|def| {
                    let path = self.models_dir.join(def.filename);
                    ModelInfo {
                        name: def.name.to_string(),
                        display_name: def.display_name.to_string(),
                        size_mb: def.size_mb as u64,
                        filename: def.filename.to_string(),
                        downloaded: path.exists(),
                        speed_score: def.speed_score,
                        accuracy_score: def.accuracy_score,
                        engine: "sensevoice".to_string(),
                    }
                })
                .collect(),
            EngineType::Cloud => {
                vec![ModelInfo {
                    name: "cloud".to_string(),
                    display_name: "Cloud STT".to_string(),
                    size_mb: 0,
                    filename: "cloud".to_string(),
                    downloaded: true,
                    speed_score: 10,
                    accuracy_score: 10,
                    engine: "cloud".to_string(),
                }]
            }
        };

        models
    }

    /// Get all models from all local engines (excludes Cloud which is configured separately)
    pub fn get_all_models(&self) -> Vec<ModelInfo> {
        let mut all_models = Vec::new();
        for engine_type in EngineType::all() {
            // Skip Cloud engine - it's configured separately and doesn't have downloadable models
            if engine_type == EngineType::Cloud {
                continue;
            }
            all_models.extend(self.get_models(engine_type));
        }
        all_models
    }

    /// Auto-detect engine type by model name
    pub fn get_engine_by_model_name(model_name: &str) -> Option<EngineType> {
        if model_name == "cloud" {
            Some(EngineType::Cloud)
        } else if whisper_models::find_by_name(model_name).is_some() {
            Some(EngineType::Whisper)
        } else if sense_voice_models::find_by_name(model_name).is_some() {
            Some(EngineType::SenseVoice)
        } else {
            None
        }
    }

    /// Check if a model is downloaded
    pub fn is_model_downloaded(&self, engine_type: EngineType, model_name: &str) -> bool {
        let filename = match engine_type {
            EngineType::Whisper => {
                if let Some(model) = whisper_models::find_by_name(model_name) {
                    model.filename
                } else {
                    return false;
                }
            }
            EngineType::SenseVoice => {
                if let Some(model) = sense_voice_models::find_by_name(model_name) {
                    model.filename
                } else {
                    return false;
                }
            }
            EngineType::Cloud => {
                return true;
            }
        };

        self.models_dir.join(filename).exists()
    }

    /// Get the file path for a model
    pub fn get_model_path(&self, engine_type: EngineType, model_name: &str) -> PathBuf {
        match engine_type {
            EngineType::Whisper => {
                if let Some(model) = whisper_models::find_by_name(model_name) {
                    self.models_dir.join(model.filename)
                } else {
                    self.models_dir.join(format!("ggml-{}.bin", model_name))
                }
            }
            EngineType::SenseVoice => {
                if let Some(model) = sense_voice_models::find_by_name(model_name) {
                    self.models_dir.join(model.filename)
                } else {
                    self.models_dir.join(format!("{}.gguf", model_name))
                }
            }
            EngineType::Cloud => PathBuf::new(),
        }
    }

    /// Download a model
    pub async fn download_model<F>(
        &self,
        engine_type: EngineType,
        model_name: &str,
        cancel_flag: Arc<AtomicBool>,
        progress_callback: F,
    ) -> Result<PathBuf, String>
    where
        F: Fn(u64, u64) + Send + Sync + 'static,
    {
        let (repo, revision, filename) = match engine_type {
            EngineType::Whisper => {
                let model = whisper_models::find_by_name(model_name)
                    .ok_or_else(|| format!("Unknown Whisper model: {}", model_name))?;
                (WHISPER_REPO, Some(WHISPER_REVISION), model.filename)
            }
            EngineType::SenseVoice => {
                let model = sense_voice_models::find_by_name(model_name)
                    .ok_or_else(|| format!("Unknown SenseVoice model: {}", model_name))?;
                (SENSEVOICE_REPO, None, model.filename)
            }
            EngineType::Cloud => {
                return Err("Cloud models do not need to be downloaded".to_string());
            }
        };

        let output_path = self.models_dir.join(filename);

        let mut source_builder = HuggingFaceSource::new(repo, filename);
        if let Some(rev) = revision {
            source_builder = source_builder.with_revision(rev);
        }
        let source = source_builder.into_source();

        let urls = source.urls();

        let options = DownloadOptions::new(&urls[0], &output_path)
            .with_fallbacks(urls[1..].to_vec())
            .with_cancel_flag(cancel_flag)
            .with_progress_callback(Arc::new(progress_callback))
            .with_model_name(model_name);

        download(options).await
    }

    /// Preload model into cache
    #[instrument(skip(self), fields(engine = ?engine_type, model = %model_name), ret, err)]
    pub fn load_model(&self, engine_type: EngineType, model_name: &str) -> Result<(), String> {
        // Check if model is downloaded
        if !self.is_model_downloaded(engine_type, model_name) {
            return Err(format!(
                "Model '{}' not downloaded. Please download it first.",
                model_name
            ));
        }

        // Create engine instance and put it into cache
        info!(
            engine = ?engine_type,
            model = %model_name,
            "model_preload_started"
        );

        let _ = self.get_or_create_engine(engine_type, model_name)?;

        info!(
            engine = ?engine_type,
            model = %model_name,
            "model_preloaded"
        );

        Ok(())
    }

    /// Delete a model file
    pub fn delete_model(&self, engine_type: EngineType, model_name: &str) -> Result<(), String> {
        let model_path = self.get_model_path(engine_type, model_name);

        // Check if file exists
        if !model_path.exists() {
            return Err(format!(
                "Model '{}' not found at path: {}",
                model_name,
                model_path.display()
            ));
        }

        // If model is in cache, clear it first

        // Delete the model file
        std::fs::remove_file(&model_path)
            .map_err(|e| format!("Failed to delete model '{}': {}", model_name, e))?;

        info!(
            engine = ?engine_type,
            model = %model_name,
            path = ?model_path,
            "model_deleted"
        );

        Ok(())
    }

    /// Recommend engines and models by language
    ///
    /// Returns a list sorted by accuracy (descending), including all engine and model combinations that support the language
    pub fn recommend_by_language(&self, lang: &str) -> Vec<RecommendedModel> {
        let mut recommendations = Vec::new();

        // Get Whisper recommended models
        let whisper_models = whisper_models::recommend_by_language(lang);
        for model in whisper_models {
            let model_path = self.models_dir.join(model.filename);
            recommendations.push(RecommendedModel {
                engine_type: EngineType::Whisper,
                model_name: model.name.to_string(),
                display_name: model.display_name.to_string(),
                size_mb: model.size_mb as u64,
                speed_score: model.speed_score,
                accuracy_score: model.accuracy_score,
                downloaded: model_path.exists(),
            });
        }

        // Get SenseVoice recommended models
        let sensevoice_models = sense_voice_models::recommend_by_language(lang);
        for model in sensevoice_models {
            let model_path = self.models_dir.join(model.filename);
            recommendations.push(RecommendedModel {
                engine_type: EngineType::SenseVoice,
                model_name: model.name.to_string(),
                display_name: model.display_name.to_string(),
                size_mb: model.size_mb as u64,
                speed_score: model.speed_score,
                accuracy_score: model.accuracy_score,
                downloaded: model_path.exists(),
            });
        }

        // Sort by accuracy descending, then by speed descending for same accuracy
        recommendations.sort_by(|a, b| {
            b.accuracy_score
                .cmp(&a.accuracy_score)
                .then_with(|| b.speed_score.cmp(&a.speed_score))
        });

        info!(
            language = %lang,
            count = recommendations.len(),
            "language_recommendations_generated"
        );

        recommendations
    }
}

/// Engine instance enum (to avoid trait object issues)
#[derive(Clone)]
pub(crate) enum EngineInstance {
    Whisper(WhisperEngine),
    SenseVoice(SenseVoiceEngine),
    Cloud(CloudSttEngine),
}

impl EngineInstance {
    pub async fn transcribe(
        &self,
        request: TranscriptionRequest,
    ) -> Result<TranscriptionResult, String> {
        match self {
            EngineInstance::Whisper(engine) => engine.transcribe(request).await,
            EngineInstance::SenseVoice(engine) => engine.transcribe(request).await,
            EngineInstance::Cloud(engine) => engine.transcribe(request).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_type_parsing() {
        assert_eq!(
            "whisper".parse::<EngineType>().unwrap(),
            EngineType::Whisper
        );
        assert_eq!(
            "sensevoice".parse::<EngineType>().unwrap(),
            EngineType::SenseVoice
        );
        assert_eq!(
            "WHISPER".parse::<EngineType>().unwrap(),
            EngineType::Whisper
        );
        assert!("unknown".parse::<EngineType>().is_err());
    }

    #[test]
    fn test_available_engines() {
        let engines = UnifiedEngineManager::available_engines();
        assert_eq!(engines.len(), 3);
        assert!(engines.contains(&EngineType::Whisper));
        assert!(engines.contains(&EngineType::SenseVoice));
        assert!(engines.contains(&EngineType::Cloud));
    }

    #[test]
    fn test_version_constants() {
        // Test Whisper version constants
        assert_eq!(whisper_versions::DEFAULT, whisper_versions::BASE);
        assert_eq!(whisper_versions::ALL.len(), 5);
        assert!(whisper_versions::ALL.contains(&whisper_versions::TINY));
        assert!(whisper_versions::ALL.contains(&whisper_versions::BASE));
        assert!(whisper_versions::ALL.contains(&whisper_versions::SMALL_Q8_0));
        assert!(whisper_versions::ALL.contains(&whisper_versions::MEDIUM_Q5_0));
        assert!(whisper_versions::ALL.contains(&whisper_versions::LARGE_V3_TURBO_Q8_0));

        // Test SenseVoice version constants
        assert_eq!(
            sense_voice_versions::DEFAULT,
            sense_voice_versions::SMALL_Q4_K
        );
        assert_eq!(sense_voice_versions::ALL.len(), 2);
        assert!(sense_voice_versions::ALL.contains(&sense_voice_versions::SMALL_Q4_K));
        assert!(sense_voice_versions::ALL.contains(&sense_voice_versions::SMALL_Q8_0));
    }

    #[test]
    fn test_model_definitions() {
        // Test Whisper model definitions
        assert_eq!(whisper_models::TINY.name, "tiny");
        assert_eq!(whisper_models::TINY.speed_score, 10);
        assert_eq!(whisper_models::BASE.name, "base");
        assert_eq!(whisper_models::DEFAULT.name, "base");
        assert_eq!(whisper_models::ALL.len(), 5);

        // Test SenseVoice model definitions
        assert_eq!(
            sense_voice_models::SMALL_Q4_K.name,
            "sense-voice-small-q4_k"
        );
        assert_eq!(sense_voice_models::SMALL_Q4_K.size_mb, 244);
        assert_eq!(sense_voice_models::DEFAULT.name, "sense-voice-small-q4_k");
        assert_eq!(sense_voice_models::ALL.len(), 2);

        // Test model lookup
        let model = whisper_models::find_by_name("tiny");
        assert!(model.is_some());
        assert_eq!(model.unwrap().name, "tiny");

        let model = sense_voice_models::find_by_name("sense-voice-small-q8_0");
        assert!(model.is_some());
        assert_eq!(model.unwrap().name, "sense-voice-small-q8_0");
    }

    #[test]
    fn test_model_recommendations() {
        // Test recommending Whisper models by language
        let zh_models = whisper_models::recommend_by_language("zh");
        assert!(!zh_models.is_empty());
        // Should be sorted by accuracy descending
        for i in 1..zh_models.len() {
            assert!(zh_models[i - 1].accuracy_score >= zh_models[i].accuracy_score);
        }

        // Test recommending Whisper models by speed
        let fast_models = whisper_models::recommend_by_speed(8);
        assert!(!fast_models.is_empty());
        for model in fast_models {
            assert!(model.speed_score >= 8);
        }

        // Test recommending SenseVoice models by language
        let zh_models = sense_voice_models::recommend_by_language("zh");
        assert_eq!(zh_models.len(), 2);
    }

    #[test]
    fn test_transcription_result_creation() {
        use super::super::traits::TranscriptionResult;

        let result = TranscriptionResult::new("test text".to_string(), EngineType::Whisper, 1000);
        assert_eq!(result.text, "test text");
        assert_eq!(result.engine, EngineType::Whisper);
        assert_eq!(result.total_ms, 1000);
        assert!(result.model_load_ms.is_none());

        let result_with_metrics = TranscriptionResult::with_metrics(
            "test text".to_string(),
            EngineType::Whisper,
            1000,
            Some(100),
            Some(200),
            Some(700),
        );
        assert_eq!(result_with_metrics.model_load_ms, Some(100));
        assert_eq!(result_with_metrics.preprocess_ms, Some(200));
        assert_eq!(result_with_metrics.inference_ms, Some(700));
    }

    #[test]
    fn test_load_model_not_downloaded() {
        let temp_dir = std::env::temp_dir().join("test_load_model");
        let _ = std::fs::create_dir_all(&temp_dir);
        let manager = UnifiedEngineManager::new(temp_dir.clone());

        // Trying to load undownloaded model should fail
        let result = manager.load_model(EngineType::Whisper, "tiny");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not downloaded"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_delete_model_not_exists() {
        let temp_dir = std::env::temp_dir().join("test_delete_model");
        let _ = std::fs::create_dir_all(&temp_dir);
        let manager = UnifiedEngineManager::new(temp_dir.clone());

        // Trying to delete non-existent model should fail
        let result = manager.delete_model(EngineType::Whisper, "tiny");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_recommend_by_language_zh() {
        let temp_dir = std::env::temp_dir().join("test_recommend_zh");
        let _ = std::fs::create_dir_all(&temp_dir);
        let manager = UnifiedEngineManager::new(temp_dir.clone());

        let recommendations = manager.recommend_by_language("zh");

        // Should have recommendations
        assert!(!recommendations.is_empty());

        // Should include SenseVoice model (optimized for Chinese)
        let has_sensevoice = recommendations
            .iter()
            .any(|r| r.engine_type == EngineType::SenseVoice);
        assert!(has_sensevoice, "Should recommend SenseVoice for Chinese");

        // Should include Whisper models that support Chinese
        let has_whisper = recommendations
            .iter()
            .any(|r| r.engine_type == EngineType::Whisper);
        assert!(has_whisper, "Should recommend Whisper models for Chinese");

        // Verify sorted by accuracy descending
        for i in 1..recommendations.len() {
            let prev = &recommendations[i - 1];
            let curr = &recommendations[i];
            assert!(
                prev.accuracy_score >= curr.accuracy_score,
                "Should be sorted by accuracy score (descending)"
            );
        }

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_recommend_by_language_en() {
        let temp_dir = std::env::temp_dir().join("test_recommend_en");
        let _ = std::fs::create_dir_all(&temp_dir);
        let manager = UnifiedEngineManager::new(temp_dir.clone());

        let recommendations = manager.recommend_by_language("en");

        // Should have recommendations
        assert!(!recommendations.is_empty());

        // All recommended models should support English
        for rec in &recommendations {
            // Verify model names are not empty
            assert!(!rec.model_name.is_empty());
            assert!(!rec.display_name.is_empty());
        }

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_recommend_by_language_includes_download_status() {
        use std::fs::File;
        use std::io::Write;

        let temp_dir = std::env::temp_dir().join("test_recommend_download");
        let _ = std::fs::create_dir_all(&temp_dir);
        let manager = UnifiedEngineManager::new(temp_dir.clone());

        // Create a fake SenseVoice model file
        let model_path = temp_dir.join("sense-voice-small-q4_k.gguf");
        let mut file = File::create(&model_path).unwrap();
        file.write_all(b"fake model data").unwrap();

        let recommendations = manager.recommend_by_language("zh");

        // Find SenseVoice Small Q4 model
        let sensevoice_q4 = recommendations
            .iter()
            .find(|r| r.model_name == "sense-voice-small-q4_k");

        assert!(sensevoice_q4.is_some());
        assert!(
            sensevoice_q4.unwrap().downloaded,
            "Should detect downloaded model"
        );

        // Other models should be undownloaded
        let other_models: Vec<_> = recommendations
            .iter()
            .filter(|r| r.model_name != "sense-voice-small-q4_k")
            .collect();

        for model in other_models {
            assert!(!model.downloaded, "Other models should not be downloaded");
        }

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_recommend_by_language_empty_for_unsupported() {
        let temp_dir = std::env::temp_dir().join("test_recommend_unsupported");
        let _ = std::fs::create_dir_all(&temp_dir);
        let manager = UnifiedEngineManager::new(temp_dir.clone());

        // Use an unsupported language code
        let recommendations = manager.recommend_by_language("xyz");

        // Should return empty list
        assert!(
            recommendations.is_empty(),
            "Should return empty list for unsupported language"
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
