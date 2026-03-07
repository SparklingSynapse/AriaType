use std::path::Path;
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, info};
use webrtc_vad::{Vad, VadMode};
use whisper_rs::{
    convert_integer_to_float_audio, convert_stereo_to_mono_audio, FullParams, SamplingStrategy,
    WhisperContext,
};

use crate::audio::processor::{denoise_audio, should_denoise};
use crate::audio::resampler::resample_to_16khz;
use crate::events::TranscriptionMetrics;

pub struct Transcriber {
    context: Arc<WhisperContext>,
}

impl Transcriber {
    pub fn from_context(context: Arc<WhisperContext>) -> Self {
        Self { context }
    }

    pub fn transcribe_with_metrics(
        &self,
        audio_path: &Path,
        language: Option<&str>,
        initial_prompt: Option<&str>,
        denoise_mode: &str,
    ) -> Result<(String, TranscriptionMetrics), String> {
        let start_time = Instant::now();

        let _load_end = Instant::now();
        let load_time_ms = start_time.elapsed().as_millis() as u64;

        let preprocess_start = Instant::now();
        if !audio_path.exists() {
            return Err(format!("Audio file not found: {:?}", audio_path));
        }

        let mut reader = hound::WavReader::open(audio_path)
            .map_err(|e| format!("Failed to open WAV file: {}", e))?;

        let spec = reader.spec();
        debug!(
            channels = spec.channels,
            sample_rate = spec.sample_rate,
            bits = spec.bits_per_sample,
            "WAV spec"
        );

        let samples_i16: Vec<i16> = reader
            .samples::<i16>()
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to read audio samples: {}", e))?;

        if samples_i16.is_empty() {
            return Ok((
                String::new(),
                TranscriptionMetrics {
                    load_time_ms,
                    preprocess_time_ms: preprocess_start.elapsed().as_millis() as u64,
                    inference_time_ms: 0,
                    polish_time_ms: 0,
                    total_time_ms: start_time.elapsed().as_millis() as u64,
                },
            ));
        }

        let mut audio = vec![0.0f32; samples_i16.len()];
        convert_integer_to_float_audio(&samples_i16, &mut audio).map_err(|e| e.to_string())?;

        if spec.channels == 2 {
            audio = convert_stereo_to_mono_audio(&audio).map_err(|e| e.to_string())?;
        } else if spec.channels > 2 {
            audio = downmix_to_mono(&audio, spec.channels as usize);
        }

        let apply_denoise = match denoise_mode {
            "on" => true,
            "off" => false,
            _ => should_denoise(&audio, spec.sample_rate), // "auto"
        };
        let audio = if apply_denoise {
            debug!(mode = denoise_mode, "applying denoising");
            denoise_audio(&audio, spec.sample_rate)?
        } else {
            debug!(mode = denoise_mode, "skipping denoising");
            audio
        };

        let audio = if spec.sample_rate != 16_000 {
            resample_to_16khz(&audio, spec.sample_rate)?
        } else {
            audio
        };

        let audio = trim_and_collapse_silence(&audio, 16_000);

        let duration = audio.len() as f32 / 16_000.0;
        if duration < 0.35 {
            return Ok((
                String::new(),
                TranscriptionMetrics {
                    load_time_ms,
                    preprocess_time_ms: preprocess_start.elapsed().as_millis() as u64,
                    inference_time_ms: 0,
                    polish_time_ms: 0,
                    total_time_ms: start_time.elapsed().as_millis() as u64,
                },
            ));
        }

        let preprocess_time_ms = preprocess_start.elapsed().as_millis() as u64;
        let inference_start = Instant::now();

        let lang_opt: Option<&str> = match language {
            Some("auto") | None => None,
            Some(l) => Some(l.split('-').next().unwrap_or(l)),
        };

        info!(
            duration_secs = format!("{:.2}", duration),
            language = ?lang_opt,
            has_prompt = initial_prompt.is_some(),
            prompt = ?initial_prompt,
            "transcribing audio"
        );

        let mut params = FullParams::new(SamplingStrategy::BeamSearch {
            beam_size: 8,
            patience: -1.0,
        });
        params.set_language(lang_opt);
        if let Some(prompt) = initial_prompt {
            params.set_initial_prompt(prompt);
        }
        params.set_translate(false);
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_suppress_blank(true);
        params.set_suppress_nst(true);
        params.set_no_speech_thold(0.35);
        params.set_temperature(0.1);

        let threads = std::thread::available_parallelism()
            .map(|n| n.get().saturating_sub(1).max(1))
            .unwrap_or(4) as i32;
        params.set_n_threads(threads);

        let mut state = self
            .context
            .create_state()
            .map_err(|e| format!("Failed to create Whisper state: {}", e))?;

        state
            .full(params, &audio)
            .map_err(|e| format!("Whisper inference failed: {}", e))?;

        let mut text = String::new();
        for segment in state.as_iter() {
            let s = segment.to_string();
            if !s.trim().is_empty() {
                text.push_str(s.trim());
                text.push(' ');
            }
        }

        let inference_time_ms = inference_start.elapsed().as_millis() as u64;

        let result = text.trim().to_string();
        info!(chars = result.len(), "transcription complete");

        let total_time_ms = start_time.elapsed().as_millis() as u64;

        Ok((
            result,
            TranscriptionMetrics {
                load_time_ms,
                preprocess_time_ms,
                inference_time_ms,
                polish_time_ms: 0,
                total_time_ms,
            },
        ))
    }
}

fn downmix_to_mono(audio: &[f32], channels: usize) -> Vec<f32> {
    let frames = audio.len() / channels;
    (0..frames)
        .map(|i| {
            let sum: f32 = (0..channels).map(|ch| audio[i * channels + ch]).sum();
            sum / channels as f32
        })
        .collect()
}

fn trim_and_collapse_silence(audio: &[f32], sample_rate: u32) -> Vec<f32> {
    if audio.is_empty() {
        return Vec::new();
    }

    let frame_size = ((sample_rate as f32) * 0.02).round() as usize;
    let frame_size = frame_size.max(1);
    let mut vad = match Vad::new(sample_rate as i32) {
        Ok(vad) => vad,
        Err(_) => return audio.to_vec(),
    };
    let _ = vad.fvad_set_mode(VadMode::Quality);
    let mut frames = Vec::new();
    let mut voiced_frames = Vec::new();
    let mut start = 0usize;
    while start < audio.len() {
        let end = (start + frame_size).min(audio.len());
        let frame = &audio[start..end];
        let mut frame_i16: Vec<i16> = frame
            .iter()
            .map(|s| (s * 32767.0).clamp(-32768.0, 32767.0) as i16)
            .collect();
        if frame_i16.len() < frame_size {
            frame_i16.resize(frame_size, 0);
        }
        let voiced = vad.is_voice_segment(&frame_i16).unwrap_or(false);
        frames.push((start, end));
        voiced_frames.push(voiced);
        start = end;
    }

    let first = voiced_frames.iter().position(|v| *v);
    let last = voiced_frames.iter().rposition(|v| *v);
    let (first, last) = match (first, last) {
        (Some(f), Some(l)) if f <= l => (f, l),
        _ => return Vec::new(),
    };

    let max_silence_frames = ((0.5f32 / 0.02f32).round() as usize).max(1);
    let mut out = Vec::with_capacity(audio.len());
    let mut silent_run = 0usize;
    for i in first..=last {
        let (s, e) = frames[i];
        let voiced = voiced_frames[i];
        if voiced {
            silent_run = 0;
            out.extend_from_slice(&audio[s..e]);
        } else {
            silent_run += 1;
            if silent_run <= max_silence_frames {
                out.extend_from_slice(&audio[s..e]);
            }
        }
    }

    debug!(
        input_samples = audio.len(),
        output_samples = out.len(),
        "silence trim complete"
    );

    out
}
