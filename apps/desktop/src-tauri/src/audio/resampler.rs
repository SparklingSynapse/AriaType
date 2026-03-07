use rubato::{
    Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction,
};
use tracing::debug;

pub fn resample(input: &[f32], from_hz: u32, to_hz: u32) -> Result<Vec<f32>, String> {
    if from_hz == to_hz {
        return Ok(input.to_vec());
    }

    debug!(from_hz, to_hz, "resampling audio");

    let ratio = to_hz as f64 / from_hz as f64;

    let params = SincInterpolationParameters {
        sinc_len: 256,
        f_cutoff: 0.95,
        interpolation: SincInterpolationType::Linear,
        oversampling_factor: 256,
        window: WindowFunction::BlackmanHarris2,
    };

    let mut resampler = SincFixedIn::<f32>::new(ratio, 2.0, params, input.len(), 1)
        .map_err(|e| format!("Failed to create resampler: {:?}", e))?;

    let output_frames = resampler.output_frames_max();
    let mut output = vec![0.0f32; output_frames];

    let (_, written) = resampler
        .process_into_buffer(&[input], &mut [&mut output], None)
        .map_err(|e| format!("Resampling failed: {:?}", e))?;

    output.truncate(written);

    debug!(input_samples = input.len(), output_samples = written, "resampling complete");

    Ok(output)
}

pub fn resample_to_16khz(input: &[f32], input_sample_rate: u32) -> Result<Vec<f32>, String> {
    resample(input, input_sample_rate, 16_000)
}
