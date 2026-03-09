use rodio::{Decoder, OutputStream, Sink};
use tracing::{info, warn};

static START_BEEP_DATA: &[u8] = include_bytes!("../../assets/start_beep.wav");
static STOP_BEEP_DATA: &[u8] = include_bytes!("../../assets/stop_beep.wav");

pub fn init_beep_player() {
    // Audio data is embedded at compile time via include_bytes!, no initialization needed.
    info!("beep audio data embedded at compile time");
}

pub fn play_start_beep() {
    std::thread::spawn(move || {
        info!("playing start beep");
        if let Err(e) = play_beep(START_BEEP_DATA) {
            warn!(error = %e, "failed to play start beep");
        } else {
            info!("start beep completed");
        }
    });
}

pub fn play_stop_beep() {
    std::thread::spawn(move || {
        info!("playing stop beep");
        if let Err(e) = play_beep(STOP_BEEP_DATA) {
            warn!(error = %e, "failed to play stop beep");
        } else {
            info!("stop beep completed");
        }
    });
}

fn play_beep(data: &'static [u8]) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (_stream, stream_handle) = OutputStream::try_default()?;
    let sink = Sink::try_new(&stream_handle)?;
    let cursor = std::io::Cursor::new(data);
    let source = Decoder::new(cursor)?;
    sink.append(source);
    sink.sleep_until_end();
    Ok(())
}
