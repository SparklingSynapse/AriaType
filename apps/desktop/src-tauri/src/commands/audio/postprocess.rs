use std::time::{Duration, Instant};

use tracing::{info, warn};

use crate::correction_learning::hotwords::{
    apply_hotwords_to_text, parse_custom_hotword_entries, HotwordEntry,
};
use crate::correction_learning::types::{CorrectionApplyResult, CorrectionMapping, CorrectionPair};

const GLOSSARY_CORRECTION_SOURCE: &str = "user_glossary";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PostSttProcessResult {
    pub text: String,
    pub postprocess_ms: u64,
    pub normalization_applied: usize,
    pub corrections_applied: usize,
    pub hotwords_applied: usize,
    pub glossary_applied: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TextNormalizationResult {
    text: String,
    applied: usize,
}

struct PostSttFinishInput<'a> {
    raw_text: &'a str,
    normalized_input_text: &'a str,
    input_normalization_applied: usize,
    correction_memory_enabled: bool,
    correction_result: Result<CorrectionApplyResult, String>,
    custom_hotword_entries: &'a [HotwordEntry],
    glossary_mappings: &'a [CorrectionMapping],
    elapsed: Duration,
    task_id: u64,
    context: &'static str,
}

pub(super) fn apply_post_stt_processing(
    raw_text: &str,
    correction_memory_enabled: bool,
    user_glossary: &str,
    custom_dictionary: &str,
    task_id: u64,
    context: &'static str,
) -> PostSttProcessResult {
    let started = Instant::now();
    let normalized_input = normalize_transcript_text(raw_text);

    let correction_result = if correction_memory_enabled {
        crate::correction_learning::storage::apply_shared_hotwords_best_effort_result(
            &normalized_input.text,
        )
    } else {
        Ok(CorrectionApplyResult {
            text: normalized_input.text.clone(),
            applied: Vec::new(),
        })
    };
    let custom_hotword_entries = parse_custom_hotword_entries(custom_dictionary);
    let glossary_mappings = parse_glossary_correction_mappings(user_glossary);

    finish_post_stt_processing(PostSttFinishInput {
        raw_text,
        normalized_input_text: &normalized_input.text,
        input_normalization_applied: normalized_input.applied,
        correction_memory_enabled,
        correction_result,
        custom_hotword_entries: &custom_hotword_entries,
        glossary_mappings: &glossary_mappings,
        elapsed: started.elapsed(),
        task_id,
        context,
    })
}

fn finish_post_stt_processing(input: PostSttFinishInput<'_>) -> PostSttProcessResult {
    let PostSttFinishInput {
        raw_text,
        normalized_input_text,
        input_normalization_applied,
        correction_memory_enabled,
        correction_result,
        custom_hotword_entries,
        glossary_mappings,
        elapsed,
        task_id,
        context,
    } = input;

    let (corrected_text, corrections_applied, fallback_reason) = match correction_result {
        Ok(result) => (result.text, result.applied.len(), None),
        Err(error) => {
            warn!(task_id, context, error = %error, "post_stt_correction_failed");
            (
                normalized_input_text.to_string(),
                0,
                Some("correction learning failed"),
            )
        }
    };
    let custom_hotword_result = apply_hotwords_to_text(&corrected_text, custom_hotword_entries);
    let glossary_result =
        apply_glossary_corrections(&custom_hotword_result.text, glossary_mappings);
    let normalized_output = normalize_transcript_text(&glossary_result.text);
    let text = normalized_output.text;
    let normalization_applied = input_normalization_applied + normalized_output.applied;
    let hotwords_applied = custom_hotword_result.applied.len();
    let glossary_applied = glossary_result.applied.len();
    let postprocess_ms = elapsed.as_millis() as u64;

    info!(
        task_id,
        context,
        postprocess_ms,
        normalization_applied,
        corrections_applied,
        hotwords_applied,
        glossary_applied,
        custom_dictionary_entries = custom_hotword_entries.len(),
        glossary_entries = glossary_mappings.len(),
        correction_memory_enabled,
        input_chars = raw_text.chars().count(),
        output_chars = text.chars().count(),
        fallback_reason,
        "post_stt_pipeline_completed"
    );

    PostSttProcessResult {
        text,
        postprocess_ms,
        normalization_applied,
        corrections_applied,
        hotwords_applied,
        glossary_applied,
    }
}

fn normalize_transcript_text(text: &str) -> TextNormalizationResult {
    let collapsed = collapse_whitespace(text);
    let normalized = normalize_punctuation_spacing(&collapsed);
    let applied = usize::from(normalized != text);

    TextNormalizationResult {
        text: normalized,
        applied,
    }
}

fn collapse_whitespace(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut pending_space = false;

    for c in text.chars() {
        if c.is_whitespace() {
            if !output.is_empty() {
                pending_space = true;
            }
            continue;
        }

        if pending_space {
            output.push(' ');
            pending_space = false;
        }
        output.push(c);
    }

    output
}

fn normalize_punctuation_spacing(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut output = String::with_capacity(text.len());

    for (index, c) in chars.iter().enumerate() {
        let previous = index.checked_sub(1).and_then(|i| chars.get(i)).copied();
        let next = chars.get(index + 1).copied();

        if *c == ' ' && should_drop_space(previous, next) {
            continue;
        }

        output.push(*c);

        if should_insert_space_after(*c, previous, next) {
            output.push(' ');
        }
    }

    output
}

fn should_drop_space(previous: Option<char>, next: Option<char>) -> bool {
    match (previous, next) {
        (_, Some(next)) if is_no_space_before_punctuation(next) => true,
        (Some(previous), _) if is_opening_punctuation(previous) => true,
        (Some(previous), _) if is_cjk_punctuation(previous) => true,
        _ => false,
    }
}

fn should_insert_space_after(c: char, previous: Option<char>, next: Option<char>) -> bool {
    if !is_latin_separator(c) {
        return false;
    }

    let Some(next) = next else {
        return false;
    };
    if next.is_whitespace() || is_no_space_before_punctuation(next) {
        return false;
    }
    if !next.is_ascii_alphanumeric() {
        return false;
    }

    let Some(previous) = previous else {
        return true;
    };
    if matches!(c, ',' | ':') && previous.is_ascii_digit() && next.is_ascii_digit() {
        return false;
    }

    true
}

fn is_latin_separator(c: char) -> bool {
    matches!(c, ',' | ';' | ':' | '?' | '!')
}

fn is_no_space_before_punctuation(c: char) -> bool {
    matches!(
        c,
        ',' | '.'
            | '?'
            | '!'
            | ';'
            | ':'
            | ')'
            | ']'
            | '}'
            | '，'
            | '。'
            | '？'
            | '！'
            | '；'
            | '：'
            | '、'
            | '）'
            | '】'
            | '」'
            | '』'
    )
}

fn is_opening_punctuation(c: char) -> bool {
    matches!(c, '(' | '[' | '{' | '（' | '【' | '「' | '『')
}

fn is_cjk_punctuation(c: char) -> bool {
    matches!(c, '，' | '。' | '？' | '！' | '；' | '：' | '、')
}

fn apply_glossary_corrections(
    text: &str,
    glossary_mappings: &[CorrectionMapping],
) -> CorrectionApplyResult {
    crate::correction_learning::storage::apply_corrections_to_text(text, glossary_mappings)
}

fn parse_glossary_correction_mappings(glossary: &str) -> Vec<CorrectionMapping> {
    let mut mappings = Vec::new();
    for entry in split_glossary_entries(glossary) {
        if let Some(pair) = correction_pair_from_glossary_entry(entry) {
            if mappings
                .iter()
                .any(|mapping: &CorrectionMapping| mapping.wrong == pair.wrong)
            {
                continue;
            }
            mappings.push(CorrectionMapping {
                wrong: pair.wrong,
                corrected: pair.corrected,
                frequency: u32::MAX,
                first_seen_at_ms: 0,
                last_seen_at_ms: 0,
                source: GLOSSARY_CORRECTION_SOURCE.to_string(),
            });
        }
    }
    mappings
}

fn split_glossary_entries(glossary: &str) -> impl Iterator<Item = &str> {
    glossary
        .split(['\n', '\r', ',', '，', ';', '；'])
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
}

fn correction_pair_from_glossary_entry(entry: &str) -> Option<CorrectionPair> {
    for delimiter in ["->", "=>", "→"] {
        if let Some((wrong, corrected)) = entry.split_once(delimiter) {
            let wrong = wrong.trim();
            let corrected = corrected.trim();
            if !wrong.is_empty() && !corrected.is_empty() && wrong != corrected {
                return Some(CorrectionPair::new(wrong, corrected));
            }
            return None;
        }
    }

    canonical_case_mapping(entry)
}

fn canonical_case_mapping(term: &str) -> Option<CorrectionPair> {
    let term = term.trim();
    if !term.chars().any(|c| c.is_ascii_uppercase()) {
        return None;
    }

    let wrong = term.to_lowercase();
    if wrong == term {
        return None;
    }

    Some(CorrectionPair::new(wrong, term))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::correction_learning::types::CorrectionPair;

    #[test]
    fn disabled_correction_memory_returns_original_text() {
        let result = finish_post_stt_processing(PostSttFinishInput {
            raw_text: "raw text",
            normalized_input_text: "raw text",
            input_normalization_applied: 0,
            correction_memory_enabled: false,
            correction_result: Ok(CorrectionApplyResult {
                text: "raw text".to_string(),
                applied: Vec::new(),
            }),
            custom_hotword_entries: &[],
            glossary_mappings: &[],
            elapsed: Duration::from_millis(3),
            task_id: 1,
            context: "test",
        });

        assert_eq!(result.text, "raw text");
        assert_eq!(result.postprocess_ms, 3);
        assert_eq!(result.normalization_applied, 0);
        assert_eq!(result.corrections_applied, 0);
        assert_eq!(result.hotwords_applied, 0);
        assert_eq!(result.glossary_applied, 0);
    }

    #[test]
    fn reports_correction_only_output_and_applied_count() {
        let result = finish_post_stt_processing(PostSttFinishInput {
            raw_text: "open 搜题",
            normalized_input_text: "open 搜题",
            input_normalization_applied: 0,
            correction_memory_enabled: true,
            correction_result: Ok(CorrectionApplyResult {
                text: "open sootie".to_string(),
                applied: vec![CorrectionPair::new("搜题", "sootie")],
            }),
            custom_hotword_entries: &[],
            glossary_mappings: &[],
            elapsed: Duration::from_millis(5),
            task_id: 2,
            context: "test",
        });

        assert_eq!(result.text, "open sootie");
        assert_eq!(result.postprocess_ms, 5);
        assert_eq!(result.normalization_applied, 0);
        assert_eq!(result.corrections_applied, 1);
        assert_eq!(result.hotwords_applied, 0);
        assert_eq!(result.glossary_applied, 0);
    }

    #[test]
    fn falls_back_to_raw_text_when_correction_stage_fails() {
        let result = finish_post_stt_processing(PostSttFinishInput {
            raw_text: "raw text",
            normalized_input_text: "raw text",
            input_normalization_applied: 0,
            correction_memory_enabled: true,
            correction_result: Err("store unavailable".to_string()),
            custom_hotword_entries: &[],
            glossary_mappings: &[],
            elapsed: Duration::from_millis(7),
            task_id: 3,
            context: "test",
        });

        assert_eq!(result.text, "raw text");
        assert_eq!(result.postprocess_ms, 7);
        assert_eq!(result.normalization_applied, 0);
        assert_eq!(result.corrections_applied, 0);
        assert_eq!(result.hotwords_applied, 0);
        assert_eq!(result.glossary_applied, 0);
    }

    #[test]
    fn normalizes_spacing_and_punctuation_without_llm() {
        let result = apply_post_stt_processing(
            "  hello   ,world  this is  fine  ",
            false,
            "",
            "",
            5,
            "test",
        );

        assert_eq!(result.text, "hello, world this is fine");
        assert_eq!(result.normalization_applied, 1);
        assert_eq!(result.corrections_applied, 0);
        assert_eq!(result.hotwords_applied, 0);
        assert_eq!(result.glossary_applied, 0);
    }

    #[test]
    fn normalizes_before_correction_failure_fallback() {
        let result = finish_post_stt_processing(PostSttFinishInput {
            raw_text: "hello ,world",
            normalized_input_text: "hello, world",
            input_normalization_applied: 1,
            correction_memory_enabled: true,
            correction_result: Err("store unavailable".to_string()),
            custom_hotword_entries: &[],
            glossary_mappings: &[],
            elapsed: Duration::from_millis(2),
            task_id: 6,
            context: "test",
        });

        assert_eq!(result.text, "hello, world");
        assert_eq!(result.normalization_applied, 1);
        assert_eq!(result.corrections_applied, 0);
        assert_eq!(result.hotwords_applied, 0);
    }

    #[test]
    fn preserves_decimal_versions_and_time_like_colons() {
        let result =
            apply_post_stt_processing("version 1.2 ,ok at 10:30", false, "", "", 7, "test");

        assert_eq!(result.text, "version 1.2, ok at 10:30");
        assert_eq!(result.normalization_applied, 1);
    }

    #[test]
    fn removes_spaces_around_cjk_punctuation() {
        let result = apply_post_stt_processing("你好 ， 世界 。", false, "", "", 8, "test");

        assert_eq!(result.text, "你好，世界。");
        assert_eq!(result.normalization_applied, 1);
    }

    #[test]
    fn keeps_raw_chinese_terms_unchanged_in_fast_path() {
        let result =
            apply_post_stt_processing("一般一起周一乱七八糟乱七八糟", false, "", "", 9, "test");

        assert_eq!(result.text, "一般一起周一乱七八糟乱七八糟");
        assert_eq!(result.normalization_applied, 0);
    }

    #[test]
    fn parses_explicit_glossary_mappings() {
        let mappings = parse_glossary_correction_mappings("搜题 -> sootie, node js=>Node.js");

        assert_eq!(mappings.len(), 2);
        assert_eq!(mappings[0].wrong, "搜题");
        assert_eq!(mappings[0].corrected, "sootie");
        assert_eq!(mappings[1].wrong, "node js");
        assert_eq!(mappings[1].corrected, "Node.js");
    }

    #[test]
    fn creates_case_correction_for_canonical_ascii_terms() {
        let mappings = parse_glossary_correction_mappings("AriaType,sootie,Node.js");

        assert_eq!(mappings.len(), 2);
        assert_eq!(mappings[0].wrong, "ariatype");
        assert_eq!(mappings[0].corrected, "AriaType");
        assert_eq!(mappings[1].wrong, "node.js");
        assert_eq!(mappings[1].corrected, "Node.js");
    }

    #[test]
    fn keeps_custom_dictionary_hotwords_separate_from_user_glossary() {
        let result = apply_post_stt_processing(
            "open 搜题 with ariatype",
            false,
            "AriaType",
            "sootie",
            10,
            "test",
        );

        assert_eq!(result.text, "open sootie with AriaType");
        assert_eq!(result.hotwords_applied, 1);
        assert_eq!(result.glossary_applied, 1);
    }

    #[test]
    fn applies_glossary_after_correction_memory() {
        let mappings = parse_glossary_correction_mappings("搜题 -> sootie, AriaType");
        let result = finish_post_stt_processing(PostSttFinishInput {
            raw_text: "open 搜题 with ariatype",
            normalized_input_text: "open 搜题 with ariatype",
            input_normalization_applied: 0,
            correction_memory_enabled: true,
            correction_result: Ok(CorrectionApplyResult {
                text: "open 搜题 with ariatype".to_string(),
                applied: Vec::new(),
            }),
            custom_hotword_entries: &[],
            glossary_mappings: &mappings,
            elapsed: Duration::from_millis(4),
            task_id: 4,
            context: "test",
        });

        assert_eq!(result.text, "open sootie with AriaType");
        assert_eq!(result.corrections_applied, 0);
        assert_eq!(result.hotwords_applied, 0);
        assert_eq!(result.glossary_applied, 2);
    }
}
