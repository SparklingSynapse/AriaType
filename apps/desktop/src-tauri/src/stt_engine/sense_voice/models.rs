pub use crate::stt_engine::whisper::models::ModelDefinition;

pub const SMALL_Q4_K: ModelDefinition = ModelDefinition {
    name: "sense-voice-small-q4_k",
    display_name: "SenseVoice Small Q4 (244M)",
    size_mb: 244,
    speed_score: 8,
    accuracy_score: 9,
    prefer_lang: &["zh-CN", "zh-TW", "yue-CN", "ja-JP", "ko-KR", "en-US"],
    filename: "sense-voice-small-q4_k.gguf",
};

pub const SMALL_Q8_0: ModelDefinition = ModelDefinition {
    name: "sense-voice-small-q8_0",
    display_name: "SenseVoice Small Q8 (488M)",
    size_mb: 488,
    speed_score: 6,
    accuracy_score: 10,
    prefer_lang: &["zh-CN", "zh-TW", "yue-CN", "ja-JP", "ko-KR", "en-US"],
    filename: "sense-voice-small-q8_0.gguf",
};

#[cfg_attr(not(test), allow(dead_code))]
pub const DEFAULT: &ModelDefinition = &SMALL_Q4_K;

pub const ALL: &[&ModelDefinition] = &[&SMALL_Q4_K, &SMALL_Q8_0];

pub fn find_by_name(name: &str) -> Option<&'static ModelDefinition> {
    ALL.iter().find(|m| m.name == name).copied()
}

pub fn recommend_by_language(lang: &str) -> Vec<&'static ModelDefinition> {
    let base_lang = lang.split('-').next().unwrap_or(lang);
    let mut models: Vec<_> = ALL
        .iter()
        .filter(|m| {
            lang == "auto"
                || m.prefer_lang.contains(&lang)
                || m.prefer_lang
                    .iter()
                    .any(|p| p.split('-').next().unwrap_or(*p) == base_lang)
        })
        .copied()
        .collect();
    models.sort_by(|a, b| b.accuracy_score.cmp(&a.accuracy_score));
    models
}

#[cfg_attr(not(test), allow(dead_code))]
pub mod versions {
    pub const SMALL_Q4_K: &str = "sense-voice-small-q4_k";
    pub const SMALL_Q8_0: &str = "sense-voice-small-q8_0";
    pub const DEFAULT: &str = SMALL_Q4_K;

    pub const ALL: &[&str] = &[SMALL_Q4_K, SMALL_Q8_0];
}
