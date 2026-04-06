mod cloud;
mod common;
pub mod gemma;
pub mod lfm;
pub mod qwen;
mod templates;
mod traits;
mod unified_manager;

pub use cloud::{CloudPolishEngine, CloudProviderConfig};
pub use gemma::{GemmaModelDef, DEFAULT_POLISH_PROMPT as GEMMA_DEFAULT_PROMPT};
pub use lfm::{LfmModelDef, DEFAULT_POLISH_PROMPT as LFM_DEFAULT_PROMPT};
pub use qwen::{QwenModelDef, DEFAULT_POLISH_PROMPT as QWEN_DEFAULT_PROMPT};
pub use templates::{get_all_templates, get_template_by_id, PolishTemplate, POLISH_TEMPLATES};
pub use traits::{PolishEngine, PolishEngineType, PolishRequest, PolishResult};
pub use unified_manager::{get_all_polish_models, PolishModelInfo, UnifiedPolishManager};

// Use Qwen's default prompt as the global default
pub const DEFAULT_POLISH_PROMPT: &str = QWEN_DEFAULT_PROMPT;

/// Legacy PolishModel enum for backward compatibility
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum PolishModel {
    Qwen3_5_0_8B,
    LFM2_5_1_2B,
    Qwen3_5_2B,
    LFM2_2_6B,
    Qwen3_4B,
    Gemma2B_IT,
}

impl PolishModel {
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "qwen3.5-0.8b" => Some(Self::Qwen3_5_0_8B),
            "lfm2.5-1.2b" => Some(Self::LFM2_5_1_2B),
            "qwen3.5-2b" => Some(Self::Qwen3_5_2B),
            "lfm2-2.6b" => Some(Self::LFM2_2_6B),
            "qwen3-4b" => Some(Self::Qwen3_4B),
            "gemma-2b-it" | "gemma-4-e2b" => Some(Self::Gemma2B_IT),
            _ => None,
        }
    }

    pub fn id(&self) -> &'static str {
        match self {
            Self::Qwen3_5_0_8B => "qwen3.5-0.8b",
            Self::LFM2_5_1_2B => "lfm2.5-1.2b",
            Self::Qwen3_5_2B => "qwen3.5-2b",
            Self::LFM2_2_6B => "lfm2-2.6b",
            Self::Qwen3_4B => "qwen3-4b",
            Self::Gemma2B_IT => "gemma-2b-it",
        }
    }

    pub fn filename(&self) -> &'static str {
        match self {
            Self::Qwen3_5_0_8B => QwenModelDef::from_id("qwen3.5-0.8b")
                .map(|m| m.filename)
                .unwrap_or(""),
            Self::LFM2_5_1_2B => LfmModelDef::from_id("lfm2.5-1.2b")
                .map(|m| m.filename)
                .unwrap_or(""),
            Self::Qwen3_5_2B => QwenModelDef::from_id("qwen3.5-2b")
                .map(|m| m.filename)
                .unwrap_or(""),
            Self::LFM2_2_6B => LfmModelDef::from_id("lfm2-2.6b")
                .map(|m| m.filename)
                .unwrap_or(""),
            Self::Qwen3_4B => QwenModelDef::from_id("qwen3-4b")
                .map(|m| m.filename)
                .unwrap_or(""),
            Self::Gemma2B_IT => gemma::GemmaModelDef::from_id("gemma-2b-it")
                .map(|m| m.filename)
                .unwrap_or(""),
        }
    }

    pub fn urls(&self) -> Vec<String> {
        match self {
            Self::Qwen3_5_0_8B => QwenModelDef::from_id("qwen3.5-0.8b")
                .map(|m| m.urls())
                .unwrap_or_default(),
            Self::LFM2_5_1_2B => LfmModelDef::from_id("lfm2.5-1.2b")
                .map(|m| m.urls())
                .unwrap_or_default(),
            Self::Qwen3_5_2B => QwenModelDef::from_id("qwen3.5-2b")
                .map(|m| m.urls())
                .unwrap_or_default(),
            Self::LFM2_2_6B => LfmModelDef::from_id("lfm2-2.6b")
                .map(|m| m.urls())
                .unwrap_or_default(),
            Self::Qwen3_4B => QwenModelDef::from_id("qwen3-4b")
                .map(|m| m.urls())
                .unwrap_or_default(),
            Self::Gemma2B_IT => gemma::GemmaModelDef::from_id("gemma-2b-it")
                .map(|m| m.urls())
                .unwrap_or_default(),
        }
    }
}

// Re-export for backward compatibility
pub fn get_all_models() -> Vec<(String, String, String)> {
    get_all_polish_models()
        .into_iter()
        .map(|m| (m.id, m.display_name, m.size_display))
        .collect()
}

pub fn get_polish_model_path_for(model: PolishModel) -> std::path::PathBuf {
    crate::utils::AppPaths::models_dir().join(model.filename())
}

pub fn is_polish_model_downloaded_for(model: PolishModel) -> bool {
    let path = get_polish_model_path_for(model);
    path.exists()
}

pub fn is_polish_model_downloaded() -> bool {
    // Default to checking the first model (Qwen3.5-0.8B)
    is_polish_model_downloaded_for(PolishModel::Qwen3_5_0_8B)
}

pub fn get_current_model() -> PolishModel {
    // Default to the first model
    PolishModel::Qwen3_5_0_8B
}
