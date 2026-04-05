use crate::utils::HuggingFaceSource;

/// Qwen model definitions
const QWEN_MODELS: &[(&str, &str, &str, &str, &str)] = &[
    (
        "qwen3.5-0.8b",
        "unsloth/Qwen3.5-0.8B-GGUF",
        "Qwen3.5-0.8B-Q5_K_M.gguf",
        "Qwen3.5-0.8B",
        "~600MB",
    ),
    (
        "qwen3.5-2b",
        "unsloth/Qwen3.5-2B-GGUF",
        "Qwen3.5-2B-Q5_K_M.gguf",
        "Qwen3.5-2B",
        "~1.4GB",
    ),
    (
        "qwen3-4b",
        "Qwen/Qwen3-4B-GGUF",
        "Qwen3-4B-Q4_K_M.gguf",
        "Qwen3-4B",
        "~2.6GB",
    ),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QwenModelDef {
    pub id: &'static str,
    pub repo: &'static str,
    pub filename: &'static str,
    pub display_name: &'static str,
    pub size_display: &'static str,
}

impl QwenModelDef {
    pub fn from_id(id: &str) -> Option<Self> {
        QWEN_MODELS
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
        QWEN_MODELS
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

pub fn get_all_models() -> Vec<QwenModelDef> {
    QWEN_MODELS
        .iter()
        .map(
            |(id, repo, filename, display_name, size_display)| QwenModelDef {
                id,
                repo,
                filename,
                display_name,
                size_display,
            },
        )
        .collect()
}

/// Check if a model ID belongs to Qwen engine
pub fn is_qwen_model(model_id: &str) -> bool {
    model_id.starts_with("qwen")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qwen_model_def_from_id() {
        let model = QwenModelDef::from_id("qwen3.5-0.8b");
        assert!(model.is_some());
        let model = model.unwrap();
        assert_eq!(model.id, "qwen3.5-0.8b");
        assert_eq!(model.display_name, "Qwen3.5-0.8B");
        assert_eq!(model.filename, "Qwen3.5-0.8B-Q5_K_M.gguf");
        assert_eq!(model.repo, "unsloth/Qwen3.5-0.8B-GGUF");
        assert_eq!(model.size_display, "~600MB");
    }

    #[test]
    fn test_qwen_model_def_from_id_not_found() {
        let model = QwenModelDef::from_id("nonexistent");
        assert!(model.is_none());
    }

    #[test]
    fn test_qwen_model_def_from_filename() {
        let model = QwenModelDef::from_filename("Qwen3.5-0.8B-Q5_K_M.gguf");
        assert!(model.is_some());
        let model = model.unwrap();
        assert_eq!(model.id, "qwen3.5-0.8b");
    }

    #[test]
    fn test_qwen_model_def_from_filename_not_found() {
        let model = QwenModelDef::from_filename("nonexistent.gguf");
        assert!(model.is_none());
    }

    #[test]
    fn test_qwen_model_def_urls() {
        let model = QwenModelDef::from_id("qwen3.5-0.8b").unwrap();
        let urls = model.urls();
        assert!(!urls.is_empty());
        // URLs should contain the filename
        assert!(urls
            .iter()
            .any(|url| url.contains("Qwen3.5-0.8B-Q5_K_M.gguf")));
    }

    #[test]
    fn test_get_all_models() {
        let models = get_all_models();
        assert!(!models.is_empty());
        assert_eq!(models.len(), 3); // qwen3.5-0.8b, qwen3.5-2b, qwen3-4b

        // Check that all expected models are present
        let ids: Vec<&str> = models.iter().map(|m| m.id).collect();
        assert!(ids.contains(&"qwen3.5-0.8b"));
        assert!(ids.contains(&"qwen3.5-2b"));
        assert!(ids.contains(&"qwen3-4b"));
    }

    #[test]
    fn test_is_qwen_model() {
        assert!(is_qwen_model("qwen3.5-0.8b"));
        assert!(is_qwen_model("qwen3.5-2b"));
        assert!(is_qwen_model("qwen3-4b"));
        assert!(is_qwen_model("qwen-anything"));

        assert!(!is_qwen_model("lfm2.5-1.2b"));
        assert!(!is_qwen_model("gpt-4"));
        assert!(!is_qwen_model(""));
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
