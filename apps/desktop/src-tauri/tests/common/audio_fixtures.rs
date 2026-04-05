//! Audio test fixtures for generating WAV files in tests
//!
//! Provides utilities for creating synthetic audio files for testing
//! STT engines and audio processing pipelines.

use hound::{WavReader, WavSpec, WavWriter};
use std::io::Cursor;
use std::path::PathBuf;
use tempfile::tempdir;
use uuid::Uuid;

/// Create a sine wave WAV file as raw bytes
///
/// # Arguments
/// * `sample_rate` - Sample rate in Hz (e.g., 16000, 44100)
/// * `channels` - Number of audio channels (1 = mono, 2 = stereo)
/// * `duration_secs` - Duration of the audio in seconds
///
/// # Returns
/// * `Vec<u8>` containing the complete WAV file data
pub fn create_test_wav(sample_rate: u32, channels: u16, duration_secs: f32) -> Vec<u8> {
    let num_samples = (sample_rate as f32 * duration_secs) as usize;
    let samples: Vec<i16> = (0..num_samples)
        .map(|i| {
            // 440 Hz sine wave (A4 note)
            let frequency = 440.0;
            let phase = (i as f32 * frequency * 2.0 * std::f32::consts::PI) / sample_rate as f32;
            (phase.sin() * 8000.0) as i16
        })
        .collect();

    let spec = WavSpec {
        channels,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    encode_wav(&spec, &samples)
}

/// Create a speech-like WAV file with multiple frequencies and noise
///
/// # Arguments
/// * `sample_rate` - Sample rate in Hz (e.g., 16000, 44100)
/// * `channels` - Number of audio channels (1 = mono, 2 = stereo)
/// * `duration_secs` - Duration of the audio in seconds
///
/// # Returns
/// * `Vec<u8>` containing the complete WAV file data
pub fn create_speech_like_wav(sample_rate: u32, channels: u16, duration_secs: f32) -> Vec<u8> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let num_samples = (sample_rate as f32 * duration_secs) as usize;
    let mut samples = Vec::with_capacity(num_samples);

    // Pseudo-random noise based on sample index
    let mut hasher = DefaultHasher::new();

    for i in 0..num_samples {
        hasher.write_usize(i);
        let noise_val = (hasher.finish() % 1000) as i16 / 50 - 10;

        // Mix multiple frequencies to simulate speech formants
        let f1 = 300.0; // First formant
        let f2 = 2000.0; // Second formant
        let f3 = 3000.0; // Third formant

        let t = i as f32 / sample_rate as f32;
        let formant1 = ((f1 * 2.0 * std::f32::consts::PI * t).sin() * 3000.0) as i16;
        let formant2 = ((f2 * 2.0 * std::f32::consts::PI * t).sin() * 2000.0) as i16;
        let formant3 = ((f3 * 2.0 * std::f32::consts::PI * t).sin() * 1500.0) as i16;

        // Add amplitude envelope variation to simulate speech cadence
        let envelope = ((t * 3.0).sin().abs() * 0.5 + 0.5) as i16;

        let sample = ((formant1 + formant2 + formant3) / 3) * envelope / 100 + noise_val;
        samples.push(sample.clamp(i16::MIN, i16::MAX));
    }

    let spec = WavSpec {
        channels,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    encode_wav(&spec, &samples)
}

/// Encode samples to WAV format
fn encode_wav(spec: &WavSpec, samples: &[i16]) -> Vec<u8> {
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut writer = WavWriter::new(&mut cursor, *spec).unwrap();
        for &sample in samples {
            writer.write_sample(sample).unwrap();
        }
        writer.finalize().unwrap();
    }
    cursor.into_inner()
}

/// Write WAV data to a temporary file
///
/// # Arguments
/// * `data` - Raw WAV file data
///
/// # Returns
/// * `PathBuf` pointing to the created temporary file
pub fn write_temp_wav(data: &[u8]) -> PathBuf {
    let temp_dir = std::env::temp_dir();
    let file_name = format!("test_wav_{}.wav", Uuid::new_v4());
    let file_path = temp_dir.join(&file_name);
    std::fs::write(&file_path, data).expect("Failed to write temp WAV file");
    file_path
}

/// Clean up temporary WAV files
///
/// # Arguments
/// * `paths` - Slice of `PathBuf` to files that should be deleted
pub fn cleanup_temp_files(paths: &[PathBuf]) {
    for path in paths {
        if path.exists() {
            let _ = std::fs::remove_file(path);
        }
    }
}

/// Read WAV file and return samples as Vec<f32>
pub fn read_wav_f32(path: &PathBuf) -> (WavSpec, Vec<f32>) {
    let reader = WavReader::open(path).expect("Failed to open WAV file");
    let spec = reader.spec();
    let samples: Vec<f32> = reader
        .into_samples::<i16>()
        .filter_map(|s| s.ok())
        .map(|s| s as f32 / i16::MAX as f32)
        .collect();
    (spec, samples)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_test_wav_basic() {
        let wav = create_test_wav(16000, 1, 1.0);
        assert!(!wav.is_empty());

        // Verify WAV header
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
    }

    #[test]
    fn test_create_test_wav_duration() {
        let sample_rate = 16000;
        let duration = 2.0;
        let wav = create_test_wav(sample_rate, 1, duration);

        let reader = WavReader::new(Cursor::new(&wav)).unwrap();
        assert_eq!(reader.spec().sample_rate, sample_rate);
        assert_eq!(reader.spec().channels, 1);

        let num_samples = reader.duration() as usize;
        let expected = (sample_rate as f32 * duration) as usize;
        assert_eq!(num_samples, expected);
    }

    #[test]
    fn test_create_test_wav_stereo() {
        let wav = create_test_wav(44100, 2, 0.5);
        let reader = WavReader::new(Cursor::new(&wav)).unwrap();
        assert_eq!(reader.spec().channels, 2);
        assert_eq!(reader.spec().sample_rate, 44100);
    }

    #[test]
    fn test_create_speech_like_wav() {
        let wav = create_speech_like_wav(16000, 1, 1.0);
        assert!(!wav.is_empty());

        // Verify WAV header
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
    }

    #[test]
    fn test_create_speech_like_wav_duration() {
        let sample_rate = 16000;
        let duration = 3.0;
        let wav = create_speech_like_wav(sample_rate, 1, duration);

        let reader = WavReader::new(Cursor::new(&wav)).unwrap();
        assert_eq!(reader.spec().sample_rate, sample_rate);
        assert_eq!(reader.spec().channels, 1);

        let num_samples = reader.duration() as usize;
        let expected = (sample_rate as f32 * duration) as usize;
        assert_eq!(num_samples, expected);
    }

    #[test]
    fn test_write_temp_wav() {
        let wav = create_test_wav(16000, 1, 0.5);
        let path = write_temp_wav(&wav);

        // Verify file exists and has content
        assert!(path.exists());
        let read_back = std::fs::read(&path).unwrap();
        assert_eq!(wav, read_back);

        // Cleanup
        cleanup_temp_files(&[path.clone()]);
        assert!(!path.exists());
    }

    #[test]
    fn test_cleanup_temp_files_nonexistent() {
        let fake_path = PathBuf::from("/tmp/nonexistent_file_12345.wav");
        // Should not panic even if file doesn't exist
        cleanup_temp_files(&[fake_path]);
    }

    #[test]
    fn test_cleanup_temp_files_multiple() {
        let paths: Vec<PathBuf> = (0..3)
            .map(|i| {
                let wav = create_test_wav(16000, 1, 0.1);
                write_temp_wav(&wav)
            })
            .collect();

        // Verify all files exist
        for p in &paths {
            assert!(p.exists());
        }

        cleanup_temp_files(&paths);

        // Verify all files are deleted
        for p in &paths {
            assert!(!p.exists());
        }
    }

    #[test]
    fn test_read_wav_f32() {
        let wav = create_test_wav(16000, 1, 0.5);
        let path = write_temp_wav(&wav);

        let (spec, samples) = read_wav_f32(&path);

        assert_eq!(spec.sample_rate, 16000);
        assert_eq!(spec.channels, 1);
        assert!(!samples.is_empty());

        // Verify samples are normalized to [-1, 1]
        for &sample in &samples {
            assert!(sample >= -1.0 && sample <= 1.0);
        }

        cleanup_temp_files(&[path]);
    }
}
