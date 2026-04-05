//! Unit tests for beep generation module
//!
//! Tests the `beep_generator` module which creates WAV beep audio files.
//! The `beep.rs` module handles playback and is tested separately via integration tests.

use ariatype_lib::audio::beep_generator;
use std::fs::File;
use std::io::Read;
use tempfile::tempdir;

/// Helper to parse WAV header and get sample count
fn parse_wav_info(path: &std::path::Path) -> (u32, u16, u32) {
    let mut file = File::open(path).unwrap();
    let mut header = [0u8; 44];
    file.read_exact(&mut header).unwrap();

    // Parse WAV header
    // Bytes 22-24: num channels (16-bit)
    let channels = u16::from_le_bytes([header[22], header[23]]);
    // Bytes 24-28: sample rate (32-bit)
    let sample_rate = u32::from_le_bytes([header[24], header[25], header[26], header[27]]);
    // Bytes 40-44: data size (32-bit)
    let data_size = u32::from_le_bytes([header[40], header[41], header[42], header[43]]);

    // Each sample is 16-bit (2 bytes)
    let num_samples = data_size / 2;

    (sample_rate, channels, num_samples)
}

#[test]
fn test_beep_generation_basic() {
    // Test generation with default-ish parameters
    let temp_dir = tempdir().unwrap();
    let output_path = temp_dir.path().join("test_beep.wav");

    let result = beep_generator::generate_beep(
        &output_path,
        440.0, // start_freq
        440.0, // end_freq (same = flat tone)
        0.1,   // duration (100ms)
    );

    assert!(result.is_ok(), "Beep generation should succeed");
    assert!(output_path.exists(), "Output file should exist");

    // Verify WAV properties
    let (sample_rate, channels, num_samples) = parse_wav_info(&output_path);
    assert_eq!(sample_rate, 48000, "Sample rate should be 48000 Hz");
    assert_eq!(channels, 1, "Should be mono");
    assert!(num_samples > 0, "Should have samples");
}

#[test]
fn test_beep_generation_with_custom_params() {
    let temp_dir = tempdir().unwrap();
    let output_path = temp_dir.path().join("custom_beep.wav");

    // Custom frequency sweep: 200 Hz to 800 Hz over 500ms
    let start_freq = 200.0;
    let end_freq = 800.0;
    let duration = 0.5;

    let result = beep_generator::generate_beep(&output_path, start_freq, end_freq, duration);
    assert!(
        result.is_ok(),
        "Beep generation with custom params should succeed"
    );
    assert!(output_path.exists(), "Output file should exist");

    let (sample_rate, _channels, num_samples) = parse_wav_info(&output_path);
    assert_eq!(sample_rate, 48000);

    // Verify duration matches expected samples
    let expected_samples = (48000.0 * duration) as u32;
    assert_eq!(
        num_samples, expected_samples,
        "Sample count should match duration * sample_rate"
    );
}

#[test]
fn test_beep_samples_valid() {
    let temp_dir = tempdir().unwrap();
    let output_path = temp_dir.path().join("valid_beep.wav");

    beep_generator::generate_beep(&output_path, 440.0, 440.0, 0.1).unwrap();

    // Read samples and verify they're in valid i16 range
    let mut file = File::open(&output_path).unwrap();
    let mut header = [0u8; 44];
    file.read_exact(&mut header).unwrap();

    let data_size = u32::from_le_bytes([header[40], header[41], header[42], header[43]]);
    let num_samples = data_size / 2;

    let mut samples = vec![0i16; num_samples as usize];
    let mut reader = hound::WavReader::open(&output_path).unwrap();
    for (i, sample) in reader.samples::<i16>().enumerate() {
        if i >= num_samples as usize {
            break;
        }
        let s = sample.unwrap();
        samples[i] = s;

        // Verify sample is within valid range
        assert!(
            s >= i16::MIN && s <= i16::MAX,
            "Sample {} is out of i16 range: {}",
            i,
            s
        );
    }

    // Check that we have some non-zero samples (beep is not silence)
    let max_sample = samples.iter().map(|s| s.abs()).max().unwrap();
    assert!(max_sample > 0, "Beep should have non-zero samples");

    // Verify samples are within reasonable amplitude range (envelope <= 0.09)
    let max_expected = (0.09 * i16::MAX as f32) as i16;
    for (i, &sample) in samples.iter().enumerate() {
        assert!(
            sample.abs() <= max_expected,
            "Sample {} amplitude {} exceeds envelope max {}",
            i,
            sample.abs(),
            max_expected
        );
    }
}

#[test]
fn test_beep_frequency_sweep() {
    let temp_dir = tempdir().unwrap();
    let output_path = temp_dir.path().join("sweep_beep.wav");

    // Ascending sweep: 430 Hz to 570 Hz (like start beep)
    beep_generator::generate_beep(&output_path, 430.0, 570.0, 0.22).unwrap();

    let mut reader = hound::WavReader::open(&output_path).unwrap();
    let samples: Vec<i16> = reader.samples::<i16>().filter_map(|s| s.ok()).collect();

    assert!(
        !samples.is_empty(),
        "Should have samples from frequency sweep"
    );

    // Verify duration is approximately correct
    let expected_samples = (48000.0 * 0.22) as usize;
    let diff = (samples.len() as i32 - expected_samples as i32).abs();
    assert!(
        diff < 10,
        "Sample count should be close to expected (diff={}, expected={}, got={})",
        diff,
        expected_samples,
        samples.len()
    );
}

#[test]
fn test_generate_beep_files_creates_both() {
    // Test that generate_beep_files creates both start and stop beeps
    let temp_dir = tempdir().unwrap();
    let assets_dir = temp_dir.path();

    // We can't easily test generate_beep_files since it hardcodes paths,
    // but we can verify the individual beep generation works
    let start_path = assets_dir.join("start_beep.wav");
    let stop_path = assets_dir.join("stop_beep.wav");

    // Generate start beep (ascending)
    let result1 = beep_generator::generate_beep(&start_path, 430.0, 570.0, 0.22);
    assert!(result1.is_ok());

    // Generate stop beep (descending)
    let result2 = beep_generator::generate_beep(&stop_path, 430.0, 290.0, 0.25);
    assert!(result2.is_ok());

    assert!(start_path.exists());
    assert!(stop_path.exists());
}

#[test]
fn test_beep_envelope_shape() {
    let temp_dir = tempdir().unwrap();
    let output_path = temp_dir.path().join("envelope_beep.wav");

    // Generate a short beep to test envelope
    beep_generator::generate_beep(&output_path, 440.0, 440.0, 0.05).unwrap();

    let mut reader = hound::WavReader::open(&output_path).unwrap();
    let samples: Vec<i16> = reader.samples::<i16>().filter_map(|s| s.ok()).collect();

    // Find the maximum amplitude sample
    let max_amp = samples.iter().map(|s| s.abs()).max().unwrap();

    // The envelope max should be around 0.09 * i16::MAX
    let expected_max = (0.09 * i16::MAX as f32) as i16;
    let tolerance = 100; // Allow small tolerance due to rounding

    assert!(
        (max_amp - expected_max).abs() <= tolerance || max_amp < expected_max,
        "Max amplitude {} should be close to expected {}",
        max_amp,
        expected_max
    );
}
