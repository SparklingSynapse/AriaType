use tauri::{AppHandle, Emitter};
use tracing::{info, warn};

use super::diff::extract_correction_pair;
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

    while tokio::time::Instant::now() < deadline {
        tokio::time::sleep(tokio::time::Duration::from_millis(OBSERVE_POLL_INTERVAL_MS)).await;

        let Some(current) = read_focused_editable_text().await else {
            info!("correction_learning_observer_stopped-focused_text_unavailable");
            return;
        };

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

    if extract_correction_pair(&delivered_text, &snapshot).is_none() {
        return false;
    }

    let delivered_chars: Vec<char> = delivered_text.chars().collect();
    let snapshot_chars: Vec<char> = snapshot.chars().collect();
    let common_context = common_affix_chars(&delivered_chars, &snapshot_chars);
    let min_len = delivered_chars.len().min(snapshot_chars.len());

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

#[cfg(test)]
mod tests {
    use super::{looks_like_direct_edit, snapshot_contains_delivery};

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
    fn rejects_unrelated_direct_edit_snapshot() {
        assert!(!looks_like_direct_edit(
            "那你进行详细完整的流程，试一试搜题现在的功能是不是符合预期的？",
            "completely unrelated focused field"
        ));
    }
}
