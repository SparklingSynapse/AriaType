use std::path::Path;
use std::time::Instant;

use nnnoiseless::DenoiseState;
use sherpa_onnx::{SileroVadModelConfig, VadModelConfig, VoiceActivityDetector};
use tracing::{debug, info, warn};

use crate::audio::resampler::{resample, resample_to_16khz};

/// RNNoise operates at 48kHz with fixed frame size.
const DENOISE_RATE: u32 = 48_000;
const FRAME_SIZE: usize = DenoiseState::FRAME_SIZE;

/// Silero VAD window size in samples at 16kHz.
const VAD_WINDOW_SIZE: usize = 512;

/// Maximum silence duration before sending a keepalive chunk.
/// Cloud STT providers (Volcengine confirmed at 5s, others unknown) disconnect
/// if no audio is received within a timeout period. We send a keepalive at 4s
/// to stay safely under the 5s limit while minimizing bandwidth waste.
/// See: https://www.volcengine.com/docs/6561/1354869 (Volcengine streaming STT)
const KEEPALIVE_INTERVAL_SECS: u64 = 4;

/// Dry/wet mix ratio for RNNoise denoising (0.0–1.0).
/// 1.0 = full denoising (wet only), can cause speech distortion in clean environments.
/// 0.7 = blend 70% denoised + 30% original, preserving speech nuances while
/// suppressing most noise. Empirically good balance for STT accuracy.
const DENOISE_STRENGTH: f32 = 0.5;

/// Processed audio chunk with speech detection result.
pub struct ProcessedChunk {
    /// Processed audio as 16-bit PCM at 16 kHz mono, ready for STT.
    pub pcm_16khz_mono: Vec<i16>,
    /// Processed audio as f32 at 16 kHz mono, for systems that need float samples.
    pub audio_f32_16khz: Vec<f32>,
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

/// Wrapper around Silero VAD for streaming use.
///
/// Silero VAD's `VoiceActivityDetector` accumulates state across calls.
/// For streaming, we create a fresh detector per recording session,
/// feed audio chunks via `accept_waveform()`, and check `detected()`
/// for per-window speech presence.
///
/// Thread-safe: All access is serialized through the outer Mutex on StreamAudioProcessor,
/// so the raw pointer inside VoiceActivityDetector is never accessed concurrently.
struct ThreadSafeVad {
    vad: VoiceActivityDetector,
}

// SAFETY: ThreadSafeVad is only accessed through Mutex<StreamAudioProcessor>,
// ensuring all operations are serialized. No concurrent access to the VAD is possible.
unsafe impl Send for ThreadSafeVad {}

impl ThreadSafeVad {
    /// Create a new Silero VAD wrapper for a recording session.
    ///
    /// Returns None if the VAD model cannot be found or loaded.
    fn new(vad_model_path: &Path) -> Option<Self> {
        if !vad_model_path.exists() {
            warn!(path = %vad_model_path.display(), "silero_vad_model_not_found");
            return None;
        }

        let silero_config = SileroVadModelConfig {
            model: Some(
                vad_model_path
                    .to_str()
                    .unwrap_or_else(|| {
                        warn!("vad_model_path_invalid_encoding");
                        ""
                    })
                    .to_string(),
            ),
            threshold: 0.15,
            min_silence_duration: 0.25,
            min_speech_duration: 0.1,
            max_speech_duration: 30.0,
            window_size: VAD_WINDOW_SIZE as i32,
        };

        let vad_config = VadModelConfig {
            silero_vad: silero_config,
            ten_vad: Default::default(),
            sample_rate: 16_000,
            num_threads: 1,
            provider: Some("cpu".to_string()),
            debug: false,
        };

        VoiceActivityDetector::create(&vad_config, 30.0)
            .map(|vad| Self { vad })
            .or_else(|| {
                warn!("silero_vad_creation_failed");
                None
            })
    }

    /// Detect speech in the given audio chunk.
    ///
    /// Feeds audio to VAD in 512-sample windows and uses `detected()` for
    /// per-window speech detection. Returns true if speech is present in
    /// any window of this batch.
    ///
    /// Uses `detected()` (not `front()`/`pop()`) because `front()` only
    /// returns completed segments after a silence gap — continuous speech
    /// without pauses would never trigger `front()`.
    fn detect_speech(&mut self, audio_16khz: &[f32]) -> bool {
        if audio_16khz.is_empty() {
            return false;
        }

        for chunk in audio_16khz.chunks(VAD_WINDOW_SIZE) {
            let mut padded = chunk.to_vec();
            if padded.len() < VAD_WINDOW_SIZE {
                padded.resize(VAD_WINDOW_SIZE, 0.0f32);
            }
            self.vad.accept_waveform(&padded);

            if self.vad.detected() {
                return true;
            }
        }

        false
    }
}

/// Real-time audio processor for streaming recording.
///
/// Provides two independent pre-processing stages for STT:
/// - **Denoise**: RNNoise neural network noise suppression at 48kHz (when enabled).
///   Denoise is a pre-processing step for both local and cloud STT.
/// - **VAD**: Silero VAD for accurate voice activity detection at 16kHz.
///   Silero VAD replaces RNNoise's built-in VAD for higher accuracy (87.7% TPR vs ~50%).
///
/// Audio pipeline:
/// 1. Resample to 48kHz → RNNoise denoise (if enabled)
/// 2. Resample denoised/original to 16kHz → Silero VAD speech detection
/// 3. Convert f32 → i16 for STT output
///
/// Designed for use in the audio callback thread via `Arc<Mutex<>>`.
pub struct StreamAudioProcessor {
    denoise_enabled: bool,
    rnnoise_state: Box<DenoiseState<'static>>,
    rnnoise_buffer: Vec<f32>,
    vad_enabled: bool,
    vad: Option<ThreadSafeVad>,
    stats: StreamProcessorStats,
    last_send_time: Option<Instant>,
}

impl StreamAudioProcessor {
    /// Create a new stream processor.
    ///
    /// - `denoise_mode`: RNNoise noise suppression mode - "on", "off", or "auto".
    ///   This is a pre-processing step for both local and cloud STT.
    /// - `vad_enabled`: Whether to use Silero VAD for speech detection.
    ///   When disabled, all audio is treated as speech.
    /// - `vad_model_path`: Path to the Silero VAD ONNX model file.
    ///   Ignored when `vad_enabled` is false.
    pub fn new(denoise_mode: &str, vad_enabled: bool, vad_model_path: Option<&Path>) -> Self {
        let denoise_enabled = match denoise_mode {
            "on" => true,
            "off" => false,
            "auto" => false, // auto currently defaults to off for streaming
            _ => false,
        };
        info!(
            denoise_mode,
            denoise_enabled, vad_enabled, "stream_audio_processor_created"
        );

        let vad = if vad_enabled {
            vad_model_path.and_then(ThreadSafeVad::new).or_else(|| {
                warn!("vad_enabled_but_model_not_provided_or_load_failed");
                None
            })
        } else {
            None
        };

        Self {
            denoise_enabled,
            rnnoise_state: DenoiseState::new(),
            rnnoise_buffer: Vec::new(),
            vad_enabled,
            vad,
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
                audio_f32_16khz: Vec::new(),
                has_speech: false,
            };
        }

        let audio_16khz = if self.denoise_enabled {
            self.denoise_and_resample(audio_f32, sample_rate)
        } else if sample_rate != 16_000 {
            resample_to_16khz(audio_f32, sample_rate).unwrap_or_else(|e| {
                warn!(error = e, sample_rate, "resample_failed");
                audio_f32.to_vec()
            })
        } else {
            audio_f32.to_vec()
        };

        let vad_result = if self.vad_enabled {
            match self.vad.as_mut() {
                Some(vad) => vad.detect_speech(&audio_16khz),
                None => true,
            }
        } else {
            true
        };

        // Keepalive: periodic chunk during silence prevents cloud STT disconnection
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

        let audio_f32 = audio_16khz;
        let pcm_i16: Vec<i16> = audio_f32
            .iter()
            .map(|&s| (s * 32767.0).clamp(-32768.0, 32767.0) as i16)
            .collect();

        if has_speech {
            self.stats.speech_chunks += 1;
        } else {
            self.stats.silent_chunks_skipped += 1;
            debug!("vad_silence_detected");
        }

        ProcessedChunk {
            pcm_16khz_mono: pcm_i16,
            audio_f32_16khz: audio_f32,
            has_speech,
        }
    }

    /// RNNoise denoising at 48kHz, output resampled to 16kHz.
    /// Discards RNNoise VAD probability (Silero VAD handles speech detection).
    fn denoise_and_resample(&mut self, audio: &[f32], sample_rate: u32) -> Vec<f32> {
        let at_48k = if sample_rate != DENOISE_RATE {
            resample(audio, sample_rate, DENOISE_RATE).unwrap_or_else(|_| audio.to_vec())
        } else {
            audio.to_vec()
        };

        let scaled: Vec<f32> = at_48k.iter().map(|&s| s * 32768.0).collect();
        self.rnnoise_buffer.extend_from_slice(&scaled);

        let mut denoised = Vec::with_capacity(self.rnnoise_buffer.len());
        let mut buf_out = [0.0f32; FRAME_SIZE];
        let processable_len = self.rnnoise_buffer.len() - (self.rnnoise_buffer.len() % FRAME_SIZE);

        for chunk in self.rnnoise_buffer[..processable_len].chunks_exact(FRAME_SIZE) {
            let _ = self.rnnoise_state.process_frame(&mut buf_out, chunk);
            denoised.extend_from_slice(&buf_out);
        }

        self.rnnoise_buffer.drain(..processable_len);

        let wet: Vec<f32> = denoised.iter().map(|&s| s / 32768.0).collect();
        let dry = &at_48k[..wet.len().min(at_48k.len())];
        let wet = &wet[..dry.len()];

        let mixed: Vec<f32> = dry
            .iter()
            .zip(wet.iter())
            .map(|(&d, &w)| d * (1.0 - DENOISE_STRENGTH) + w * DENOISE_STRENGTH)
            .collect();

        resample_to_16khz(&mixed, DENOISE_RATE).unwrap_or(mixed)
    }

    /// Log session statistics.
    pub fn log_stats(&self) {
        info!(
            total_chunks = self.stats.total_chunks,
            speech_chunks = self.stats.speech_chunks,
            silent_chunks_skipped = self.stats.silent_chunks_skipped,
            denoise_enabled = self.denoise_enabled,
            vad_enabled = self.vad_enabled,
            vad_available = self.vad.is_some(),
            "stream_processor_session_stats"
        );
    }
}

impl Drop for StreamAudioProcessor {
    fn drop(&mut self) {
        self.log_stats();
    }
}
