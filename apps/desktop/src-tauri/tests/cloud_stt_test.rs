use std::io::Cursor;

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
            let freq = 440.0;
            let amplitude = 16000.0;
            let sample = (amplitude * (2.0 * std::f32::consts::PI * freq * t).sin()) as i16;
            writer.write_sample(sample).unwrap();
        }
        writer.finalize().unwrap();
    }

    cursor.into_inner()
}

fn resample_to_16khz_mono(
    samples_i16: &[i16],
    input_sample_rate: u32,
    input_channels: u16,
) -> Vec<i16> {
    let mut audio_f32: Vec<f32> = samples_i16.iter().map(|&s| s as f32 / 32768.0).collect();

    if input_channels == 2 {
        let mono: Vec<f32> = audio_f32
            .chunks(2)
            .map(|stereo| (stereo[0] + stereo.get(1).copied().unwrap_or(0.0)) / 2.0)
            .collect();
        audio_f32 = mono;
    }

    if input_sample_rate != 16000 {
        let resampled =
            ariatype_lib::audio::resampler::resample_to_16khz(&audio_f32, input_sample_rate)
                .unwrap();
        resampled
            .iter()
            .map(|&s| (s * 32767.0).clamp(-32768.0, 32767.0) as i16)
            .collect()
    } else {
        audio_f32
            .iter()
            .map(|&s| (s * 32767.0).clamp(-32768.0, 32767.0) as i16)
            .collect()
    }
}

#[test]
fn test_resample_44100hz_to_16000hz() {
    let wav_data = create_test_wav(44100, 1, 1.0);
    let reader = hound::WavReader::new(Cursor::new(wav_data)).unwrap();
    let samples_i16: Vec<i16> = reader
        .into_samples::<i16>()
        .filter_map(|s| s.ok())
        .collect();

    let resampled = resample_to_16khz_mono(&samples_i16, 44100, 1);

    let expected_samples = (16000 as f32 * 1.0) as usize;
    let tolerance = (expected_samples as f32 * 0.05) as usize;
    assert!(
        resampled.len() >= expected_samples - tolerance
            && resampled.len() <= expected_samples + tolerance,
        "Expected ~{} samples, got {}",
        expected_samples,
        resampled.len()
    );
}

#[test]
fn test_downmix_stereo_to_mono() {
    let wav_data = create_test_wav(16000, 2, 1.0);
    let reader = hound::WavReader::new(Cursor::new(wav_data)).unwrap();
    let samples_i16: Vec<i16> = reader
        .into_samples::<i16>()
        .filter_map(|s| s.ok())
        .collect();

    assert_eq!(samples_i16.len(), 32000, "Stereo should have 32000 samples");

    let mono = resample_to_16khz_mono(&samples_i16, 16000, 2);

    assert_eq!(mono.len(), 16000, "Mono should have 16000 samples");
}

#[test]
fn test_chunk_size_for_streaming() {
    let wav_data = create_test_wav(16000, 1, 3.0);
    let reader = hound::WavReader::new(Cursor::new(wav_data)).unwrap();
    let samples_i16: Vec<i16> = reader
        .into_samples::<i16>()
        .filter_map(|s| s.ok())
        .collect();

    const CHUNK_SIZE: usize = 3200;
    let chunks: Vec<&[i16]> = samples_i16.chunks(CHUNK_SIZE).collect();

    assert!(chunks.len() > 0, "Should have at least one chunk");

    for (i, chunk) in chunks.iter().enumerate() {
        let expected_size = if i < chunks.len() - 1 {
            CHUNK_SIZE
        } else {
            samples_i16.len() % CHUNK_SIZE
        };
        if expected_size > 0 {
            assert!(
                chunk.len() == expected_size || chunk.len() == CHUNK_SIZE,
                "Chunk {} has unexpected size: {}",
                i,
                chunk.len()
            );
        }
    }

    let chunk_duration_ms = (CHUNK_SIZE as f64 / 16000.0) * 1000.0;
    assert!(
        (chunk_duration_ms - 200.0).abs() < 1.0,
        "Chunk duration should be ~200ms"
    );
}

#[test]
fn test_protocol_header_values() {
    let protocol_version: u8 = 0b0001;
    let header_size: u8 = 0b0001;
    let message_type_audio: u8 = 0b0010;
    let serialization_none: u8 = 0b0000;
    let compression_none: u8 = 0b0000;

    let byte0 = (protocol_version << 4) | header_size;
    let byte1 = (message_type_audio << 4) | 0b0000;
    let byte2 = (serialization_none << 4) | compression_none;
    let byte3 = 0x00;

    assert_eq!(byte0, 0b00010001);
    assert_eq!(byte1, 0b00100000);
    assert_eq!(byte2, 0b00000000);
    assert_eq!(byte3, 0x00);
}

#[test]
fn test_last_packet_header_flag() {
    let message_type_audio: u8 = 0b0010;
    let last_packet_flag: u8 = 0b0010;

    let byte1 = (message_type_audio << 4) | last_packet_flag;

    assert_eq!(byte1, 0b00100010, "Last packet flag should be set");
}

#[test]
fn test_pcm_to_bytes_conversion() {
    let samples: Vec<i16> = vec![0, 1000, -1000, 32767, -32768];
    let bytes: Vec<u8> = samples.iter().flat_map(|&s| s.to_le_bytes()).collect();

    assert_eq!(
        bytes.len(),
        samples.len() * 2,
        "Each sample should be 2 bytes"
    );

    let reconstructed: Vec<i16> = bytes
        .chunks(2)
        .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
        .collect();

    assert_eq!(reconstructed, samples, "Round-trip should preserve values");
}

#[test]
fn test_streaming_mode_urls() {
    use ariatype_lib::stt_engine::cloud::volcengine_streaming::*;

    assert!(URL_BIGMODEL_NOSTREAM.contains("nostream"));
    assert!(URL_BIGMODEL.contains("bigmodel"));
    assert!(!URL_BIGMODEL.contains("nostream"));
}

#[test]
fn test_recommended_chunk_duration() {
    use ariatype_lib::stt_engine::cloud::volcengine_streaming::RECOMMENDED_CHUNK_SAMPLES;

    let duration_ms = (RECOMMENDED_CHUNK_SAMPLES as f64 / 16000.0) * 1000.0;
    assert!(
        (duration_ms - 100.0).abs() < 1.0,
        "Recommended chunk should be ~100ms"
    );
}

#[test]
fn test_full_integration_flow_simulation() {
    let wav_data = create_test_wav(44100, 1, 2.0);
    let reader = hound::WavReader::new(Cursor::new(wav_data)).unwrap();
    let spec = reader.spec();
    let samples_i16: Vec<i16> = reader
        .into_samples::<i16>()
        .filter_map(|s| s.ok())
        .collect();

    let samples_16khz_mono = resample_to_16khz_mono(&samples_i16, spec.sample_rate, spec.channels);

    let expected_samples = 16000 * 2;
    let tolerance = (expected_samples as f32 * 0.05) as usize;
    assert!(
        samples_16khz_mono.len() >= expected_samples - tolerance,
        "Should have approximately {} samples at 16kHz for 2s audio",
        expected_samples
    );

    const CHUNK_SIZE: usize = 3200;
    let chunk_count = (samples_16khz_mono.len() + CHUNK_SIZE - 1) / CHUNK_SIZE;

    assert!(
        chunk_count >= 5,
        "Should have at least 5 chunks for 2s audio at 200ms/chunk"
    );

    let total_samples_sent: usize = samples_16khz_mono.chunks(CHUNK_SIZE).map(|c| c.len()).sum();
    assert_eq!(
        total_samples_sent,
        samples_16khz_mono.len(),
        "All samples should be in chunks"
    );
}
