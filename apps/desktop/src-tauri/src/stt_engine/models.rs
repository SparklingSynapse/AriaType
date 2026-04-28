use crate::stt_engine::traits::EngineType;

/// Model file entry for download tracking
#[derive(Debug, Clone)]
pub struct ModelFile {
    pub filename: &'static str,
    pub size_mb: u32,
}

/// Unified model definition for all local STT models
#[derive(Debug, Clone)]
pub struct ModelDefinition {
    pub name: &'static str,
    pub display_name: &'static str,
    pub size_mb: u32,
    pub speed_score: u8,
    pub accuracy_score: u8,
    pub engine_type: EngineType,
    pub files: &'static [&'static ModelFile],
    pub prefer_lang: &'static [&'static str],
    pub description: &'static str,
}

/// Language codes for which SenseVoice is the preferred engine
pub const SENSEVOICE_PREFERRED_CODES: &[&str] = &["zh", "yue", "ja", "ko", "en"];

// ============================================================================
// Model Definitions
// ============================================================================

/// SenseVoice Small - optimized for CJK + English
pub const SENSE_VOICE_SMALL: ModelDefinition = ModelDefinition {
    name: "sense-voice-small",
    display_name: "SenseVoice Small (229M)",
    size_mb: 229,
    speed_score: 8,
    accuracy_score: 9,
    engine_type: EngineType::SenseVoice,
    files: &[
        &ModelFile {
            filename: "model.int8.onnx",
            size_mb: 228,
        },
        &ModelFile {
            filename: "tokens.txt",
            size_mb: 1,
        },
    ],
    prefer_lang: &["zh", "yue", "ja", "ko", "en"],
    description: "SenseVoice Small for Chinese, Japanese, Korean, Cantonese, and English",
};

/// Whisper Base - general purpose for all languages
pub const WHISPER_BASE: ModelDefinition = ModelDefinition {
    name: "whisper-base",
    display_name: "Whisper Base (279M)",
    size_mb: 279,
    speed_score: 9,
    accuracy_score: 7,
    engine_type: EngineType::Whisper,
    files: &[
        &ModelFile {
            filename: "base-encoder.onnx",
            size_mb: 91,
        },
        &ModelFile {
            filename: "base-decoder.onnx",
            size_mb: 187,
        },
        &ModelFile {
            filename: "base-tokens.txt",
            size_mb: 1,
        },
    ],
    prefer_lang: &[], // Empty = all languages
    description: "Whisper Base for all languages, fast and lightweight",
};

/// Whisper Small - better accuracy for all languages
pub const WHISPER_SMALL: ModelDefinition = ModelDefinition {
    name: "whisper-small",
    display_name: "Whisper Small (925M)",
    size_mb: 925,
    speed_score: 7,
    accuracy_score: 8,
    engine_type: EngineType::Whisper,
    files: &[
        &ModelFile {
            filename: "small-encoder.onnx",
            size_mb: 391,
        },
        &ModelFile {
            filename: "small-decoder.onnx",
            size_mb: 533,
        },
        &ModelFile {
            filename: "small-tokens.txt",
            size_mb: 1,
        },
    ],
    prefer_lang: &[], // Empty = all languages
    description: "Whisper Small for all languages, better accuracy than Base",
};

/// Default model for general use
pub const DEFAULT: &ModelDefinition = &SENSE_VOICE_SMALL;

/// All available local models
pub const ALL: &[&ModelDefinition] = &[&SENSE_VOICE_SMALL, &WHISPER_BASE, &WHISPER_SMALL];

// ============================================================================
// Helper Functions
// ============================================================================

/// Find a model by its name
pub fn find_by_name(name: &str) -> Option<&'static ModelDefinition> {
    ALL.iter().find(|m| m.name == name).copied()
}

/// Check if a language is a SenseVoice-preferred language (based on base language code)
pub fn is_sensevoice_preferred(lang: &str) -> bool {
    let base_lang = lang.split('-').next().unwrap_or(lang);
    SENSEVOICE_PREFERRED_CODES.contains(&base_lang)
}

/// Recommend models by language
///
/// For SenseVoice-preferred languages: returns SenseVoice Small only
/// For other languages: returns Whisper Base only
pub fn recommend_by_language(lang: &str) -> Vec<&'static ModelDefinition> {
    if lang == "auto" {
        // Return all models for auto-detect, sorted by accuracy
        let mut models: Vec<_> = ALL.to_vec();
        models.sort_by(|a, b| b.accuracy_score.cmp(&a.accuracy_score));
        return models;
    }

    let base_lang = lang.split('-').next().unwrap_or(lang);

    // Check if it's a SenseVoice-preferred language
    if SENSEVOICE_PREFERRED_CODES.contains(&base_lang) {
        // For preferred languages, recommend SenseVoice only
        vec![&SENSE_VOICE_SMALL]
    } else {
        // For other languages, recommend Whisper models
        vec![&WHISPER_BASE]
    }
}

/// Get the default model for a given language
///
/// For SenseVoice-preferred languages: returns SenseVoice Small
/// For other languages: returns Whisper Base
pub fn default_for_language(lang: &str) -> &'static ModelDefinition {
    if lang == "auto" {
        return DEFAULT;
    }

    let base_lang = lang.split('-').next().unwrap_or(lang);

    if SENSEVOICE_PREFERRED_CODES.contains(&base_lang) {
        &SENSE_VOICE_SMALL
    } else {
        &WHISPER_BASE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_definitions() {
        assert_eq!(SENSE_VOICE_SMALL.name, "sense-voice-small");
        assert_eq!(SENSE_VOICE_SMALL.speed_score, 8);
        assert_eq!(SENSE_VOICE_SMALL.accuracy_score, 9);
        assert_eq!(SENSE_VOICE_SMALL.engine_type, EngineType::SenseVoice);
        assert_eq!(SENSE_VOICE_SMALL.files.len(), 2);

        assert_eq!(WHISPER_BASE.name, "whisper-base");
        assert_eq!(WHISPER_BASE.speed_score, 9);
        assert_eq!(WHISPER_BASE.accuracy_score, 7);
        assert_eq!(WHISPER_BASE.engine_type, EngineType::Whisper);
        assert_eq!(WHISPER_BASE.files.len(), 3);

        assert_eq!(WHISPER_SMALL.name, "whisper-small");
        assert_eq!(WHISPER_SMALL.speed_score, 7);
        assert_eq!(WHISPER_SMALL.accuracy_score, 8);
        assert_eq!(WHISPER_SMALL.engine_type, EngineType::Whisper);
        assert_eq!(WHISPER_SMALL.files.len(), 3);

        assert_eq!(ALL.len(), 3);
    }

    #[test]
    fn test_find_by_name() {
        assert!(find_by_name("sense-voice-small").is_some());
        assert!(find_by_name("whisper-base").is_some());
        assert!(find_by_name("whisper-small").is_some());
        assert!(find_by_name("unknown").is_none());

        let model = find_by_name("sense-voice-small").unwrap();
        assert_eq!(model.name, "sense-voice-small");
    }

    #[test]
    fn test_is_sensevoice_preferred() {
        // Full codes
        assert!(is_sensevoice_preferred("zh-CN"));
        assert!(is_sensevoice_preferred("zh-TW"));
        assert!(is_sensevoice_preferred("yue-CN"));
        assert!(is_sensevoice_preferred("ja-JP"));
        assert!(is_sensevoice_preferred("ko-KR"));
        assert!(is_sensevoice_preferred("en-US"));

        // Base codes
        assert!(is_sensevoice_preferred("zh"));
        assert!(is_sensevoice_preferred("yue"));
        assert!(is_sensevoice_preferred("ja"));
        assert!(is_sensevoice_preferred("ko"));
        assert!(is_sensevoice_preferred("en"));

        // Non-preferred
        assert!(!is_sensevoice_preferred("es-ES"));
        assert!(!is_sensevoice_preferred("fr-FR"));
    }

    #[test]
    fn test_recommend_by_language_cjk() {
        // Chinese variants
        let zh_models = recommend_by_language("zh");
        assert_eq!(zh_models.len(), 1);
        assert_eq!(zh_models[0].name, "sense-voice-small");

        let zh_cn_models = recommend_by_language("zh-CN");
        assert_eq!(zh_cn_models.len(), 1);
        assert_eq!(zh_cn_models[0].name, "sense-voice-small");

        // Japanese
        let ja_models = recommend_by_language("ja");
        assert_eq!(ja_models.len(), 1);
        assert_eq!(ja_models[0].name, "sense-voice-small");

        // Korean
        let ko_models = recommend_by_language("ko");
        assert_eq!(ko_models.len(), 1);
        assert_eq!(ko_models[0].name, "sense-voice-small");

        // Cantonese
        let yue_models = recommend_by_language("yue");
        assert_eq!(yue_models.len(), 1);
        assert_eq!(yue_models[0].name, "sense-voice-small");
    }

    #[test]
    fn test_recommend_by_language_non_preferred() {
        // English is now SenseVoice-preferred
        let en_models = recommend_by_language("en");
        assert_eq!(en_models.len(), 1);
        assert_eq!(en_models[0].name, "sense-voice-small");

        // Spanish
        let es_models = recommend_by_language("es");
        assert_eq!(es_models.len(), 1);
        assert_eq!(es_models[0].name, "whisper-base");

        // French
        let fr_models = recommend_by_language("fr");
        assert_eq!(fr_models.len(), 1);
        assert_eq!(fr_models[0].name, "whisper-base");
    }

    #[test]
    fn test_recommend_by_language_auto() {
        let auto_models = recommend_by_language("auto");
        assert_eq!(auto_models.len(), 3);
        // Should be sorted by accuracy descending
        assert!(auto_models[0].accuracy_score >= auto_models[1].accuracy_score);
        assert!(auto_models[1].accuracy_score >= auto_models[2].accuracy_score);
    }

    #[test]
    fn test_default_for_language() {
        // SenseVoice-preferred languages should default to SenseVoice
        assert_eq!(default_for_language("zh").name, "sense-voice-small");
        assert_eq!(default_for_language("zh-CN").name, "sense-voice-small");
        assert_eq!(default_for_language("ja").name, "sense-voice-small");
        assert_eq!(default_for_language("ko").name, "sense-voice-small");
        assert_eq!(default_for_language("yue").name, "sense-voice-small");
        assert_eq!(default_for_language("en").name, "sense-voice-small");
        assert_eq!(default_for_language("en-US").name, "sense-voice-small");

        // Other languages should default to Whisper Base
        assert_eq!(default_for_language("es").name, "whisper-base");
        assert_eq!(default_for_language("fr").name, "whisper-base");

        // Auto should use global default
        assert_eq!(default_for_language("auto").name, DEFAULT.name);
    }
}
