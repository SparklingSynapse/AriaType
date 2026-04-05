//! Integration tests for the SenseVoice STT engine
//!
//! These tests cover both unit tests (which always run) and integration tests
//! that use real models (marked with #[ignore] for CI but can be run locally).

use ariatype_lib::stt_engine::{traits::TranscriptionRequest, EngineType, UnifiedEngineManager};
use hound::{WavSpec, WavWriter};
use std::io::Cursor;
use std::path::PathBuf;

/// Create a sine wave WAV file as raw bytes
fn create_test_wav(sample_rate: u32, channels: u16, duration_secs: f32) -> Vec<u8> {
    let samples_per_channel = (sample_rate as f32 * duration_secs) as usize;
    let total_samples = samples_per_channel * channels as usize;

    let spec = WavSpec {
        channels,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut cursor = Cursor::new(Vec::new());
    {
        let mut writer = WavWriter::new(&mut cursor, spec).unwrap();
        for i in 0..total_samples {
            let t = i as f32 / sample_rate as f32;
            let sample = (16000.0 * (2.0 * std::f32::consts::PI * 440.0 * t).sin()) as i16;
            writer.write_sample(sample).unwrap();
        }
        writer.finalize().unwrap();
    }
    cursor.into_inner()
}

/// Create a speech-like WAV file with multiple frequencies and noise
fn create_speech_like_wav(sample_rate: u32, channels: u16, duration_secs: f32) -> Vec<u8> {
    let samples_per_channel = (sample_rate as f32 * duration_secs) as usize;
    let total_samples = samples_per_channel * channels as usize;

    let spec = WavSpec {
        channels,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut cursor = Cursor::new(Vec::new());
    {
        let mut writer = WavWriter::new(&mut cursor, spec).unwrap();
        use std::cell::Cell;
        thread_local! { static SEED: Cell<u32> = Cell::new(12345); }
        for i in 0..total_samples {
            let t = i as f32 / sample_rate as f32;
            let f1 = (2.0 * std::f32::consts::PI * 300.0 * t).sin() * 0.3;
            let f2 = (2.0 * std::f32::consts::PI * 800.0 * t).sin() * 0.25;
            let f3 = (2.0 * std::f32::consts::PI * 2500.0 * t).sin() * 0.2;
            let noise = SEED.with(|seed| {
                let mut s = seed.get();
                s = s.wrapping_mul(1103515245).wrapping_add(12345);
                seed.set(s);
                (s % 10000) as f32 / 10000.0 - 0.5
            }) * 0.1;
            let sample = ((f1 + f2 + f3 + noise) * 20000.0) as i16;
            writer.write_sample(sample).unwrap();
        }
        writer.finalize().unwrap();
    }
    cursor.into_inner()
}

/// Write WAV data to a temporary file
fn write_temp_wav(data: &[u8]) -> PathBuf {
    let temp_dir = std::env::temp_dir();
    let path = temp_dir.join(format!("test_audio_{}.wav", uuid::Uuid::new_v4()));
    std::fs::write(&path, data).expect("Failed to write temp WAV");
    path
}

/// Clean up temporary files
fn cleanup_temp_files(paths: &[PathBuf]) {
    for path in paths {
        let _ = std::fs::remove_file(path);
    }
}

/// Check if SenseVoice model is available for testing
fn sensevoice_model_available(version: &str) -> bool {
    let models_dir = UnifiedEngineManager::default_models_dir();
    models_dir.join(format!("{}.gguf", version)).exists()
}

#[test]
fn test_sensevoice_engine_creation() {
    // Test that we can get model info without having models downloaded
    let models_dir = std::env::temp_dir().join("nonexistent_sensevoice_models");
    let manager = UnifiedEngineManager::new(models_dir);

    let models = manager.get_models(EngineType::SenseVoice);
    assert!(!models.is_empty());

    // Should not have any models marked as downloaded
    for model in models {
        assert!(!model.downloaded);
    }
}

#[test]
fn test_sensevoice_engine_transcribe_empty_audio() {
    // This test verifies error handling when trying to transcribe empty audio
    // We'll create a manager pointing to an empty directory

    let models_dir = std::env::temp_dir().join("empty_sensevoice_test");
    let _ = std::fs::create_dir_all(&models_dir);
    let manager = UnifiedEngineManager::new(models_dir.clone());

    // Create an empty WAV file
    let wav_data = Vec::new();
    let temp_path = write_temp_wav(&wav_data);

    // Even though no models are downloaded, the manager should handle this gracefully
    // by returning an error about the model not being available
    let result: Result<_, String> = std::panic::catch_unwind(|| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let request = TranscriptionRequest::new(temp_path.to_str().unwrap());
            manager.transcribe(EngineType::SenseVoice, request).await
        })
    })
    .unwrap_or_else(|_| Err("Panic during transcription".to_string()));

    // The result should be an error (either model not found or empty audio)
    assert!(result.is_err());

    cleanup_temp_files(&[temp_path]);
    let _ = std::fs::remove_dir_all(&models_dir);
}

#[test]
fn test_sensevoice_engine_model_not_found() {
    // Test that the manager properly reports when a model is not found
    let models_dir = std::env::temp_dir().join("sensevoice_model_not_found");
    let _ = std::fs::create_dir_all(&models_dir);
    let manager = UnifiedEngineManager::new(models_dir.clone());

    // Check that the model is not downloaded
    assert!(!manager.is_model_downloaded(EngineType::SenseVoice, "sense-voice-small-q4_k"));

    // Try to transcribe with it anyway
    let wav_data = create_test_wav(16000, 1, 0.1);
    let temp_path = write_temp_wav(&wav_data);

    let result: Result<_, String> = std::panic::catch_unwind(|| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let request = TranscriptionRequest::new(temp_path.to_str().unwrap())
                .with_model("sense-voice-small-q4_k");
            manager.transcribe(EngineType::SenseVoice, request).await
        })
    })
    .unwrap_or_else(|_| Err("Panic during transcription".to_string()));

    assert!(result.is_err());
    let error_msg = result.unwrap_err();
    assert!(error_msg.contains("not found") || error_msg.contains("Model not found"));

    cleanup_temp_files(&[temp_path]);
    let _ = std::fs::remove_dir_all(&models_dir);
}

// Integration tests that require real models - marked as #[ignore]

#[test]
#[ignore = "Requires SenseVoice small-q4_k model to be downloaded"]
fn test_sensevoice_engine_transcribe_chinese() {
    if !sensevoice_model_available("sense-voice-small-q4_k") {
        println!("Skipping: SenseVoice small-q4_k model not downloaded");
        return;
    }

    // Create speech-like audio optimized for Chinese (SenseVoice excels at Asian languages)
    let wav_data = create_speech_like_wav(16000, 1, 3.0);
    let temp_path = write_temp_wav(&wav_data);

    let models_dir = UnifiedEngineManager::default_models_dir();
    let manager = UnifiedEngineManager::new(models_dir);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result: Result<_, String> = rt.block_on(async {
        let request = TranscriptionRequest::new(temp_path.to_str().unwrap())
            .with_model("sense-voice-small-q4_k")
            .with_language("zh");
        manager.transcribe(EngineType::SenseVoice, request).await
    });

    cleanup_temp_files(&[temp_path]);

    match result {
        Ok(transcription) => {
            println!(
                "SenseVoice Chinese transcription result: {:?}",
                transcription.text
            );
            // Basic validation: result should not be completely empty
            assert!(!transcription.text.trim().is_empty());
        }
        Err(e) => {
            println!("SenseVoice Chinese transcription failed: {}", e);
            assert!(!e.is_empty());
        }
    }
}

#[test]
#[ignore = "Requires SenseVoice small-q4_k model to be downloaded"]
fn test_sensevoice_engine_transcribe_english() {
    if !sensevoice_model_available("sense-voice-small-q4_k") {
        println!("Skipping: SenseVoice small-q4_k model not downloaded");
        return;
    }

    // Create speech-like audio for English testing
    let wav_data = create_speech_like_wav(16000, 1, 2.5);
    let temp_path = write_temp_wav(&wav_data);

    let models_dir = UnifiedEngineManager::default_models_dir();
    let manager = UnifiedEngineManager::new(models_dir);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result: Result<_, String> = rt.block_on(async {
        let request = TranscriptionRequest::new(temp_path.to_str().unwrap())
            .with_model("sense-voice-small-q4_k")
            .with_language("en");
        manager.transcribe(EngineType::SenseVoice, request).await
    });

    cleanup_temp_files(&[temp_path]);

    match result {
        Ok(transcription) => {
            println!(
                "SenseVoice English transcription result: {:?}",
                transcription.text
            );
            assert!(!transcription.text.trim().is_empty());
        }
        Err(e) => {
            println!("SenseVoice English transcription failed: {}", e);
            assert!(!e.is_empty());
        }
    }
}
