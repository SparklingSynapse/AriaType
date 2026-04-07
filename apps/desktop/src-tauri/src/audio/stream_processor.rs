use nnnoiseless::DenoiseState;
use std::time::Instant;
use tracing::{debug, info};

use crate::audio::resampler::{resample, resample_to_16khz};

const DENOISE_RATE: u32 = 48_000;
const FRAME_SIZE: usize = DenoiseState::FRAME_SIZE;

/// Default VAD probability threshold.
/// RNNoise returns low probabilities (0.1-0.5) even for clear speech.
/// Lower threshold (0.1) ensures we don't miss speech.
const DEFAULT_VAD_THRESHOLD: f32 = 0.06;

/// Maximum silence duration before sending a keepalive chunk.
/// Cloud STT providers (Volcengine confirmed at 5s, others unknown) disconnect
/// if no audio is received within a timeout period. We send a keepalive at 4s
/// to stay safely under the 5s limit while minimizing bandwidth waste.
/// See: https://www.volcengine.com/docs/6561/1354869 (Volcengine streaming STT)
const KEEPALIVE_INTERVAL_SECS: u64 = 4;

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
/// Maintains persistent RNNoise state across chunks. RNNoise provides:
/// - **Denoise**: Noise suppression at 48 kHz (when enabled)
/// - **VAD**: Neural network-based voice activity detection (always available)
/// - **Resampling**: Output at 16 kHz mono for STT engines
///
/// When denoise is disabled, we still run RNNoise to get VAD probability,
/// but discard the denoised output and use the original audio instead.
///
/// Designed for use in the audio callback thread via `Arc<Mutex<>>`.
pub struct StreamAudioProcessor {
    rnnoise_state: Box<DenoiseState<'static>>,
    denoise_enabled: bool,
    vad_enabled: bool,
    vad_threshold: f32,
    rnnoise_buffer: Vec<f32>,
    vad_prob_max: f32,
    stats: StreamProcessorStats,
    last_send_time: Option<Instant>,
}

impl StreamAudioProcessor {
    /// Create a new stream processor.
    pub fn new(denoise_enabled: bool, vad_enabled: bool) -> Self {
        info!(
            denoise_enabled,
            vad_enabled, "stream_audio_processor_created"
        );
        Self {
            rnnoise_state: DenoiseState::new(),
            denoise_enabled,
            vad_enabled,
            vad_threshold: DEFAULT_VAD_THRESHOLD,
            rnnoise_buffer: Vec::new(),
            vad_prob_max: 0.0,
            stats: StreamProcessorStats::default(),
            last_send_time: None,
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

        let (processed_audio, _) = self.process_with_rnnoise(audio_f32, sample_rate);

        let audio_16khz = if self.denoise_enabled {
            processed_audio
        } else if sample_rate != 16_000 {
            resample_to_16khz(audio_f32, sample_rate).unwrap_or_else(|_| audio_f32.to_vec())
        } else {
            audio_f32.to_vec()
        };

        let vad_result = if self.vad_enabled {
            let has_speech = self.vad_prob_max > self.vad_threshold;
            debug!(
                vad_prob_max = self.vad_prob_max,
                threshold = self.vad_threshold,
                has_speech,
                "vad_check"
            );
            has_speech
        } else {
            true
        };

        let now = Instant::now();
        let needs_keepalive = self.vad_enabled
            && !vad_result
            && self
                .last_send_time
                .is_none_or(|t| now.duration_since(t).as_secs() >= KEEPALIVE_INTERVAL_SECS);

        let has_speech = vad_result || needs_keepalive;

        if has_speech {
            self.last_send_time = Some(now);
        }

        let pcm_i16: Vec<i16> = audio_16khz
            .iter()
            .map(|&s| (s * 32767.0).clamp(-32768.0, 32767.0) as i16)
            .collect();

        if has_speech {
            self.stats.speech_chunks += 1;
        } else {
            self.stats.silent_chunks_skipped += 1;
            debug!(
                vad_prob_max = self.vad_prob_max,
                threshold = self.vad_threshold,
                "vad_silence_detected"
            );
        }

        ProcessedChunk {
            pcm_16khz_mono: pcm_i16,
            has_speech,
        }
    }

    /// Process audio through RNNoise to get both denoised output and VAD probability.
    ///
    /// Returns (denoised_audio_at_16khz, max_vad_probability).
    /// Uses max instead of average because a single frame with speech indicates presence.
    fn process_with_rnnoise(&mut self, audio: &[f32], sample_rate: u32) -> (Vec<f32>, f32) {
        let at_48k = if sample_rate != DENOISE_RATE {
            resample(audio, sample_rate, DENOISE_RATE).unwrap_or_else(|_| audio.to_vec())
        } else {
            audio.to_vec()
        };

        let scaled: Vec<f32> = at_48k.iter().map(|&s| s * 32768.0).collect();
        self.rnnoise_buffer.extend_from_slice(&scaled);

        self.vad_prob_max = 0.0;

        let mut denoised = Vec::with_capacity(self.rnnoise_buffer.len());
        let mut buf_out = [0.0f32; FRAME_SIZE];
        let processable_len = self.rnnoise_buffer.len() - (self.rnnoise_buffer.len() % FRAME_SIZE);

        for chunk in self.rnnoise_buffer[..processable_len].chunks_exact(FRAME_SIZE) {
            let vad_prob = self.rnnoise_state.process_frame(&mut buf_out, chunk);
            denoised.extend_from_slice(&buf_out);
            self.vad_prob_max = self.vad_prob_max.max(vad_prob);
            debug!(
                vad_prob,
                vad_prob_max = self.vad_prob_max,
                "rnnoise_frame_vad"
            );
        }

        self.rnnoise_buffer.drain(..processable_len);

        let normalized: Vec<f32> = denoised.iter().map(|&s| s / 32768.0).collect();
        let resampled = resample_to_16khz(&normalized, DENOISE_RATE).unwrap_or(normalized);

        (resampled, self.vad_prob_max)
    }

    /// Log session statistics.
    pub fn log_stats(&self) {
        info!(
            total_chunks = self.stats.total_chunks,
            speech_chunks = self.stats.speech_chunks,
            silent_chunks_skipped = self.stats.silent_chunks_skipped,
            denoise_enabled = self.denoise_enabled,
            vad_enabled = self.vad_enabled,
            vad_threshold = self.vad_threshold,
            "stream_processor_session_stats"
        );
    }
}

impl Drop for StreamAudioProcessor {
    fn drop(&mut self) {
        self.log_stats();
    }
}
