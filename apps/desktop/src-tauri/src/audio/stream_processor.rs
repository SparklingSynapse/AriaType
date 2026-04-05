use nnnoiseless::DenoiseState;
use tracing::{debug, info};

use crate::audio::resampler::{resample, resample_to_16khz};

const DENOISE_RATE: u32 = 48_000;
const FRAME_SIZE: usize = DenoiseState::FRAME_SIZE;

/// VAD speech-on threshold (RMS on f32 [-1.0, 1.0] scale, ~-44 dB).
/// Chunks with RMS at or above this level are classified as speech.
const VAD_SPEECH_ON_RMS: f32 = 0.006;

/// VAD speech-off threshold (RMS on f32 [-1.0, 1.0] scale, ~-54 dB).
/// Once speech is detected, the chunk stays in speech state until RMS
/// drops below this level (hysteresis to prevent rapid switching).
const VAD_SPEECH_OFF_RMS: f32 = 0.002;

/// Processed audio chunk with speech detection result.
pub struct ProcessedChunk {
    /// Processed audio as 16-bit PCM at 16 kHz mono, ready for STT.
    pub pcm_16khz_mono: Vec<i16>,
    /// Whether speech was detected in this chunk.
    /// When false, the chunk can be safely skipped for cloud STT
    /// to save tokens and bandwidth.
    pub has_speech: bool,
}

/// Session statistics for the stream processor.
#[derive(Debug, Default)]
pub struct StreamProcessorStats {
    pub total_chunks: u32,
    pub speech_chunks: u32,
    pub silent_chunks_skipped: u32,
}

/// Real-time audio processor for streaming recording.
///
/// Maintains persistent denoise state across chunks for high-quality
/// real-time audio processing. Applies:
/// - **Denoise**: RNNoise-based noise suppression at 48 kHz
/// - **VAD**: Energy-based voice activity detection with hysteresis
/// - **Resampling**: Output at 16 kHz mono for STT engines
///
/// Designed for use in the audio callback thread via `Arc<Mutex<>>`.
pub struct StreamAudioProcessor {
    /// Persistent denoise state (maintains filter history across frames).
    denoise_state: Option<Box<DenoiseState<'static>>>,
    /// Whether denoise is enabled.
    denoise_enabled: bool,
    /// Whether VAD-based chunk skipping is enabled.
    vad_enabled: bool,
    /// Current speech detection state (for hysteresis).
    is_speech: bool,
    /// Buffer for partial denoise frames (must align to FRAME_SIZE = 480).
    denoise_buffer: Vec<f32>,
    /// Processing statistics for this session.
    stats: StreamProcessorStats,
}

impl StreamAudioProcessor {
    /// Create a new stream processor.
    pub fn new(denoise_enabled: bool, vad_enabled: bool) -> Self {
        info!(
            denoise_enabled,
            vad_enabled, "stream_audio_processor_created"
        );
        Self {
            denoise_state: if denoise_enabled {
                Some(DenoiseState::new())
            } else {
                None
            },
            denoise_enabled,
            vad_enabled,
            is_speech: false,
            denoise_buffer: Vec::new(),
            stats: StreamProcessorStats::default(),
        }
    }

    /// Process a mono f32 audio chunk at the given sample rate.
    ///
    /// Returns processed 16 kHz mono i16 PCM and speech detection result.
    /// When `has_speech` is false, the chunk can be safely skipped
    /// for cloud STT to save tokens/bandwidth.
    pub fn process_chunk(&mut self, audio_f32: &[f32], sample_rate: u32) -> ProcessedChunk {
        self.stats.total_chunks += 1;

        if audio_f32.is_empty() {
            return ProcessedChunk {
                pcm_16khz_mono: Vec::new(),
                has_speech: false,
            };
        }

        // Step 1: Denoise (at 48 kHz) + resample to 16 kHz
        let audio_16khz = if self.denoise_enabled {
            self.denoise_and_resample(audio_f32, sample_rate)
        } else if sample_rate != 16_000 {
            resample_to_16khz(audio_f32, sample_rate).unwrap_or_else(|_| audio_f32.to_vec())
        } else {
            audio_f32.to_vec()
        };

        // Step 2: VAD check on the 16 kHz audio
        let has_speech = if self.vad_enabled {
            self.detect_speech(&audio_16khz)
        } else {
            true
        };

        // Step 3: Convert f32 → i16 for STT engines
        let pcm_i16: Vec<i16> = audio_16khz
            .iter()
            .map(|&s| (s * 32767.0).clamp(-32768.0, 32767.0) as i16)
            .collect();

        if has_speech {
            self.stats.speech_chunks += 1;
        } else {
            self.stats.silent_chunks_skipped += 1;
        }

        ProcessedChunk {
            pcm_16khz_mono: pcm_i16,
            has_speech,
        }
    }

    /// Denoise at 48 kHz using persistent RNNoise state, then resample to 16 kHz.
    fn denoise_and_resample(&mut self, audio: &[f32], sample_rate: u32) -> Vec<f32> {
        // Resample native → 48 kHz for denoising
        let at_48k = if sample_rate != DENOISE_RATE {
            resample(audio, sample_rate, DENOISE_RATE).unwrap_or_else(|_| audio.to_vec())
        } else {
            audio.to_vec()
        };

        // Scale [-1.0, 1.0] → [-32768.0, 32767.0] for nnnoiseless
        let scaled: Vec<f32> = at_48k.iter().map(|&s| s * 32768.0).collect();

        // Append to persistent buffer for frame alignment
        self.denoise_buffer.extend_from_slice(&scaled);

        // Process complete frames only (keep remainder for next call)
        let mut denoised = Vec::with_capacity(self.denoise_buffer.len());

        if let Some(ref mut state) = self.denoise_state {
            let mut buf_out = [0.0f32; FRAME_SIZE];
            let processable_len =
                self.denoise_buffer.len() - (self.denoise_buffer.len() % FRAME_SIZE);

            for chunk in self.denoise_buffer[..processable_len].chunks_exact(FRAME_SIZE) {
                state.process_frame(&mut buf_out, chunk);
                denoised.extend_from_slice(&buf_out);
            }

            // Retain unprocessed remainder for next call
            self.denoise_buffer.drain(..processable_len);
        }

        // Scale back to [-1.0, 1.0]
        let normalized: Vec<f32> = denoised.iter().map(|&s| s / 32768.0).collect();

        // Resample 48 kHz → 16 kHz
        resample_to_16khz(&normalized, DENOISE_RATE).unwrap_or(normalized)
    }

    /// Simple energy-based VAD with hysteresis to avoid rapid switching.
    ///
    /// Uses RMS energy with two thresholds:
    /// - Above SPEECH_ON → classify as speech
    /// - Below SPEECH_OFF → classify as silence
    /// - In between → maintain previous state (hysteresis)
    fn detect_speech(&mut self, audio: &[f32]) -> bool {
        if audio.is_empty() {
            return self.is_speech;
        }

        let sum_sq: f32 = audio.iter().map(|&s| s * s).sum();
        let rms = (sum_sq / audio.len() as f32).sqrt();

        if self.is_speech {
            if rms < VAD_SPEECH_OFF_RMS {
                self.is_speech = false;
                debug!(rms, threshold = VAD_SPEECH_OFF_RMS, "vad_silence_detected");
            }
        } else if rms >= VAD_SPEECH_ON_RMS {
            self.is_speech = true;
            debug!(rms, threshold = VAD_SPEECH_ON_RMS, "vad_speech_detected");
        }

        self.is_speech
    }

    /// Log session statistics.
    pub fn log_stats(&self) {
        info!(
            total_chunks = self.stats.total_chunks,
            speech_chunks = self.stats.speech_chunks,
            silent_chunks_skipped = self.stats.silent_chunks_skipped,
            denoise_enabled = self.denoise_enabled,
            vad_enabled = self.vad_enabled,
            "stream_processor_session_stats"
        );
    }
}

impl Drop for StreamAudioProcessor {
    fn drop(&mut self) {
        self.log_stats();
    }
}
