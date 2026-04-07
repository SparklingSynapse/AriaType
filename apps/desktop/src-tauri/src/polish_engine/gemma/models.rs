use crate::utils::HuggingFaceSource;

const GEMMA_MODELS: &[(&str, &str, &str, &str, &str)] = &[(
    "gemma-2b-it",
    "reach-vb/gemma-2b-it-Q4_K_M-GGUF",
    "gemma-2b-it.Q4_K_M.gguf",
    "Gemma 2B IT",
    "~1.52GB",
)];

const LEGACY_GEMMA_MODEL_IDS: &[(&str, &str)] = &[("gemma-4-e2b", "gemma-2b-it")];

fn canonical_model_id(id: &str) -> &str {
    LEGACY_GEMMA_MODEL_IDS
        .iter()
        .find_map(|(legacy_id, canonical_id)| (*legacy_id == id).then_some(*canonical_id))
        .unwrap_or(id)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GemmaModelDef {
    pub id: &'static str,
    pub repo: &'static str,
    pub filename: &'static str,
    pub display_name: &'static str,
    pub size_display: &'static str,
}

impl GemmaModelDef {
    pub fn from_id(id: &str) -> Option<Self> {
        let canonical_id = canonical_model_id(id);
        GEMMA_MODELS
            .iter()
            .find(|(model_id, _, _, _, _)| *model_id == canonical_id)
            .map(|(id, repo, filename, display_name, size_display)| Self {
                id,
                repo,
                filename,
                display_name,
                size_display,
            })
    }

    pub fn from_filename(filename: &str) -> Option<Self> {
        GEMMA_MODELS
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

pub fn get_all_models() -> Vec<GemmaModelDef> {
    GEMMA_MODELS
        .iter()
        .map(
            |(id, repo, filename, display_name, size_display)| GemmaModelDef {
                id,
                repo,
                filename,
                display_name,
                size_display,
            },
        )
        .collect()
}

pub fn is_gemma_model(model_id: &str) -> bool {
    model_id.starts_with("gemma")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gemma_model_def_from_id() {
        let model = GemmaModelDef::from_id("gemma-2b-it");
        assert!(model.is_some());
        let model = model.unwrap();
        assert_eq!(model.id, "gemma-2b-it");
        assert_eq!(model.display_name, "Gemma 2B IT");
        assert_eq!(model.filename, "gemma-2b-it.Q4_K_M.gguf");
        assert_eq!(model.repo, "reach-vb/gemma-2b-it-Q4_K_M-GGUF");
        assert_eq!(model.size_display, "~1.52GB");
    }

    #[test]
    fn test_gemma_model_def_from_legacy_id() {
        let model = GemmaModelDef::from_id("gemma-4-e2b");
        assert!(model.is_some());
        let model = model.unwrap();
        assert_eq!(model.id, "gemma-2b-it");
        assert_eq!(model.filename, "gemma-2b-it.Q4_K_M.gguf");
    }

    #[test]
    fn test_gemma_model_def_from_id_not_found() {
        let model = GemmaModelDef::from_id("nonexistent");
        assert!(model.is_none());
    }

    #[test]
    fn test_gemma_model_def_from_filename() {
        let model = GemmaModelDef::from_filename("gemma-2b-it.Q4_K_M.gguf");
        assert!(model.is_some());
        let model = model.unwrap();
        assert_eq!(model.id, "gemma-2b-it");
    }

    #[test]
    fn test_gemma_model_def_from_filename_not_found() {
        let model = GemmaModelDef::from_filename("nonexistent.gguf");
        assert!(model.is_none());
    }

    #[test]
    fn test_gemma_model_def_urls() {
        let model = GemmaModelDef::from_id("gemma-2b-it").unwrap();
        let urls = model.urls();
        assert!(!urls.is_empty());
        assert!(urls
            .iter()
            .any(|url| url.contains("gemma-2b-it.Q4_K_M.gguf")));
    }

    #[test]
    fn test_get_all_models() {
        let models = get_all_models();
        assert_eq!(models.len(), 1);
        let ids: Vec<&str> = models.iter().map(|m| m.id).collect();
        assert!(ids.contains(&"gemma-2b-it"));
    }

    #[test]
    fn test_is_gemma_model() {
        assert!(is_gemma_model("gemma-2b-it"));
        assert!(is_gemma_model("gemma-4-e2b"));
        assert!(is_gemma_model("gemma-anything"));

        assert!(!is_gemma_model("qwen3.5-0.8b"));
        assert!(!is_gemma_model("lfm2.5-1.2b"));
        assert!(!is_gemma_model(""));
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
