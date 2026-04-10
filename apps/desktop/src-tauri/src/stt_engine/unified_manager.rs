use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use tracing::{error, info, instrument};

use super::models;
use super::sherpa_onnx::SherpaOnnxEngine;
use super::traits::{EngineType, TranscriptionRequest, TranscriptionResult};
use crate::utils::AppPaths;
use crate::utils::{download, DownloadOptions, HuggingFaceSource};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InferenceProvider {
    Cpu,
    CoreML,
    Cuda,
}

impl InferenceProvider {
    pub fn as_str(&self) -> &'static str {
        match self {
            InferenceProvider::Cpu => "cpu",
            InferenceProvider::CoreML => "coreml",
            InferenceProvider::Cuda => "cuda",
        }
    }

    pub fn from_gpu_setting(gpu_acceleration: bool) -> Self {
        if gpu_acceleration {
            #[cfg(target_os = "macos")]
            {
                InferenceProvider::CoreML
            }
            #[cfg(not(target_os = "macos"))]
            {
                InferenceProvider::Cuda
            }
        } else {
            InferenceProvider::Cpu
        }
    }
}

impl fmt::Display for InferenceProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

type EngineCacheKey = (EngineType, String, Option<String>, InferenceProvider);

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

const SENSEVOICE_REPO: &str = "csukuangfj/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17";
const WHISPER_BASE_REPO: &str = "csukuangfj/sherpa-onnx-whisper-base";
const WHISPER_SMALL_REPO: &str = "csukuangfj/sherpa-onnx-whisper-small";
const SILERO_VAD_REPO: &str = "onnx-community/silero-vad";
const SILERO_VAD_REMOTE_FILE: &str = "onnx/model.onnx";
const SILERO_VAD_LOCAL_FILE: &str = "silero_vad.onnx";

pub struct UnifiedEngineManager {
    models_dir: PathBuf,
    engine_cache: Arc<Mutex<Option<(EngineCacheKey, EngineInstance)>>>,
    provider: Arc<Mutex<InferenceProvider>>,
}

impl UnifiedEngineManager {
    pub fn new(models_dir: PathBuf) -> Self {
        info!(models_dir = ?models_dir, "engine_manager_initialized");
        Self {
            models_dir,
            engine_cache: Arc::new(Mutex::new(None)),
            provider: Arc::new(Mutex::new(InferenceProvider::Cpu)),
        }
    }

    pub fn set_provider(&self, gpu_acceleration: bool) {
        let new_provider = InferenceProvider::from_gpu_setting(gpu_acceleration);
        let mut provider = self.provider.lock().unwrap();
        if *provider != new_provider {
            info!(old = %*provider, new = %new_provider, "provider_updated");
            *provider = new_provider;
        }
    }

    pub fn default_models_dir() -> PathBuf {
        AppPaths::models_dir()
    }

    fn create_engine_instance(
        &self,
        engine_type: EngineType,
        version: &str,
        language: Option<&str>,
    ) -> Result<EngineInstance, String> {
        match engine_type {
            EngineType::Whisper | EngineType::SenseVoice => {
                let model_def = models::find_by_name(version)
                    .ok_or_else(|| format!("Unknown model: {}", version))?;
                if model_def.engine_type != engine_type {
                    return Err(format!(
                        "Model '{}' is for {:?}, not {:?}",
                        version, model_def.engine_type, engine_type
                    ));
                }
                let provider = *self.provider.lock().unwrap();
                let engine =
                    SherpaOnnxEngine::new(&self.models_dir, model_def, language, provider)?;
                Ok(EngineInstance::Local(engine))
            }
            EngineType::Cloud => {
                Err("Cloud STT uses streaming lifecycle, not batch transcription.".to_string())
            }
        }
    }

    pub(crate) fn get_or_create_engine(
        &self,
        engine_type: EngineType,
        version: &str,
        language: Option<&str>,
    ) -> Result<EngineInstance, String> {
        let provider = *self.provider.lock().unwrap();
        let cache_key = (
            engine_type,
            version.to_string(),
            language.map(|s| s.to_string()),
            provider,
        );
        let mut cache = self.engine_cache.lock().unwrap();

        if let Some((cached_key, cached_engine)) = cache.as_ref() {
            if *cached_key == cache_key {
                return Ok(cached_engine.clone());
            }
        }

        let engine = self.create_engine_instance(engine_type, version, language)?;
        *cache = Some((cache_key, engine.clone()));
        Ok(engine)
    }

    pub(crate) fn clear_cache(&self) {
        let mut cache = self.engine_cache.lock().unwrap();
        *cache = None;
    }

    #[instrument(
        skip(self, request),
        fields(
            engine = engine_type.as_str(),
            model = request.model_name.as_deref().unwrap_or("default"),
            language = request.language.as_deref().unwrap_or("auto"),
            samples = request.samples.len(),
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
        info!("transcription_started");

        let version =
            if request.model_name.as_deref() == Some("default") || request.model_name.is_none() {
                let default_model = models::default_for_language(&lang);
                default_model.name.to_string()
            } else {
                request
                    .model_name
                    .clone()
                    .unwrap_or_else(|| "default".to_string())
            };

        let lang_for_engine = if lang == "auto" {
            None
        } else {
            Some(lang.as_str())
        };
        let engine = self.get_or_create_engine(engine_type, &version, lang_for_engine)?;

        let result = engine.transcribe(request).await;

        match &result {
            Ok(r) => {
                info!(engine = engine_type.as_str(), model = %model, text_len = r.text.len(), duration_ms = r.total_ms, "transcription_completed")
            }
            Err(e) => {
                error!(engine = engine_type.as_str(), model = %model, error = %e, "transcription_failed")
            }
        }

        result
    }

    pub fn available_engines() -> Vec<EngineType> {
        EngineType::all()
    }

    // ==================== Model Management Functions ====================

    pub fn get_models(&self, engine_type: EngineType) -> Vec<ModelInfo> {
        models::ALL
            .iter()
            .filter(|m| m.engine_type == engine_type)
            .map(|def| {
                let downloaded = self.is_model_downloaded(engine_type, def.name);
                ModelInfo {
                    name: def.name.to_string(),
                    display_name: def.display_name.to_string(),
                    size_mb: def.size_mb as u64,
                    filename: def.name.to_string(),
                    downloaded,
                    speed_score: def.speed_score,
                    accuracy_score: def.accuracy_score,
                    engine: def.engine_type.as_str().to_string(),
                }
            })
            .collect()
    }

    pub fn get_all_models(&self) -> Vec<ModelInfo> {
        models::ALL
            .iter()
            .map(|def| {
                let downloaded = self.is_model_downloaded(def.engine_type, def.name);
                ModelInfo {
                    name: def.name.to_string(),
                    display_name: def.display_name.to_string(),
                    size_mb: def.size_mb as u64,
                    filename: def.name.to_string(),
                    downloaded,
                    speed_score: def.speed_score,
                    accuracy_score: def.accuracy_score,
                    engine: def.engine_type.as_str().to_string(),
                }
            })
            .collect()
    }

    pub fn get_engine_by_model_name(model_name: &str) -> Option<EngineType> {
        if model_name == "cloud" {
            Some(EngineType::Cloud)
        } else {
            models::find_by_name(model_name).map(|m| m.engine_type)
        }
    }

    pub fn is_model_downloaded(&self, engine_type: EngineType, model_name: &str) -> bool {
        let model_def = match models::find_by_name(model_name) {
            Some(def) => def,
            None => return false,
        };

        if model_def.engine_type != engine_type {
            return false;
        }

        let model_subdir = self.models_dir.join(model_def.name);
        model_def
            .files
            .iter()
            .all(|f| model_subdir.join(f.filename).exists())
    }

    pub fn resolve_available_model(
        &self,
        requested_model: &str,
        language: &str,
    ) -> (EngineType, String) {
        if let Some(model_def) = models::find_by_name(requested_model) {
            if self.is_model_downloaded(model_def.engine_type, model_def.name) {
                return (model_def.engine_type, model_def.name.to_string());
            }
        }

        let candidates = models::recommend_by_language(language);
        for model_def in &candidates {
            if self.is_model_downloaded(model_def.engine_type, model_def.name) {
                tracing::warn!(
                    requested = requested_model,
                    fallback = model_def.name,
                    "configured_model_not_downloaded_falling_back"
                );
                return (model_def.engine_type, model_def.name.to_string());
            }
        }

        for model_def in models::ALL {
            if self.is_model_downloaded(model_def.engine_type, model_def.name) {
                tracing::warn!(
                    requested = requested_model,
                    fallback = model_def.name,
                    "no_recommended_model_available_using_first_downloaded"
                );
                return (model_def.engine_type, model_def.name.to_string());
            }
        }

        let default = models::default_for_language(language);
        (default.engine_type, default.name.to_string())
    }

    pub fn get_model_path(&self, engine_type: EngineType, model_name: &str) -> PathBuf {
        if engine_type == EngineType::Cloud {
            return PathBuf::new();
        }
        self.models_dir.join(model_name)
    }

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
        let model_def = models::find_by_name(model_name)
            .ok_or_else(|| format!("Unknown model: {}", model_name))?;

        if model_def.engine_type != engine_type {
            return Err(format!(
                "Model '{}' is for {:?}, not {:?}",
                model_name, model_def.engine_type, engine_type
            ));
        }

        let repo = match engine_type {
            EngineType::SenseVoice => SENSEVOICE_REPO,
            EngineType::Whisper => {
                if model_name == "whisper-small" {
                    WHISPER_SMALL_REPO
                } else {
                    WHISPER_BASE_REPO
                }
            }
            EngineType::Cloud => {
                return Err("Cloud models do not need to be downloaded".to_string());
            }
        };

        let model_subdir = self.models_dir.join(model_name);
        std::fs::create_dir_all(&model_subdir)
            .map_err(|e| format!("Failed to create model directory: {}", e))?;

        let total_size_bytes: u64 = model_def
            .files
            .iter()
            .map(|f| u64::from(f.size_mb) * 1024 * 1024)
            .sum();

        let mut last_output_path: Option<PathBuf> = None;
        let progress_cb = Arc::new(progress_callback) as Arc<dyn Fn(u64, u64) + Send + Sync>;
        let completed_bytes = Arc::new(std::sync::atomic::AtomicU64::new(0u64));

        for model_file in model_def.files {
            let output_path = model_subdir.join(model_file.filename);

            if output_path.exists() {
                let file_bytes = u64::from(model_file.size_mb) * 1024 * 1024;
                completed_bytes.fetch_add(file_bytes, std::sync::atomic::Ordering::Relaxed);
                progress_cb(
                    completed_bytes.load(std::sync::atomic::Ordering::Relaxed),
                    total_size_bytes,
                );
                info!(file = %model_file.filename, "file_already_exists_skipping");
                last_output_path = Some(output_path);
                continue;
            }

            info!(
                repo = repo,
                file = %model_file.filename,
                "downloading_model_file"
            );

            let source = HuggingFaceSource::new(repo, model_file.filename).into_source();
            let urls = source.urls();

            let cb = progress_cb.clone();
            let completed_ref = completed_bytes.clone();
            let file_size_bytes = u64::from(model_file.size_mb) * 1024 * 1024;
            let options = DownloadOptions::new(&urls[0], &output_path)
                .with_fallbacks(urls[1..].to_vec())
                .with_cancel_flag(cancel_flag.clone())
                .with_progress_callback(Arc::new(move |downloaded, total| {
                    let file_total = if total > 0 { total } else { file_size_bytes };
                    let file_done = if file_total > 0 { downloaded } else { 0 };
                    let base = completed_ref.load(std::sync::atomic::Ordering::Relaxed);
                    cb(base + file_done, total_size_bytes);
                }))
                .with_model_name(model_name);

            let path = download(options).await?;
            completed_bytes.fetch_add(file_size_bytes, std::sync::atomic::Ordering::Relaxed);
            last_output_path = Some(path);
        }

        let output_path = last_output_path.unwrap_or(model_subdir);
        info!(engine = ?engine_type, model = %model_name, path = ?output_path, "model_download_completed");

        Ok(output_path)
    }

    #[instrument(skip(self), fields(engine = ?engine_type, model = %model_name), ret, err)]
    pub fn load_model(&self, engine_type: EngineType, model_name: &str) -> Result<(), String> {
        if !self.is_model_downloaded(engine_type, model_name) {
            return Err(format!(
                "Model '{}' not downloaded. Please download it first.",
                model_name
            ));
        }

        info!(
            engine = ?engine_type,
            model = %model_name,
            "model_preload_started"
        );

        let _ = self.get_or_create_engine(engine_type, model_name, None)?;

        info!(
            engine = ?engine_type,
            model = %model_name,
            "model_preloaded"
        );

        Ok(())
    }

    pub fn delete_model(&self, engine_type: EngineType, model_name: &str) -> Result<(), String> {
        let model_subdir = self.models_dir.join(model_name);

        if !model_subdir.exists() {
            return Err(format!(
                "Model '{}' not found at path: {}",
                model_name,
                model_subdir.display()
            ));
        }

        std::fs::remove_dir_all(&model_subdir)
            .map_err(|e| format!("Failed to delete model '{}': {}", model_name, e))?;

        info!(
            engine = ?engine_type,
            model = %model_name,
            path = ?model_subdir,
            "model_deleted"
        );

        Ok(())
    }

    pub fn recommend_by_language(&self, lang: &str) -> Vec<RecommendedModel> {
        let recommended = models::recommend_by_language(lang);

        recommended
            .into_iter()
            .map(|model_def| {
                let downloaded = self.is_model_downloaded(model_def.engine_type, model_def.name);
                RecommendedModel {
                    engine_type: model_def.engine_type,
                    model_name: model_def.name.to_string(),
                    display_name: model_def.display_name.to_string(),
                    size_mb: model_def.size_mb as u64,
                    speed_score: model_def.speed_score,
                    accuracy_score: model_def.accuracy_score,
                    downloaded,
                }
            })
            .collect()
    }

    pub fn vad_model_path(&self) -> PathBuf {
        self.models_dir.join(SILERO_VAD_LOCAL_FILE)
    }

    pub fn is_vad_model_downloaded(&self) -> bool {
        self.vad_model_path().exists()
    }

    pub async fn download_vad_model(
        &self,
        cancel_flag: Arc<AtomicBool>,
    ) -> Result<PathBuf, String> {
        let output_path = self.vad_model_path();

        if output_path.exists() {
            info!("vad_model_already_exists");
            return Ok(output_path);
        }

        info!(
            repo = SILERO_VAD_REPO,
            remote_file = SILERO_VAD_REMOTE_FILE,
            local_file = SILERO_VAD_LOCAL_FILE,
            "downloading_vad_model"
        );

        let source = HuggingFaceSource::new(SILERO_VAD_REPO, SILERO_VAD_REMOTE_FILE).into_source();
        let urls = source.urls();

        let options = DownloadOptions::new(&urls[0], &output_path)
            .with_fallbacks(urls[1..].to_vec())
            .with_cancel_flag(cancel_flag)
            .with_model_name("silero-vad");

        let path = download(options).await?;

        info!(path = ?path, "vad_model_downloaded");
        Ok(path)
    }

    pub async fn ensure_vad_model(&self) -> Result<PathBuf, String> {
        if self.is_vad_model_downloaded() {
            return Ok(self.vad_model_path());
        }

        info!("vad_model_missing_auto_downloading");
        self.download_vad_model(Arc::new(AtomicBool::new(false)))
            .await
    }

    pub async fn ensure_default_model(&self, language: &str) -> Result<(), String> {
        let default = models::default_for_language(language);
        let engine_type = default.engine_type;
        let model_name = default.name;

        if self.is_model_downloaded(engine_type, model_name) {
            info!(model = model_name, "default_model_already_downloaded");
            return Ok(());
        }

        info!(
            engine = ?engine_type,
            model = model_name,
            language = language,
            "default_model_missing_auto_downloading"
        );

        self.download_model(
            engine_type,
            model_name,
            std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            |_, _| {},
        )
        .await?;

        if let Err(e) = self.ensure_vad_model().await {
            tracing::warn!(error = %e, "vad_model_ensure_failed");
        }

        Ok(())
    }
}

#[derive(Clone)]
pub(crate) enum EngineInstance {
    Local(SherpaOnnxEngine),
}

impl EngineInstance {
    pub async fn transcribe(
        &self,
        request: TranscriptionRequest,
    ) -> Result<TranscriptionResult, String> {
        match self {
            EngineInstance::Local(engine) => engine.transcribe(request).await,
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
    fn test_model_definitions() {
        assert_eq!(models::ALL.len(), 3);
        assert_eq!(models::SENSE_VOICE_SMALL.name, "sense-voice-small");
        assert_eq!(models::WHISPER_BASE.name, "whisper-base");
        assert_eq!(models::WHISPER_SMALL.name, "whisper-small");
    }

    #[test]
    fn test_get_engine_by_model_name() {
        assert_eq!(
            UnifiedEngineManager::get_engine_by_model_name("sense-voice-small"),
            Some(EngineType::SenseVoice)
        );
        assert_eq!(
            UnifiedEngineManager::get_engine_by_model_name("whisper-base"),
            Some(EngineType::Whisper)
        );
        assert_eq!(
            UnifiedEngineManager::get_engine_by_model_name("whisper-small"),
            Some(EngineType::Whisper)
        );
        assert_eq!(
            UnifiedEngineManager::get_engine_by_model_name("cloud"),
            Some(EngineType::Cloud)
        );
        assert_eq!(
            UnifiedEngineManager::get_engine_by_model_name("unknown"),
            None
        );
    }

    #[test]
    fn test_recommend_by_language_zh() {
        let temp_dir = std::env::temp_dir().join("test_recommend_zh_v2");
        let _ = std::fs::create_dir_all(&temp_dir);
        let manager = UnifiedEngineManager::new(temp_dir.clone());

        let recommendations = manager.recommend_by_language("zh");
        assert!(!recommendations.is_empty());

        let has_sensevoice = recommendations
            .iter()
            .any(|r| r.engine_type == EngineType::SenseVoice);
        assert!(has_sensevoice, "Should recommend SenseVoice for Chinese");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_recommend_by_language_en() {
        let temp_dir = std::env::temp_dir().join("test_recommend_en_v2");
        let _ = std::fs::create_dir_all(&temp_dir);
        let manager = UnifiedEngineManager::new(temp_dir.clone());

        let recommendations = manager.recommend_by_language("en");
        assert!(!recommendations.is_empty());

        // English is SenseVoice-preferred (zh/yue/ja/ko/en)
        let has_sensevoice = recommendations
            .iter()
            .any(|r| r.engine_type == EngineType::SenseVoice);
        assert!(has_sensevoice, "Should recommend SenseVoice for English");

        let _ = std::fs::remove_dir_all(&temp_dir);
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
        let temp_dir = std::env::temp_dir().join("test_load_model_v2");
        let _ = std::fs::create_dir_all(&temp_dir);
        let manager = UnifiedEngineManager::new(temp_dir.clone());

        let result = manager.load_model(EngineType::Whisper, "whisper-base");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not downloaded"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_delete_model_not_exists() {
        let temp_dir = std::env::temp_dir().join("test_delete_model_v2");
        let _ = std::fs::create_dir_all(&temp_dir);
        let manager = UnifiedEngineManager::new(temp_dir.clone());

        let result = manager.delete_model(EngineType::Whisper, "whisper-base");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_resolve_available_model_falls_back_to_downloaded() {
        let temp_dir = std::env::temp_dir().join("test_resolve_model_fallback");
        let _ = std::fs::create_dir_all(&temp_dir);

        let sensevoice_dir = temp_dir.join("sense-voice-small");
        let _ = std::fs::create_dir_all(&sensevoice_dir);
        std::fs::File::create(sensevoice_dir.join("model.int8.onnx")).unwrap();
        std::fs::File::create(sensevoice_dir.join("tokens.txt")).unwrap();

        let manager = UnifiedEngineManager::new(temp_dir.clone());

        let (engine_type, model_name) = manager.resolve_available_model("whisper-base", "zh");
        assert_eq!(model_name, "sense-voice-small");
        assert_eq!(engine_type, EngineType::SenseVoice);

        let (engine_type2, model_name2) =
            manager.resolve_available_model("sense-voice-small", "zh");
        assert_eq!(model_name2, "sense-voice-small");
        assert_eq!(engine_type2, EngineType::SenseVoice);

        let (engine_type3, model_name3) =
            manager.resolve_available_model("nonexistent-model", "auto");
        assert_eq!(model_name3, "sense-voice-small");
        assert_eq!(engine_type3, EngineType::SenseVoice);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_resolve_available_model_nothing_downloaded() {
        let temp_dir = std::env::temp_dir().join("test_resolve_model_nothing");
        let _ = std::fs::create_dir_all(&temp_dir);
        let manager = UnifiedEngineManager::new(temp_dir.clone());

        let (_, model_name) = manager.resolve_available_model("whisper-base", "auto");
        assert_eq!(model_name, models::DEFAULT.name);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
