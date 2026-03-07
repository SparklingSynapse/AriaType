#[test]
fn test_denoise_empty_input() {
    let input: Vec<f32> = vec![];
    let result = ariatype_lib::audio::processor::denoise_audio(&input, 48000);

    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.is_empty(), "Empty input should return empty output");
}

#[test]
fn test_denoise_silent_input() {
    let input: Vec<f32> = vec![0.0f32; 4800];
    let result = ariatype_lib::audio::processor::denoise_audio(&input, 48000);

    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(!output.is_empty());
}

#[test]
fn test_denoise_48khz_input() {
    let duration = 1.0;
    let sample_rate = 48000u32;
    let num_samples = (sample_rate as f32 * duration) as usize;

    let input: Vec<f32> = (0..num_samples)
        .map(|i| {
            let t = i as f32 / sample_rate as f32;
            (2.0 * std::f32::consts::PI * 440.0 * t).sin() * 0.5
        })
        .collect();

    let result = ariatype_lib::audio::processor::denoise_audio(&input, sample_rate);

    assert!(result.is_ok());
    let output = result.unwrap();

    assert!(!output.is_empty(), "Output should not be empty");
    assert!(
        output.len() <= input.len(),
        "Output length should not exceed input length"
    );
}

#[test]
fn test_denoise_16khz_input() {
    let duration = 1.0;
    let sample_rate = 16000u32;
    let num_samples = (sample_rate as f32 * duration) as usize;

    let input: Vec<f32> = (0..num_samples)
        .map(|i| {
            let t = i as f32 / sample_rate as f32;
            (2.0 * std::f32::consts::PI * 440.0 * t).sin() * 0.5
        })
        .collect();

    let result = ariatype_lib::audio::processor::denoise_audio(&input, sample_rate);

    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(!output.is_empty());
}

#[test]
fn test_denoise_44khz_input() {
    let duration = 0.5;
    let sample_rate = 44100u32;
    let num_samples = (sample_rate as f32 * duration) as usize;

    let input: Vec<f32> = (0..num_samples)
        .map(|i| {
            let t = i as f32 / sample_rate as f32;
            (2.0 * std::f32::consts::PI * 1000.0 * t).sin() * 0.3
        })
        .collect();

    let result = ariatype_lib::audio::processor::denoise_audio(&input, sample_rate);

    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(!output.is_empty());
}

#[test]
fn test_denoise_short_input() {
    let input: Vec<f32> = (0..100).map(|i| (i as f32 * 0.1).sin() * 0.5).collect();

    let result = ariatype_lib::audio::processor::denoise_audio(&input, 16000);

    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(
        output.len() <= input.len(),
        "Output length should not exceed input"
    );
}

#[test]
fn test_denoise_output_normalized() {
    let duration = 0.5;
    let sample_rate = 48000u32;
    let num_samples = (sample_rate as f32 * duration) as usize;

    let input: Vec<f32> = (0..num_samples)
        .map(|i| {
            let t = i as f32 / sample_rate as f32;
            (2.0 * std::f32::consts::PI * 440.0 * t).sin()
        })
        .collect();

    let result = ariatype_lib::audio::processor::denoise_audio(&input, sample_rate);

    assert!(result.is_ok());
    let output = result.unwrap();

    let max_val = output.iter().fold(0.0f32, |max, &v| max.max(v.abs()));
    assert!(max_val <= 2.0, "Output should be reasonably normalized");
}
