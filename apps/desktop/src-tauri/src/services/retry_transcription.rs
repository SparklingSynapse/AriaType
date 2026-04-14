use crate::history::models::TranscriptionEntry;
use crate::history::store::EntryUpdates;
use crate::services::recording_lifecycle::allocate_task_id;
use crate::state::app_state::AppState;
use crate::stt_engine::traits::{EngineType, TranscriptionRequest};
use std::path::Path;
use tracing::warn;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryProviderPolicy {
    CurrentLocalConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetryExecutionPlan {
    pub policy: RetryProviderPolicy,
    pub engine_type: EngineType,
    pub engine_name: String,
    pub model_name: String,
    pub language: String,
    pub is_cloud: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedRetryTranscription {
    pub task_id: u64,
    pub entry_id: String,
    pub audio_path: String,
    pub plan: RetryExecutionPlan,
}

pub fn prepare_retry_transcription(
    state: &AppState,
    entry_id: String,
    entry: TranscriptionEntry,
) -> Result<PreparedRetryTranscription, String> {
    if entry.status != "error" {
        return Err("Entry is not in error state".to_string());
    }

    let audio_path = entry
        .audio_path
        .ok_or_else(|| "No audio file saved for this entry".to_string())?;

    if !Path::new(&audio_path).exists() {
        return Err(format!("Audio file not found: {audio_path}"));
    }

    let plan = resolve_retry_execution_plan(state);

    Ok(PreparedRetryTranscription {
        task_id: allocate_task_id(state),
        entry_id,
        audio_path,
        plan,
    })
}

pub fn build_retry_entry_updates(
    output: &RetryTranscriptionOutput,
    final_text: &str,
    polish_time_ms: u64,
) -> EntryUpdates {
    EntryUpdates {
        raw_text: output.raw_text.clone(),
        final_text: final_text.to_string(),
        stt_engine: output.plan.engine_name.clone(),
        stt_model: Some(output.plan.model_name.clone()),
        language: Some(output.plan.language.clone()),
        stt_duration_ms: Some(output.stt_duration_ms as i64),
        polish_duration_ms: (polish_time_ms > 0).then_some(polish_time_ms as i64),
        polish_applied: polish_time_ms > 0,
        polish_engine: None,
        is_cloud: output.plan.is_cloud,
    }
}

pub fn mark_retry_entry_error(state: &AppState, entry_id: &str, error: &str) -> Result<(), String> {
    let store = state.history_store.lock();
    store
        .mark_error(entry_id, error)
        .map_err(|e| format!("Failed to update entry: {e}"))
}

pub fn update_retry_entry_success(
    state: &AppState,
    entry_id: &str,
    updates: EntryUpdates,
) -> Result<(), String> {
    let store = state.history_store.lock();
    store
        .update_entry(entry_id, updates)
        .map_err(|e| format!("Failed to update entry: {e}"))
}

pub fn cleanup_retry_audio_file(audio_path: &str) {
    if let Err(error) = std::fs::remove_file(audio_path) {
        warn!(
            error = %error,
            path = %audio_path,
            "audio_cleanup_failed_after_retry"
        );
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetryTranscriptionOutput {
    pub raw_text: String,
    pub stt_duration_ms: u64,
    pub plan: RetryExecutionPlan,
}

pub async fn transcribe_retry_audio_file(
    state: &AppState,
    prepared: &PreparedRetryTranscription,
) -> Result<RetryTranscriptionOutput, String> {
    let samples_16k_f32 = load_retry_audio_samples_f32(&prepared.audio_path)?;
    let request = TranscriptionRequest::new(samples_16k_f32)
        .with_model(prepared.plan.model_name.clone())
        .with_language(prepared.plan.language.clone());

    let result = state
        .engine_manager
        .transcribe(prepared.plan.engine_type, request)
        .await
        .map_err(|e| format!("Transcription failed: {e}"))?;

    Ok(RetryTranscriptionOutput {
        raw_text: result.text,
        stt_duration_ms: result.total_ms,
        plan: prepared.plan.clone(),
    })
}

fn resolve_retry_execution_plan(state: &AppState) -> RetryExecutionPlan {
    let (requested_model, language) = {
        let settings = state.settings.lock();
        (settings.model.clone(), settings.stt_engine_language.clone())
    };
    let (engine_type, model_name) = state
        .engine_manager
        .resolve_available_model(&requested_model, &language);

    RetryExecutionPlan {
        policy: RetryProviderPolicy::CurrentLocalConfig,
        engine_name: engine_type.as_str().to_string(),
        engine_type,
        model_name,
        language,
        is_cloud: false,
    }
}

fn load_retry_audio_samples_f32(audio_path: &str) -> Result<Vec<f32>, String> {
    let reader = hound::WavReader::open(audio_path)
        .map_err(|e| format!("Failed to open audio file: {e}"))?;

    let spec = reader.spec();
    let samples: Vec<i16> = reader
        .into_samples()
        .filter_map(|sample| sample.ok())
        .collect();

    if samples.is_empty() {
        return Err("Audio file is empty".to_string());
    }

    let samples_f32: Vec<f32> = samples
        .iter()
        .map(|&sample| sample as f32 / 32768.0)
        .collect();

    let mono_f32 = if spec.channels == 2 {
        samples_f32
            .chunks(2)
            .map(|chunk| {
                let left = chunk.first().copied().unwrap_or(0.0);
                let right = chunk.get(1).copied().unwrap_or(0.0);
                (left + right) / 2.0
            })
            .collect::<Vec<f32>>()
    } else {
        samples_f32
    };

    if spec.sample_rate != 16000 {
        let ratio = 16000.0 / spec.sample_rate as f32;
        let target_len = (mono_f32.len() as f32 * ratio) as usize;
        Ok(mono_f32
            .iter()
            .enumerate()
            .filter_map(|(i, _)| {
                let src_idx = (i as f32 / ratio) as usize;
                mono_f32.get(src_idx).copied()
            })
            .take(target_len)
            .collect())
    } else {
        Ok(mono_f32)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_retry_entry_updates, cleanup_retry_audio_file, load_retry_audio_samples_f32,
        prepare_retry_transcription, RetryExecutionPlan, RetryProviderPolicy,
        RetryTranscriptionOutput,
    };
    use crate::history::models::TranscriptionEntry;
    use crate::state::app_state::AppState;
    use crate::stt_engine::traits::EngineType;
    use tempfile::NamedTempFile;

    fn retry_entry(status: &str, audio_path: Option<String>) -> TranscriptionEntry {
        TranscriptionEntry {
            id: "entry-1".to_string(),
            created_at: 0,
            raw_text: String::new(),
            final_text: String::new(),
            stt_engine: "whisper".to_string(),
            stt_model: Some("whisper-base".to_string()),
            language: Some("en-US".to_string()),
            audio_duration_ms: None,
            stt_duration_ms: None,
            polish_duration_ms: None,
            total_duration_ms: None,
            polish_applied: false,
            polish_engine: None,
            is_cloud: false,
            audio_path,
            status: status.to_string(),
            error: Some("failed".to_string()),
        }
    }

    #[test]
    fn prepare_retry_transcription_requires_error_state_and_audio_file() {
        let state = AppState::new();
        let success_err = prepare_retry_transcription(
            &state,
            "entry-1".to_string(),
            retry_entry("success", None),
        )
        .unwrap_err();
        assert_eq!(success_err, "Entry is not in error state");

        let missing_audio_err =
            prepare_retry_transcription(&state, "entry-1".to_string(), retry_entry("error", None))
                .unwrap_err();
        assert_eq!(missing_audio_err, "No audio file saved for this entry");
    }

    #[test]
    fn prepare_retry_transcription_accepts_existing_failed_audio() {
        let state = AppState::new();
        let audio = NamedTempFile::new().unwrap();
        let prepared = prepare_retry_transcription(
            &state,
            "entry-9".to_string(),
            retry_entry("error", Some(audio.path().display().to_string())),
        )
        .unwrap();

        assert_eq!(prepared.task_id, 1);
        assert_eq!(prepared.entry_id, "entry-9");
        assert_eq!(prepared.audio_path, audio.path().display().to_string());
        assert_eq!(
            prepared.plan.policy,
            RetryProviderPolicy::CurrentLocalConfig
        );
        assert!(!prepared.plan.is_cloud);
        assert!(!prepared.plan.engine_name.is_empty());
        assert!(!prepared.plan.model_name.is_empty());
        assert!(!prepared.plan.language.is_empty());
    }

    #[test]
    fn build_retry_entry_updates_uses_execution_plan_metadata() {
        let output = RetryTranscriptionOutput {
            raw_text: "raw text".to_string(),
            stt_duration_ms: 222,
            plan: RetryExecutionPlan {
                policy: RetryProviderPolicy::CurrentLocalConfig,
                engine_type: EngineType::SenseVoice,
                engine_name: "sensevoice".to_string(),
                model_name: "sense-voice-small".to_string(),
                language: "zh-CN".to_string(),
                is_cloud: false,
            },
        };

        let updates = build_retry_entry_updates(&output, "final text", 321);

        assert_eq!(updates.raw_text, "raw text");
        assert_eq!(updates.final_text, "final text");
        assert_eq!(updates.stt_engine, "sensevoice");
        assert_eq!(updates.stt_model, Some("sense-voice-small".to_string()));
        assert_eq!(updates.language, Some("zh-CN".to_string()));
        assert_eq!(updates.stt_duration_ms, Some(222));
        assert_eq!(updates.polish_duration_ms, Some(321));
        assert!(updates.polish_applied);
        assert!(!updates.is_cloud);
    }

    #[test]
    fn prepare_retry_transcription_rejects_missing_audio_file_on_disk() {
        let state = AppState::new();
        let missing_path = std::env::temp_dir()
            .join(format!("retry-missing-{}.wav", std::process::id()))
            .display()
            .to_string();

        let error = prepare_retry_transcription(
            &state,
            "entry-404".to_string(),
            retry_entry("error", Some(missing_path.clone())),
        )
        .unwrap_err();

        assert_eq!(error, format!("Audio file not found: {missing_path}"));
    }

    #[test]
    fn cleanup_retry_audio_file_removes_existing_audio() {
        let audio = NamedTempFile::new().unwrap();
        let audio_path = audio.into_temp_path();
        let owned_path = audio_path.to_path_buf();
        let path_string = owned_path.display().to_string();
        audio_path.keep().unwrap();

        cleanup_retry_audio_file(&path_string);

        assert!(!owned_path.exists());
    }

    #[test]
    fn load_retry_audio_samples_f32_converts_stereo_audio_to_mono() {
        let audio = NamedTempFile::new().unwrap();
        let audio_path = audio.path().to_path_buf();
        {
            let spec = hound::WavSpec {
                channels: 2,
                sample_rate: 16_000,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            };
            let mut writer = hound::WavWriter::create(&audio_path, spec).unwrap();
            for sample in [1000_i16, -1000, 2000, 0, 0, 2000, -2000, -2000] {
                writer.write_sample(sample).unwrap();
            }
            writer.finalize().unwrap();
        }

        let samples = load_retry_audio_samples_f32(&audio_path.display().to_string()).unwrap();

        assert_eq!(samples.len(), 4);
        assert_eq!(samples[0], 0.0);
        assert!((samples[1] - (1000.0 / 32768.0)).abs() < f32::EPSILON);
        assert!((samples[2] - (1000.0 / 32768.0)).abs() < f32::EPSILON);
        assert!((samples[3] - (-2000.0 / 32768.0)).abs() < f32::EPSILON);
    }

    #[test]
    fn load_retry_audio_samples_f32_rejects_empty_audio_file() {
        let audio = NamedTempFile::new().unwrap();
        let audio_path = audio.path().to_path_buf();
        {
            let spec = hound::WavSpec {
                channels: 1,
                sample_rate: 16_000,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            };
            let writer = hound::WavWriter::create(&audio_path, spec).unwrap();
            writer.finalize().unwrap();
        }

        let error = load_retry_audio_samples_f32(&audio_path.display().to_string()).unwrap_err();

        assert_eq!(error, "Audio file is empty");
    }
}
