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
            engine = "sensevoice",
            binary = ?binary_path,
            model = ?model_path,
            "transcriber_initialized"
        );

        Ok(Self {
            binary_path,
            model_path: model_path.to_path_buf(),
        })
    }

    fn get_sidecar_path() -> Result<PathBuf, String> {
        let binary_name = Self::get_sidecar_binary_name();
        let relative_path = PathBuf::from("bin")
            .join(Self::get_sidecar_platform_dir())
            .join(binary_name);

        for candidate in Self::sidecar_candidates(&relative_path) {
            if candidate.exists() {
                return Ok(candidate);
            }
        }

        Err(format!(
            "SenseVoice binary not found at: {:?}",
            relative_path
        ))
    }

    fn get_sidecar_platform_dir() -> &'static str {
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        return "apple-silicon";

        #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
        return "apple-silicon";

        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        return "linux";

        #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
        return "windows";
    }

    fn get_sidecar_binary_name() -> &'static str {
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        return "sense-voice-main-aarch64-apple-darwin";

        #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
        return "sense-voice-main-x86_64-apple-darwin";

        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        return "sense-voice-main-x86_64-unknown-linux-gnu";

        #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
        return "sense-voice-main-x86_64-pc-windows.exe";
    }

    fn sidecar_candidates(relative_path: &Path) -> Vec<PathBuf> {
        let mut candidates = Vec::new();

        if let Ok(current_dir) = std::env::current_dir() {
            candidates.push(current_dir.join(relative_path));
            candidates.push(current_dir.join("src-tauri").join(relative_path));
        }

        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        candidates.push(manifest_dir.join(relative_path));

        if let Ok(current_exe) = std::env::current_exe() {
            if let Some(exe_dir) = current_exe.parent() {
                candidates.push(exe_dir.join(relative_path.file_name().unwrap_or_default()));
                candidates.push(exe_dir.join(relative_path));
                candidates.push(exe_dir.join("../Resources").join(relative_path));
                candidates.push(exe_dir.join("../../Resources").join(relative_path));
            }
        }

        candidates
    }

    pub fn transcribe(&self, audio_path: &Path, language: Option<&str>) -> Result<String, String> {
        if !audio_path.exists() {
            return Err(format!("Audio file not found: {:?}", audio_path));
        }

        info!(
            engine = "sensevoice",
            audio = ?audio_path,
            model = ?self.model_path,
            language = ?language,
            "transcription_started"
        );

        let processed_audio = self.prepare_audio(audio_path)?;
        let audio_to_use = processed_audio.as_deref().unwrap_or(audio_path);

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

        let cli_language = normalize_cli_language(language);
        if let Some(lang) = cli_language.as_deref() {
            cmd.arg("-l").arg(lang);
        } else if language.is_some_and(|lang| lang != "auto") {
            warn!(engine = "sensevoice", requested_language = ?language, "language_not_supported_fallback_auto");
        }

        let output = cmd
            .output()
            .map_err(|e| format!("Failed to execute SenseVoice binary: {}", e))?;

        if let Some(ref temp_audio) = processed_audio {
            let _ = std::fs::remove_file(temp_audio);
        }

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!(engine = "sensevoice", stderr = %stderr, "execution_failed");
            return Err(format!("SenseVoice failed: {}", stderr));
        }

        // Parse stdout directly - SenseVoice outputs text to stdout
        let stdout = String::from_utf8_lossy(&output.stdout);
        info!(engine = "sensevoice", stdout = %stdout, "raw_stdout");

        self.parse_text_output(&stdout)
    }

    fn prepare_audio(&self, audio_path: &Path) -> Result<Option<PathBuf>, String> {
        let reader =
            WavReader::open(audio_path).map_err(|e| format!("Failed to open audio file: {}", e))?;

        let spec = reader.spec();
        info!(
            engine = "sensevoice",
            sample_rate = spec.sample_rate,
            channels = spec.channels,
            "input_audio_format"
        );

        if spec.sample_rate == TARGET_SAMPLE_RATE {
            return Ok(None);
        }

        info!(
            engine = "sensevoice",
            from = spec.sample_rate,
            to = TARGET_SAMPLE_RATE,
            "resampling_audio"
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
            warn!(engine = "sensevoice", "empty_output");
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
            warn!(engine = "sensevoice", output = %trimmed, "output_format_not_recognized");
            return Ok(String::new());
        }

        // Join all segments without extra spaces (Chinese text doesn't need spaces)
        Ok(segments.join(""))
    }
}

fn normalize_cli_language(language: Option<&str>) -> Option<String> {
    let lang = match language {
        Some("auto") | None => return None,
        Some(lang) => lang,
    };

    let base = lang.split('-').next().unwrap_or(lang).to_ascii_lowercase();
    match base.as_str() {
        "zh" | "en" | "yue" | "ja" | "ko" => Some(base),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::normalize_cli_language;

    #[test]
    fn normalize_cli_language_keeps_supported_base_codes() {
        assert_eq!(normalize_cli_language(Some("zh")), Some("zh".to_string()));
        assert_eq!(normalize_cli_language(Some("en")), Some("en".to_string()));
        assert_eq!(normalize_cli_language(Some("yue")), Some("yue".to_string()));
    }

    #[test]
    fn normalize_cli_language_converts_bcp47_tags() {
        assert_eq!(
            normalize_cli_language(Some("zh-CN")),
            Some("zh".to_string())
        );
        assert_eq!(
            normalize_cli_language(Some("zh-TW")),
            Some("zh".to_string())
        );
        assert_eq!(
            normalize_cli_language(Some("en-US")),
            Some("en".to_string())
        );
        assert_eq!(
            normalize_cli_language(Some("yue-CN")),
            Some("yue".to_string())
        );
        assert_eq!(
            normalize_cli_language(Some("ja-JP")),
            Some("ja".to_string())
        );
        assert_eq!(
            normalize_cli_language(Some("ko-KR")),
            Some("ko".to_string())
        );
    }

    #[test]
    fn normalize_cli_language_falls_back_to_auto_for_unsupported_tags() {
        assert_eq!(normalize_cli_language(None), None);
        assert_eq!(normalize_cli_language(Some("auto")), None);
        assert_eq!(normalize_cli_language(Some("fr-FR")), None);
        assert_eq!(normalize_cli_language(Some("de-DE")), None);
    }
}
