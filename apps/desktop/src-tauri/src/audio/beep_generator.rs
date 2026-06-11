use hound::{WavSpec, WavWriter};
use std::f32::consts::PI;
use std::path::Path;
use tracing::info;

const PROFILE_SAMPLE_RATE: u32 = 44_100;
const START_DURATION_SECONDS: f32 = 0.15;
const STOP_DURATION_SECONDS: f32 = 0.15;
const EDGE_FADE_SECONDS: f32 = 0.018;

struct BeepProfile {
    sample_rate: u32,
    duration: f32,
    start_freq: f32,
    end_freq: f32,
    amplitude_at: fn(f32) -> f32,
}

pub fn generate_beep_files() -> Result<(), Box<dyn std::error::Error>> {
    let assets_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");
    std::fs::create_dir_all(&assets_dir)?;

    generate_start_beep(&assets_dir.join("start_beep.wav"))?;
    generate_stop_beep(&assets_dir.join("stop_beep.wav"))?;

    info!(path = ?assets_dir, "beep_files_generated");
    Ok(())
}

pub fn generate_start_beep(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    write_profile_beep(
        path,
        BeepProfile {
            sample_rate: PROFILE_SAMPLE_RATE,
            duration: START_DURATION_SECONDS,
            start_freq: 400.0,
            end_freq: 550.0,
            amplitude_at: start_beep_amplitude,
        },
    )
}

pub fn generate_stop_beep(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    write_profile_beep(
        path,
        BeepProfile {
            sample_rate: PROFILE_SAMPLE_RATE,
            duration: STOP_DURATION_SECONDS,
            start_freq: 500.0,
            end_freq: 350.0,
            amplitude_at: stop_beep_amplitude,
        },
    )
}

pub fn generate_beep(
    path: &Path,
    start_freq: f32,
    end_freq: f32,
    duration: f32,
) -> Result<(), Box<dyn std::error::Error>> {
    let sample_rate = 48_000;
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

        let base_envelope = if t < 0.015 {
            // Attack (15ms)
            t / 0.015 * 0.09
        } else if t < 0.08 {
            // Sustain (65ms)
            0.09
        } else {
            // Decay (rest of duration)
            0.09 * ((duration - t) / (duration - 0.08)).powf(2.0)
        };
        let envelope = base_envelope * edge_fade(t, duration);

        // Generate sine wave
        let phase = 2.0 * PI * freq * t;
        let sample = phase.sin() * envelope;

        // Convert to 16-bit PCM
        let amplitude = i16::MAX as f32;
        writer.write_sample((sample * amplitude) as i16)?;
    }

    writer.finalize()?;
    info!(path = ?path, "beep_file_generated");
    Ok(())
}

fn write_profile_beep(path: &Path, profile: BeepProfile) -> Result<(), Box<dyn std::error::Error>> {
    let spec = WavSpec {
        channels: 1,
        sample_rate: profile.sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = WavWriter::create(path, spec)?;
    let total_samples = (profile.sample_rate as f32 * profile.duration) as usize;

    for i in 0..total_samples {
        let t = i as f32 / profile.sample_rate as f32;
        let progress = t / profile.duration;
        let freq = profile.start_freq + (profile.end_freq - profile.start_freq) * progress;
        let envelope = (profile.amplitude_at)(progress) * edge_fade(t, profile.duration);
        let phase = 2.0 * PI * freq * t;
        let sample = phase.sin() * envelope;

        writer.write_sample((sample * i16::MAX as f32) as i16)?;
    }

    writer.finalize()?;
    info!(path = ?path, "beep_file_generated");
    Ok(())
}

fn start_beep_amplitude(progress: f32) -> f32 {
    0.15 + smoothstep(progress) * 0.15
}

fn stop_beep_amplitude(progress: f32) -> f32 {
    0.30 - smoothstep(progress) * 0.18
}

fn edge_fade(t: f32, duration: f32) -> f32 {
    let fade = EDGE_FADE_SECONDS.min(duration * 0.4);
    if fade <= f32::EPSILON {
        return 1.0;
    }

    let attack = smoothstep((t / fade).clamp(0.0, 1.0));
    let release = smoothstep(((duration - t) / fade).clamp(0.0, 1.0));
    attack.min(release)
}

fn smoothstep(progress: f32) -> f32 {
    let x = progress.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}
