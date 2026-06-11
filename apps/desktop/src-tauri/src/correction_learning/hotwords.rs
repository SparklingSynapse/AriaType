use std::collections::HashSet;

use pinyin::ToPinyin;

use super::types::{CorrectionApplyResult, CorrectionPair};

pub const CUSTOM_DICTIONARY_SOURCE: &str = "custom_dictionary";
pub const HOTWORD_REPLACEMENT_THRESHOLD: f64 = 0.85;
const MAX_HOTWORD_WINDOW_TOKENS: usize = 4;
const MAX_CJK_WINDOW_CHARS: usize = 6;
const MIN_FUZZY_KEY_LEN: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HotwordEntry {
    pub term: String,
    pub aliases: Vec<String>,
    pub frequency: u32,
    pub source: String,
}

impl HotwordEntry {
    pub fn new(
        term: impl Into<String>,
        aliases: Vec<String>,
        frequency: u32,
        source: impl Into<String>,
    ) -> Result<Self, String> {
        let term = normalize_hotword_term(&term.into())?;
        let aliases = normalize_aliases(&term, aliases);

        Ok(Self {
            term,
            aliases,
            frequency,
            source: source.into(),
        })
    }
}

#[derive(Debug, Clone)]
struct CandidateSpan {
    start: usize,
    end: usize,
    text: String,
    key: String,
}

#[derive(Debug, Clone)]
struct ReplacementMatch {
    start: usize,
    end: usize,
    original: String,
    replacement: String,
    score: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CsvHeaderKind {
    None,
    Hotword,
    LegacyMapping,
}

pub fn normalize_hotword_term(term: &str) -> Result<String, String> {
    let term = term.trim().to_string();
    if term.is_empty() {
        return Err("dictionary entry must include a word".to_string());
    }

    if !term.chars().any(is_hotword_content_char) {
        return Err("dictionary entry must include text content".to_string());
    }

    Ok(term)
}

pub fn parse_custom_hotword_entries(content: &str) -> Vec<HotwordEntry> {
    let mut entries = Vec::new();
    for raw_entry in split_hotword_entries(content) {
        if let Some(entry) = parse_hotword_entry(&raw_entry, CUSTOM_DICTIONARY_SOURCE, 1) {
            upsert_hotword_entry(&mut entries, entry);
        }
    }
    entries.sort_by(|left, right| left.term.cmp(&right.term));
    entries
}

pub fn serialize_custom_hotword_entries(entries: &[HotwordEntry]) -> String {
    entries
        .iter()
        .map(|entry| {
            let mut parts = vec![entry.term.trim().to_string()];
            parts.extend(entry.aliases.iter().map(|alias| alias.trim().to_string()));
            parts
                .into_iter()
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>()
                .join(" | ")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn detect_csv_header(row: &[String]) -> CsvHeaderKind {
    let normalized = row
        .iter()
        .map(|value| value.trim().to_lowercase())
        .collect::<Vec<_>>();
    let first = normalized.first().map(String::as_str);
    let second = normalized.get(1).map(String::as_str);

    match (first, second) {
        (Some("wrong"), Some("corrected"))
        | (Some("heard"), Some("write"))
        | (Some("heard as"), Some("write as"))
        | (Some("from"), Some("to")) => CsvHeaderKind::LegacyMapping,
        (Some("write as"), _)
        | (Some("term"), _)
        | (Some("word"), _)
        | (Some("hotword"), _)
        | (Some("phrase"), _) => CsvHeaderKind::Hotword,
        _ => CsvHeaderKind::None,
    }
}

pub fn hotword_entry_from_csv_row(
    row: &[String],
    header: CsvHeaderKind,
) -> Result<HotwordEntry, String> {
    let term_index = usize::from(header == CsvHeaderKind::LegacyMapping);
    let Some(term) = row.get(term_index) else {
        return Err("dictionary row is missing a word".to_string());
    };

    let aliases = if header == CsvHeaderKind::LegacyMapping {
        row.first().cloned().into_iter().collect()
    } else {
        row.iter().skip(1).cloned().collect()
    };

    HotwordEntry::new(term, aliases, 1, CUSTOM_DICTIONARY_SOURCE)
}

pub fn upsert_hotword_entry(entries: &mut Vec<HotwordEntry>, incoming: HotwordEntry) -> bool {
    if let Some(existing) = entries
        .iter_mut()
        .find(|entry| entry.term.eq_ignore_ascii_case(&incoming.term))
    {
        let before = existing.aliases.clone();
        for alias in incoming.aliases {
            if !existing
                .aliases
                .iter()
                .any(|existing_alias| existing_alias.eq_ignore_ascii_case(&alias))
            {
                existing.aliases.push(alias);
            }
        }
        existing.frequency = existing.frequency.max(incoming.frequency);
        return before != existing.aliases;
    }

    entries.push(incoming);
    true
}

pub fn apply_hotwords_to_text(text: &str, entries: &[HotwordEntry]) -> CorrectionApplyResult {
    if text.is_empty() || entries.is_empty() {
        return CorrectionApplyResult {
            text: text.to_string(),
            applied: Vec::new(),
        };
    }

    let candidates = build_candidate_spans(text);
    let mut matches = Vec::new();

    for candidate in candidates {
        if let Some(best_match) = best_hotword_match(&candidate, entries) {
            matches.push(best_match);
        }
    }

    matches.sort_by(|left, right| {
        left.start
            .cmp(&right.start)
            .then_with(|| right.score.total_cmp(&left.score))
            .then_with(|| (right.end - right.start).cmp(&(left.end - left.start)))
    });

    let mut selected = Vec::new();
    let mut covered_until = 0;
    for candidate in matches {
        if candidate.start < covered_until {
            continue;
        }
        covered_until = candidate.end;
        selected.push(candidate);
    }

    if selected.is_empty() {
        return CorrectionApplyResult {
            text: text.to_string(),
            applied: Vec::new(),
        };
    }

    let mut output = String::with_capacity(text.len());
    let mut applied = Vec::new();
    let mut cursor = 0;
    for replacement in selected {
        output.push_str(&text[cursor..replacement.start]);
        output.push_str(&replacement.replacement);
        applied.push(CorrectionPair::new(
            replacement.original,
            replacement.replacement,
        ));
        cursor = replacement.end;
    }
    output.push_str(&text[cursor..]);

    CorrectionApplyResult {
        text: output,
        applied,
    }
}

fn split_hotword_entries(content: &str) -> Vec<String> {
    content
        .split(['\n', '\r', ',', '，', ';', '；'])
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn parse_hotword_entry(entry: &str, source: &str, frequency: u32) -> Option<HotwordEntry> {
    for separator in ["->", "=>", "→"] {
        if let Some((wrong, corrected)) = entry.split_once(separator) {
            return HotwordEntry::new(corrected, vec![wrong.to_string()], frequency, source).ok();
        }
    }

    let parts = entry
        .split('|')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let term = parts.first()?;
    let aliases = parts.iter().skip(1).cloned().collect::<Vec<_>>();
    HotwordEntry::new(term, aliases, frequency, source).ok()
}

fn normalize_aliases(term: &str, aliases: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    aliases
        .into_iter()
        .map(|alias| alias.trim().to_string())
        .filter(|alias| !alias.is_empty())
        .filter(|alias| !alias.eq_ignore_ascii_case(term))
        .filter(|alias| seen.insert(alias.to_lowercase()))
        .collect()
}

fn best_hotword_match(
    candidate: &CandidateSpan,
    entries: &[HotwordEntry],
) -> Option<ReplacementMatch> {
    let mut best: Option<ReplacementMatch> = None;

    for entry in entries {
        if entry.term == candidate.text {
            continue;
        }

        let score = hotword_similarity(&candidate.key, entry);
        if score < HOTWORD_REPLACEMENT_THRESHOLD {
            continue;
        }

        let replacement = ReplacementMatch {
            start: candidate.start,
            end: candidate.end,
            original: candidate.text.clone(),
            replacement: entry.term.clone(),
            score,
        };

        let should_replace = best.as_ref().is_none_or(|current| {
            replacement.score > current.score
                || (replacement.score == current.score
                    && replacement.replacement.chars().count()
                        > current.replacement.chars().count())
        });

        if should_replace {
            best = Some(replacement);
        }
    }

    best
}

fn hotword_similarity(candidate_key: &str, entry: &HotwordEntry) -> f64 {
    std::iter::once(entry.term.as_str())
        .chain(entry.aliases.iter().map(String::as_str))
        .map(phonetic_key)
        .filter(|key| key.len() >= MIN_FUZZY_KEY_LEN)
        .map(|key| normalized_similarity(candidate_key, &key))
        .fold(0.0, f64::max)
}

fn build_candidate_spans(text: &str) -> Vec<CandidateSpan> {
    let tokens = tokenize_text(text);
    let mut seen = HashSet::new();
    let mut spans = Vec::new();

    for start_index in 0..tokens.len() {
        let max_end = (start_index + MAX_HOTWORD_WINDOW_TOKENS).min(tokens.len());
        for end_index in start_index..max_end {
            let start = tokens[start_index].0;
            let end = tokens[end_index].1;
            push_candidate_span(text, start, end, &mut seen, &mut spans);
        }
    }

    for (start, end) in tokens {
        let token = &text[start..end];
        if !token.chars().all(is_cjk) {
            continue;
        }

        let chars = token.char_indices().collect::<Vec<_>>();
        for char_start in 0..chars.len() {
            let max_len = MAX_CJK_WINDOW_CHARS.min(chars.len() - char_start);
            for len in 2..=max_len {
                let byte_start = start + chars[char_start].0;
                let byte_end = if char_start + len >= chars.len() {
                    end
                } else {
                    start + chars[char_start + len].0
                };
                push_candidate_span(text, byte_start, byte_end, &mut seen, &mut spans);
            }
        }
    }

    spans
}

fn push_candidate_span(
    text: &str,
    start: usize,
    end: usize,
    seen: &mut HashSet<(usize, usize)>,
    spans: &mut Vec<CandidateSpan>,
) {
    if start >= end || !seen.insert((start, end)) {
        return;
    }

    let span_text = text[start..end].trim();
    if span_text.is_empty() {
        return;
    }

    let key = phonetic_key(span_text);
    if key.len() < MIN_FUZZY_KEY_LEN {
        return;
    }

    spans.push(CandidateSpan {
        start,
        end,
        text: span_text.to_string(),
        key,
    });
}

fn tokenize_text(text: &str) -> Vec<(usize, usize)> {
    let mut tokens = Vec::new();
    let mut start = None;

    for (index, c) in text.char_indices() {
        if is_token_char(c) {
            if start.is_none() {
                start = Some(index);
            }
            continue;
        }

        if let Some(token_start) = start.take() {
            tokens.push((token_start, index));
        }
    }

    if let Some(token_start) = start {
        tokens.push((token_start, text.len()));
    }

    tokens
}

fn phonetic_key(text: &str) -> String {
    let mut key = String::new();

    for c in text.chars() {
        if is_cjk(c) {
            if let Some(pinyin) = c.to_pinyin() {
                key.push_str(pinyin.plain());
            } else {
                key.push(c);
            }
            continue;
        }

        if c.is_ascii_alphanumeric() {
            key.push(c.to_ascii_lowercase());
            continue;
        }

        if c.is_alphanumeric() {
            for lower in c.to_lowercase() {
                key.push(lower);
            }
        }
    }

    fold_phonetic_key(&key)
}

fn fold_phonetic_key(key: &str) -> String {
    let mut folded = key.to_string();
    for (from, to) in [
        ("iong", "iung"),
        ("uang", "wang"),
        ("iang", "yang"),
        ("ou", "u"),
        ("oo", "u"),
        ("ee", "i"),
        ("ea", "i"),
        ("ie", "i"),
    ] {
        folded = folded.replace(from, to);
    }
    folded
}

fn normalized_similarity(left: &str, right: &str) -> f64 {
    if left == right {
        return 1.0;
    }
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }

    let distance = levenshtein_distance(left, right);
    1.0 - (distance as f64 / left.chars().count().max(right.chars().count()) as f64)
}

fn levenshtein_distance(left: &str, right: &str) -> usize {
    let right_len = right.chars().count();
    let mut previous = (0..=right_len).collect::<Vec<_>>();
    let mut current = vec![0; right_len + 1];

    for (left_index, left_char) in left.chars().enumerate() {
        current[0] = left_index + 1;
        for (right_index, right_char) in right.chars().enumerate() {
            let deletion = previous[right_index + 1] + 1;
            let insertion = current[right_index] + 1;
            let substitution = previous[right_index] + usize::from(left_char != right_char);
            current[right_index + 1] = deletion.min(insertion).min(substitution);
        }
        std::mem::swap(&mut previous, &mut current);
    }

    previous[right_len]
}

fn is_token_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || is_cjk(c)
}

fn is_hotword_content_char(c: char) -> bool {
    c.is_alphanumeric() || is_cjk(c)
}

fn is_cjk(c: char) -> bool {
    matches!(
        c as u32,
        0x4E00..=0x9FFF
            | 0x3400..=0x4DBF
            | 0x20000..=0x2A6DF
            | 0x2A700..=0x2B73F
            | 0x2B740..=0x2B81F
            | 0x2B820..=0x2CEAF
            | 0xF900..=0xFAFF
            | 0x2F800..=0x2FA1F
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hotword_lines_and_legacy_mappings() {
        let entries = parse_custom_hotword_entries("Claude | Cloud\n搜题 -> sootie\nAriaType");

        assert_eq!(entries.len(), 3);
        assert!(entries
            .iter()
            .any(|entry| entry.term == "Claude" && entry.aliases == vec!["Cloud"]));
        assert!(entries
            .iter()
            .any(|entry| entry.term == "sootie" && entry.aliases == vec!["搜题"]));
        assert!(entries.iter().any(|entry| entry.term == "AriaType"));
    }

    #[test]
    fn applies_pinyin_fuzzy_hotword_replacement() {
        let entries = vec![HotwordEntry::new("sootie", Vec::new(), 1, "test").unwrap()];
        let result = apply_hotwords_to_text("打开搜题", &entries);

        assert_eq!(result.text, "打开sootie");
        assert_eq!(result.applied, vec![CorrectionPair::new("搜题", "sootie")]);
    }

    #[test]
    fn applies_alias_hotword_replacement() {
        let entries =
            vec![HotwordEntry::new("Claude", vec!["Cloud".to_string()], 1, "test").unwrap()];
        let result = apply_hotwords_to_text("ask Cloud", &entries);

        assert_eq!(result.text, "ask Claude");
        assert_eq!(result.applied, vec![CorrectionPair::new("Cloud", "Claude")]);
    }
}
