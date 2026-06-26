use super::types::CorrectionPair;
use similar::{ChangeTag, TextDiff};

const MAX_CORRECTION_CHARS: usize = 40;
const MAX_CORRECTION_TOKENS: usize = 4;
const MAX_CJK_DENSE_TERM_CHARS: usize = 12;
const TECHNICAL_MARKER_TOKEN_THRESHOLD: usize = 3;
const CJK_SENTENCE_MARKER_MIN_CHARS: usize = 6;
const KNOWN_TECHNICAL_TOKENS: &[&str] = &[
    "ai", "api", "cli", "csv", "gpt", "http", "https", "json", "llm", "pdf", "sdk", "sql", "stt",
    "tts", "ui", "ux",
];
const CJK_SENTENCE_MARKERS: &[&str] = &[
    "请",
    "帮",
    "把",
    "将",
    "让",
    "这个",
    "那个",
    "我们",
    "你们",
    "他们",
    "需要",
    "可以",
    "进行",
    "开始",
    "处理",
    "打开",
    "看看",
    "试试",
    "是不是",
    "因为",
    "所以",
    "然后",
];

pub fn extract_correction_pair(before: &str, after: &str) -> Option<CorrectionPair> {
    let before = normalize_text_snapshot(before);
    let after = normalize_text_snapshot(after);
    if before == after {
        return None;
    }

    let before_chars: Vec<char> = before.chars().collect();
    let after_chars: Vec<char> = after.chars().collect();

    let (mut before_start, mut before_end, mut after_start, mut after_end) =
        changed_char_spans(&before, &after)?;

    expand_cjk_single_char_replacement(
        &before_chars,
        &after_chars,
        &mut before_start,
        &mut after_start,
        before_end,
        after_end,
    );
    expand_ascii_word_replacement(
        &before_chars,
        &after_chars,
        &mut before_start,
        &mut after_start,
        &mut before_end,
        &mut after_end,
    );

    let wrong = chars_to_string(&before_chars[before_start..before_end]);
    let corrected = chars_to_string(&after_chars[after_start..after_end]);
    normalize_pair(wrong, corrected)
}

pub(crate) fn is_word_level_correction_pair(wrong: &str, corrected: &str) -> bool {
    normalize_pair(wrong.to_string(), corrected.to_string())
        .is_some_and(|pair| pair.wrong == wrong.trim() && pair.corrected == corrected.trim())
}

pub(crate) fn extract_deleted_correction_term(before: &str, after: &str) -> Option<String> {
    let before = normalize_text_snapshot(before);
    let after = normalize_text_snapshot(after);
    if before == after {
        return None;
    }

    let before_chars: Vec<char> = before.chars().collect();
    let (mut before_start, mut before_end, after_start, after_end) =
        changed_char_spans(&before, &after)?;
    if before_start == before_end || after_start != after_end {
        return None;
    }

    expand_ascii_deleted_term(&before_chars, &mut before_start, &mut before_end);
    let deleted = chars_to_string(&before_chars[before_start..before_end]);
    normalize_correction_term(deleted).filter(|term| is_compact_deleted_term(term))
}

fn changed_char_spans(before: &str, after: &str) -> Option<(usize, usize, usize, usize)> {
    let diff = TextDiff::from_chars(before, after);
    let mut before_pos = 0usize;
    let mut after_pos = 0usize;
    let mut before_start: Option<usize> = None;
    let mut after_start: Option<usize> = None;
    let mut before_end = 0usize;
    let mut after_end = 0usize;

    for change in diff.iter_all_changes() {
        let len = change.value().chars().count();
        match change.tag() {
            ChangeTag::Equal => {
                before_pos += len;
                after_pos += len;
            }
            ChangeTag::Delete => {
                if before_start.is_none() {
                    before_start = Some(before_pos);
                    after_start = Some(after_pos);
                }
                before_pos += len;
                before_end = before_pos;
                after_end = after_pos;
            }
            ChangeTag::Insert => {
                if before_start.is_none() {
                    before_start = Some(before_pos);
                    after_start = Some(after_pos);
                }
                after_pos += len;
                before_end = before_pos;
                after_end = after_pos;
            }
        }
    }

    Some((before_start?, before_end, after_start?, after_end))
}

fn normalize_text_snapshot(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

fn chars_to_string(chars: &[char]) -> String {
    chars.iter().collect::<String>()
}

fn normalize_pair(wrong: String, corrected: String) -> Option<CorrectionPair> {
    let wrong = normalize_correction_term(wrong)?;
    let corrected = normalize_correction_term(corrected)?;

    if wrong == corrected || !is_vocabulary_like_pair(&wrong, &corrected) {
        return None;
    }

    Some(CorrectionPair::new(wrong, corrected))
}

fn normalize_correction_term(text: String) -> Option<String> {
    let term = text
        .trim()
        .trim_matches(is_sentence_boundary_punctuation)
        .trim()
        .to_string();

    if term.is_empty()
        || term.contains('\n')
        || term.chars().count() > MAX_CORRECTION_CHARS
        || is_single_cjk_term(&term)
        || contains_sentence_boundary_punctuation(&term)
        || !has_word_like_char(&term)
        || !is_vocabulary_like_term(&term)
    {
        return None;
    }

    Some(term)
}

fn has_word_like_char(text: &str) -> bool {
    text.chars().any(|c| c.is_alphanumeric() || is_cjk(c))
}

fn is_compact_deleted_term(text: &str) -> bool {
    if !is_vocabulary_like_term(text) {
        return false;
    }

    let token_count = content_token_count(text);
    token_count <= 1 || has_cjk_content(text) || has_dictionary_phrase_signal(text)
}

fn is_vocabulary_like_term(text: &str) -> bool {
    if content_token_count(text) > MAX_CORRECTION_TOKENS {
        return false;
    }

    let cjk_chars = text.chars().filter(|c| is_cjk(*c)).count();
    let has_whitespace = text.split_whitespace().count() > 1;
    if cjk_chars > 0 {
        if !has_whitespace && text.chars().count() > MAX_CJK_DENSE_TERM_CHARS {
            return false;
        }

        if cjk_chars >= CJK_SENTENCE_MARKER_MIN_CHARS
            && CJK_SENTENCE_MARKERS
                .iter()
                .any(|marker| text.contains(marker))
        {
            return false;
        }
    }

    true
}

fn is_vocabulary_like_pair(wrong: &str, corrected: &str) -> bool {
    let max_tokens = content_token_count(wrong).max(content_token_count(corrected));
    let has_pair_signal =
        has_dictionary_phrase_signal(wrong) || has_dictionary_phrase_signal(corrected);
    if max_tokens >= TECHNICAL_MARKER_TOKEN_THRESHOLD && !has_pair_signal {
        return false;
    }

    if is_plain_latin_multi_token_phrase(wrong)
        && is_plain_latin_multi_token_phrase(corrected)
        && !has_pair_signal
    {
        return false;
    }

    true
}

fn content_token_count(text: &str) -> usize {
    content_tokens(text).len()
}

fn content_tokens(text: &str) -> Vec<&str> {
    text.split_whitespace()
        .filter(|token| has_word_like_char(token))
        .collect()
}

fn has_cjk_content(text: &str) -> bool {
    text.chars().any(is_cjk)
}

fn is_plain_latin_multi_token_phrase(text: &str) -> bool {
    let tokens = content_tokens(text);
    tokens.len() > 1
        && tokens
            .iter()
            .all(|token| token.chars().all(|c| c.is_ascii_alphabetic()))
}

fn has_dictionary_phrase_signal(text: &str) -> bool {
    has_technical_marker(text) || has_titlecase_phrase_signal(text)
}

fn has_technical_marker(text: &str) -> bool {
    text.chars()
        .any(|c| c.is_ascii_digit() || matches!(c, '_' | '-' | '.' | '+' | '#' | '/' | '&'))
        || content_tokens(text).iter().any(|token| {
            is_acronym_like_token(token)
                || has_internal_uppercase(token)
                || is_known_technical_token(token)
        })
}

fn has_titlecase_phrase_signal(text: &str) -> bool {
    let tokens = content_tokens(text);
    tokens.len() > 1
        && tokens
            .iter()
            .filter(|token| is_titlecase_token(token))
            .count()
            >= 2
}

fn is_known_technical_token(token: &str) -> bool {
    KNOWN_TECHNICAL_TOKENS
        .iter()
        .any(|known| token.eq_ignore_ascii_case(known))
}

fn is_acronym_like_token(token: &str) -> bool {
    let mut has_letter = false;
    let mut uppercase_count = 0usize;
    for c in token.chars() {
        if c.is_ascii_alphabetic() {
            has_letter = true;
            if c.is_ascii_uppercase() {
                uppercase_count += 1;
            } else {
                return false;
            }
        } else if !c.is_ascii_digit() {
            return false;
        }
    }

    has_letter && uppercase_count >= 2
}

fn has_internal_uppercase(token: &str) -> bool {
    token
        .chars()
        .enumerate()
        .any(|(index, c)| index > 0 && c.is_ascii_uppercase())
}

fn is_titlecase_token(token: &str) -> bool {
    let mut chars = token.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_uppercase() && chars.any(|c| c.is_ascii_lowercase())
}

fn is_single_cjk_term(text: &str) -> bool {
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    chars.next().is_none() && is_cjk(first)
}

fn contains_sentence_boundary_punctuation(text: &str) -> bool {
    let chars = text.chars().collect::<Vec<_>>();
    chars.iter().enumerate().any(|(index, c)| {
        if *c == '.' {
            let previous = index.checked_sub(1).and_then(|i| chars.get(i)).copied();
            let next = chars.get(index + 1).copied();
            return !matches!(
                (previous, next),
                (Some(left), Some(right)) if left.is_ascii_alphanumeric() && right.is_ascii_alphanumeric()
            );
        }

        is_sentence_boundary_punctuation(*c)
    })
}

fn is_sentence_boundary_punctuation(c: char) -> bool {
    matches!(
        c,
        ',' | '.'
            | '!'
            | '?'
            | ';'
            | ':'
            | '"'
            | '\''
            | '('
            | ')'
            | '['
            | ']'
            | '{'
            | '}'
            | '，'
            | '。'
            | '、'
            | '！'
            | '？'
            | '；'
            | '：'
            | '（'
            | '）'
            | '【'
            | '】'
            | '「'
            | '」'
            | '『'
            | '』'
            | '《'
            | '》'
            | '“'
            | '”'
            | '‘'
            | '’'
            | '…'
            | '—'
    )
}

fn expand_cjk_single_char_replacement(
    before: &[char],
    after: &[char],
    before_start: &mut usize,
    after_start: &mut usize,
    before_end: usize,
    after_end: usize,
) {
    if *before_start == 0 || *after_start == 0 {
        return;
    }

    let before_changed_len = before_end.saturating_sub(*before_start);
    let after_changed_len = after_end.saturating_sub(*after_start);
    if before_changed_len != 1 || after_changed_len != 1 {
        return;
    }

    if !is_cjk(before[*before_start]) || !is_cjk(after[*after_start]) {
        return;
    }

    let before_prev = before[*before_start - 1];
    let after_prev = after[*after_start - 1];
    if before_prev == after_prev && is_cjk(before_prev) {
        *before_start -= 1;
        *after_start -= 1;
    }
}

fn expand_ascii_word_replacement(
    before: &[char],
    after: &[char],
    before_start: &mut usize,
    after_start: &mut usize,
    before_end: &mut usize,
    after_end: &mut usize,
) {
    if *before_start == *before_end || *after_start == *after_end {
        return;
    }

    if !has_ascii_word_change_or_context(before, *before_start, *before_end)
        || !has_ascii_word_change_or_context(after, *after_start, *after_end)
    {
        return;
    }

    while *before_start > 0
        && *after_start > 0
        && before[*before_start - 1] == after[*after_start - 1]
        && is_ascii_word_char(before[*before_start - 1])
    {
        *before_start -= 1;
        *after_start -= 1;
    }

    while *before_end < before.len()
        && *after_end < after.len()
        && before[*before_end] == after[*after_end]
        && is_ascii_word_char(before[*before_end])
    {
        *before_end += 1;
        *after_end += 1;
    }
}

fn expand_ascii_deleted_term(before: &[char], before_start: &mut usize, before_end: &mut usize) {
    if *before_start == *before_end {
        return;
    }

    let starts_inside_ascii_word = *before_start > 0
        && before
            .get(*before_start)
            .copied()
            .is_some_and(is_ascii_word_char)
        && before
            .get(*before_start - 1)
            .copied()
            .is_some_and(is_ascii_word_char);
    if starts_inside_ascii_word {
        while *before_start > 0 && is_ascii_word_char(before[*before_start - 1]) {
            *before_start -= 1;
        }
    }

    let ends_inside_ascii_word = *before_end > 0
        && before
            .get(*before_end - 1)
            .copied()
            .is_some_and(is_ascii_word_char)
        && before
            .get(*before_end)
            .copied()
            .is_some_and(is_ascii_word_char);
    if ends_inside_ascii_word {
        if before[*before_start..*before_end]
            .iter()
            .any(|c| c.is_whitespace())
        {
            while *before_end > *before_start && is_ascii_word_char(before[*before_end - 1]) {
                *before_end -= 1;
            }
        } else {
            while *before_end < before.len() && is_ascii_word_char(before[*before_end]) {
                *before_end += 1;
            }
        }
    }
}

fn is_ascii_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.')
}

fn has_ascii_word_change(chars: &[char], start: usize, end: usize) -> bool {
    start < end && chars[start..end].iter().copied().any(is_ascii_word_char)
}

fn has_ascii_word_change_or_context(chars: &[char], start: usize, end: usize) -> bool {
    has_ascii_word_change(chars, start, end)
        || start
            .checked_sub(1)
            .and_then(|index| chars.get(index))
            .copied()
            .is_some_and(is_ascii_word_char)
        || chars.get(end).copied().is_some_and(is_ascii_word_char)
}

fn is_cjk(c: char) -> bool {
    matches!(
        c as u32,
        0x3400..=0x4DBF
            | 0x4E00..=0x9FFF
            | 0xF900..=0xFAFF
            | 0x3040..=0x30FF
            | 0xAC00..=0xD7AF
    )
}

#[cfg(test)]
mod tests {
    use super::{extract_correction_pair, extract_deleted_correction_term};

    #[test]
    fn extracts_ascii_word_replacement() {
        let pair = extract_correction_pair(
            "Please ship this with OpenAI tomorrow",
            "Please ship this with OpenRouter tomorrow",
        )
        .unwrap();

        assert_eq!(pair.wrong, "OpenAI");
        assert_eq!(pair.corrected, "OpenRouter");
    }

    #[test]
    fn expands_single_cjk_character_to_term_pair() {
        let pair = extract_correction_pair(
            "这个分析错误可能是由于标点引起的",
            "这个分词错误可能是由于标点引起的",
        )
        .unwrap();

        assert_eq!(pair.wrong, "分析");
        assert_eq!(pair.corrected, "分词");
    }

    #[test]
    fn extracts_cjk_to_ascii_product_name_replacement() {
        let pair = extract_correction_pair(
            "那你试一试搜题现在的功能是不是符合预期的？",
            "那你试一试sootie现在的功能是不是符合预期的？",
        )
        .unwrap();

        assert_eq!(pair.wrong, "搜题");
        assert_eq!(pair.corrected, "sootie");
    }

    #[test]
    fn ignores_insert_only_edits() {
        assert!(extract_correction_pair("hello", "hello world").is_none());
    }

    #[test]
    fn ignores_punctuation_only_edits() {
        assert!(extract_correction_pair("hello.", "hello!").is_none());
    }

    #[test]
    fn ignores_unicode_punctuation_only_edits() {
        assert!(extract_correction_pair("你好，", "你好。").is_none());
    }

    #[test]
    fn ignores_sentence_to_ui_label_edits() {
        assert!(extract_correction_pair(
            "运行一下这个recipe，看看效果",
            "Ask for follow-up changes"
        )
        .is_none());
    }

    #[test]
    fn ignores_single_cjk_character_replacements() {
        assert!(extract_correction_pair("我", "而不").is_none());
    }

    #[test]
    fn accepts_technical_terms_with_internal_punctuation() {
        let pair = extract_correction_pair("请使用 Node js 运行", "请使用 Node.js 运行").unwrap();

        assert_eq!(pair.wrong, "Node js");
        assert_eq!(pair.corrected, "Node.js");
    }

    #[test]
    fn trims_sentence_punctuation_from_term_pairs() {
        let pair = extract_correction_pair("请使用搜题。", "请使用sootie.").unwrap();

        assert_eq!(pair.wrong, "搜题");
        assert_eq!(pair.corrected, "sootie");
    }

    #[test]
    fn rejects_plain_multi_token_sentence_fragments() {
        assert!(extract_correction_pair("this is wrong text", "that is right text").is_none());
    }

    #[test]
    fn rejects_plain_two_token_sentence_fragments() {
        assert!(extract_correction_pair("delete this", "new text").is_none());
    }

    #[test]
    fn accepts_titlecase_product_phrase_replacement() {
        let pair = extract_correction_pair("Try Air Tap here", "Try AriaType here").unwrap();

        assert_eq!(pair.wrong, "Air Tap");
        assert_eq!(pair.corrected, "AriaType");
    }

    #[test]
    fn accepts_multi_token_terms_with_technical_markers() {
        let pair =
            extract_correction_pair("Use open ai api key here", "Use OpenAI API key here").unwrap();

        assert_eq!(pair.wrong, "open ai api");
        assert_eq!(pair.corrected, "OpenAI API");
    }

    #[test]
    fn rejects_cjk_sentence_rewrite_without_boundary_punctuation() {
        assert!(extract_correction_pair("帮我打开设置页面", "现在开始处理任务").is_none());
    }

    #[test]
    fn accepts_compact_cjk_domain_terms() {
        let pair = extract_correction_pair("请打开上海浦东航站楼", "请打开北京朝阳航站楼").unwrap();

        assert_eq!(pair.wrong, "上海浦东");
        assert_eq!(pair.corrected, "北京朝阳");
    }

    #[test]
    fn identifies_word_level_pairs_only() {
        assert!(super::is_word_level_correction_pair("搜题", "sootie"));
        assert!(super::is_word_level_correction_pair("C++", "Rust"));
        assert!(!super::is_word_level_correction_pair("，", "。"));
        assert!(!super::is_word_level_correction_pair("hello!", "hi!"));
    }

    #[test]
    fn extracts_deleted_correction_term_for_pending_replacement() {
        let deleted = extract_deleted_correction_term(
            "Please try Air Tap in this field",
            "Please try  in this field",
        )
        .unwrap();

        assert_eq!(deleted, "Air Tap");
    }

    #[test]
    fn extracts_deleted_term_when_adjacent_context_shares_prefix() {
        let deleted =
            extract_deleted_correction_term("Before Air Tap After", "Before After").unwrap();

        assert_eq!(deleted, "Air Tap");
    }

    #[test]
    fn extracts_deleted_technical_phrase_for_pending_replacement() {
        let deleted = extract_deleted_correction_term(
            "Please configure open ai api before launch",
            "Please configure  before launch",
        )
        .unwrap();

        assert_eq!(deleted, "open ai api");
    }

    #[test]
    fn rejects_deleted_sentence_as_pending_replacement_term() {
        assert!(extract_deleted_correction_term(
            "Please delete this whole sentence. Keep this.",
            "Keep this."
        )
        .is_none());
    }

    #[test]
    fn rejects_deleted_plain_sentence_fragment_as_pending_replacement_term() {
        assert!(extract_deleted_correction_term(
            "Please replace one short sentence today",
            "Please replace  today"
        )
        .is_none());
        assert!(
            extract_deleted_correction_term("Before delete this After", "Before  After").is_none()
        );
    }

    #[test]
    fn rejects_completed_replacement_as_deleted_term() {
        assert!(extract_deleted_correction_term(
            "Please try Air Tap in this field",
            "Please try AriaType in this field",
        )
        .is_none());
    }
}
