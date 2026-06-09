use crate::polish_engine::local_http::{polish_via_local_http, LocalHttpPolishConfig};
use crate::polish_engine::traits::{PolishEngine, PolishEngineType, PolishRequest, PolishResult};
use async_trait::async_trait;

pub struct QwenPolishEngine;

impl QwenPolishEngine {
    pub fn new() -> Self {
        Self
    }

    fn model_alias(model_name: &str) -> String {
        super::QwenModelDef::from_filename(model_name)
            .map(|model| model.id.to_string())
            .unwrap_or_else(|| model_name.to_string())
    }
}

impl Default for QwenPolishEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PolishEngine for QwenPolishEngine {
    fn engine_type(&self) -> PolishEngineType {
        PolishEngineType::Qwen
    }

    async fn polish(&self, request: PolishRequest) -> Result<PolishResult, String> {
        let model_name = request.model_name.clone().ok_or("Model name required")?;
        let config = LocalHttpPolishConfig {
            engine_type: PolishEngineType::Qwen,
            engine_label: "polish:qwen",
            model_alias: Self::model_alias(&model_name),
            model_filename: model_name,
            min_model_size_mb: 400,
            no_think_directive: true,
        };

        polish_via_local_http(request, config).await
    }
}
