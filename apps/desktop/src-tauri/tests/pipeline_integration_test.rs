//! Core Pipeline Integration Test
//!
//! Tests the complete flow: Audio Input → Transcribe → Polish → Output
//! This is the ultimate validation that the application works correctly.

use ariatype_lib::stt_engine::SttEngine;
use std::io::Cursor;
use std::path::PathBuf;

fn create_test_wav(sample_rate: u32, channels: u16, duration_secs: f32) -> Vec<u8> {
    let samples_per_channel = (sample_rate as f32 * duration_secs) as usize;
    let total_samples = samples_per_channel * channels as usize;

    let spec = hound::WavSpec {
        channels,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut cursor = Cursor::new(Vec::new());
    {
        let mut writer = hound::WavWriter::new(&mut cursor, spec).unwrap();
        for i in 0..total_samples {
            let t = i as f32 / sample_rate as f32;
            let sample = (16000.0 * (2.0 * std::f32::consts::PI * 440.0 * t).sin()) as i16;
            writer.write_sample(sample).unwrap();
        }
        writer.finalize().unwrap();
    }
    cursor.into_inner()
}

fn create_speech_like_wav(sample_rate: u32, channels: u16, duration_secs: f32) -> Vec<u8> {
    let samples_per_channel = (sample_rate as f32 * duration_secs) as usize;
    let total_samples = samples_per_channel * channels as usize;

    let spec = hound::WavSpec {
        channels,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut cursor = Cursor::new(Vec::new());
    {
        let mut writer = hound::WavWriter::new(&mut cursor, spec).unwrap();
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

fn write_temp_wav(data: &[u8]) -> PathBuf {
    let temp_dir = std::env::temp_dir();
    let path = temp_dir.join(format!("test_audio_{}.wav", uuid::Uuid::new_v4()));
    std::fs::write(&path, data).expect("Failed to write temp WAV");
    path
}

fn cleanup_temp_files(paths: &[PathBuf]) {
    for path in paths {
        let _ = std::fs::remove_file(path);
    }
}

// ==================== WAV Format Tests ====================

#[test]
fn test_create_test_wav_basic() {
    let wav_data = create_test_wav(16000, 1, 1.0);
    let reader = hound::WavReader::new(Cursor::new(&wav_data)).unwrap();
    let spec = reader.spec();

    assert_eq!(spec.sample_rate, 16000);
    assert_eq!(spec.channels, 1);
    assert_eq!(spec.bits_per_sample, 16);

    let duration = reader.duration() as f32 / spec.sample_rate as f32;
    assert!((duration - 1.0).abs() < 0.01);
}

#[test]
fn test_create_test_wav_stereo() {
    let wav_data = create_test_wav(44100, 2, 0.5);
    let reader = hound::WavReader::new(Cursor::new(&wav_data)).unwrap();
    let spec = reader.spec();

    assert_eq!(spec.sample_rate, 44100);
    assert_eq!(spec.channels, 2);

    let samples: Vec<i16> = reader.into_samples().filter_map(|s| s.ok()).collect();
    let expected = (44100.0 * 0.5 * 2.0) as usize;
    assert!(samples.len() >= expected - 100 && samples.len() <= expected + 100);
}

// ==================== Resampling Tests ====================

#[test]
fn test_resampling_pipeline() {
    let wav_data = create_test_wav(44100, 1, 2.0);
    let reader = hound::WavReader::new(Cursor::new(&wav_data)).unwrap();
    let samples_i16: Vec<i16> = reader
        .into_samples::<i16>()
        .filter_map(|s| s.ok())
        .collect();

    let audio_f32: Vec<f32> = samples_i16.iter().map(|&s| s as f32 / 32768.0).collect();
    let resampled = ariatype_lib::audio::resampler::resample_to_16khz(&audio_f32, 44100).unwrap();

    let expected = 32000.0;
    let tolerance = expected * 0.05;
    assert!(
        resampled.len() as f32 >= expected - tolerance
            && resampled.len() as f32 <= expected + tolerance,
        "Expected ~{} samples, got {}",
        expected,
        resampled.len()
    );
}

#[test]
fn test_stereo_downmix() {
    let wav_data = create_test_wav(16000, 2, 1.0);
    let reader = hound::WavReader::new(Cursor::new(&wav_data)).unwrap();
    let samples_i16: Vec<i16> = reader
        .into_samples::<i16>()
        .filter_map(|s| s.ok())
        .collect();

    assert_eq!(samples_i16.len(), 32000, "Stereo should have 32000 samples");

    let audio_f32: Vec<f32> = samples_i16.iter().map(|&s| s as f32 / 32768.0).collect();
    let mono: Vec<f32> = audio_f32
        .chunks(2)
        .map(|stereo| (stereo[0] + stereo.get(1).copied().unwrap_or(0.0)) / 2.0)
        .collect();

    assert_eq!(mono.len(), 16000, "Mono should have 16000 samples");
}

#[test]
fn test_chunking_for_streaming() {
    let wav_data = create_test_wav(16000, 1, 5.0);
    let reader = hound::WavReader::new(Cursor::new(&wav_data)).unwrap();
    let samples_i16: Vec<i16> = reader
        .into_samples::<i16>()
        .filter_map(|s| s.ok())
        .collect();

    const CHUNK_SIZE: usize = 3200;
    let chunks: Vec<&[i16]> = samples_i16.chunks(CHUNK_SIZE).collect();

    assert!(chunks.len() >= 25, "Should have at least 25 chunks");

    let chunk_duration_ms = (CHUNK_SIZE as f64 / 16000.0) * 1000.0;
    assert!(
        (chunk_duration_ms - 200.0).abs() < 1.0,
        "Chunk should be 200ms"
    );
}

// ==================== Protocol Header Tests ====================

#[test]
fn test_protocol_header_construction() {
    const PROTOCOL_VERSION: u8 = 0b0001;
    const HEADER_SIZE: u8 = 0b0001;
    const MESSAGE_TYPE_FULL_CLIENT_REQUEST: u8 = 0b0001;
    const SERIALIZATION_JSON: u8 = 0b0001;
    const COMPRESSION_NONE: u8 = 0b0000;

    fn build_header(message_type: u8, flags: u8, serialization: u8, compression: u8) -> [u8; 4] {
        let byte0 = (PROTOCOL_VERSION << 4) | HEADER_SIZE;
        let byte1 = (message_type << 4) | flags;
        let byte2 = (serialization << 4) | compression;
        [byte0, byte1, byte2, 0x00]
    }

    let header = build_header(
        MESSAGE_TYPE_FULL_CLIENT_REQUEST,
        0b0000,
        SERIALIZATION_JSON,
        COMPRESSION_NONE,
    );

    assert_eq!(header[0], 0b00010001);
    assert_eq!(header[1], 0b00010000);
    assert_eq!(header[2], 0b00010000);
    assert_eq!(header[3], 0x00);
}

#[test]
fn test_audio_chunk_header() {
    const PROTOCOL_VERSION: u8 = 0b0001;
    const HEADER_SIZE: u8 = 0b0001;
    const MESSAGE_TYPE_AUDIO_ONLY_REQUEST: u8 = 0b0010;
    const SERIALIZATION_NONE: u8 = 0b0000;
    const COMPRESSION_NONE: u8 = 0b0000;

    fn build_header(message_type: u8, flags: u8, serialization: u8, compression: u8) -> [u8; 4] {
        let byte0 = (PROTOCOL_VERSION << 4) | HEADER_SIZE;
        let byte1 = (message_type << 4) | flags;
        let byte2 = (serialization << 4) | compression;
        [byte0, byte1, byte2, 0x00]
    }

    let header = build_header(
        MESSAGE_TYPE_AUDIO_ONLY_REQUEST,
        0b0000,
        SERIALIZATION_NONE,
        COMPRESSION_NONE,
    );

    assert_eq!(header[0], 0b00010001);
    assert_eq!(header[1], 0b00100000);
    assert_eq!(header[2], 0b00000000);
}

#[test]
fn test_last_packet_flag() {
    const PROTOCOL_VERSION: u8 = 0b0001;
    const HEADER_SIZE: u8 = 0b0001;
    const MESSAGE_TYPE_AUDIO_ONLY_REQUEST: u8 = 0b0010;
    const SERIALIZATION_NONE: u8 = 0b0000;
    const COMPRESSION_NONE: u8 = 0b0000;

    fn build_header(message_type: u8, flags: u8, serialization: u8, compression: u8) -> [u8; 4] {
        let byte0 = (PROTOCOL_VERSION << 4) | HEADER_SIZE;
        let byte1 = (message_type << 4) | flags;
        let byte2 = (serialization << 4) | compression;
        [byte0, byte1, byte2, 0x00]
    }

    let header = build_header(
        MESSAGE_TYPE_AUDIO_ONLY_REQUEST,
        0b0010,
        SERIALIZATION_NONE,
        COMPRESSION_NONE,
    );

    assert_eq!(header[1], 0b00100010);
}

#[test]
fn test_recommended_chunk_duration() {
    use ariatype_lib::stt_engine::cloud::volcengine_streaming::RECOMMENDED_CHUNK_SAMPLES;

    let duration_ms = (RECOMMENDED_CHUNK_SAMPLES as f64 / 16000.0) * 1000.0;
    assert!((duration_ms - 100.0).abs() < 1.0);
}

#[test]
fn test_streaming_mode_urls() {
    use ariatype_lib::stt_engine::cloud::volcengine_streaming::*;

    assert!(URL_BIGMODEL_NOSTREAM.contains("nostream"));
    assert!(URL_BIGMODEL.contains("bigmodel"));
    assert!(!URL_BIGMODEL.contains("nostream"));
    assert!(URL_BIGMODEL_ASYNC.contains("async"));
}

// ==================== Edge Cases ====================

#[test]
fn test_very_short_audio() {
    let wav_data = create_test_wav(16000, 1, 0.1);
    let reader = hound::WavReader::new(Cursor::new(&wav_data)).unwrap();

    let samples: Vec<i16> = reader.into_samples().filter_map(|s| s.ok()).collect();
    assert!(
        samples.len() >= 1600,
        "Should have at least 1600 samples for 100ms"
    );
}

#[test]
fn test_high_sample_rate() {
    let wav_data = create_test_wav(96000, 1, 1.0);
    let reader = hound::WavReader::new(Cursor::new(&wav_data)).unwrap();
    let samples_i16: Vec<i16> = reader
        .into_samples::<i16>()
        .filter_map(|s| s.ok())
        .collect();

    let audio_f32: Vec<f32> = samples_i16.iter().map(|&s| s as f32 / 32768.0).collect();
    let resampled = ariatype_lib::audio::resampler::resample_to_16khz(&audio_f32, 96000).unwrap();

    let expected = 16000.0;
    let tolerance = expected * 0.05;
    assert!(
        resampled.len() as f32 >= expected - tolerance
            && resampled.len() as f32 <= expected + tolerance
    );
}

// ==================== Mock Engine Tests ====================

mod mock_stt {
    use ariatype_lib::stt_engine::{EngineType, TranscriptionRequest, TranscriptionResult};

    pub struct MockSttEngine {
        pub result_text: String,
        pub latency_ms: u64,
        pub should_fail: bool,
    }

    impl MockSttEngine {
        pub fn new(text: impl Into<String>) -> Self {
            Self {
                result_text: text.into(),
                latency_ms: 100,
                should_fail: false,
            }
        }

        pub fn with_latency(mut self, latency_ms: u64) -> Self {
            self.latency_ms = latency_ms;
            self
        }

        pub fn with_failure(mut self) -> Self {
            self.should_fail = true;
            self
        }

        pub async fn transcribe(
            &self,
            _request: TranscriptionRequest,
        ) -> Result<TranscriptionResult, String> {
            tokio::time::sleep(tokio::time::Duration::from_millis(self.latency_ms)).await;

            if self.should_fail {
                return Err("Mock STT engine failed".to_string());
            }

            Ok(TranscriptionResult::with_metrics(
                self.result_text.clone(),
                EngineType::Whisper,
                self.latency_ms,
                Some(50),
                Some(20),
                Some(30),
            ))
        }
    }
}

mod mock_polish {
    use ariatype_lib::polish_engine::{PolishEngineType, PolishRequest, PolishResult};

    pub struct MockPolishEngine {
        pub result_text: String,
        pub latency_ms: u64,
        pub should_fail: bool,
    }

    impl MockPolishEngine {
        pub fn new(text: impl Into<String>) -> Self {
            Self {
                result_text: text.into(),
                latency_ms: 50,
                should_fail: false,
            }
        }

        pub fn with_latency(mut self, latency_ms: u64) -> Self {
            self.latency_ms = latency_ms;
            self
        }

        pub fn with_failure(mut self) -> Self {
            self.should_fail = true;
            self
        }

        pub async fn polish(&self, _request: PolishRequest) -> Result<PolishResult, String> {
            tokio::time::sleep(tokio::time::Duration::from_millis(self.latency_ms)).await;

            if self.should_fail {
                return Err("Mock polish engine failed".to_string());
            }

            Ok(PolishResult::with_metrics(
                self.result_text.clone(),
                PolishEngineType::Qwen,
                self.latency_ms,
                Some(10),
                Some(40),
            ))
        }
    }
}

#[tokio::test]
async fn test_mock_stt_engine_success() {
    let engine = mock_stt::MockSttEngine::new("Hello world");

    let request = ariatype_lib::stt_engine::TranscriptionRequest::new("test.wav");
    let result = engine.transcribe(request).await.unwrap();

    assert_eq!(result.text, "Hello world");
    assert_eq!(result.engine, ariatype_lib::stt_engine::EngineType::Whisper);
    assert!(result.total_ms >= 100);
}

#[tokio::test]
async fn test_mock_stt_engine_failure() {
    let engine = mock_stt::MockSttEngine::new("Should not appear").with_failure();

    let request = ariatype_lib::stt_engine::TranscriptionRequest::new("test.wav");
    let result = engine.transcribe(request).await;

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("failed"));
}

#[tokio::test]
async fn test_mock_polish_engine_success() {
    let engine = mock_polish::MockPolishEngine::new("Polished text");

    let request =
        ariatype_lib::polish_engine::PolishRequest::new("Raw text", "System prompt", "en");
    let result = engine.polish(request).await.unwrap();

    assert_eq!(result.text, "Polished text");
    assert_eq!(
        result.engine,
        ariatype_lib::polish_engine::PolishEngineType::Qwen
    );
}

#[tokio::test]
async fn test_mock_polish_engine_failure() {
    let engine = mock_polish::MockPolishEngine::new("Should not appear").with_failure();

    let request =
        ariatype_lib::polish_engine::PolishRequest::new("Raw text", "System prompt", "en");
    let result = engine.polish(request).await;

    assert!(result.is_err());
}

// ==================== Pipeline Integration Tests ====================

async fn run_pipeline(
    audio_path: &str,
    stt_result: &str,
    polish_result: &str,
    polish_enabled: bool,
) -> Result<String, String> {
    let stt = mock_stt::MockSttEngine::new(stt_result);
    let polish = mock_polish::MockPolishEngine::new(polish_result);

    let stt_request = ariatype_lib::stt_engine::TranscriptionRequest::new(audio_path);
    let stt_result = stt.transcribe(stt_request).await?;

    if polish_enabled && !stt_result.text.is_empty() {
        let polish_request = ariatype_lib::polish_engine::PolishRequest::new(
            stt_result.text.clone(),
            "Polish this text",
            "en",
        );
        let polish_result = polish.polish(polish_request).await?;
        Ok(polish_result.text)
    } else {
        Ok(stt_result.text)
    }
}

#[tokio::test]
async fn test_pipeline_transcribe_only() {
    let wav_data = create_test_wav(16000, 1, 1.0);
    let temp_path = write_temp_wav(&wav_data);

    let result = run_pipeline(
        temp_path.to_str().unwrap(),
        "This is a test transcription",
        "This should not be used",
        false,
    )
    .await
    .unwrap();

    assert_eq!(result, "This is a test transcription");

    cleanup_temp_files(&[temp_path]);
}

#[tokio::test]
async fn test_pipeline_transcribe_and_polish() {
    let wav_data = create_test_wav(16000, 1, 1.0);
    let temp_path = write_temp_wav(&wav_data);

    let result = run_pipeline(
        temp_path.to_str().unwrap(),
        "um hello world uh",
        "hello world",
        true,
    )
    .await
    .unwrap();

    assert_eq!(result, "hello world");

    cleanup_temp_files(&[temp_path]);
}

#[tokio::test]
async fn test_pipeline_stt_fails_gracefully() {
    let wav_data = create_test_wav(16000, 1, 1.0);
    let temp_path = write_temp_wav(&wav_data);

    let stt = mock_stt::MockSttEngine::new("Should fail").with_failure();
    let request = ariatype_lib::stt_engine::TranscriptionRequest::new(temp_path.to_str().unwrap());
    let result = stt.transcribe(request).await;

    assert!(result.is_err());

    cleanup_temp_files(&[temp_path]);
}

#[tokio::test]
async fn test_pipeline_empty_transcription() {
    let wav_data = create_test_wav(16000, 1, 1.0);
    let temp_path = write_temp_wav(&wav_data);

    let result = run_pipeline(
        temp_path.to_str().unwrap(),
        "",
        "Should not be called",
        true,
    )
    .await
    .unwrap();

    assert_eq!(result, "");

    cleanup_temp_files(&[temp_path]);
}

// ==================== Performance Benchmarks ====================

#[test]
fn benchmark_resampling_performance() {
    use std::time::Instant;

    let samples: Vec<f32> = (0..441000)
        .map(|i| {
            let t = i as f32 / 44100.0;
            (2.0 * std::f32::consts::PI * 440.0 * t).sin() * 0.5
        })
        .collect();

    let start = Instant::now();
    let resampled = ariatype_lib::audio::resampler::resample_to_16khz(&samples, 44100).unwrap();
    let duration = start.elapsed();

    println!("Resampled 10s of 44.1kHz audio to 16kHz in {:?}", duration);
    println!("Output samples: {}", resampled.len());

    // Allow up to 3 seconds for resampling on slower machines
    // This is a performance benchmark, not a correctness test
    assert!(duration.as_millis() < 3000, "Resampling took too long");
}

#[tokio::test]
async fn benchmark_pipeline_throughput() {
    let iterations = 10;
    let mut total_time = 0u64;

    for _ in 0..iterations {
        let wav_data = create_test_wav(16000, 1, 1.0);
        let temp_path = write_temp_wav(&wav_data);

        let start = std::time::Instant::now();
        let _ = run_pipeline(temp_path.to_str().unwrap(), "Test", "Test", true).await;
        total_time += start.elapsed().as_millis() as u64;

        cleanup_temp_files(&[temp_path]);
    }

    let avg_time = total_time / iterations;
    println!("Average pipeline time: {}ms", avg_time);

    assert!(avg_time < 200, "Pipeline too slow for mocks");
}

// ==================== Real Model Tests (Ignored by Default) ====================

#[test]
#[ignore = "Requires real model to be downloaded"]
fn test_whisper_real_transcription() {
    let wav_data = create_test_wav(16000, 1, 2.0);
    let temp_path = write_temp_wav(&wav_data);

    let models_dir = ariatype_lib::stt_engine::UnifiedEngineManager::default_models_dir();
    let manager = ariatype_lib::stt_engine::UnifiedEngineManager::new(models_dir);

    if !manager.is_model_downloaded(ariatype_lib::stt_engine::EngineType::Whisper, "base") {
        println!("Skipping: Whisper base model not downloaded");
        cleanup_temp_files(&[temp_path]);
        return;
    }

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        let request =
            ariatype_lib::stt_engine::TranscriptionRequest::new(temp_path.to_str().unwrap())
                .with_model("base")
                .with_language("en");
        manager
            .transcribe(ariatype_lib::stt_engine::EngineType::Whisper, request)
            .await
    });

    println!("Whisper result: {:?}", result);

    cleanup_temp_files(&[temp_path]);
}

#[test]
#[ignore = "Requires real model to be downloaded"]
fn test_sensevoice_real_transcription() {
    let wav_data = create_test_wav(16000, 1, 2.0);
    let temp_path = write_temp_wav(&wav_data);

    let models_dir = ariatype_lib::stt_engine::UnifiedEngineManager::default_models_dir();
    let manager = ariatype_lib::stt_engine::UnifiedEngineManager::new(models_dir);

    if !manager.is_model_downloaded(
        ariatype_lib::stt_engine::EngineType::SenseVoice,
        "sense-voice-small-q4_k",
    ) {
        println!("Skipping: SenseVoice model not downloaded");
        cleanup_temp_files(&[temp_path]);
        return;
    }

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        let request =
            ariatype_lib::stt_engine::TranscriptionRequest::new(temp_path.to_str().unwrap())
                .with_model("sense-voice-small-q4_k")
                .with_language("zh");
        manager
            .transcribe(ariatype_lib::stt_engine::EngineType::SenseVoice, request)
            .await
    });

    println!("SenseVoice result: {:?}", result);

    cleanup_temp_files(&[temp_path]);
}

#[test]
#[ignore = "Requires all models to be downloaded"]
fn test_full_pipeline_integration() {
    let wav_data = create_speech_like_wav(16000, 1, 5.0);
    let temp_path = write_temp_wav(&wav_data);

    let models_dir = ariatype_lib::stt_engine::UnifiedEngineManager::default_models_dir();
    let stt_manager = ariatype_lib::stt_engine::UnifiedEngineManager::new(models_dir);
    let polish_manager = ariatype_lib::polish_engine::UnifiedPolishManager::new();

    if !stt_manager.is_model_downloaded(
        ariatype_lib::stt_engine::EngineType::SenseVoice,
        "sense-voice-small-q4_k",
    ) {
        println!("Skipping: STT model not downloaded");
        cleanup_temp_files(&[temp_path]);
        return;
    }

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        let stt_request =
            ariatype_lib::stt_engine::TranscriptionRequest::new(temp_path.to_str().unwrap())
                .with_model("sense-voice-small-q4_k")
                .with_language("zh");

        let stt_result = stt_manager
            .transcribe(
                ariatype_lib::stt_engine::EngineType::SenseVoice,
                stt_request,
            )
            .await?;

        let stt_text = stt_result.text.clone();
        println!("STT result: {}", stt_text);

        let final_text = if !stt_text.is_empty() {
            let polish_model_id = "lfm2.5-1.2b";
            if let Some(engine_type) =
                ariatype_lib::polish_engine::UnifiedPolishManager::get_engine_by_model_id(
                    polish_model_id,
                )
            {
                if polish_manager.is_model_downloaded(engine_type, polish_model_id) {
                    let polish_request = ariatype_lib::polish_engine::PolishRequest::new(
                        stt_text.clone(),
                        "Polish and remove filler words",
                        "zh",
                    )
                    .with_model(polish_model_id);

                    match polish_manager.polish(engine_type, polish_request).await {
                        Ok(result) if !result.text.is_empty() => result.text,
                        _ => stt_text,
                    }
                } else {
                    stt_text
                }
            } else {
                stt_text
            }
        } else {
            stt_text
        };

        Ok::<_, String>((stt_result, final_text))
    });

    match result {
        Ok((stt_result, final_text)) => {
            println!("Pipeline completed successfully!");
            println!("STT time: {}ms", stt_result.total_ms);
            println!("Final text: {}", final_text);
            assert!(
                !final_text.is_empty() || stt_result.text.is_empty(),
                "Pipeline should produce output"
            );
        }
        Err(e) => {
            println!("Pipeline failed: {}", e);
            panic!("Pipeline integration test failed");
        }
    }

    cleanup_temp_files(&[temp_path]);
}

// ==================== New Pipeline Integration Tests ====================

#[tokio::test]
#[ignore = "Requires cloud STT API key configuration"]
async fn test_pipeline_with_cloud_stt() {
    // Skip if no cloud STT config is available
    if std::env::var("ARIATYPE_CLOUD_STT_API_KEY").is_err() {
        println!("Skipping cloud STT test: ARIATYPE_CLOUD_STT_API_KEY not set");
        return;
    }

    let wav_data = create_speech_like_wav(16000, 1, 2.0);
    let temp_path = write_temp_wav(&wav_data);

    let mut cloud_config = ariatype_lib::commands::settings::CloudSttConfig::default();
    cloud_config.enabled = true;
    cloud_config.api_key = std::env::var("ARIATYPE_CLOUD_STT_API_KEY").unwrap_or_default();
    cloud_config.provider_type =
        std::env::var("ARIATYPE_CLOUD_STT_PROVIDER").unwrap_or_else(|_| "openai".to_string());
    cloud_config.model =
        std::env::var("ARIATYPE_CLOUD_STT_MODEL").unwrap_or_else(|_| "whisper-1".to_string());
    cloud_config.language =
        std::env::var("ARIATYPE_CLOUD_STT_LANGUAGE").unwrap_or_else(|_| "en".to_string());

    // Set optional base URL if provided
    if let Ok(base_url) = std::env::var("ARIATYPE_CLOUD_STT_BASE_URL") {
        cloud_config.base_url = base_url;
    }

    // Set app_id for volcengine providers if provided
    if let Ok(app_id) = std::env::var("ARIATYPE_CLOUD_STT_APP_ID") {
        cloud_config.app_id = app_id;
    }

    let stt_request =
        ariatype_lib::stt_engine::TranscriptionRequest::new(temp_path.to_str().unwrap())
            .with_cloud_config(cloud_config)
            .with_language("en");

    let engine = ariatype_lib::stt_engine::cloud::engine::CloudSttEngine::new()
        .map_err(|e| format!("Failed to create cloud STT engine: {}", e))
        .unwrap();

    let stt_result = engine.transcribe(stt_request).await.unwrap();

    // Test with polish disabled first
    let final_text_no_polish = stt_result.text.clone();
    assert!(
        !final_text_no_polish.trim().is_empty(),
        "Cloud STT should produce non-empty transcription"
    );

    // Test with polish enabled
    let polish_manager = ariatype_lib::polish_engine::UnifiedPolishManager::new();
    let polish_enabled = std::env::var("ARIATYPE_TEST_WITH_POLISH").is_ok();

    if polish_enabled && !stt_result.text.is_empty() {
        let polish_model_id = "lfm2.5-1.2b";
        if let Some(engine_type) =
            ariatype_lib::polish_engine::UnifiedPolishManager::get_engine_by_model_id(
                polish_model_id,
            )
        {
            if polish_manager.is_model_downloaded(engine_type, polish_model_id) {
                let polish_request = ariatype_lib::polish_engine::PolishRequest::new(
                    stt_result.text.clone(),
                    "Polish and remove filler words",
                    "en",
                )
                .with_model(polish_model_id);

                let polish_result = polish_manager
                    .polish(engine_type, polish_request)
                    .await
                    .unwrap();
                let final_text = polish_result.text;
                println!("Cloud STT + Polish result: {}", final_text);
                assert!(
                    !final_text.trim().is_empty(),
                    "Pipeline should produce non-empty output"
                );
            } else {
                println!("Polish model not downloaded, skipping polish step");
            }
        } else {
            println!("Unknown polish engine type for model: {}", polish_model_id);
        }
    } else {
        println!("Cloud STT result: {}", final_text_no_polish);
    }

    cleanup_temp_files(&[temp_path]);
}

#[tokio::test]
#[ignore = "Requires Whisper model to be downloaded"]
async fn test_pipeline_with_local_whisper() {
    let wav_data = create_speech_like_wav(16000, 1, 2.0);
    let temp_path = write_temp_wav(&wav_data);

    let models_dir = ariatype_lib::stt_engine::UnifiedEngineManager::default_models_dir();
    let manager = ariatype_lib::stt_engine::UnifiedEngineManager::new(models_dir);

    // Try different Whisper models in order of preference
    let whisper_models = ["medium", "small", "base", "tiny"];
    let mut selected_model = None;

    for model in &whisper_models {
        if manager.is_model_downloaded(ariatype_lib::stt_engine::EngineType::Whisper, model) {
            selected_model = Some(*model);
            break;
        }
    }

    if selected_model.is_none() {
        println!(
            "Skipping: No Whisper model downloaded (tried: {:?})",
            whisper_models
        );
        cleanup_temp_files(&[temp_path]);
        return;
    }

    let model_name = selected_model.unwrap();
    println!("Testing with Whisper {} model", model_name);

    let request = ariatype_lib::stt_engine::TranscriptionRequest::new(temp_path.to_str().unwrap())
        .with_model(model_name)
        .with_language("en");
    let stt_result = manager
        .transcribe(ariatype_lib::stt_engine::EngineType::Whisper, request)
        .await
        .unwrap();

    assert!(
        !stt_result.text.trim().is_empty(),
        "Whisper should produce non-empty transcription"
    );
    println!("Whisper STT result: {}", stt_result.text);

    // Test with polish
    let polish_manager = ariatype_lib::polish_engine::UnifiedPolishManager::new();
    let polish_model_id = "lfm2.5-1.2b";

    if let Some(engine_type) =
        ariatype_lib::polish_engine::UnifiedPolishManager::get_engine_by_model_id(polish_model_id)
    {
        if polish_manager.is_model_downloaded(engine_type, polish_model_id) {
            let polish_request = ariatype_lib::polish_engine::PolishRequest::new(
                stt_result.text.clone(),
                "Polish and remove filler words",
                "en",
            )
            .with_model(polish_model_id);

            match polish_manager.polish(engine_type, polish_request).await {
                Ok(polish_result) => {
                    let final_text = polish_result.text;
                    println!("Whisper + Polish result: {}", final_text);
                    assert!(
                        !final_text.trim().is_empty(),
                        "Polished result should be non-empty"
                    );
                }
                Err(e) => {
                    println!("Polish failed: {}", e);
                    // Don't fail the test if polish fails, as we're primarily testing STT
                }
            }
        } else {
            println!("Polish model not downloaded, testing STT only");
        }
    } else {
        println!("Unknown polish engine type, testing STT only");
    }

    cleanup_temp_files(&[temp_path]);
}

#[tokio::test]
#[ignore = "Requires SenseVoice model to be downloaded"]
async fn test_pipeline_with_local_sensevoice() {
    let wav_data = create_speech_like_wav(16000, 1, 2.0);
    let temp_path = write_temp_wav(&wav_data);

    let models_dir = ariatype_lib::stt_engine::UnifiedEngineManager::default_models_dir();
    let manager = ariatype_lib::stt_engine::UnifiedEngineManager::new(models_dir);

    // Try different SenseVoice models
    let sensevoice_models = ["sense-voice-small-q4_k", "sense-voice-small"];
    let mut selected_model = None;

    for model in &sensevoice_models {
        if manager.is_model_downloaded(ariatype_lib::stt_engine::EngineType::SenseVoice, model) {
            selected_model = Some(*model);
            break;
        }
    }

    if selected_model.is_none() {
        println!(
            "Skipping: No SenseVoice model downloaded (tried: {:?})",
            sensevoice_models
        );
        cleanup_temp_files(&[temp_path]);
        return;
    }

    let model_name = selected_model.unwrap();
    println!("Testing with SenseVoice {} model", model_name);

    let request = ariatype_lib::stt_engine::TranscriptionRequest::new(temp_path.to_str().unwrap())
        .with_model(model_name)
        .with_language("zh");
    let stt_result = manager
        .transcribe(ariatype_lib::stt_engine::EngineType::SenseVoice, request)
        .await
        .unwrap();

    assert!(
        !stt_result.text.trim().is_empty(),
        "SenseVoice should produce non-empty transcription"
    );
    println!("SenseVoice STT result: {}", stt_result.text);

    // Test with polish
    let polish_manager = ariatype_lib::polish_engine::UnifiedPolishManager::new();
    let polish_model_id = "lfm2.5-1.2b";

    if let Some(engine_type) =
        ariatype_lib::polish_engine::UnifiedPolishManager::get_engine_by_model_id(polish_model_id)
    {
        if polish_manager.is_model_downloaded(engine_type, polish_model_id) {
            let polish_request = ariatype_lib::polish_engine::PolishRequest::new(
                stt_result.text.clone(),
                "Polish and remove filler words",
                "zh",
            )
            .with_model(polish_model_id);

            match polish_manager.polish(engine_type, polish_request).await {
                Ok(polish_result) => {
                    let final_text = polish_result.text;
                    println!("SenseVoice + Polish result: {}", final_text);
                    assert!(
                        !final_text.trim().is_empty(),
                        "Polished result should be non-empty"
                    );
                }
                Err(e) => {
                    println!("Polish failed: {}", e);
                    // Don't fail the test if polish fails, as we're primarily testing STT
                }
            }
        } else {
            println!("Polish model not downloaded, testing STT only");
        }
    } else {
        println!("Unknown polish engine type, testing STT only");
    }

    cleanup_temp_files(&[temp_path]);
}

#[tokio::test]
async fn test_pipeline_stt_failure_recovery() {
    // Create mock STT engine that fails
    let stt = mock_stt::MockSttEngine::new("Should fail").with_failure();
    let polish = mock_polish::MockPolishEngine::new("Should not be called");

    let wav_data = create_test_wav(16000, 1, 1.0);
    let temp_path = write_temp_wav(&wav_data);

    let request = ariatype_lib::stt_engine::TranscriptionRequest::new(temp_path.to_str().unwrap());
    let stt_result = stt.transcribe(request).await;

    // Verify STT fails
    assert!(stt_result.is_err(), "STT should fail");

    // Test error handling in a full pipeline context
    let pipeline_result: Result<String, String> = async {
        let stt_request =
            ariatype_lib::stt_engine::TranscriptionRequest::new(temp_path.to_str().unwrap());
        let stt_result = stt.transcribe(stt_request).await?;

        if !stt_result.text.is_empty() {
            let polish_request = ariatype_lib::polish_engine::PolishRequest::new(
                stt_result.text.clone(),
                "Polish this",
                "en",
            );
            let polish_result = polish.polish(polish_request).await?;
            Ok(polish_result.text)
        } else {
            Ok(stt_result.text)
        }
    }
    .await;

    // Pipeline should also fail
    assert!(
        pipeline_result.is_err(),
        "Pipeline should propagate STT failure"
    );

    cleanup_temp_files(&[temp_path]);
}

#[tokio::test]
async fn test_pipeline_polish_failure_recovery() {
    // Create mock STT engine that succeeds
    let stt = mock_stt::MockSttEngine::new("um hello world uh");
    // Create mock polish engine that fails
    let polish = mock_polish::MockPolishEngine::new("Should fail").with_failure();

    let wav_data = create_test_wav(16000, 1, 1.0);
    let temp_path = write_temp_wav(&wav_data);

    let stt_request =
        ariatype_lib::stt_engine::TranscriptionRequest::new(temp_path.to_str().unwrap());
    let stt_result = stt.transcribe(stt_request).await.unwrap();

    // Verify STT succeeds
    assert_eq!(stt_result.text, "um hello world uh");

    // Test polish failure
    let polish_request = ariatype_lib::polish_engine::PolishRequest::new(
        stt_result.text.clone(),
        "Polish this",
        "en",
    );
    let polish_result = polish.polish(polish_request).await;

    assert!(polish_result.is_err(), "Polish should fail");

    // Test full pipeline with graceful degradation
    let pipeline_result: Result<String, String> = async {
        let stt_request =
            ariatype_lib::stt_engine::TranscriptionRequest::new(temp_path.to_str().unwrap());
        let stt_result = stt.transcribe(stt_request).await?;

        if !stt_result.text.is_empty() {
            let polish_request = ariatype_lib::polish_engine::PolishRequest::new(
                stt_result.text.clone(),
                "Polish this",
                "en",
            );
            match polish.polish(polish_request).await {
                Ok(polish_result) => Ok(polish_result.text),
                Err(_) => {
                    // Graceful degradation: return original STT result on polish failure
                    Ok(stt_result.text)
                }
            }
        } else {
            Ok(stt_result.text)
        }
    }
    .await;

    // Pipeline should succeed with graceful degradation
    assert!(
        pipeline_result.is_ok(),
        "Pipeline should handle polish failure gracefully"
    );
    let final_text = pipeline_result.unwrap();
    assert_eq!(
        final_text, "um hello world uh",
        "Should return original STT text on polish failure"
    );

    cleanup_temp_files(&[temp_path]);
}
