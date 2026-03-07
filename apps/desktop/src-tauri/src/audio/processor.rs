use nnnoiseless::DenoiseState;
use tracing::debug;

use crate::audio::resampler::resample;

/// Estimate whether the audio has significant background noise.
/// Returns true if the noise floor exceeds ~-42dB (RMS ≈ 0.008).
/// Uses the bottom 20% of 20ms frame energies as the noise floor estimate.
pub fn should_denoise(audio: &[f32], sample_rate: u32) -> bool {
    if audio.is_empty() {
        return false;
    }
    let frame_size = (sample_rate as usize * 20 / 1000).max(1);
    let mut frame_rms: Vec<f32> = audio
        .chunks(frame_size)
        .map(|f| {
            let sum_sq: f32 = f.iter().map(|&s| s * s).sum();
            (sum_sq / f.len() as f32).sqrt()
        })
        .collect();
    frame_rms.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let noise_frames = (frame_rms.len() / 5).max(1);
    let noise_floor: f32 = frame_rms[..noise_frames].iter().sum::<f32>() / noise_frames as f32;
    // Threshold: ~-42dB. Clean mic in quiet room ≈ 0.001–0.003; noisy env ≈ 0.01+
    noise_floor > 0.008
}

/// Denoise mono audio using the RNNoise algorithm (via nnnoiseless).
///
/// Input: mono f32 audio at any sample rate, normalized to [-1.0, 1.0].
/// Output: denoised audio resampled back to the original sample rate.
///
/// nnnoiseless requires 48kHz input and f32 values in the i16 scale
/// [-32768.0, 32767.0]. This function handles all conversions internally.
pub fn denoise_audio(input: &[f32], sample_rate: u32) -> Result<Vec<f32>, String> {
    if input.is_empty() {
        return Ok(Vec::new());
    }

    debug!(samples = input.len(), sample_rate, "starting audio denoising");

    const DENOISE_RATE: u32 = 48_000;
    const FRAME_SIZE: usize = DenoiseState::FRAME_SIZE;

    // Resample to 48kHz if needed
    let at_48k = resample(input, sample_rate, DENOISE_RATE)?;

    // Scale [-1.0, 1.0] → [-32768.0, 32767.0] as required by nnnoiseless
    let mut scaled: Vec<f32> = at_48k.iter().map(|&s| s * 32768.0).collect();

    // Zero-pad to a multiple of FRAME_SIZE so every chunk is full-sized
    let remainder = scaled.len() % FRAME_SIZE;
    if remainder != 0 {
        scaled.resize(scaled.len() + (FRAME_SIZE - remainder), 0.0);
    }

    let mut denoiser = DenoiseState::new();
    let mut denoised = Vec::with_capacity(scaled.len());
    let mut buf_out = [0.0f32; FRAME_SIZE];
    let mut first = true;

    for chunk in scaled.chunks_exact(FRAME_SIZE) {
        denoiser.process_frame(&mut buf_out, chunk);
        // Discard the first frame: it contains fade-in artifacts
        if first {
            first = false;
            continue;
        }
        denoised.extend_from_slice(&buf_out);
    }

    // Scale back to [-1.0, 1.0]
    let normalized: Vec<f32> = denoised.iter().map(|&s| s / 32768.0).collect();

    // Resample back to the original sample rate
    let output = resample(&normalized, DENOISE_RATE, sample_rate)?;

    // Trim to original length (padding and the skipped first frame may shift length slightly)
    let out_len = output.len().min(input.len());

    debug!(
        input_samples = input.len(),
        output_samples = out_len,
        "audio denoising complete"
    );

    Ok(output[..out_len].to_vec())
}
