use hound::{WavSpec, WavWriter};
use std::f32::consts::PI;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let assets_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");
    std::fs::create_dir_all(&assets_dir)?;

    // Generate start beep (ascending tone, rising volume - 二声)
    generate_start_beep(&assets_dir.join("start_beep.wav"))?;

    // Generate stop beep (descending tone, falling volume - 四声)
    generate_stop_beep(&assets_dir.join("stop_beep.wav"))?;

    println!("✓ Generated beep files in {:?}", assets_dir);
    Ok(())
}

/// Generate start beep: rising tone with rising volume (二声效果)
/// Frequency: 400 Hz → 550 Hz
/// Volume: gradual fade-in, then rise from weak to strong
fn generate_start_beep(path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let sample_rate = 44100;
    let duration = 0.15; // 150ms
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

        // Frequency sweep: 400 Hz → 550 Hz (rising tone)
        let freq = 400.0 + 150.0 * progress;

        // Volume envelope: fade-in + rising (二声: 从弱到强)
        let envelope = if t < 0.02 {
            // Initial fade-in (20ms) to avoid harsh start
            let fade_in = t / 0.02;
            fade_in * 0.15
        } else {
            // Rising volume from weak to strong
            let rise_progress = (t - 0.02) / (duration - 0.02);
            0.15 + rise_progress * 0.15 // 0.15 → 0.30
        };

        // Generate sine wave
        let phase = 2.0 * PI * freq * t;
        let sample = phase.sin() * envelope;

        // Convert to 16-bit PCM
        let amplitude = i16::MAX as f32;
        writer.write_sample((sample * amplitude) as i16)?;
    }

    writer.finalize()?;
    println!("✓ Generated start beep (二声): {:?}", path);
    Ok(())
}

/// Generate stop beep: falling tone with falling volume (四声效果)
/// Frequency: 500 Hz → 350 Hz
/// Volume: gradual fade-in, then fall from strong to weak
fn generate_stop_beep(path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let sample_rate = 44100;
    let duration = 0.15; // 150ms
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

        // Frequency sweep: 500 Hz → 350 Hz (falling tone)
        let freq = 500.0 - 150.0 * progress;

        // Volume envelope: fade-in + falling (四声: 从强到弱)
        let envelope = if t < 0.02 {
            // Initial fade-in (20ms) to avoid harsh start
            let fade_in = t / 0.02;
            fade_in * 0.30
        } else {
            // Falling volume from strong to weak
            let fall_progress = (t - 0.02) / (duration - 0.02);
            0.30 - fall_progress * 0.18 // 0.30 → 0.12
        };

        // Generate sine wave
        let phase = 2.0 * PI * freq * t;
        let sample = phase.sin() * envelope;

        // Convert to 16-bit PCM
        let amplitude = i16::MAX as f32;
        writer.write_sample((sample * amplitude) as i16)?;
    }

    writer.finalize()?;
    println!("✓ Generated stop beep (四声): {:?}", path);
    Ok(())
}

// Legacy function kept for reference
#[allow(dead_code)]
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
