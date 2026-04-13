use crate::commands::settings::CloudSttConfig;
use crate::state::app_state::AppState;
use std::sync::atomic::Ordering;

pub struct PreparedRecordingStart {
    pub task_id: u64,
    pub cloud_stt_enabled: bool,
    pub cloud_stt_config: CloudSttConfig,
    pub language: String,
}

pub struct PreparedRecordingStop {
    pub task_id: u64,
}

pub struct PreparedRecordingCancellation {
    pub task_id: u64,
    pub should_stop_recorder: bool,
}

pub struct RecordingStartGuard<'a> {
    state: &'a AppState,
    task_id: u64,
    committed: bool,
}

impl RecordingStartGuard<'_> {
    pub fn new(state: &AppState, task_id: u64) -> RecordingStartGuard<'_> {
        RecordingStartGuard {
            state,
            task_id,
            committed: false,
        }
    }

    pub fn commit(&mut self) {
        self.committed = true;
    }
}

impl Drop for RecordingStartGuard<'_> {
    fn drop(&mut self) {
        if self.committed {
            return;
        }

        if self.state.finish_session(self.task_id).is_some() {
            self.state.is_recording.store(false, Ordering::SeqCst);
            self.state.is_transcribing.store(false, Ordering::SeqCst);
            self.state.recording_start_time.store(0, Ordering::SeqCst);
        }
    }
}

pub fn allocate_task_id(state: &AppState) -> u64 {
    state.task_counter.fetch_add(1, Ordering::SeqCst) + 1
}

pub fn prepare_recording_start(state: &AppState) -> PreparedRecordingStart {
    let (cloud_stt_enabled, cloud_stt_config, language) = {
        let settings = state.settings.lock();
        (
            #[allow(deprecated)]
            settings.is_volcengine_streaming_active(),
            settings.get_active_cloud_stt_config(),
            settings.stt_engine_language.clone(),
        )
    };

    let task_id = allocate_task_id(state);
    state.start_session(task_id);

    let start_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    state.recording_start_time.store(start_ms, Ordering::SeqCst);

    PreparedRecordingStart {
        task_id,
        cloud_stt_enabled,
        cloud_stt_config,
        language,
    }
}

pub fn prepare_recording_stop(state: &AppState) -> Option<PreparedRecordingStop> {
    if !state.is_recording.load(Ordering::SeqCst) {
        return None;
    }

    let task_id = state.task_counter.load(Ordering::SeqCst);
    state.is_recording.store(false, Ordering::SeqCst);

    if let Some(tx) = state.level_monitor_tx.lock().as_ref() {
        let _ = tx.send(false);
    }

    Some(PreparedRecordingStop { task_id })
}

pub fn prepare_recording_cancellation(state: &AppState) -> Option<PreparedRecordingCancellation> {
    let is_recording = state.is_recording.load(Ordering::SeqCst);
    let is_transcribing = state.is_transcribing.load(Ordering::SeqCst);

    if !is_recording && !is_transcribing {
        return None;
    }

    let task_id = state.task_counter.load(Ordering::SeqCst);
    state.request_cancellation(task_id);
    state.is_recording.store(false, Ordering::SeqCst);
    state.is_transcribing.store(false, Ordering::SeqCst);

    if let Some(tx) = state.level_monitor_tx.lock().as_ref() {
        let _ = tx.send(false);
    }

    state.clear_session();

    Some(PreparedRecordingCancellation {
        task_id,
        should_stop_recorder: is_recording,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        prepare_recording_cancellation, prepare_recording_start, prepare_recording_stop,
        RecordingStartGuard,
    };
    use crate::commands::settings::CloudSttConfig;
    use crate::state::app_state::AppState;
    use std::sync::atomic::Ordering;
    use std::sync::mpsc::TryRecvError;

    #[test]
    fn prepare_recording_start_captures_settings_and_starts_session() {
        let state = AppState::new();
        {
            let mut settings = state.settings.lock();
            settings.stt_engine_language = "ja-JP".to_string();
            settings.cloud_stt_enabled = true;
            settings.active_cloud_stt_provider = "volcengine-streaming".to_string();
            settings.cloud_stt_configs.insert(
                "volcengine-streaming".to_string(),
                CloudSttConfig {
                    enabled: true,
                    provider_type: "volcengine-streaming".to_string(),
                    api_key: "test-key".to_string(),
                    app_id: "test-app".to_string(),
                    base_url: "https://example.com".to_string(),
                    model: "bigmodel_nostream".to_string(),
                    language: "ja-JP".to_string(),
                },
            );
        }

        let prepared = prepare_recording_start(&state);

        assert_eq!(prepared.task_id, 1);
        assert!(prepared.cloud_stt_enabled);
        assert_eq!(prepared.language, "ja-JP");
        assert_eq!(
            prepared.cloud_stt_config.provider_type,
            "volcengine-streaming"
        );
        assert!(state.get_session_text(prepared.task_id).is_some());
        assert!(state.recording_start_time.load(Ordering::SeqCst) > 0);
    }

    #[test]
    fn uncommitted_recording_start_guard_rolls_back_session_state() {
        let state = AppState::new();
        state.is_recording.store(true, Ordering::SeqCst);
        state.is_transcribing.store(false, Ordering::SeqCst);
        state.start_session(9);
        state.recording_start_time.store(1234, Ordering::SeqCst);

        {
            let _guard = RecordingStartGuard::new(&state, 9);
            assert!(state.get_session_text(9).is_some());
        }

        assert!(state.get_session_text(9).is_none());
        assert!(!state.is_recording.load(Ordering::SeqCst));
        assert!(!state.is_transcribing.load(Ordering::SeqCst));
        assert_eq!(state.recording_start_time.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn committed_recording_start_guard_preserves_session_state() {
        let state = AppState::new();
        state.is_recording.store(true, Ordering::SeqCst);
        state.start_session(10);
        state.recording_start_time.store(5678, Ordering::SeqCst);

        let mut guard = RecordingStartGuard::new(&state, 10);
        guard.commit();
        drop(guard);

        assert!(state.get_session_text(10).is_some());
        assert!(state.is_recording.load(Ordering::SeqCst));
        assert_eq!(state.recording_start_time.load(Ordering::SeqCst), 5678);
    }

    #[test]
    fn prepare_recording_stop_returns_none_when_not_recording() {
        let state = AppState::new();

        assert!(prepare_recording_stop(&state).is_none());
    }

    #[test]
    fn prepare_recording_stop_clears_recording_flag_and_closes_level_monitor() {
        let state = AppState::new();
        state.is_recording.store(true, Ordering::SeqCst);
        state.task_counter.store(12, Ordering::SeqCst);
        let rx = state
            .level_monitor_rx
            .lock()
            .take()
            .expect("level monitor receiver should exist");

        let prepared = prepare_recording_stop(&state).expect("stop preparation should exist");

        assert_eq!(prepared.task_id, 12);
        assert!(!state.is_recording.load(Ordering::SeqCst));
        assert_eq!(rx.recv().expect("stop should notify monitor"), false);
    }

    #[test]
    fn prepare_recording_cancellation_returns_none_when_idle() {
        let state = AppState::new();

        assert!(prepare_recording_cancellation(&state).is_none());
    }

    #[test]
    fn prepare_recording_cancellation_marks_task_and_clears_session() {
        let state = AppState::new();
        state.is_recording.store(true, Ordering::SeqCst);
        state.is_transcribing.store(true, Ordering::SeqCst);
        state.task_counter.store(14, Ordering::SeqCst);
        state.start_session(14);
        state.append_session_text(14, "partial");
        let rx = state
            .level_monitor_rx
            .lock()
            .take()
            .expect("level monitor receiver should exist");

        let prepared =
            prepare_recording_cancellation(&state).expect("cancellation should be prepared");

        assert_eq!(prepared.task_id, 14);
        assert!(prepared.should_stop_recorder);
        assert!(state.is_cancellation_requested(14));
        assert!(!state.is_recording.load(Ordering::SeqCst));
        assert!(!state.is_transcribing.load(Ordering::SeqCst));
        assert!(state.get_session_text(14).is_none());
        assert_eq!(rx.recv().expect("cancel should notify monitor"), false);
    }

    #[test]
    fn prepare_recording_cancellation_keeps_recorder_running_flag_false_for_transcribe_only() {
        let state = AppState::new();
        state.is_transcribing.store(true, Ordering::SeqCst);
        state.task_counter.store(15, Ordering::SeqCst);

        let prepared =
            prepare_recording_cancellation(&state).expect("transcribing task should cancel");

        assert_eq!(prepared.task_id, 15);
        assert!(!prepared.should_stop_recorder);
        assert!(state.is_cancellation_requested(15));
        assert!(!state.is_transcribing.load(Ordering::SeqCst));
        assert!(!state.is_recording.load(Ordering::SeqCst));
        let rx = state
            .level_monitor_rx
            .lock()
            .take()
            .expect("level monitor receiver should exist");
        assert_eq!(rx.try_recv(), Ok(false));
        assert_eq!(rx.try_recv(), Err(TryRecvError::Empty));
    }
}
