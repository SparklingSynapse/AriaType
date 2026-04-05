#[derive(Debug, Clone)]
pub struct ModelDefinition {
    pub name: &'static str,
    pub display_name: &'static str,
    pub size_mb: u32,
    pub speed_score: u8,
    pub accuracy_score: u8,
    pub prefer_lang: &'static [&'static str],
    pub filename: &'static str,
}

pub const TINY: ModelDefinition = ModelDefinition {
    name: "tiny",
    display_name: "Whisper Tiny (39M)",
    size_mb: 39,
    speed_score: 10,
    accuracy_score: 6,
    prefer_lang: &["en-US"],
    filename: "ggml-tiny.bin",
};

pub const BASE: ModelDefinition = ModelDefinition {
    name: "base",
    display_name: "Whisper Base (74M)",
    size_mb: 74,
    speed_score: 9,
    accuracy_score: 7,
    prefer_lang: &["en-US"],
    filename: "ggml-base.bin",
};

pub const SMALL_Q8_0: ModelDefinition = ModelDefinition {
    name: "small-q8_0",
    display_name: "Whisper Small Q8 (252M)",
    size_mb: 252,
    speed_score: 7,
    accuracy_score: 8,
    prefer_lang: &["en-US", "zh-CN", "zh-TW", "yue-CN", "ja-JP", "ko-KR"],
    filename: "ggml-small-q8_0.bin",
};

pub const MEDIUM_Q5_0: ModelDefinition = ModelDefinition {
    name: "medium-q5_0",
    display_name: "Whisper Medium Q5 (515M)",
    size_mb: 515,
    speed_score: 5,
    accuracy_score: 9,
    prefer_lang: &[
        "en-US", "zh-CN", "zh-TW", "yue-CN", "ja-JP", "ko-KR", "es-ES", "fr-FR", "de-DE",
    ],
    filename: "ggml-medium-q5_0.bin",
};

pub const LARGE_V3_TURBO_Q8_0: ModelDefinition = ModelDefinition {
    name: "large-v3-turbo-q8_0",
    display_name: "Whisper Large V3 Turbo Q8 (800M)",
    size_mb: 800,
    speed_score: 3,
    accuracy_score: 10,
    prefer_lang: &[
        "en-US", "zh-CN", "zh-TW", "yue-CN", "ja-JP", "ko-KR", "es-ES", "fr-FR", "de-DE", "ru-RU",
        "ar-SA", "pt-BR", "hi-IN", "it-IT", "nl-NL", "pl-PL", "tr-TR", "vi-VN", "th-TH",
    ],
    filename: "ggml-large-v3-turbo-q8_0.bin",
};

#[cfg_attr(not(test), allow(dead_code))]
pub const DEFAULT: &ModelDefinition = &BASE;

pub const ALL: &[&ModelDefinition] = &[
    &TINY,
    &BASE,
    &SMALL_Q8_0,
    &MEDIUM_Q5_0,
    &LARGE_V3_TURBO_Q8_0,
];

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
pub fn recommend_by_speed(min_speed: u8) -> Vec<&'static ModelDefinition> {
    let mut models: Vec<_> = ALL
        .iter()
        .filter(|m| m.speed_score >= min_speed)
        .copied()
        .collect();
    models.sort_by(|a, b| b.accuracy_score.cmp(&a.accuracy_score));
    models
}

#[cfg_attr(not(test), allow(dead_code))]
pub mod versions {
    pub const TINY: &str = "tiny";
    pub const BASE: &str = "base";
    pub const SMALL_Q8_0: &str = "small-q8_0";
    pub const MEDIUM_Q5_0: &str = "medium-q5_0";
    pub const LARGE_V3_TURBO_Q8_0: &str = "large-v3-turbo-q8_0";
    pub const DEFAULT: &str = BASE;

    pub const ALL: &[&str] = &[TINY, BASE, SMALL_Q8_0, MEDIUM_Q5_0, LARGE_V3_TURBO_Q8_0];
}
