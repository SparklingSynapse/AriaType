use crate::state::unified_state::StreamingSttState;
use tracing::{info, warn};

/// Save raw audio buffer to WAV file.
/// Returns the path to the saved file, or None if no audio was recorded.
pub fn save_raw_audio_to_file(
    streaming_state: &StreamingSttState,
    sample_rate: u32,
    channels: u16,
) -> Option<String> {
    let path = streaming_state.audio_save_path.as_ref()?;
    let buffer = streaming_state.raw_audio_buffer.lock();

    if buffer.is_empty() {
        warn!("raw_audio_buffer_empty-no_file_saved");
        return None;
    }

    if sample_rate == 0 || channels == 0 {
        warn!(
            sample_rate,
            channels, "audio_params_not_set-cannot_save_file"
        );
        return None;
    }

    let spec = hound::WavSpec {
        channels,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    match hound::WavWriter::create(path, spec) {
        Ok(mut writer) => {
            for sample in buffer.iter() {
                if let Err(e) = writer.write_sample(*sample) {
                    warn!(error = %e, "wav_write_failed");
                    return None;
                }
            }
            if let Err(e) = writer.finalize() {
                warn!(error = %e, "wav_finalize_failed");
                return None;
            }
            info!(path = %path.display(), samples = buffer.len(), sample_rate, channels, "raw_audio_saved");
            Some(path.to_string_lossy().to_string())
        }
        Err(e) => {
            warn!(error = %e, path = %path.display(), "wav_create_failed");
            None
        }
    }
}
