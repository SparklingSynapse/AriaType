use hound::{WavSpec, WavWriter};
use std::f32::consts::PI;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let assets_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");
    std::fs::create_dir_all(&assets_dir)?;

    // Generate start beep (ascending: 430 Hz → 570 Hz, 220ms)
    generate_beep(
        &assets_dir.join("start_beep.wav"),
        430.0,
        570.0,
        0.22,
    )?;

    // Generate stop beep (descending: 430 Hz → 290 Hz, 250ms)
    generate_beep(
        &assets_dir.join("stop_beep.wav"),
        430.0,
        290.0,
        0.25,
    )?;

    println!("✓ Generated beep files in {:?}", assets_dir);
    Ok(())
}

fn generate_beep(
    path: &std::path::Path,
    start_freq: f32,
    end_freq: f32,
    duration: f32,
) -> Result<(), Box<dyn std::error::Error>> {
    // Use 16000 Hz sample rate for smaller file size
    // This is sufficient for simple beep sounds (human hearing range is 20-20000 Hz)
    let sample_rate = 16000;
    let spec = WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = WavWriter::create(path, spec)?;
    let total_samples = (sample_rate as f32 * duration) as usize;

    for i in 0..total_samples {
        let t = i as f32 / sample_rate as f32;
        let progress = t / duration;

        // Linear frequency sweep
        let freq = start_freq + (end_freq - start_freq) * progress;

        // Envelope: soft attack, brief sustain, smooth decay
        let envelope = if t < 0.015 {
            t / 0.015 * 0.09
        } else if t < 0.08 {
            0.09
        } else {
            0.09 * ((duration - t) / (duration - 0.08)).powf(2.0)
        };

        // Generate sine wave
        let phase = 2.0 * PI * freq * t;
        let sample = phase.sin() * envelope;

        // Convert to 16-bit PCM
        let amplitude = i16::MAX as f32;
        writer.write_sample((sample * amplitude) as i16)?;
    }

    writer.finalize()?;
    println!("✓ Generated: {:?}", path);
    Ok(())
}
