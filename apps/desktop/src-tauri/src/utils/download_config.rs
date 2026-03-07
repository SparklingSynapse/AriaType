/// Download source with URL and optional mirror
#[derive(Debug, Clone)]
pub struct DownloadSource {
    pub primary_url: String,
    pub mirror_url: Option<String>,
}

impl DownloadSource {
    pub fn new(primary_url: impl Into<String>) -> Self {
        Self {
            primary_url: primary_url.into(),
            mirror_url: None,
        }
    }

    pub fn with_mirror(mut self, mirror_url: impl Into<String>) -> Self {
        self.mirror_url = Some(mirror_url.into());
        self
    }

    pub fn urls(&self) -> Vec<String> {
        let mut urls = vec![self.primary_url.clone()];
        if let Some(mirror) = &self.mirror_url {
            urls.push(mirror.clone());
        }
        urls
    }
}

/// HuggingFace download source builder
pub struct HuggingFaceSource {
    repo: String,
    file: String,
    revision: Option<String>,
}

impl HuggingFaceSource {
    pub fn new(repo: impl Into<String>, file: impl Into<String>) -> Self {
        Self {
            repo: repo.into(),
            file: file.into(),
            revision: None,
        }
    }

    pub fn with_revision(mut self, revision: impl Into<String>) -> Self {
        self.revision = Some(revision.into());
        self
    }

    fn build_url(&self, base: &str) -> String {
        let revision = self.revision.as_deref().unwrap_or("main");
        format!("{}/{}/resolve/{}/{}", base, self.repo, revision, self.file)
    }

    pub fn into_source(self) -> DownloadSource {
        let primary = self.build_url("https://huggingface.co");
        let mirror = self.build_url("https://hf-mirror.com");
        DownloadSource::new(primary).with_mirror(mirror)
    }
}

/// Model download configuration
pub struct ModelDownloadConfig {
    pub source: DownloadSource,
    pub filename: String,
    pub display_name: String,
    pub size_bytes: u64,
    pub model_id: String,
}

impl ModelDownloadConfig {
    pub fn new(
        model_id: impl Into<String>,
        display_name: impl Into<String>,
        filename: impl Into<String>,
        size_bytes: u64,
        source: DownloadSource,
    ) -> Self {
        Self {
            model_id: model_id.into(),
            display_name: display_name.into(),
            filename: filename.into(),
            size_bytes,
            source,
        }
    }

    pub fn urls(&self) -> Vec<String> {
        self.source.urls()
    }
}

/// Global download sources configuration
pub struct DownloadSources {
    pub huggingface_primary: &'static str,
    pub huggingface_mirror: &'static str,
}

impl Default for DownloadSources {
    fn default() -> Self {
        Self {
            huggingface_primary: "https://huggingface.co",
            huggingface_mirror: "https://hf-mirror.com",
        }
    }
}

impl DownloadSources {
    pub fn global() -> Self {
        Self::default()
    }
}
