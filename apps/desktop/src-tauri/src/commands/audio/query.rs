use std::sync::atomic::Ordering;
use tauri::State;

use crate::state::app_state::AppState;

use super::shared::RecordingState;

#[tauri::command]
pub fn get_audio_level(state: State<'_, AppState>) -> u32 {
    state.audio_level.load(Ordering::SeqCst)
}

#[tauri::command]
pub fn get_recording_state(state: State<'_, AppState>) -> RecordingState {
    RecordingState {
        is_recording: state.is_recording.load(Ordering::SeqCst),
        is_transcribing: state.is_transcribing.load(Ordering::SeqCst),
        audio_level: state.audio_level.load(Ordering::SeqCst),
        output_path: state.output_path.lock().clone(),
    }
}
