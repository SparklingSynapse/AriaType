use rodio::{Decoder, OutputStream, Sink};
use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;
use tracing::{info, warn};

static BEEP_DATA: std::sync::OnceLock<BeepAudioData> = std::sync::OnceLock::new();

struct BeepAudioData {
    start_beep: Arc<Vec<u8>>,
    stop_beep: Arc<Vec<u8>>,
}

pub fn init_beep_player() {
    let start_beep = match load_audio_file("assets/start_beep.wav") {
        Ok(data) => Arc::new(data),
        Err(e) => {
            tracing::error!(error = %e, "failed to load start beep audio");
            return;
        }
    };

    let stop_beep = match load_audio_file("assets/stop_beep.wav") {
        Ok(data) => Arc::new(data),
        Err(e) => {
            tracing::error!(error = %e, "failed to load stop beep audio");
            return;
        }
    };

    let data = BeepAudioData {
        start_beep,
        stop_beep,
    };
    let _ = BEEP_DATA.set(data);
    tracing::info!("beep audio data preloaded successfully");
}

fn load_audio_file(path: &str) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let exe_dir = std::env::current_exe()?
        .parent()
        .ok_or("failed to get executable directory")?
        .to_path_buf();

    let audio_path = exe_dir.join(path);
    info!(path = %audio_path.display(), "loading beep audio file");

    let data = std::fs::read(&audio_path)?;
    Ok(data)
}

fn get_beep_data() -> Option<&'static BeepAudioData> {
    BEEP_DATA.get()
}

pub fn play_start_beep() {
    let data = get_beep_data().map(|d| d.start_beep.clone());

    std::thread::spawn(move || {
        info!("playing start beep");
        if let Err(e) = play_beep_from_memory_or_file(data.as_deref(), "assets/start_beep.wav") {
            warn!(error = %e, "failed to play start beep");
        } else {
            info!("start beep completed");
        }
    });
}

pub fn play_stop_beep() {
    let data = get_beep_data().map(|d| d.stop_beep.clone());

    std::thread::spawn(move || {
        info!("playing stop beep");
        if let Err(e) = play_beep_from_memory_or_file(data.as_deref(), "assets/stop_beep.wav") {
            warn!(error = %e, "failed to play stop beep");
        } else {
            info!("stop beep completed");
        }
    });
}

fn play_beep_from_memory_or_file(
    data: Option<&Vec<u8>>,
    fallback_path: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (_stream, stream_handle) = OutputStream::try_default()?;
    let sink = Sink::try_new(&stream_handle)?;

    match data {
        Some(data) => {
            let cursor = std::io::Cursor::new(data.clone());
            let source = Decoder::new(cursor)?;
            sink.append(source);
        }
        None => {
            let exe_dir = std::env::current_exe()?
                .parent()
                .ok_or("failed to get executable directory")?
                .to_path_buf();
            let audio_path = exe_dir.join(fallback_path);
            let file = File::open(&audio_path)?;
            let source = Decoder::new(BufReader::new(file))?;
            sink.append(source);
        }
    }

    sink.sleep_until_end();

    Ok(())
}
