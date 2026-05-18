use super::types::CorrectionPair;
use similar::{ChangeTag, TextDiff};

const MAX_CORRECTION_CHARS: usize = 80;

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
    let wrong = normalize_term(wrong);
    let corrected = normalize_term(corrected);

    if wrong.is_empty()
        || corrected.is_empty()
        || wrong == corrected
        || wrong.contains('\n')
        || corrected.contains('\n')
        || wrong.chars().count() > MAX_CORRECTION_CHARS
        || corrected.chars().count() > MAX_CORRECTION_CHARS
        || !has_word_like_char(&wrong)
        || !has_word_like_char(&corrected)
    {
        return None;
    }

    Some(CorrectionPair::new(wrong, corrected))
}

fn normalize_term(text: String) -> String {
    text.trim()
        .trim_matches(is_sentence_boundary_punctuation)
        .trim()
        .to_string()
}

fn has_word_like_char(text: &str) -> bool {
    text.chars().any(|c| c.is_alphanumeric() || is_cjk(c))
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
    if !has_ascii_word_change(before, *before_start, *before_end)
        || !has_ascii_word_change(after, *after_start, *after_end)
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

fn is_ascii_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.')
}

fn has_ascii_word_change(chars: &[char], start: usize, end: usize) -> bool {
    start < end && chars[start..end].iter().copied().any(is_ascii_word_char)
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
    use super::extract_correction_pair;

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
    fn trims_sentence_punctuation_from_term_pairs() {
        let pair = extract_correction_pair("请使用搜题。", "请使用sootie.").unwrap();

        assert_eq!(pair.wrong, "搜题");
        assert_eq!(pair.corrected, "sootie");
    }

    #[test]
    fn identifies_word_level_pairs_only() {
        assert!(super::is_word_level_correction_pair("搜题", "sootie"));
        assert!(super::is_word_level_correction_pair("C++", "Rust"));
        assert!(!super::is_word_level_correction_pair("，", "。"));
        assert!(!super::is_word_level_correction_pair("hello!", "hi!"));
    }
}
