use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use wiremock::matchers::{body_partial_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::commands::settings::CloudProviderConfig;
use crate::state::app_state::AppState;

use super::polish::maybe_polish_transcription_text;
use super::shared::{
    await_streaming_task_in_background, discard_canceled_result, flush_pending_chunk_for_stop,
    recording_chunk_size_samples, send_flushed_chunk_for_stop, should_emit_error_recovery_idle,
    should_unregister_cancel_hotkey_after_async_cleanup, ParkingMutex, ProcessingEventTarget,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn waiting_for_streaming_task_does_not_block_the_active_runtime() {
    let (done_tx, done_rx) = tokio::sync::oneshot::channel();
    let handle = tauri::async_runtime::spawn(async move {
        let _ = done_tx.send(());
    });

    await_streaming_task_in_background(1, handle);

    tokio::time::timeout(Duration::from_secs(1), done_rx)
        .await
        .expect("streaming task should complete without blocking the runtime")
        .expect("streaming task completion signal should be delivered");
}

#[tokio::test]
async fn streaming_finalization_honors_cloud_polish_settings() {
    let mock_server = MockServer::start().await;

    let user_prompt = "System instruction here";
    let expected_system_content = format!(
        "{}\n\nUSER RULES:\n{}",
        crate::polish_engine::cloud::engine::CORE_POLISH_CONSTRAINT,
        user_prompt
    );

    let expected_body = serde_json::json!({
        "model": "gpt-4o-mini",
        "max_tokens": 4096,
        "messages": [
            {
                "role": "system",
                "content": expected_system_content
            },
            {
                "role": "user",
                "content": "User text here"
            }
        ]
    });

    let response_body = serde_json::json!({
        "choices": [
            {
                "message": {
                    "content": "Polished streaming text"
                }
            }
        ]
    });

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(header("Authorization", "Bearer test_openai_api_key"))
        .and(header("Content-Type", "application/json"))
        .and(body_partial_json(expected_body))
        .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
        .mount(&mock_server)
        .await;

    let state = AppState::new();
    {
        let mut settings = state.settings.lock();
        settings.polish_enabled = false;
        settings.cloud_polish_enabled = true;
        settings.active_cloud_polish_provider = "openai".to_string();
        settings.stt_engine_language = "en-US".to_string();
        settings.polish_system_prompt = user_prompt.to_string();
        settings.cloud_polish_configs.insert(
            "openai".to_string(),
            CloudProviderConfig {
                enabled: true,
                provider_type: "openai".to_string(),
                api_key: "test_openai_api_key".to_string(),
                base_url: mock_server.uri(),
                model: "gpt-4o-mini".to_string(),
                enable_thinking: false,
            },
        );
    }

    let (final_text, _polish_time_ms) = maybe_polish_transcription_text(
        &ProcessingEventTarget::None,
        &state,
        1,
        "User text here".to_string(),
    )
    .await;

    assert_eq!(final_text, "Polished streaming text");
}

#[test]
fn async_cleanup_keeps_cancel_hotkey_while_hotkey_cancel_is_still_active() {
    assert!(!should_unregister_cancel_hotkey_after_async_cleanup(
        1, 1, true
    ));
}

#[test]
fn async_cleanup_ignores_stale_task_after_a_new_recording_starts() {
    assert!(!should_unregister_cancel_hotkey_after_async_cleanup(
        2, 1, false
    ));
}

#[test]
fn async_cleanup_unregisters_cancel_hotkey_only_for_current_non_canceled_task() {
    assert!(should_unregister_cancel_hotkey_after_async_cleanup(
        3, 3, false
    ));
}

#[test]
fn error_recovery_idle_emits_only_for_the_same_finished_task() {
    assert!(should_emit_error_recovery_idle(7, 7, false, false));
}

#[test]
fn error_recovery_idle_skips_after_a_new_recording_starts() {
    assert!(!should_emit_error_recovery_idle(8, 7, true, false));
    assert!(!should_emit_error_recovery_idle(8, 7, false, true));
    assert!(!should_emit_error_recovery_idle(8, 7, false, false));
}

#[test]
fn recording_chunk_size_uses_200ms_of_device_audio() {
    assert_eq!(recording_chunk_size_samples(16_000, 1), 3_200);
    assert_eq!(recording_chunk_size_samples(48_000, 2), 19_200);
}

#[test]
fn flush_pending_chunk_for_stop_processes_sub_threshold_tail_audio() {
    let chunk_buffer = Arc::new(ParkingMutex::new(vec![1_000; 1_600]));
    let processor = Arc::new(ParkingMutex::new(
        crate::audio::stream_processor::StreamAudioProcessor::new("off", false, None),
    ));

    let flushed = flush_pending_chunk_for_stop(&chunk_buffer, &processor, 16_000, 1);

    assert_eq!(flushed, Some(vec![999; 1_600]));
    assert!(chunk_buffer.lock().is_empty());
}

#[test]
fn flush_pending_chunk_for_stop_does_not_drop_tail_when_vad_rejects_it() {
    let chunk_buffer = Arc::new(ParkingMutex::new(vec![1_000; 1_600]));
    let processor = Arc::new(ParkingMutex::new(
        crate::audio::stream_processor::StreamAudioProcessor::new("off", true, None),
    ));
    {
        let mut processor_guard = processor.lock();
        processor_guard.force_vad_result_for_test(false);
        processor_guard.set_last_send_time_for_test(std::time::Instant::now());
    }

    let flushed = flush_pending_chunk_for_stop(&chunk_buffer, &processor, 16_000, 1);

    assert_eq!(flushed, Some(vec![999; 1_600]));
    assert!(chunk_buffer.lock().is_empty());
}

#[tokio::test(flavor = "current_thread")]
async fn send_flushed_chunk_for_stop_waits_for_capacity_when_channel_is_full() {
    let (tx, mut rx) = tokio::sync::mpsc::channel(1);
    tx.send(vec![1]).await.unwrap();

    let send_task = tokio::spawn({
        let tx = tx.clone();
        async move { send_flushed_chunk_for_stop(&tx, vec![2]).await }
    });

    assert_eq!(rx.recv().await, Some(vec![1]));
    send_task.await.unwrap().unwrap();
    assert_eq!(rx.recv().await, Some(vec![2]));
}

#[test]
fn discarding_a_canceled_result_only_clears_that_task() {
    let state = AppState::new();
    state.request_cancellation(5);
    state.request_cancellation(6);
    state.start_session(5);
    state.is_transcribing.store(true, Ordering::SeqCst);

    discard_canceled_result(&state, 5, None);

    assert!(!state.is_cancellation_requested(5));
    assert!(state.is_cancellation_requested(6));
    assert!(!state.is_transcribing.load(Ordering::SeqCst));
    assert!(state.get_session_text(5).is_none());
}
