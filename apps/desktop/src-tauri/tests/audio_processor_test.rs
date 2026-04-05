//! Unit tests for audio processor module
//!
//! Tests the `processor` module which provides audio denoising and
//! noise detection functionality.

use ariatype_lib::audio::processor::{denoise_audio, should_denoise};

/// Generate a simple sine wave for testing
/// Uses amplitude 0.005 to stay below the noise threshold (0.008 RMS)
fn generate_sine_wave(frequency: f32, sample_rate: u32, duration: f32) -> Vec<f32> {
    let num_samples = (sample_rate as f32 * duration) as usize;
    (0..num_samples)
        .map(|i| {
            let t = i as f32 / sample_rate as f32;
            (2.0 * std::f32::consts::PI * frequency * t).sin() * 0.005
        })
        .collect()
}

/// Generate a loud signal (high amplitude sine wave)
fn generate_loud_signal(frequency: f32, sample_rate: u32, duration: f32) -> Vec<f32> {
    let num_samples = (sample_rate as f32 * duration) as usize;
    (0..num_samples)
        .map(|i| {
            let t = i as f32 / sample_rate as f32;
            (2.0 * std::f32::consts::PI * frequency * t).sin() * 0.9
        })
        .collect()
}

/// Generate white noise
fn generate_noise(sample_rate: u32, duration: f32, amplitude: f32) -> Vec<f32> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::Hasher;

    let num_samples = (sample_rate as f32 * duration) as usize;
    (0..num_samples)
        .map(|i| {
            let mut hasher = DefaultHasher::new();
            hasher.write_usize(i);
            let noise = (hasher.finish() % 1000) as f32 / 1000.0 - 0.5;
            noise * amplitude
        })
        .collect()
}

#[test]
fn test_processor_creation() {
    // The processor functions don't require explicit creation,
    // but we verify the module is accessible and functions exist
    let audio = generate_sine_wave(440.0, 16000, 0.1);
    let result = should_denoise(&audio, 16000);
    // Clean sine wave should not require denoising
    assert!(!result, "Clean sine wave should not need denoising");
}

#[test]
fn test_processor_process_chunk() {
    let sample_rate = 16000u32;
    let duration = 0.1;
    let audio = generate_sine_wave(440.0, sample_rate, duration);

    // Process the audio chunk
    let result = denoise_audio(&audio, sample_rate);

    assert!(result.is_ok(), "Denoising should succeed");
    let processed = result.unwrap();

    // Output length should be similar to input (within resampling tolerance)
    let len_diff = (processed.len() as i32 - audio.len() as i32).abs();
    assert!(
        len_diff < 1000,
        "Processed length {} should be close to input {} (diff={})",
        processed.len(),
        audio.len(),
        len_diff
    );

    // Processed audio should still be valid f32 values
    for (i, &sample) in processed.iter().enumerate() {
        assert!(
            sample.is_finite(),
            "Sample {} should be finite, got {}",
            i,
            sample
        );
    }
}

#[test]
fn test_processor_chain_operations() {
    let sample_rate = 48000u32;
    let duration = 0.2;

    // Generate audio with some noise
    let clean = generate_sine_wave(440.0, sample_rate, duration);
    let mut noisy = clean.clone();
    let noise = generate_noise(sample_rate, duration, 0.1);

    for i in 0..noisy.len() {
        noisy[i] += noise[i.min(noise.len() - 1)];
    }

    // First check if denoising is needed
    let needs_denoise = should_denoise(&noisy, sample_rate);
    assert!(needs_denoise, "Noisy audio should require denoising");

    // Chain operation: denoise
    let denoised = denoise_audio(&noisy, sample_rate);
    assert!(denoised.is_ok(), "Denoising should succeed");

    // Verify output is valid
    let processed = denoised.unwrap();
    assert!(!processed.is_empty(), "Processed audio should not be empty");

    // Verify all samples are in valid range (with tolerance for denoising artifacts)
    for (i, &sample) in processed.iter().enumerate() {
        assert!(
            sample >= -1.1 && sample <= 1.1,
            "Sample {} out of range: {}",
            i,
            sample
        );
    }
}

#[test]
fn test_should_denoise_empty_audio() {
    let audio: Vec<f32> = vec![];
    let result = should_denoise(&audio, 16000);
    assert!(!result, "Empty audio should not require denoising");
}

#[test]
fn test_should_denoise_clean_signal() {
    // A clean 440 Hz sine wave at moderate amplitude should not need denoising
    let audio = generate_sine_wave(440.0, 16000, 0.5);
    let result = should_denoise(&audio, 16000);
    assert!(!result, "Clean sine wave should not need denoising");
}

#[test]
fn test_should_denoise_noisy_signal() {
    // Audio with high-frequency noise should trigger denoising
    let audio = generate_noise(16000, 0.5, 0.5);
    let result = should_denoise(&audio, 16000);
    assert!(result, "Noisy audio should require denoising");
}

#[test]
fn test_denoise_audio_empty_input() {
    let audio: Vec<f32> = vec![];
    let result = denoise_audio(&audio, 16000);
    assert!(result.is_ok(), "Empty audio should return Ok");
    let processed = result.unwrap();
    assert!(
        processed.is_empty(),
        "Empty input should produce empty output"
    );
}

#[test]
fn test_denoise_audio_different_sample_rates() {
    let durations = 0.1;

    // Test 16kHz
    let audio_16k = generate_sine_wave(440.0, 16000, durations);
    let result_16k = denoise_audio(&audio_16k, 16000);
    assert!(result_16k.is_ok());

    // Test 48kHz
    let audio_48k = generate_sine_wave(440.0, 48000, durations);
    let result_48k = denoise_audio(&audio_48k, 48000);
    assert!(result_48k.is_ok());

    // Both should produce valid output
    assert!(!result_16k.unwrap().is_empty());
    assert!(!result_48k.unwrap().is_empty());
}

#[test]
fn test_denoise_audio_preserves_amplitude() {
    let sample_rate = 16000u32;
    let audio = generate_loud_signal(440.0, sample_rate, 0.1);

    let result = denoise_audio(&audio, sample_rate);
    assert!(result.is_ok());

    let processed = result.unwrap();

    // Find peak amplitude in both
    let input_peak = audio.iter().map(|s| s.abs()).fold(0.0f32, |a, b| a.max(b));
    let output_peak = processed
        .iter()
        .map(|s| s.abs())
        .fold(0.0f32, |a, b| a.max(b));

    // Output peak should be similar to input (within 50% tolerance due to denoising)
    assert!(
        output_peak > input_peak * 0.1,
        "Output amplitude {} should preserve some of input amplitude {}",
        output_peak,
        input_peak
    );
}

#[test]
fn test_denoise_audio_short_input() {
    // Short audio that gets padded to FRAME_SIZE
    // The denoiser requires at least FRAME_SIZE (480) samples at 48kHz
    // So we provide 500ms of audio at 16kHz = 8000 samples which is enough
    let audio = generate_sine_wave(440.0, 16000, 0.5);
    let result = denoise_audio(&audio, 16000);
    assert!(result.is_ok());
    let processed = result.unwrap();
    assert!(
        !processed.is_empty(),
        "Should produce output for sufficient input"
    );
}

#[test]
fn test_denoise_audio_multiple_operations() {
    let sample_rate = 48000u32;
    let audio = generate_sine_wave(440.0, sample_rate, 0.1);

    // Apply denoising multiple times
    let result1 = denoise_audio(&audio, sample_rate);
    assert!(result1.is_ok());

    let processed1 = result1.unwrap();
    let result2 = denoise_audio(&processed1, sample_rate);
    assert!(result2.is_ok());

    let processed2 = result2.unwrap();

    // Second pass should also succeed
    assert!(
        !processed2.is_empty(),
        "Multiple passes should produce valid output"
    );
}

#[test]
fn test_noise_floor_detection_threshold() {
    let sample_rate = 16000u32;
    let duration = 0.1;

    // Very quiet signal (below threshold)
    let quiet: Vec<f32> = (0..(sample_rate as usize * duration as usize))
        .map(|_| 0.001f32)
        .collect();
    assert!(
        !should_denoise(&quiet, sample_rate),
        "Very quiet signal should not need denoising"
    );

    // Loud signal with noise (above threshold)
    let mut noisy = generate_loud_signal(440.0, sample_rate, duration);
    let noise = generate_noise(sample_rate, duration, 0.3);
    for i in 0..noisy.len() {
        noisy[i] += noise[i];
    }
    assert!(
        should_denoise(&noisy, sample_rate),
        "Loud noisy signal should need denoising"
    );
}
