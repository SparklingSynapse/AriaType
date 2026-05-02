use crate::polish_engine::common::{polish_text_blocking, EngineConfig, PromptFormat};
use crate::polish_engine::traits::{PolishEngine, PolishEngineType, PolishRequest, PolishResult};
use crate::utils::AppPaths;
use async_trait::async_trait;
use std::path::PathBuf;

pub struct LfmPolishEngine;

impl LfmPolishEngine {
    pub fn new() -> Self {
        Self
    }

    fn get_model_path(model_name: &str) -> PathBuf {
        AppPaths::models_dir().join(model_name)
    }
}

impl Default for LfmPolishEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PolishEngine for LfmPolishEngine {
    fn engine_type(&self) -> PolishEngineType {
        PolishEngineType::Lfm
    }

    async fn polish(&self, request: PolishRequest) -> Result<PolishResult, String> {
        let model_name = request.model_name.clone().ok_or("Model name required")?;
        let model_path = Self::get_model_path(&model_name);

        if !model_path.exists() {
            return Err(format!("Model not found: {}", model_name));
        }

        let text = request.text.clone();
        let system_prompt = request.system_context.effective_prompt();
        let language = request.language.clone();
        let default_prompt = super::DEFAULT_POLISH_PROMPT.to_string();

        let config = EngineConfig {
            log_prefix: "polish:lfm",
            strip_think_tags: false,
            prompt_format: PromptFormat::ChatMl,
            // LFM2.5-1.2B Q4_K_M ≈ 697MB, LFM2-2.6B Q4_K_M ≈ 1.6GB
            min_model_size_mb: 600,
        };

        let t0 = std::time::Instant::now();

        // Run blocking polish in a separate thread
        let result = tokio::task::spawn_blocking(move || {
            polish_text_blocking(
                &text,
                &system_prompt,
                &language,
                &model_path,
                &default_prompt,
                &config,
            )
        })
        .await
        .map_err(|e| format!("Task join error: {}", e))??;

        let total_ms = t0.elapsed().as_millis() as u64;

        Ok(PolishResult::new(result, PolishEngineType::Lfm, total_ms))
    }
}
