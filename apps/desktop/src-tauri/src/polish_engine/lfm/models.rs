use crate::utils::HuggingFaceSource;

/// LFM model definitions
const LFM_MODELS: &[(&str, &str, &str, &str, &str)] = &[
    (
        "lfm2.5-1.2b",
        "LiquidAI/LFM2.5-1.2B-Instruct-GGUF",
        "LFM2.5-1.2B-Instruct-Q4_K_M.gguf",
        "LFM2.5-1.2B",
        "~770MB",
    ),
    (
        "lfm2-2.6b",
        "LiquidAI/LFM2-2.6B-GGUF",
        "LFM2-2.6B-Q4_K_M.gguf",
        "LFM2-2.6B",
        "~1.6GB",
    ),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LfmModelDef {
    pub id: &'static str,
    pub repo: &'static str,
    pub filename: &'static str,
    pub display_name: &'static str,
    pub size_display: &'static str,
}

impl LfmModelDef {
    pub fn from_id(id: &str) -> Option<Self> {
        LFM_MODELS
            .iter()
            .find(|(model_id, _, _, _, _)| *model_id == id)
            .map(|(id, repo, filename, display_name, size_display)| Self {
                id,
                repo,
                filename,
                display_name,
                size_display,
            })
    }

    pub fn from_filename(filename: &str) -> Option<Self> {
        LFM_MODELS
            .iter()
            .find(|(_, _, fname, _, _)| *fname == filename)
            .map(|(id, repo, filename, display_name, size_display)| Self {
                id,
                repo,
                filename,
                display_name,
                size_display,
            })
    }

    pub fn urls(&self) -> Vec<String> {
        HuggingFaceSource::new(self.repo, self.filename)
            .into_source()
            .urls()
    }
}

pub fn get_all_models() -> Vec<LfmModelDef> {
    LFM_MODELS
        .iter()
        .map(
            |(id, repo, filename, display_name, size_display)| LfmModelDef {
                id,
                repo,
                filename,
                display_name,
                size_display,
            },
        )
        .collect()
}

/// Check if a model ID belongs to LFM engine
pub fn is_lfm_model(model_id: &str) -> bool {
    model_id.starts_with("lfm")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lfm_model_def_from_id() {
        let model = LfmModelDef::from_id("lfm2.5-1.2b");
        assert!(model.is_some());
        let model = model.unwrap();
        assert_eq!(model.id, "lfm2.5-1.2b");
        assert_eq!(model.display_name, "LFM2.5-1.2B");
        assert_eq!(model.filename, "LFM2.5-1.2B-Instruct-Q4_K_M.gguf");
        assert_eq!(model.repo, "LiquidAI/LFM2.5-1.2B-Instruct-GGUF");
        assert_eq!(model.size_display, "~770MB");
    }

    #[test]
    fn test_lfm_model_def_from_id_not_found() {
        let model = LfmModelDef::from_id("nonexistent");
        assert!(model.is_none());
    }

    #[test]
    fn test_lfm_model_def_from_filename() {
        let model = LfmModelDef::from_filename("LFM2.5-1.2B-Instruct-Q4_K_M.gguf");
        assert!(model.is_some());
        let model = model.unwrap();
        assert_eq!(model.id, "lfm2.5-1.2b");
    }

    #[test]
    fn test_lfm_model_def_from_filename_not_found() {
        let model = LfmModelDef::from_filename("nonexistent.gguf");
        assert!(model.is_none());
    }

    #[test]
    fn test_lfm_model_def_urls() {
        let model = LfmModelDef::from_id("lfm2.5-1.2b").unwrap();
        let urls = model.urls();
        assert!(!urls.is_empty());
        // URLs should contain the filename
        assert!(urls
            .iter()
            .any(|url| url.contains("LFM2.5-1.2B-Instruct-Q4_K_M.gguf")));
    }

    #[test]
    fn test_get_all_models() {
        let models = get_all_models();
        assert!(!models.is_empty());
        assert_eq!(models.len(), 2); // lfm2.5-1.2b, lfm2-2.6b

        // Check that all expected models are present
        let ids: Vec<&str> = models.iter().map(|m| m.id).collect();
        assert!(ids.contains(&"lfm2.5-1.2b"));
        assert!(ids.contains(&"lfm2-2.6b"));
    }

    #[test]
    fn test_is_lfm_model() {
        assert!(is_lfm_model("lfm2.5-1.2b"));
        assert!(is_lfm_model("lfm2-2.6b"));
        assert!(is_lfm_model("lfm-anything"));

        assert!(!is_lfm_model("qwen3.5-0.8b"));
        assert!(!is_lfm_model("gpt-4"));
        assert!(!is_lfm_model(""));
    }

    #[test]
    fn test_all_models_have_valid_fields() {
        for model in get_all_models() {
            assert!(!model.id.is_empty());
            assert!(!model.repo.is_empty());
            assert!(!model.filename.is_empty());
            assert!(!model.display_name.is_empty());
            assert!(!model.size_display.is_empty());
            assert!(model.filename.ends_with(".gguf"));
        }
    }
}
