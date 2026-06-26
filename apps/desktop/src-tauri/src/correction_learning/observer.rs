use tauri::{AppHandle, Emitter};
use tracing::{debug, info, warn};

use super::diff::{
    extract_correction_pair, extract_deleted_correction_term, is_word_level_correction_pair,
};
use super::platform::read_focused_editable_text;
use super::storage::CorrectionStore;
use super::types::CorrectionLearnedEvent;
use crate::events::{emit_pill_tooltip, EventName};

const OBSERVE_INITIAL_DELAY_MS: u64 = 200;
const BASELINE_MAX_WAIT_MS: u64 = 2_000;
const BASELINE_POLL_INTERVAL_MS: u64 = 250;
const OBSERVE_POLL_INTERVAL_MS: u64 = 1_500;
const OBSERVE_MAX_DURATION_MS: u64 = 45_000;
const REQUIRED_STABLE_READS: u8 = 2;
const MAX_UNAVAILABLE_READS: u8 = 3;
const DIRECT_EDIT_MIN_COMMON_CONTEXT_CHARS: usize = 6;
const CORRECTION_TOOLTIP_DURATION_MS: u64 = 3_200;

pub fn observe_post_delivery_edit(app: AppHandle, delivered_text: String) {
    if delivered_text.trim().is_empty() {
        return;
    }

    tauri::async_runtime::spawn(async move {
        observe_post_delivery_edit_inner(app, delivered_text).await;
    });
}

async fn observe_post_delivery_edit_inner(app: AppHandle, delivered_text: String) {
    info!(
        delivered_chars = delivered_text.chars().count(),
        "correction_learning_observer_armed"
    );
    tokio::time::sleep(tokio::time::Duration::from_millis(OBSERVE_INITIAL_DELAY_MS)).await;

    let Some(baseline) = wait_for_baseline_or_quick_edit(&app, &delivered_text).await else {
        return;
    };

    info!(
        baseline_chars = baseline.chars().count(),
        "correction_learning_observer_started"
    );

    let deadline =
        tokio::time::Instant::now() + tokio::time::Duration::from_millis(OBSERVE_MAX_DURATION_MS);
    let mut last_candidate: Option<String> = None;
    let mut stable_reads: u8 = 0;
    let mut unavailable_reads: u8 = 0;

    while tokio::time::Instant::now() < deadline {
        tokio::time::sleep(tokio::time::Duration::from_millis(OBSERVE_POLL_INTERVAL_MS)).await;

        let Some(current) = read_focused_editable_text().await else {
            unavailable_reads = unavailable_reads.saturating_add(1);
            if unavailable_reads >= MAX_UNAVAILABLE_READS {
                info!("correction_learning_observer_stopped-focused_text_unavailable");
                return;
            }
            continue;
        };
        unavailable_reads = 0;

        if current == baseline {
            last_candidate = None;
            stable_reads = 0;
            continue;
        }

        if last_candidate.as_deref() == Some(current.as_str()) {
            stable_reads = stable_reads.saturating_add(1);
        } else {
            last_candidate = Some(current.clone());
            stable_reads = 1;
        }

        if stable_reads < REQUIRED_STABLE_READS {
            continue;
        }

        if !should_learn_stable_edit(&baseline, &current) {
            if should_wait_for_pending_replacement_edit(&baseline, &current) {
                debug!(
                    baseline_chars = baseline.chars().count(),
                    current_chars = current.chars().count(),
                    "correction_learning_observer_waiting-pending_replacement"
                );
                continue;
            }

            info!(
                baseline_chars = baseline.chars().count(),
                current_chars = current.chars().count(),
                "correction_learning_observer_stopped-non_direct_edit"
            );
            return;
        }

        learn_and_emit(&app, &baseline, &current, "stable_edit");
        return;
    }

    info!("correction_learning_observer_stopped-timeout");
}

async fn wait_for_baseline_or_quick_edit(app: &AppHandle, delivered_text: &str) -> Option<String> {
    let deadline =
        tokio::time::Instant::now() + tokio::time::Duration::from_millis(BASELINE_MAX_WAIT_MS);
    let mut read_attempts: u32 = 0;
    let mut unrelated_reads: u32 = 0;
    let mut unavailable_reads: u32 = 0;

    while tokio::time::Instant::now() < deadline {
        read_attempts += 1;
        match read_focused_editable_text().await {
            Some(snapshot) if snapshot_contains_delivery(&snapshot, delivered_text) => {
                info!(
                    read_attempts,
                    snapshot_chars = snapshot.chars().count(),
                    "correction_learning_observer_baseline_captured"
                );
                return Some(snapshot);
            }
            Some(snapshot) if looks_like_direct_edit(delivered_text, &snapshot) => {
                info!(
                    read_attempts,
                    snapshot_chars = snapshot.chars().count(),
                    "correction_learning_observer_quick_edit_detected"
                );
                learn_and_emit(app, delivered_text, &snapshot, "quick_edit");
                return None;
            }
            Some(_) => {
                unrelated_reads += 1;
            }
            None => {
                unavailable_reads += 1;
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(
            BASELINE_POLL_INTERVAL_MS,
        ))
        .await;
    }

    info!(
        read_attempts,
        unrelated_reads,
        unavailable_reads,
        "correction_learning_observer_skipped-baseline_unavailable"
    );
    None
}

fn learn_and_emit(app: &AppHandle, before: &str, after: &str, reason: &str) {
    match CorrectionStore::shared().learn_from_edit(before, after) {
        Ok(Some(mapping)) => {
            let event = CorrectionLearnedEvent::from(&mapping);
            let _ = app.emit(EventName::CORRECTION_LEARNED, event);
            emit_pill_tooltip(
                app,
                format!("已记录纠错词：{} -> {}", mapping.wrong, mapping.corrected),
                CORRECTION_TOOLTIP_DURATION_MS,
                None,
            );
            info!(
                reason,
                frequency = mapping.frequency,
                wrong_chars = mapping.wrong.chars().count(),
                corrected_chars = mapping.corrected.chars().count(),
                "correction_learning_mapping_recorded"
            );
        }
        Ok(None) => {
            info!(reason, "correction_learning_mapping_not_recorded");
        }
        Err(error) => {
            warn!(reason, error = %error, "correction_learning_record_failed");
        }
    }
}

fn looks_like_direct_edit(delivered_text: &str, snapshot: &str) -> bool {
    let delivered_text = normalize_for_containment(delivered_text);
    let snapshot = normalize_for_containment(snapshot);
    if delivered_text == snapshot || delivered_text.is_empty() || snapshot.is_empty() {
        return false;
    }

    let Some(pair) = extract_correction_pair(&delivered_text, &snapshot) else {
        return false;
    };

    let delivered_chars: Vec<char> = delivered_text.chars().collect();
    let snapshot_chars: Vec<char> = snapshot.chars().collect();
    let common_context = common_affix_chars(&delivered_chars, &snapshot_chars);
    let min_len = delivered_chars.len().min(snapshot_chars.len());

    common_context >= DIRECT_EDIT_MIN_COMMON_CONTEXT_CHARS
        || common_context * 2 >= min_len
        || is_whole_output_term_replacement(&delivered_text, &snapshot, &pair)
}

fn should_learn_stable_edit(baseline: &str, current: &str) -> bool {
    looks_like_direct_edit(baseline, current)
}

fn should_wait_for_pending_replacement_edit(baseline: &str, current: &str) -> bool {
    let baseline = normalize_for_containment(baseline);
    let current = normalize_for_containment(current);
    if baseline == current || current.is_empty() {
        return false;
    }

    if extract_deleted_correction_term(&baseline, &current).is_none() {
        return false;
    }

    let baseline_chars: Vec<char> = baseline.chars().collect();
    let current_chars: Vec<char> = current.chars().collect();
    let common_context = common_affix_chars(&baseline_chars, &current_chars);
    let min_len = baseline_chars.len().min(current_chars.len());

    common_context >= DIRECT_EDIT_MIN_COMMON_CONTEXT_CHARS || common_context * 2 >= min_len
}

fn common_affix_chars(left: &[char], right: &[char]) -> usize {
    let mut prefix = 0;
    while prefix < left.len() && prefix < right.len() && left[prefix] == right[prefix] {
        prefix += 1;
    }

    let mut suffix = 0;
    while suffix < left.len().saturating_sub(prefix)
        && suffix < right.len().saturating_sub(prefix)
        && left[left.len() - 1 - suffix] == right[right.len() - 1 - suffix]
    {
        suffix += 1;
    }

    prefix + suffix
}

fn snapshot_contains_delivery(snapshot: &str, delivered_text: &str) -> bool {
    let snapshot = normalize_for_containment(snapshot);
    let delivered_text = normalize_for_containment(delivered_text);
    !delivered_text.is_empty()
        && (snapshot == delivered_text || snapshot.contains(delivered_text.as_str()))
}

fn normalize_for_containment(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn is_whole_output_term_replacement(
    delivered_text: &str,
    snapshot: &str,
    pair: &super::types::CorrectionPair,
) -> bool {
    let delivered_term = normalize_whole_output_term(delivered_text);
    let snapshot_term = normalize_whole_output_term(snapshot);

    pair.wrong == delivered_term
        && pair.corrected == snapshot_term
        && is_word_level_correction_pair(&delivered_term, &snapshot_term)
}

fn normalize_whole_output_term(text: &str) -> String {
    text.trim()
        .trim_matches(is_observer_boundary_punctuation)
        .trim()
        .to_string()
}

fn is_observer_boundary_punctuation(c: char) -> bool {
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

#[cfg(test)]
mod tests {
    use super::{
        looks_like_direct_edit, should_learn_stable_edit, should_wait_for_pending_replacement_edit,
        snapshot_contains_delivery,
    };

    #[test]
    fn accepts_exact_delivery_snapshot() {
        assert!(snapshot_contains_delivery("hello world", "hello world"));
    }

    #[test]
    fn accepts_delivery_embedded_in_existing_text() {
        assert!(snapshot_contains_delivery(
            "Before. hello world After.",
            "hello world"
        ));
    }

    #[test]
    fn rejects_unrelated_focused_text() {
        assert!(!snapshot_contains_delivery(
            "different document",
            "hello world"
        ));
    }

    #[test]
    fn accepts_fast_user_correction_as_direct_edit() {
        assert!(looks_like_direct_edit(
            "那你进行详细完整的流程，试一试搜题现在的功能是不是符合预期的？",
            "那你进行详细完整的流程，试一试sootie现在的功能是不是符合预期的？"
        ));
    }

    #[test]
    fn accepts_whole_output_term_replacement_after_user_retypes() {
        assert!(looks_like_direct_edit("搜题", "sootie"));
        assert!(looks_like_direct_edit("Air Tap", "AriaType"));
    }

    #[test]
    fn rejects_whole_output_plain_sentence_fragment_replacement() {
        assert!(!looks_like_direct_edit("delete this", "new text"));
    }

    #[test]
    fn rejects_whole_output_deletion_without_replacement_text() {
        assert!(!looks_like_direct_edit("搜题", ""));
    }

    #[test]
    fn rejects_unrelated_direct_edit_snapshot() {
        assert!(!looks_like_direct_edit(
            "那你进行详细完整的流程，试一试搜题现在的功能是不是符合预期的？",
            "completely unrelated focused field"
        ));
    }

    #[test]
    fn rejects_stable_edit_to_unrelated_ui_label() {
        assert!(!should_learn_stable_edit(
            "运行一下这个recipe，看看效果",
            "Ask for follow-up changes"
        ));
    }

    #[test]
    fn accepts_stable_embedded_direct_edit() {
        assert!(should_learn_stable_edit(
            "Before. 那你试一试搜题现在的功能是不是符合预期的？ After.",
            "Before. 那你试一试sootie现在的功能是不是符合预期的？ After."
        ));
    }

    #[test]
    fn waits_when_user_deleted_a_term_before_typing_replacement() {
        assert!(should_wait_for_pending_replacement_edit(
            "Before Air Tap After",
            "Before After"
        ));
        assert!(should_wait_for_pending_replacement_edit(
            "Before open ai api After",
            "Before After"
        ));

        assert!(!should_learn_stable_edit(
            "Before Air Tap After",
            "Before After"
        ));
    }

    #[test]
    fn does_not_wait_when_user_deleted_sentence_fragment_before_retyping() {
        assert!(!should_wait_for_pending_replacement_edit(
            "Before one short sentence After",
            "Before After"
        ));
        assert!(!should_wait_for_pending_replacement_edit(
            "Before delete this After",
            "Before After"
        ));
    }

    #[test]
    fn does_not_wait_for_completed_replacement_or_unrelated_text() {
        assert!(!should_wait_for_pending_replacement_edit(
            "Before Air Tap After",
            "Before AriaType After"
        ));
        assert!(!should_wait_for_pending_replacement_edit(
            "Before Air Tap After",
            "completely unrelated focused field"
        ));
    }
}
