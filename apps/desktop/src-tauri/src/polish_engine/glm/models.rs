use crate::utils::HuggingFaceSource;

const GLM_MODELS: &[(&str, &str, &str, &str, &str)] = &[(
    "glm-4.7-flash-reap-23b-a3b",
    "unsloth/GLM-4.7-Flash-REAP-23B-A3B-GGUF",
    "GLM-4.7-Flash-REAP-23B-A3B-UD-Q4_K_XL.gguf",
    "GLM-4.7 Flash REAP 23B-A3B",
    "~14.2GB",
)];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlmModelDef {
    pub id: &'static str,
    pub repo: &'static str,
    pub filename: &'static str,
    pub display_name: &'static str,
    pub size_display: &'static str,
}

impl GlmModelDef {
    pub fn from_id(id: &str) -> Option<Self> {
        GLM_MODELS
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
        GLM_MODELS
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

pub fn get_all_models() -> Vec<GlmModelDef> {
    GLM_MODELS
        .iter()
        .map(
            |(id, repo, filename, display_name, size_display)| GlmModelDef {
                id,
                repo,
                filename,
                display_name,
                size_display,
            },
        )
        .collect()
}

pub fn is_glm_model(model_id: &str) -> bool {
    model_id.starts_with("glm-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glm_model_def_from_id() {
        let model = GlmModelDef::from_id("glm-4.7-flash-reap-23b-a3b");
        assert!(model.is_some());
        let model = model.unwrap();
        assert_eq!(model.id, "glm-4.7-flash-reap-23b-a3b");
        assert_eq!(model.display_name, "GLM-4.7 Flash REAP 23B-A3B");
        assert_eq!(model.filename, "GLM-4.7-Flash-REAP-23B-A3B-UD-Q4_K_XL.gguf");
        assert_eq!(model.repo, "unsloth/GLM-4.7-Flash-REAP-23B-A3B-GGUF");
        assert_eq!(model.size_display, "~14.2GB");
    }

    #[test]
    fn test_glm_model_def_from_id_not_found() {
        let model = GlmModelDef::from_id("nonexistent");
        assert!(model.is_none());
    }

    #[test]
    fn test_glm_model_def_from_filename() {
        let model = GlmModelDef::from_filename("GLM-4.7-Flash-REAP-23B-A3B-UD-Q4_K_XL.gguf");
        assert!(model.is_some());
        let model = model.unwrap();
        assert_eq!(model.id, "glm-4.7-flash-reap-23b-a3b");
    }

    #[test]
    fn test_glm_model_def_urls() {
        let model = GlmModelDef::from_id("glm-4.7-flash-reap-23b-a3b").unwrap();
        let urls = model.urls();
        assert!(!urls.is_empty());
        assert!(urls
            .iter()
            .any(|url| url.contains("GLM-4.7-Flash-REAP-23B-A3B-UD-Q4_K_XL.gguf")));
    }

    #[test]
    fn test_get_all_models() {
        let models = get_all_models();
        assert_eq!(models.len(), 1);
        let ids: Vec<&str> = models.iter().map(|m| m.id).collect();
        assert!(ids.contains(&"glm-4.7-flash-reap-23b-a3b"));
    }

    #[test]
    fn test_is_glm_model() {
        assert!(is_glm_model("glm-4.7-flash-reap-23b-a3b"));
        assert!(is_glm_model("glm-anything"));

        assert!(!is_glm_model("qwen3.5-0.8b"));
        assert!(!is_glm_model("lfm2.5-1.2b"));
        assert!(!is_glm_model(""));
    }
}
