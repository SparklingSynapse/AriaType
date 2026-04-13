use tracing::warn;

use crate::state::app_state::AppState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FinalizeResult {
    DeliverText(String),
    TransitionToIdle,
    TransitionToError,
}

pub fn finalize_successful_transcription(
    state: &AppState,
    raw_text: &str,
    final_text: &str,
    polish_time_ms: u64,
    audio_path: Option<String>,
) -> FinalizeResult {
    crate::history::commands::save_to_history(
        state,
        raw_text,
        final_text,
        None,
        (polish_time_ms > 0).then_some(polish_time_ms as i64),
        polish_time_ms > 0,
        audio_path.clone(),
    );

    if let Some(path) = audio_path.as_deref() {
        if let Err(error) = std::fs::remove_file(path) {
            warn!(error = %error, path = %path, "audio_cleanup_failed");
        }
    }

    FinalizeResult::DeliverText(final_text.to_string())
}

pub fn finalize_empty_transcription(state: &AppState, audio_path: Option<String>) -> FinalizeResult {
    crate::history::commands::save_failed_history(state, audio_path, "Empty transcription result");
    FinalizeResult::TransitionToIdle
}

pub fn finalize_failed_transcription(
    state: &AppState,
    audio_path: Option<String>,
    error: &str,
) -> FinalizeResult {
    crate::history::commands::save_failed_history(state, audio_path, error);
    FinalizeResult::TransitionToError
}

#[cfg(test)]
mod tests {
    use super::{
        finalize_empty_transcription, finalize_failed_transcription,
        finalize_successful_transcription, FinalizeResult,
    };
    use crate::history::models::HistoryFilter;
    use crate::state::app_state::AppState;
    use std::sync::atomic::Ordering;
    use std::time::{SystemTime, UNIX_EPOCH};
    use tempfile::NamedTempFile;

    fn set_recording_long_enough(state: &AppState) {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        state
            .recording_start_time
            .store(now_ms.saturating_sub(800), Ordering::SeqCst);
    }

    #[test]
    fn finalize_successful_transcription_requests_text_delivery() {
        let state = AppState::new();
        let audio = NamedTempFile::new().unwrap();
        let audio_path = audio.path().to_path_buf();

        let action = finalize_successful_transcription(
            &state,
            "raw text",
            "final text",
            123,
            Some(audio_path.display().to_string()),
        );

        assert_eq!(
            action,
            FinalizeResult::DeliverText("final text".to_string())
        );
        assert!(!audio_path.exists());
    }

    #[test]
    fn finalize_empty_transcription_saves_failed_history_and_returns_idle() {
        let state = AppState::new();
        set_recording_long_enough(&state);
        let audio = NamedTempFile::new().unwrap();

        let action = finalize_empty_transcription(&state, Some(audio.path().display().to_string()));

        assert_eq!(action, FinalizeResult::TransitionToIdle);

        let entries = state
            .history_store
            .lock()
            .get_history(&HistoryFilter {
                search: None,
                engine: None,
                status: Some("error".to_string()),
                date_from: None,
                date_to: None,
                limit: Some(5),
                offset: Some(0),
            })
            .unwrap();
        assert!(!entries.is_empty());
        assert!(entries
            .iter()
            .any(|entry| entry.error.as_deref() == Some("Empty transcription result")));
    }

    #[test]
    fn finalize_failed_transcription_saves_failed_history_and_returns_error() {
        let state = AppState::new();
        set_recording_long_enough(&state);

        let action = finalize_failed_transcription(&state, None, "network failed");

        assert_eq!(action, FinalizeResult::TransitionToError);

        let entries = state
            .history_store
            .lock()
            .get_history(&HistoryFilter {
                search: None,
                engine: None,
                status: Some("error".to_string()),
                date_from: None,
                date_to: None,
                limit: Some(5),
                offset: Some(0),
            })
            .unwrap();
        assert!(!entries.is_empty());
        assert!(entries
            .iter()
            .any(|entry| entry.error.as_deref() == Some("network failed")));
    }
}
