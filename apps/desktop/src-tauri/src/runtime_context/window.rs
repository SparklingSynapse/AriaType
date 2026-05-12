const MAX_CONTEXT_CHARS: usize = 800;
const MAX_STT_CONTEXT_CHARS: usize = 360;
const MAX_POLISH_CONTEXT_CHARS: usize = 900;
const MAX_STT_TERMS: usize = 18;
const MAX_TOPIC_KEYWORDS: usize = 12;
const MIN_TOPIC_KEYWORD_COUNT: usize = 2;
const FULL_CONTEXT_CONFIDENCE_RATIO: f64 = 0.80;
const MINIMAL_CONTEXT_CONFIDENCE_RATIO: f64 = 0.65;
const STRUCTURED_TERM_SUFFIXES: &[&str] = &[
    ".rs", ".md", ".json", ".toml", ".yaml", ".yml", ".png", ".jpg", ".jpeg", ".webp", ".com",
    ".io", ".dev", ".app",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowContextSource {
    FocusedWindow,
    PrimaryMonitor,
}

impl WindowContextSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::FocusedWindow => "focused_window",
            Self::PrimaryMonitor => "primary_monitor",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WindowContextBundle {
    pub raw_ocr_text: String,
    pub filtered_text: String,
    pub source: WindowContextSource,
    pub window_title: Option<String>,
    pub image_width: u32,
    pub image_height: u32,
    pub ocr_confidence: Option<OcrConfidenceSummary>,
    pub captured_at_ms: i64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OcrConfidenceSummary {
    pub average: f64,
    pub max: f64,
    pub observations: usize,
    pub provider_raw: Option<f64>,
}

impl OcrConfidenceSummary {
    pub fn new(
        average: f64,
        max: f64,
        observations: usize,
        provider_raw: Option<f64>,
    ) -> Option<Self> {
        if !average.is_finite() || !max.is_finite() || max <= 0.0 {
            return None;
        }

        Some(Self {
            average: average.clamp(0.0, max),
            max,
            observations,
            provider_raw,
        })
    }

    pub fn from_single_score(score: f64) -> Option<Self> {
        let max = if score <= 1.0 { 1.0 } else { 100.0 };
        Self::new(score, max, 0, Some(score))
    }

    fn to_context_line(self) -> String {
        let mut line = format!("OCR confidence: avg={:.2}/{:.2}", self.average, self.max);
        if self.observations > 0 {
            line.push_str(&format!(", observations={}", self.observations));
        }
        line
    }

    fn ratio(self) -> f64 {
        if self.max <= 0.0 {
            0.0
        } else {
            (self.average / self.max).clamp(0.0, 1.0)
        }
    }
}

impl WindowContextBundle {
    pub fn from_ocr_result(
        raw_ocr_text: impl Into<String>,
        source: WindowContextSource,
        window_title: Option<String>,
        image_width: u32,
        image_height: u32,
        ocr_confidence: Option<f64>,
    ) -> Option<Self> {
        Self::from_ocr_result_with_confidence(
            raw_ocr_text,
            source,
            window_title,
            image_width,
            image_height,
            ocr_confidence.and_then(OcrConfidenceSummary::from_single_score),
        )
    }

    pub fn from_ocr_result_with_confidence(
        raw_ocr_text: impl Into<String>,
        source: WindowContextSource,
        window_title: Option<String>,
        image_width: u32,
        image_height: u32,
        ocr_confidence: Option<OcrConfidenceSummary>,
    ) -> Option<Self> {
        let raw_ocr_text = raw_ocr_text.into();
        let filtered_text = normalize_ocr_text(&raw_ocr_text, MAX_CONTEXT_CHARS);
        if filtered_text.is_empty() {
            return None;
        }

        Some(Self {
            raw_ocr_text: raw_ocr_text.trim().to_string(),
            filtered_text,
            source,
            window_title: clean_optional_text(window_title),
            image_width,
            image_height,
            ocr_confidence,
            captured_at_ms: chrono::Utc::now().timestamp_millis(),
        })
    }

    pub fn to_stt_prompt_hint(&self) -> Option<String> {
        let mut parts = Vec::new();
        let detail = self.context_detail();

        if let Some(title) = self.window_title.as_deref() {
            parts.push(format!("Active window: {title}"));
        }

        let terms = extract_terms(&self.filtered_text, detail.max_terms());
        if !terms.is_empty() {
            parts.push(format!("Candidate visible terms: {}", terms.join(", ")));
        }

        let keywords = extract_frequent_keywords(&self.filtered_text, detail.max_keywords());
        if !keywords.is_empty() {
            parts.push(format!(
                "Frequent context keywords: {}",
                keywords.join(", ")
            ));
        }

        if detail.include_visible_text() {
            let nearby_text = truncate_chars(&self.filtered_text, MAX_STT_CONTEXT_CHARS);
            if !nearby_text.is_empty() {
                parts.push(format!("Nearby visible text: {nearby_text}"));
            }
        }

        if parts.is_empty() {
            None
        } else {
            Some(parts.join(". "))
        }
    }

    pub fn to_polish_context(&self) -> Option<String> {
        let detail = self.context_detail();
        let mut lines = vec![format!("Source: {}", self.source.as_str())];
        if let Some(title) = self.window_title.as_deref() {
            lines.push(format!("Window title: {title}"));
        }
        let terms = extract_terms(&self.filtered_text, detail.max_terms());
        if !terms.is_empty() {
            lines.push(format!("Candidate visible terms: {}", terms.join(", ")));
        }
        let keywords = extract_frequent_keywords(&self.filtered_text, detail.max_keywords());
        if !keywords.is_empty() {
            lines.push(format!(
                "Frequent context keywords: {}",
                keywords.join(", ")
            ));
        }
        if let Some(confidence) = self.ocr_confidence {
            lines.push(confidence.to_context_line());
        }
        if detail.include_visible_text() {
            let visible_text = truncate_chars(&self.filtered_text, MAX_POLISH_CONTEXT_CHARS);
            if !visible_text.is_empty() {
                lines.push(format!("Visible text:\n\"\"\"\n{visible_text}\n\"\"\""));
            }
        }

        if lines.len() <= 1 {
            None
        } else {
            Some(lines.join("\n"))
        }
    }

    fn context_detail(&self) -> ContextDetail {
        let Some(confidence) = self.ocr_confidence else {
            return ContextDetail::Full;
        };
        let ratio = confidence.ratio();
        if ratio >= FULL_CONTEXT_CONFIDENCE_RATIO {
            ContextDetail::Full
        } else if ratio >= MINIMAL_CONTEXT_CONFIDENCE_RATIO {
            ContextDetail::Conservative
        } else {
            ContextDetail::Minimal
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ContextDetail {
    Full,
    Conservative,
    Minimal,
}

impl ContextDetail {
    fn max_terms(self) -> usize {
        match self {
            Self::Full => MAX_STT_TERMS,
            Self::Conservative => 10,
            Self::Minimal => 6,
        }
    }

    fn max_keywords(self) -> usize {
        match self {
            Self::Full => MAX_TOPIC_KEYWORDS,
            Self::Conservative => 6,
            Self::Minimal => 3,
        }
    }

    fn include_visible_text(self) -> bool {
        matches!(self, Self::Full)
    }
}

fn clean_optional_text(value: Option<String>) -> Option<String> {
    value
        .map(|text| normalize_spaces(text.trim()))
        .filter(|text| !text.is_empty())
}

fn normalize_ocr_text(text: &str, max_chars: usize) -> String {
    let mut lines = Vec::new();
    let mut previous = String::new();

    for line in text.lines() {
        let normalized = normalize_spaces(line.trim());
        if normalized.is_empty() || normalized == previous {
            continue;
        }
        previous = normalized.clone();
        lines.push(normalized);
    }

    truncate_chars(&lines.join("\n"), max_chars)
}

fn normalize_spaces(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    let trimmed = text.trim();
    if trimmed.chars().count() > max_chars {
        trimmed.chars().take(max_chars).collect()
    } else {
        trimmed.to_string()
    }
}

fn extract_terms(text: &str, max_terms: usize) -> Vec<String> {
    let mut terms: Vec<(String, String, u8, usize)> = Vec::new();

    for (position, token) in split_context_tokens(text) {
        let term = normalize_candidate_term(&clean_context_token(token));
        let Some(score) = score_context_term(&term) else {
            continue;
        };

        let key = normalize_context_term_key(&term);
        if let Some((existing_term, _, existing_score, _)) = terms
            .iter_mut()
            .find(|(_, existing_key, _, _)| existing_key == &key)
        {
            if score > *existing_score {
                *existing_term = term;
                *existing_score = score;
            }
        } else {
            terms.push((term, key, score, position));
        }
    }

    terms.sort_by(|left, right| right.2.cmp(&left.2).then_with(|| left.3.cmp(&right.3)));
    terms
        .into_iter()
        .take(max_terms)
        .map(|(term, _, _, _)| term)
        .collect()
}

fn extract_frequent_keywords(text: &str, max_keywords: usize) -> Vec<String> {
    let mut keywords: Vec<(String, String, usize, usize)> = Vec::new();

    for (position, token) in split_context_tokens(text) {
        let term = normalize_candidate_term(&clean_context_token(token));
        if !looks_like_topic_keyword(&term) {
            continue;
        }

        let normalized = normalize_keyword(&term);
        if let Some((existing_term, existing_normalized, count, _)) = keywords
            .iter_mut()
            .find(|(_, existing, _, _)| keywords_match(existing, &normalized))
        {
            if should_replace_keyword_display(existing_normalized, &normalized) {
                *existing_term = term;
                *existing_normalized = normalized;
            }
            *count += 1;
        } else {
            keywords.push((term, normalized, 1, position));
        }
    }

    keywords.retain(|(_, _, count, _)| *count >= MIN_TOPIC_KEYWORD_COUNT);
    keywords.sort_by(|left, right| right.2.cmp(&left.2).then_with(|| left.3.cmp(&right.3)));
    keywords
        .into_iter()
        .take(max_keywords)
        .map(|(term, _, _, _)| term)
        .collect()
}

fn split_context_tokens(text: &str) -> impl Iterator<Item = (usize, &str)> {
    text.split(is_context_separator).enumerate()
}

fn is_context_separator(c: char) -> bool {
    c.is_whitespace()
        || matches!(
            c,
            ',' | ';'
                | ':'
                | '('
                | ')'
                | '['
                | ']'
                | '{'
                | '}'
                | '<'
                | '>'
                | '"'
                | '\''
                | '/'
                | '\\'
                | '|'
                | '='
                | '，'
                | '。'
                | '、'
                | '；'
                | '：'
                | '（'
                | '）'
        )
}

fn clean_context_token(token: &str) -> String {
    let token = token
        .trim_matches(|c: char| !is_allowed_term_char(c))
        .trim();
    trim_edge_term_punctuation(token).to_string()
}

fn normalize_candidate_term(term: &str) -> String {
    let without_numeric_prefix = term.trim_start_matches(|c: char| c.is_ascii_digit() || c == '.');
    let without_numeric_suffix = without_numeric_prefix
        .trim_end_matches(|c: char| c.is_ascii_digit() && has_mixed_case(without_numeric_prefix));
    let trimmed = trim_edge_term_punctuation(without_numeric_suffix);

    if let Some(structured_term) = truncate_after_structured_suffix(trimmed) {
        return structured_term;
    }
    if let Some(camel_term) = normalize_repeated_camel_segments(trimmed) {
        return camel_term;
    }
    if let Some(camel_term) = normalize_camel_noise_edges(trimmed) {
        return camel_term;
    }

    trimmed.to_string()
}

fn is_allowed_term_char(c: char) -> bool {
    c.is_alphanumeric() || c == '-' || c == '_' || c == '.'
}

fn trim_edge_term_punctuation(token: &str) -> &str {
    token.trim_matches(|c: char| matches!(c, '.' | '-' | '_'))
}

fn has_repeated_term_punctuation(term: &str) -> bool {
    let mut previous = None;
    for c in term.chars() {
        if matches!(c, '.' | '-' | '_') && previous == Some(c) {
            return true;
        }
        previous = Some(c);
    }
    false
}

fn score_context_term(term: &str) -> Option<u8> {
    let len = term.chars().count();
    if !(2..=40).contains(&len) {
        return None;
    }
    if !term.chars().all(is_allowed_term_char) {
        return None;
    }
    if has_repeated_term_punctuation(term) {
        return None;
    }
    if looks_like_numeric_token(term) || looks_like_machine_id(term) || is_common_context_word(term)
    {
        return None;
    }
    if looks_like_ocr_noise(term) {
        return None;
    }

    let has_uppercase = term.chars().any(|c| c.is_uppercase());
    let has_lowercase = term.chars().any(|c| c.is_lowercase());
    let has_numeric = term.chars().any(|c| c.is_numeric());
    let has_symbol = term.contains('-') || term.contains('_') || term.contains('.');
    let has_cjk = term.chars().any(is_cjk);
    let uppercase_count = term.chars().filter(|c| c.is_uppercase()).count();

    if has_cjk && len <= 12 {
        return Some(2);
    }
    if has_uppercase && has_lowercase && uppercase_count > 1 {
        return Some(5);
    }
    if term.chars().all(|c| c.is_ascii_uppercase()) && (2..=6).contains(&len) {
        return Some(4);
    }
    if has_symbol && (has_lowercase || has_uppercase) {
        return Some(3);
    }
    if has_numeric && (has_lowercase || has_uppercase) {
        return Some(3);
    }
    if is_title_case_word(term) {
        return Some(2);
    }

    None
}

fn looks_like_ocr_noise(term: &str) -> bool {
    looks_like_mixed_script_noise(term)
        || looks_like_digit_letter_noise(term)
        || looks_like_uppercase_run_noise(term)
        || looks_like_short_embedded_digit_noise(term)
        || looks_like_latin_digit_noise(term)
}

fn looks_like_numeric_token(term: &str) -> bool {
    term.chars()
        .all(|c| c.is_ascii_digit() || matches!(c, '.' | '-' | '_'))
}

fn looks_like_machine_id(term: &str) -> bool {
    let len = term.chars().count();
    if len < 12 {
        return false;
    }

    let machine_chars = term
        .chars()
        .filter(|c| c.is_ascii_hexdigit() || matches!(c, '-' | '_'))
        .count();
    machine_chars == len
}

fn looks_like_digit_letter_noise(term: &str) -> bool {
    if looks_like_file_or_domain(term) {
        return false;
    }

    let mut transitions = 0;
    let mut previous = None;
    for current in term.chars().filter_map(char_class_for_digit_noise) {
        if previous.is_some_and(|previous| previous != current) {
            transitions += 1;
        }
        previous = Some(current);
    }

    transitions >= 3
}

fn looks_like_short_embedded_digit_noise(term: &str) -> bool {
    if looks_like_file_or_domain(term) {
        return false;
    }

    let len = term.chars().count();
    len <= 5
        && term.chars().any(|c| c.is_ascii_digit())
        && term.chars().any(|c| c.is_ascii_lowercase())
        && term.chars().any(|c| c.is_ascii_alphabetic())
}

fn looks_like_latin_digit_noise(term: &str) -> bool {
    if looks_like_file_or_domain(term) {
        return false;
    }

    let has_digit = term.chars().any(|c| c.is_ascii_digit());
    let has_latin_extended = term
        .chars()
        .any(|c| (0x00C0..=0x024F).contains(&(c as u32)));
    has_digit && has_latin_extended
}

fn char_class_for_digit_noise(c: char) -> Option<u8> {
    if c.is_ascii_digit() {
        Some(0)
    } else if c.is_ascii_alphabetic() {
        Some(1)
    } else {
        None
    }
}

fn looks_like_uppercase_run_noise(term: &str) -> bool {
    if looks_like_file_or_domain(term) {
        return false;
    }

    let mut run = 0;
    let mut max_run = 0;
    for c in term.chars() {
        if c.is_ascii_uppercase() {
            run += 1;
            max_run = max_run.max(run);
        } else {
            run = 0;
        }
    }

    max_run >= 4 && term.chars().any(|c| c.is_ascii_lowercase())
}

fn looks_like_file_or_domain(term: &str) -> bool {
    let lower = term.to_ascii_lowercase();
    STRUCTURED_TERM_SUFFIXES
        .iter()
        .any(|suffix| lower.ends_with(suffix))
}

fn truncate_after_structured_suffix(term: &str) -> Option<String> {
    let lower = term.to_ascii_lowercase();
    for suffix in STRUCTURED_TERM_SUFFIXES {
        if let Some(index) = lower.find(suffix) {
            let end = index + suffix.len();
            let candidate = &term[..end];
            if candidate.chars().any(|c| c.is_ascii_alphabetic()) {
                return Some(candidate.to_string());
            }
        }
    }

    None
}

fn normalize_repeated_camel_segments(term: &str) -> Option<String> {
    if !term.chars().all(|c| c.is_ascii_alphanumeric()) {
        return None;
    }

    let segments = camel_segments(term);
    if segments.len() < 2 {
        return None;
    }

    for sequence_len in (2..=segments.len() / 2).rev() {
        for start in 0..=segments.len().saturating_sub(sequence_len * 2) {
            if camel_sequences_match(term, &segments, start, start + sequence_len, sequence_len) {
                let candidate = join_segments(term, &segments[start..start + sequence_len]);
                if candidate.chars().count() >= 6 && has_mixed_case(&candidate) {
                    return Some(candidate);
                }
            }
        }
    }

    if segments.len() >= 3
        && is_disposable_lowercase_prefix(term, segments[0], segments.len() - 1)
        && segments[1..]
            .iter()
            .all(|segment| is_title_ascii_segment(term, *segment))
    {
        let candidate = join_segments(term, &segments[1..]);
        if candidate.chars().count() >= 6 {
            return Some(candidate);
        }
    }

    None
}

fn normalize_camel_noise_edges(term: &str) -> Option<String> {
    if !term.chars().all(|c| c.is_ascii_alphanumeric()) {
        return None;
    }

    let segments = camel_segments(term);
    if segments.len() < 3 {
        return None;
    }

    let first = segment_text(term, segments[0]);
    let last = segment_text(term, *segments.last()?);
    if is_common_context_word(last) {
        let candidate = if first.len() == 1 && first.chars().all(|c| c.is_ascii_uppercase()) {
            join_segments(term, &segments[1..segments.len() - 1])
        } else if let Some(stripped_first) = strip_single_uppercase_noise_prefix(first) {
            format!(
                "{}{}",
                stripped_first,
                join_segments(term, &segments[1..segments.len() - 1])
            )
        } else {
            return None;
        };

        if candidate.chars().count() >= 6 && has_mixed_case(&candidate) {
            return Some(candidate);
        }
    }

    None
}

fn strip_single_uppercase_noise_prefix(segment: &str) -> Option<&str> {
    let mut chars = segment.chars();
    let first = chars.next()?;
    let second = chars.next()?;
    if !first.is_ascii_uppercase() || !second.is_ascii_uppercase() {
        return None;
    }
    if !chars.all(|c| c.is_ascii_lowercase()) {
        return None;
    }

    Some(&segment[first.len_utf8()..])
}

fn camel_segments(term: &str) -> Vec<(usize, usize)> {
    let mut segments = Vec::new();
    let mut start = 0;
    let mut previous = None;

    for (index, c) in term.char_indices() {
        if index > 0
            && c.is_ascii_uppercase()
            && previous.is_some_and(|p: char| p.is_ascii_lowercase())
        {
            segments.push((start, index));
            start = index;
        }
        previous = Some(c);
    }

    segments.push((start, term.len()));
    segments
}

fn camel_sequences_match(
    term: &str,
    segments: &[(usize, usize)],
    left_start: usize,
    right_start: usize,
    len: usize,
) -> bool {
    (0..len).all(|offset| {
        let left = segment_text(term, segments[left_start + offset]);
        let right = segment_text(term, segments[right_start + offset]);
        left.eq_ignore_ascii_case(right)
    })
}

fn join_segments(term: &str, segments: &[(usize, usize)]) -> String {
    let start = segments.first().map(|segment| segment.0).unwrap_or(0);
    let end = segments.last().map(|segment| segment.1).unwrap_or(start);
    term[start..end].to_string()
}

fn segment_text(term: &str, segment: (usize, usize)) -> &str {
    &term[segment.0..segment.1]
}

fn is_disposable_lowercase_prefix(
    term: &str,
    segment: (usize, usize),
    following_segments: usize,
) -> bool {
    let text = segment_text(term, segment);
    text.chars().all(|c| c.is_ascii_lowercase())
        && (text.len() >= 3 || text.len() == 1 && following_segments >= 2)
}

fn is_title_ascii_segment(term: &str, segment: (usize, usize)) -> bool {
    let text = segment_text(term, segment);
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    first.is_ascii_uppercase() && chars.all(|c| c.is_ascii_lowercase())
}

fn looks_like_topic_keyword(term: &str) -> bool {
    let len = term.chars().count();
    if !((3..=32).contains(&len) || term.chars().any(is_cjk) && (2..=12).contains(&len)) {
        return false;
    }
    if !term.chars().all(is_allowed_term_char) {
        return false;
    }
    if has_repeated_term_punctuation(term) {
        return false;
    }
    if looks_like_numeric_token(term)
        || looks_like_machine_id(term)
        || is_common_topic_word(term)
        || looks_like_long_uppercase_noise(term)
        || looks_like_ocr_noise(term)
    {
        return false;
    }

    term.chars().any(|c| c.is_alphabetic()) || term.chars().any(is_cjk)
}

fn normalize_keyword(term: &str) -> String {
    term.to_lowercase()
}

fn keywords_match(left: &str, right: &str) -> bool {
    if left == right {
        return true;
    }

    let left_len = left.chars().count();
    let right_len = right.chars().count();
    let min_len = left_len.min(right_len);
    if min_len < 6 {
        return false;
    }

    left.contains(right) || right.contains(left)
}

fn should_replace_keyword_display(existing: &str, candidate: &str) -> bool {
    let existing_len = existing.chars().count();
    let candidate_len = candidate.chars().count();
    candidate_len >= 6 && candidate_len < existing_len && existing.contains(candidate)
}

fn normalize_context_term_key(term: &str) -> String {
    term.to_lowercase()
}

fn has_mixed_case(term: &str) -> bool {
    term.chars().any(|c| c.is_ascii_uppercase()) && term.chars().any(|c| c.is_ascii_lowercase())
}

fn looks_like_mixed_script_noise(term: &str) -> bool {
    let mut has_ascii_latin = false;
    let mut has_latin_extended = false;
    let mut has_cyrillic = false;
    let mut has_greek = false;

    for c in term.chars().filter(|c| c.is_alphabetic()) {
        let code = c as u32;
        if c.is_ascii_alphabetic() {
            has_ascii_latin = true;
        } else if (0x00C0..=0x024F).contains(&code) {
            has_latin_extended = true;
        } else if (0x0370..=0x03FF).contains(&code) {
            has_greek = true;
        } else if (0x0400..=0x052F).contains(&code) {
            has_cyrillic = true;
        }
    }

    ((has_ascii_latin || has_latin_extended) && (has_cyrillic || has_greek))
        || (has_ascii_latin && has_latin_extended && looks_like_latin_extended_noise(term))
}

fn looks_like_latin_extended_noise(term: &str) -> bool {
    let len = term.chars().count();
    if len < 6 {
        return false;
    }

    let uppercase_count = term.chars().filter(|c| c.is_uppercase()).count();
    let latin_extended_count = term
        .chars()
        .filter(|c| (0x00C0..=0x024F).contains(&(*c as u32)))
        .count();

    uppercase_count >= 2 && latin_extended_count >= 1
}

fn looks_like_long_uppercase_noise(term: &str) -> bool {
    let len = term.chars().count();
    len > 6 && term.chars().all(|c| c.is_ascii_uppercase())
}

fn is_common_context_word(term: &str) -> bool {
    matches!(
        term.to_ascii_lowercase().as_str(),
        "about"
            | "active"
            | "add"
            | "after"
            | "all"
            | "api"
            | "app"
            | "apps"
            | "available"
            | "before"
            | "bookmarks"
            | "candidate"
            | "draft"
            | "faq"
            | "from"
            | "inspect"
            | "implement"
            | "jfk"
            | "language"
            | "lax"
            | "notes"
            | "or"
            | "output"
            | "plan"
            | "planv"
            | "please"
            | "pvg"
            | "release"
            | "reply"
            | "run"
            | "ran"
            | "select"
            | "send"
            | "the"
            | "this"
            | "utc"
            | "update"
            | "updated"
            | "user"
            | "users"
            | "waited"
            | "with"
            | "xapps"
    )
}

fn is_common_topic_word(term: &str) -> bool {
    matches!(
        term.to_ascii_lowercase().as_str(),
        "about"
            | "add"
            | "again"
            | "also"
            | "and"
            | "apps"
            | "are"
            | "background"
            | "bookmarks"
            | "but"
            | "candidate"
            | "can"
            | "confidence"
            | "desktop"
            | "dev"
            | "for"
            | "from"
            | "has"
            | "have"
            | "inspect"
            | "implement"
            | "into"
            | "language"
            | "lib"
            | "not"
            | "or"
            | "output"
            | "our"
            | "plan"
            | "planv"
            | "please"
            | "ran"
            | "select"
            | "run"
            | "src"
            | "that"
            | "the"
            | "this"
            | "users"
            | "was"
            | "waited"
            | "with"
            | "xapps"
            | "you"
            | "your"
    )
}

fn is_title_case_word(term: &str) -> bool {
    let mut chars = term.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    first.is_uppercase() && chars.clone().all(|c| c.is_lowercase()) && chars.count() >= 2
}

fn is_cjk(c: char) -> bool {
    matches!(
        c as u32,
        0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundle_filters_empty_and_duplicate_ocr_lines() {
        let bundle = WindowContextBundle::from_ocr_result(
            "  Project Alpha  \n\nProject Alpha\nSend update",
            WindowContextSource::FocusedWindow,
            Some("Mail".to_string()),
            1200,
            800,
            Some(0.92),
        )
        .unwrap();

        assert_eq!(bundle.filtered_text, "Project Alpha\nSend update");
        assert_eq!(bundle.window_title.as_deref(), Some("Mail"));
    }

    #[test]
    fn stt_hint_prefers_title_terms_and_short_visible_text() {
        let bundle = WindowContextBundle::from_ocr_result(
            "ProjectNebula 0.5 release notes\nVAD\nComputer Use\nworkflow prompt workflow prompt",
            WindowContextSource::FocusedWindow,
            Some("README.md".to_string()),
            1000,
            700,
            None,
        )
        .unwrap();

        let hint = bundle.to_stt_prompt_hint().unwrap();
        assert!(hint.contains("Active window: README.md"));
        assert!(hint.contains("Candidate visible terms:"));
        assert!(hint.contains("ProjectNebula"));
        assert!(hint.contains("Frequent context keywords:"));
        assert!(hint.contains("workflow"));
        assert!(hint.contains("prompt"));
        assert!(hint.contains("Nearby visible text"));
    }

    #[test]
    fn polish_context_is_structured() {
        let bundle = WindowContextBundle::from_ocr_result_with_confidence(
            "Reply to Jane about ProjectNebula Q2 roadmap\nworkflow prompt workflow prompt",
            WindowContextSource::PrimaryMonitor,
            None,
            1024,
            768,
            OcrConfidenceSummary::new(0.81, 1.0, 12, Some(9.72)),
        )
        .unwrap();

        let context = bundle.to_polish_context().unwrap();
        assert!(context.contains("Source: primary_monitor"));
        assert!(context.contains("OCR confidence: avg=0.81/1.00, observations=12"));
        assert!(context.contains("Candidate visible terms:"));
        assert!(context.contains("ProjectNebula"));
        assert!(context.contains("Frequent context keywords:"));
        assert!(context.contains("workflow"));
        assert!(context.contains("Visible text:"));
    }

    #[test]
    fn extract_terms_prefers_product_terms_over_ocr_noise() {
        let terms = extract_terms(
            "trace_id=9F8A7C6D5E4B3A21 /Users/me/tmp ABCDEFGHIJK ProjectNebula README.md VAD",
            8,
        );

        assert!(terms.contains(&"ProjectNebula".to_string()));
        assert!(terms.contains(&"README.md".to_string()));
        assert!(terms.contains(&"VAD".to_string()));
        assert!(!terms.contains(&"9F8A7C6D5E4B3A21".to_string()));
        assert!(!terms.contains(&"ABCDEFGHIJK".to_string()));
        assert!(!terms.iter().any(|term| term.contains('/')));
    }

    #[test]
    fn extract_terms_rejects_mixed_script_and_joined_digit_noise() {
        let terms = extract_terms(
            "ArаТуpеЕX Bookmarks19333SyntaxV2EX AІT5K.10121FSSTTS RÆtItMarkdown \
             GitHubFlavored Markdown V2EX window_context.rs AriaType81 0.64.3AriaType",
            12,
        );

        assert!(terms.contains(&"GitHubFlavored".to_string()));
        assert!(terms.contains(&"Markdown".to_string()));
        assert!(terms.contains(&"V2EX".to_string()));
        assert!(terms.contains(&"window_context.rs".to_string()));
        assert!(terms.contains(&"AriaType".to_string()));
        assert!(!terms.contains(&"AriaType81".to_string()));
        assert!(!terms.contains(&"ArаТуpеЕX".to_string()));
        assert!(!terms.contains(&"Bookmarks19333SyntaxV2EX".to_string()));
        assert!(!terms.contains(&"AІT5K.10121FSSTTS".to_string()));
        assert!(!terms.contains(&"RÆtItMarkdown".to_string()));
    }

    #[test]
    fn extract_terms_normalizes_latest_log_glued_terms() {
        let terms = extract_terms(
            "sootieAriaTypeAriaType window_context.rsM window.rsUpdated Xapps Planv OR \
             Inspect Implement Add Run xAriaType NAriaTypePlanv Users Ran Waited \
             A1EmE T\u{00FD}0ele o0p V2EX SDK juejin.cn confidence-aware Rust",
            16,
        );

        assert!(terms.contains(&"AriaType".to_string()));
        assert!(terms.contains(&"window_context.rs".to_string()));
        assert!(terms.contains(&"window.rs".to_string()));
        assert!(terms.contains(&"V2EX".to_string()));
        assert!(terms.contains(&"SDK".to_string()));
        assert!(terms.contains(&"juejin.cn".to_string()));
        assert!(terms.contains(&"confidence-aware".to_string()));
        assert!(terms.contains(&"Rust".to_string()));
        assert!(!terms.contains(&"sootieAriaTypeAriaType".to_string()));
        assert!(!terms.contains(&"window_context.rsM".to_string()));
        assert!(!terms.contains(&"window.rsUpdated".to_string()));
        assert!(!terms.contains(&"Xapps".to_string()));
        assert!(!terms.contains(&"Planv".to_string()));
        assert!(!terms.contains(&"OR".to_string()));
        assert!(!terms.contains(&"Inspect".to_string()));
        assert!(!terms.contains(&"Implement".to_string()));
        assert!(!terms.contains(&"Add".to_string()));
        assert!(!terms.contains(&"Run".to_string()));
        assert!(!terms.contains(&"xAriaType".to_string()));
        assert!(!terms.contains(&"NAriaTypePlanv".to_string()));
        assert!(!terms.contains(&"Users".to_string()));
        assert!(!terms.contains(&"Ran".to_string()));
        assert!(!terms.contains(&"Waited".to_string()));
        assert!(!terms.contains(&"A1EmE".to_string()));
        assert!(!terms.contains(&"T\u{00FD}0ele".to_string()));
        assert!(!terms.contains(&"o0p".to_string()));
    }

    #[test]
    fn extract_terms_removes_decorative_punctuation() {
        let terms = extract_terms(
            "\"AriaType,\" vision-sid. ...refactor serv...refactor --log-level README.md",
            8,
        );

        assert!(terms.contains(&"AriaType".to_string()));
        assert!(terms.contains(&"vision-sid".to_string()));
        assert!(terms.contains(&"log-level".to_string()));
        assert!(terms.contains(&"README.md".to_string()));
        assert!(!terms.iter().any(|term| term.ends_with('.')));
        assert!(!terms.iter().any(|term| term.starts_with('-')));
        assert!(!terms.iter().any(|term| term.starts_with('.')));
        assert!(!terms.iter().any(|term| term.contains("...")));
    }

    #[test]
    fn extract_terms_deduplicates_candidate_terms() {
        let terms = extract_terms(
            "Ariatype AriaType ariatype README.md README.md_ VAD vad ProjectNebula projectnebula",
            12,
        );

        assert_eq!(
            terms
                .iter()
                .filter(|term| term.eq_ignore_ascii_case("ariatype"))
                .count(),
            1
        );
        assert_eq!(
            terms
                .iter()
                .filter(|term| term.eq_ignore_ascii_case("readme.md"))
                .count(),
            1
        );
        assert_eq!(
            terms
                .iter()
                .filter(|term| term.eq_ignore_ascii_case("vad"))
                .count(),
            1
        );
        assert_eq!(
            terms
                .iter()
                .filter(|term| term.eq_ignore_ascii_case("projectnebula"))
                .count(),
            1
        );
        assert!(terms.contains(&"AriaType".to_string()));
    }

    #[test]
    fn extract_frequent_keywords_counts_topic_words_without_noise() {
        let keywords = extract_frequent_keywords(
            "workflow prompt context workflow prompt context the the 9F8A7C6D5E4B3A21 ABCDEFGHIJK",
            8,
        );

        assert!(keywords.contains(&"workflow".to_string()));
        assert!(keywords.contains(&"prompt".to_string()));
        assert!(keywords.contains(&"context".to_string()));
        assert!(!keywords.contains(&"the".to_string()));
        assert!(!keywords.contains(&"9F8A7C6D5E4B3A21".to_string()));
        assert!(!keywords.contains(&"ABCDEFGHIJK".to_string()));
    }

    #[test]
    fn extract_frequent_keywords_counts_contained_keyword_variants() {
        let keywords = extract_frequent_keywords(
            "AriaType4 sootieAriaType AriaType ProjectNebula2 ProjectNebula type prototype",
            8,
        );

        assert!(keywords.contains(&"AriaType".to_string()));
        assert!(keywords.contains(&"ProjectNebula".to_string()));
        assert!(!keywords.contains(&"type".to_string()));
        assert!(!keywords.contains(&"prototype".to_string()));
    }

    #[test]
    fn low_confidence_context_omits_visible_text() {
        let bundle = WindowContextBundle::from_ocr_result_with_confidence(
            "ArаТуpеЕX Bookmarks19333SyntaxV2EX GitHubFlavored Markdown V2EX",
            WindowContextSource::FocusedWindow,
            Some("V2EX › 创作新主题".to_string()),
            1728,
            1080,
            OcrConfidenceSummary::new(0.70, 1.0, 48, Some(33.7)),
        )
        .unwrap();

        let stt_hint = bundle.to_stt_prompt_hint().unwrap();
        assert!(stt_hint.contains("Candidate visible terms:"));
        assert!(stt_hint.contains("Markdown"));
        assert!(!stt_hint.contains("Nearby visible text"));
        assert!(!stt_hint.contains("ArаТуpеЕX"));
        assert!(!stt_hint.contains("Bookmarks19333SyntaxV2EX"));

        let polish_context = bundle.to_polish_context().unwrap();
        assert!(polish_context.contains("OCR confidence: avg=0.70/1.00, observations=48"));
        assert!(!polish_context.contains("Visible text:"));
    }
}
