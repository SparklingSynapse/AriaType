use crate::polish_engine::traits::{PolishEngine, PolishEngineType, PolishRequest, PolishResult};
use crate::polish_engine::{cloud::CloudPolishEngine, gemma, lfm, qwen};
use crate::utils::AppPaths;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::{debug, info, instrument};

type EngineCacheKey = (PolishEngineType, String);

/// Unified manager for all polish engines with instance caching
pub struct UnifiedPolishManager {
    engines: HashMap<PolishEngineType, Arc<dyn PolishEngine>>,
    /// Cache for engine instances (engine_type, model_filename) -> instance
    engine_cache: Arc<Mutex<HashMap<EngineCacheKey, Arc<PolishEngineInstance>>>>,
}

impl UnifiedPolishManager {
    pub fn new() -> Self {
        let mut engines: HashMap<PolishEngineType, Arc<dyn PolishEngine>> = HashMap::new();

        // Register Qwen engine
        engines.insert(
            PolishEngineType::Qwen,
            Arc::new(qwen::QwenPolishEngine::new()),
        );

        // Register LFM engine
        engines.insert(PolishEngineType::Lfm, Arc::new(lfm::LfmPolishEngine::new()));

        // Register Gemma engine
        engines.insert(
            PolishEngineType::Gemma,
            Arc::new(gemma::GemmaPolishEngine::new()),
        );

        info!(engine_count = engines.len(), "polish_manager_initialized");
        Self {
            engines,
            engine_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Auto-detect engine type from model ID
    pub fn get_engine_by_model_id(model_id: &str) -> Option<PolishEngineType> {
        if qwen::is_qwen_model(model_id) {
            Some(PolishEngineType::Qwen)
        } else if lfm::is_lfm_model(model_id) {
            Some(PolishEngineType::Lfm)
        } else if gemma::is_gemma_model(model_id) {
            Some(PolishEngineType::Gemma)
        } else {
            None
        }
    }

    /// Get or create cached engine instance
    fn get_or_create_engine_instance(
        &self,
        engine_type: PolishEngineType,
        model_filename: &str,
    ) -> Result<Arc<PolishEngineInstance>, String> {
        let cache_key = (engine_type, model_filename.to_string());
        let mut cache = self.engine_cache.lock().unwrap();

        if let Some(cached_instance) = cache.get(&cache_key) {
            info!(
                engine = ?engine_type,
                model = %model_filename,
                "polish_engine_cache_hit"
            );
            return Ok(Arc::clone(cached_instance));
        }

        info!(
            engine = ?engine_type,
            model = %model_filename,
            "polish_engine_cache_miss"
        );
        let instance = Arc::new(PolishEngineInstance::new(engine_type, model_filename)?);
        cache.insert(cache_key, Arc::clone(&instance));
        Ok(instance)
    }

    /// Clear engine cache
    pub fn clear_cache(&self) {
        let mut cache = self.engine_cache.lock().unwrap();
        cache.clear();
        info!("polish_engine_cache_cleared");
    }

    /// Clear specific engine cache
    pub fn clear_engine_cache(&self, engine_type: PolishEngineType, model_filename: Option<&str>) {
        let mut cache = self.engine_cache.lock().unwrap();

        if let Some(filename) = model_filename {
            // Clear specific model
            let cache_key = (engine_type, filename.to_string());
            if cache.remove(&cache_key).is_some() {
                info!(
                    engine = ?engine_type,
                    model = %filename,
                    "polish_engine_cache_entry_cleared"
                );
            }
        } else {
            // Clear all models for this engine type
            let keys_to_remove: Vec<_> = cache
                .keys()
                .filter(|(et, _)| *et == engine_type)
                .cloned()
                .collect();

            for key in keys_to_remove {
                cache.remove(&key);
            }

            info!(
                engine = ?engine_type,
                "polish_engine_cache_type_cleared"
            );
        }
    }

    /// Load model into cache (for preloading)
    pub fn load_model(&self, engine_type: PolishEngineType, model_id: &str) -> Result<(), String> {
        let model_filename = self
            .get_model_filename(engine_type, model_id)
            .ok_or_else(|| format!("Model not found: {}", model_id))?;

        // Check if model file exists
        let model_path = AppPaths::models_dir().join(&model_filename);
        if !model_path.exists() {
            return Err(format!("Model file not found: {}", model_filename));
        }

        // Create instance (will be cached)
        self.get_or_create_engine_instance(engine_type, &model_filename)?;
        info!(
            engine = ?engine_type,
            model = %model_id,
            "polish_model_preloaded"
        );
        Ok(())
    }

    /// Unload model from cache
    pub fn unload_model(&self, engine_type: PolishEngineType, model_id: &str) {
        if let Some(model_filename) = self.get_model_filename(engine_type, model_id) {
            self.clear_engine_cache(engine_type, Some(&model_filename));
            info!(
                engine = ?engine_type,
                model = %model_id,
                "polish_model_unloaded"
            );
        }
    }

    /// Polish text using specified engine with caching support
    #[instrument(skip(self, request), fields(engine = ?engine_type))]
    pub async fn polish(
        &self,
        engine_type: PolishEngineType,
        request: PolishRequest,
    ) -> Result<PolishResult, String> {
        let engine = self
            .engines
            .get(&engine_type)
            .ok_or_else(|| format!("Engine not found: {:?}", engine_type))?;

        debug!(engine = ?engine_type, "polish_operation_start");

        // If model_name is provided, use cached instance
        if let Some(ref model_filename) = request.model_name {
            let _instance = self.get_or_create_engine_instance(engine_type, model_filename)?;
            // Instance is now cached, proceed with polish
        }

        engine.polish(request).await
    }

    /// Check if a model is downloaded for a specific engine
    pub fn is_model_downloaded(&self, engine_type: PolishEngineType, model_id: &str) -> bool {
        match engine_type {
            PolishEngineType::Qwen => {
                if let Some(model) = qwen::QwenModelDef::from_id(model_id) {
                    let path = AppPaths::models_dir().join(model.filename);
                    path.exists()
                } else {
                    false
                }
            }
            PolishEngineType::Lfm => {
                if let Some(model) = lfm::LfmModelDef::from_id(model_id) {
                    let path = AppPaths::models_dir().join(model.filename);
                    path.exists()
                } else {
                    false
                }
            }
            PolishEngineType::Gemma => {
                if let Some(model) = gemma::GemmaModelDef::from_id(model_id) {
                    let path = AppPaths::models_dir().join(model.filename);
                    path.exists()
                } else {
                    false
                }
            }
            PolishEngineType::Cloud => {
                // Cloud engine doesn't have local models
                false
            }
        }
    }

    /// Get model filename for a specific engine and model ID
    pub fn get_model_filename(
        &self,
        engine_type: PolishEngineType,
        model_id: &str,
    ) -> Option<String> {
        match engine_type {
            PolishEngineType::Qwen => {
                qwen::QwenModelDef::from_id(model_id).map(|m| m.filename.to_string())
            }
            PolishEngineType::Lfm => {
                lfm::LfmModelDef::from_id(model_id).map(|m| m.filename.to_string())
            }
            PolishEngineType::Gemma => {
                gemma::GemmaModelDef::from_id(model_id).map(|m| m.filename.to_string())
            }
            PolishEngineType::Cloud => {
                // Cloud engine uses the model ID as the model name directly
                Some(model_id.to_string())
            }
        }
    }

    /// Get all available engines
    pub fn available_engines(&self) -> Vec<PolishEngineType> {
        self.engines.keys().copied().collect()
    }

    /// Polish using cloud provider
    #[instrument(skip(self, request, api_key), fields(provider = %provider_type, model = %model))]
    pub async fn polish_cloud(
        &self,
        request: PolishRequest,
        provider_type: &str,
        api_key: &str,
        base_url: &str,
        model: &str,
        enable_thinking: bool,
    ) -> Result<PolishResult, String> {
        let config = crate::polish_engine::cloud::engine::CloudProviderConfig {
            provider_type: provider_type.to_string(),
            api_key: api_key.to_string(),
            base_url: base_url.to_string(),
            model: model.to_string(),
            enable_thinking,
        };
        let engine = CloudPolishEngine::new(config);
        engine.polish(request).await
    }
}

impl Default for UnifiedPolishManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Polish engine instance wrapper for caching
#[derive(Debug)]
pub(crate) struct PolishEngineInstance {}

impl PolishEngineInstance {
    fn new(_engine_type: PolishEngineType, model_filename: &str) -> Result<Self, String> {
        // Verify model file exists
        let model_path = AppPaths::models_dir().join(model_filename);
        if !model_path.exists() {
            return Err(format!("Model file not found: {}", model_filename));
        }

        Ok(Self {})
    }
}

/// Model information for UI display
#[derive(Debug, Clone)]
pub struct PolishModelInfo {
    pub id: String,
    pub display_name: String,
    pub size_display: String,
    pub engine_type: PolishEngineType,
}

/// Get all available polish models across all engines
pub fn get_all_polish_models() -> Vec<PolishModelInfo> {
    let mut models = Vec::new();

    // Add Qwen models
    for model in qwen::get_all_models() {
        models.push(PolishModelInfo {
            id: model.id.to_string(),
            display_name: model.display_name.to_string(),
            size_display: model.size_display.to_string(),
            engine_type: PolishEngineType::Qwen,
        });
    }

    // Add LFM models
    for model in lfm::get_all_models() {
        models.push(PolishModelInfo {
            id: model.id.to_string(),
            display_name: model.display_name.to_string(),
            size_display: model.size_display.to_string(),
            engine_type: PolishEngineType::Lfm,
        });
    }

    // Add Gemma models
    for model in gemma::get_all_models() {
        models.push(PolishModelInfo {
            id: model.id.to_string(),
            display_name: model.display_name.to_string(),
            size_display: model.size_display.to_string(),
            engine_type: PolishEngineType::Gemma,
        });
    }

    models
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unified_polish_manager_new() {
        let manager = UnifiedPolishManager::new();
        let engines = manager.available_engines();
        assert_eq!(engines.len(), 3);
        assert!(engines.contains(&PolishEngineType::Qwen));
        assert!(engines.contains(&PolishEngineType::Lfm));
        assert!(engines.contains(&PolishEngineType::Gemma));
    }

    #[test]
    fn test_unified_polish_manager_default() {
        let manager = UnifiedPolishManager::default();
        let engines = manager.available_engines();
        assert_eq!(engines.len(), 3);
    }

    #[test]
    fn test_cloud_polish_engine_type_available() {
        assert!(PolishEngineType::all().contains(&PolishEngineType::Cloud));
    }

    #[test]
    fn test_get_engine_by_model_id_qwen() {
        let engine = UnifiedPolishManager::get_engine_by_model_id("qwen3.5-0.8b");
        assert_eq!(engine, Some(PolishEngineType::Qwen));

        let engine = UnifiedPolishManager::get_engine_by_model_id("qwen3.5-2b");
        assert_eq!(engine, Some(PolishEngineType::Qwen));

        let engine = UnifiedPolishManager::get_engine_by_model_id("qwen3-4b");
        assert_eq!(engine, Some(PolishEngineType::Qwen));
    }

    #[test]
    fn test_get_engine_by_model_id_lfm() {
        let engine = UnifiedPolishManager::get_engine_by_model_id("lfm2.5-1.2b");
        assert_eq!(engine, Some(PolishEngineType::Lfm));

        let engine = UnifiedPolishManager::get_engine_by_model_id("lfm2-2.6b");
        assert_eq!(engine, Some(PolishEngineType::Lfm));
    }

    #[test]
    fn test_get_engine_by_model_id_unknown() {
        let engine = UnifiedPolishManager::get_engine_by_model_id("gpt-4");
        assert_eq!(engine, None);

        let engine = UnifiedPolishManager::get_engine_by_model_id("unknown");
        assert_eq!(engine, None);
    }

    #[test]
    fn test_get_engine_by_model_id_gemma() {
        let engine = UnifiedPolishManager::get_engine_by_model_id("gemma-2b-it");
        assert_eq!(engine, Some(PolishEngineType::Gemma));

        let engine = UnifiedPolishManager::get_engine_by_model_id("gemma-4-e2b");
        assert_eq!(engine, Some(PolishEngineType::Gemma));
    }

    #[test]
    fn test_get_model_filename_qwen() {
        let manager = UnifiedPolishManager::new();
        let filename = manager.get_model_filename(PolishEngineType::Qwen, "qwen3.5-0.8b");
        assert_eq!(filename, Some("Qwen3.5-0.8B-Q5_K_M.gguf".to_string()));
    }

    #[test]
    fn test_get_model_filename_lfm() {
        let manager = UnifiedPolishManager::new();
        let filename = manager.get_model_filename(PolishEngineType::Lfm, "lfm2.5-1.2b");
        assert_eq!(
            filename,
            Some("LFM2.5-1.2B-Instruct-Q4_K_M.gguf".to_string())
        );
    }

    #[test]
    fn test_get_model_filename_not_found() {
        let manager = UnifiedPolishManager::new();
        let filename = manager.get_model_filename(PolishEngineType::Qwen, "nonexistent");
        assert_eq!(filename, None);
    }

    #[test]
    fn test_get_model_filename_gemma() {
        let manager = UnifiedPolishManager::new();
        let filename = manager.get_model_filename(PolishEngineType::Gemma, "gemma-2b-it");
        assert_eq!(filename, Some("gemma-2b-it.Q4_K_M.gguf".to_string()));

        let legacy_filename =
            manager.get_model_filename(PolishEngineType::Gemma, "gemma-4-e2b");
        assert_eq!(
            legacy_filename,
            Some("gemma-2b-it.Q4_K_M.gguf".to_string())
        );
    }

    #[test]
    fn test_clear_cache() {
        let manager = UnifiedPolishManager::new();
        manager.clear_cache();
        // Should not panic
    }

    #[test]
    fn test_clear_engine_cache() {
        let manager = UnifiedPolishManager::new();
        manager.clear_engine_cache(PolishEngineType::Qwen, None);
        manager.clear_engine_cache(PolishEngineType::Qwen, Some("model.gguf"));
        // Should not panic
    }

    #[test]
    fn test_polish_model_info() {
        let info = PolishModelInfo {
            id: "test-id".to_string(),
            display_name: "Test Model".to_string(),
            size_display: "~1GB".to_string(),
            engine_type: PolishEngineType::Qwen,
        };

        assert_eq!(info.id, "test-id");
        assert_eq!(info.display_name, "Test Model");
        assert_eq!(info.size_display, "~1GB");
        assert_eq!(info.engine_type, PolishEngineType::Qwen);
    }

    #[test]
    fn test_get_all_polish_models() {
        let models = get_all_polish_models();
        assert!(!models.is_empty());
        assert!(models.len() >= 6); // At least 3 Qwen + 2 LFM + 1 Gemma models

        // Check that we have all engine types
        let has_qwen = models
            .iter()
            .any(|m| m.engine_type == PolishEngineType::Qwen);
        let has_lfm = models
            .iter()
            .any(|m| m.engine_type == PolishEngineType::Lfm);
        let has_gemma = models
            .iter()
            .any(|m| m.engine_type == PolishEngineType::Gemma);
        assert!(has_qwen);
        assert!(has_lfm);
        assert!(has_gemma);

        // Check that all models have valid fields
        for model in models {
            assert!(!model.id.is_empty());
            assert!(!model.display_name.is_empty());
            assert!(!model.size_display.is_empty());
        }
    }

    #[test]
    fn test_polish_engine_instance_new_invalid_path() {
        let result = PolishEngineInstance::new(PolishEngineType::Qwen, "nonexistent-model.gguf");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Model file not found"));
    }
}
