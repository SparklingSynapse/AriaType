use crate::utils::AppPaths;
use hound::WavReader;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::{info, warn};

const TARGET_SAMPLE_RATE: u32 = 16000;

pub struct SenseVoiceTranscriber {
    binary_path: PathBuf,
    model_path: PathBuf,
}

impl SenseVoiceTranscriber {
    pub fn new(model_path: &Path) -> Result<Self, String> {
        let binary_path = Self::get_sidecar_path()?;

        if !binary_path.exists() {
            return Err(format!("SenseVoice binary not found at: {:?}", binary_path));
        }

        if !model_path.exists() {
            return Err(format!("SenseVoice model not found at: {:?}", model_path));
        }

        info!(
            binary = ?binary_path,
            model = ?model_path,
            "SenseVoice transcriber initialized"
        );

        Ok(Self {
            binary_path,
            model_path: model_path.to_path_buf(),
        })
    }

    fn get_sidecar_path() -> Result<PathBuf, String> {
        let binary_name = Self::get_sidecar_binary_name();

        #[cfg(debug_assertions)]
        {
            let platform_dir = {
                #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
                {
                    "apple-silicon"
                }

                #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
                {
                    "apple-silicon"
                }

                #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
                {
                    "linux"
                }

                #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
                {
                    "windows"
                }
            };

            let bin_path = std::env::current_dir()
                .map_err(|e| format!("Failed to get current directory: {}", e))?
                .join("bin")
                .join(platform_dir)
                .join(binary_name);

            if bin_path.exists() {
                return Ok(bin_path);
            }
        }

        #[cfg(not(debug_assertions))]
        {
            return Ok(PathBuf::from(binary_name));
        }

        #[cfg(debug_assertions)]
        Err(format!("SenseVoice binary not found: {}", binary_name))
    }

    fn get_sidecar_binary_name() -> &'static str {
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        return "sense-voice-main-aarch64-apple-darwin";

        #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
        return "sense-voice-main-x86_64-apple-darwin";

        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        return "sense-voice-main-x86_64-unknown-linux-gnu";

        #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
        return "sense-voice-main-x86_64-pc-windows-msvc.exe";
    }

    pub fn transcribe(&self, audio_path: &Path, language: Option<&str>) -> Result<String, String> {
        if !audio_path.exists() {
            return Err(format!("Audio file not found: {:?}", audio_path));
        }

        info!(
            audio = ?audio_path,
            model = ?self.model_path,
            language = ?language,
            "starting SenseVoice transcription"
        );

        let processed_audio = self.prepare_audio(audio_path)?;
        let audio_to_use = processed_audio
            .as_ref()
            .map(|p| p.as_path())
            .unwrap_or(audio_path);

        let threads = std::thread::available_parallelism()
            .map(|n| n.get().min(4))
            .unwrap_or(4);

        let mut cmd = Command::new(&self.binary_path);
        cmd.arg("-m")
            .arg(&self.model_path)
            .arg("-f")
            .arg(audio_to_use)
            .arg("-t")
            .arg(threads.to_string())
            .arg("-np")
            .arg("-itn");

        if let Some(lang) = language {
            if lang != "auto" {
                cmd.arg("-l").arg(lang);
            }
        }

        let output = cmd
            .output()
            .map_err(|e| format!("Failed to execute SenseVoice binary: {}", e))?;

        if let Some(ref temp_audio) = processed_audio {
            let _ = std::fs::remove_file(temp_audio);
        }

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!(stderr = %stderr, "SenseVoice execution failed");
            return Err(format!("SenseVoice failed: {}", stderr));
        }

        // Parse stdout directly - SenseVoice outputs text to stdout
        let stdout = String::from_utf8_lossy(&output.stdout);
        info!(stdout = %stdout, "SenseVoice raw stdout");

        self.parse_text_output(&stdout)
    }

    fn prepare_audio(&self, audio_path: &Path) -> Result<Option<PathBuf>, String> {
        let reader =
            WavReader::open(audio_path).map_err(|e| format!("Failed to open audio file: {}", e))?;

        let spec = reader.spec();
        info!(
            sample_rate = spec.sample_rate,
            channels = spec.channels,
            "input audio format"
        );

        if spec.sample_rate == TARGET_SAMPLE_RATE {
            return Ok(None);
        }

        info!(
            from = spec.sample_rate,
            to = TARGET_SAMPLE_RATE,
            "resampling audio for SenseVoice"
        );

        let samples: Vec<f32> = reader
            .into_samples::<i16>()
            .filter_map(|s| s.ok())
            .map(|s| s as f32 / i16::MAX as f32)
            .collect();

        let resampled =
            crate::audio::resampler::resample(&samples, spec.sample_rate, TARGET_SAMPLE_RATE)
                .map_err(|e| format!("Failed to resample audio: {}", e))?;

        let temp_path =
            AppPaths::temp_dir().join(format!("sensevoice_input_{}.wav", uuid::Uuid::new_v4()));

        let mut writer = hound::WavWriter::create(
            &temp_path,
            hound::WavSpec {
                channels: 1,
                sample_rate: TARGET_SAMPLE_RATE,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            },
        )
        .map_err(|e| format!("Failed to create temp audio file: {}", e))?;

        for sample in resampled {
            let sample = (sample * i16::MAX as f32).clamp(i16::MIN as f32, i16::MAX as f32) as i16;
            writer
                .write_sample(sample)
                .map_err(|e| format!("Failed to write audio sample: {}", e))?;
        }

        writer
            .finalize()
            .map_err(|e| format!("Failed to finalize audio file: {}", e))?;

        Ok(Some(temp_path))
    }

    fn parse_text_output(&self, text: &str) -> Result<String, String> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            warn!("SenseVoice produced empty output");
            return Ok(String::new());
        }

        // SenseVoice stdout contains multiple lines. Find lines matching:
        // [start-end] <|optional_prefix_tags|> transcribed text
        // or without prefix tags: [start-end] transcribed text
        let mut segments: Vec<String> = Vec::new();

        for line in trimmed.lines() {
            let line = line.trim();
            // Match pattern: [number-number] ...
            if line.starts_with('[') {
                if let Some(bracket_end) = line.find(']') {
                    let after_bracket = line[bracket_end + 1..].trim();

                    // Skip empty content after bracket
                    if after_bracket.is_empty() {
                        continue;
                    }

                    // Check if there's prefix tags like <|zh|><|NEUTRAL|><|Speech|><|withitn|>
                    let text_part = if let Some(pos) = after_bracket.rfind(">") {
                        after_bracket[pos + 1..].trim()
                    } else {
                        // No prefix tags, the whole part after bracket is the text
                        after_bracket
                    };

                    if !text_part.is_empty() {
                        segments.push(text_part.to_string());
                    }
                }
            }
        }

        if segments.is_empty() {
            warn!(output = %trimmed, "SenseVoice output format not recognized");
            return Ok(String::new());
        }

        // Join all segments without extra spaces (Chinese text doesn't need spaces)
        Ok(segments.join(""))
    }
}
