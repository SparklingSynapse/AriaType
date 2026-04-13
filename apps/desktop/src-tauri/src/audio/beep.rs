use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rodio::{Decoder, OutputStream, Sink};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{info, warn};

const BEEP_DEBOUNCE_MS: u64 = 300;

static START_BEEP_DATA: &[u8] = include_bytes!("../../assets/start_beep.wav");
static STOP_BEEP_DATA: &[u8] = include_bytes!("../../assets/stop_beep.wav");

enum BeepMessage {
    PlayStart,
    PlayStop,
}

struct BeepPlayerInner {
    enabled: bool,
}

pub struct BeepPlayer {
    inner: Mutex<BeepPlayerInner>,
    last_beep_time: AtomicU64,
    tx: Mutex<Option<mpsc::Sender<BeepMessage>>>,
}

impl BeepPlayer {
    fn new() -> Self {
        Self {
            inner: Mutex::new(BeepPlayerInner { enabled: false }),
            last_beep_time: AtomicU64::new(0),
            tx: Mutex::new(None),
        }
    }

    pub fn initialize(&self, enabled: bool) {
        let mut inner = self.inner.lock();
        inner.enabled = enabled;
        if enabled {
            info!("beep_player_initialized-enabled");
        } else {
            info!("beep_player_initialized-disabled");
        }
    }

    pub fn enable(&self) {
        let mut inner = self.inner.lock();
        inner.enabled = true;
        info!("beep_player_enabled");
    }

    pub fn disable(&self) {
        let mut inner = self.inner.lock();
        inner.enabled = false;
        info!("beep_player_disabled");
    }

    fn should_play(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let last = self.last_beep_time.load(Ordering::SeqCst);
        if now.saturating_sub(last) < BEEP_DEBOUNCE_MS {
            info!(elapsed_ms = now.saturating_sub(last), "beep_debounced");
            return false;
        }

        self.last_beep_time.store(now, Ordering::SeqCst);
        true
    }

    fn is_enabled(&self) -> bool {
        self.inner.lock().enabled
    }

    fn play(&self, msg: BeepMessage) {
        if let Some(tx) = self.tx.lock().as_ref() {
            let _ = tx.send(msg);
        }
    }
}

static BEEP_PLAYER: Lazy<BeepPlayer> = Lazy::new(BeepPlayer::new);

pub fn init_beep_player() {
    info!("beep_audio_embedded");

    let (tx, rx) = mpsc::channel();
    *BEEP_PLAYER.tx.lock() = Some(tx);

    std::thread::spawn(move || {
        let (_stream, stream_handle) = match OutputStream::try_default() {
            Ok(s) => s,
            Err(e) => {
                warn!(error = %e, "output_stream_failed");
                return;
            }
        };

        for msg in rx {
            let (data, beep_name) = match msg {
                BeepMessage::PlayStart => (START_BEEP_DATA, "start"),
                BeepMessage::PlayStop => (STOP_BEEP_DATA, "stop"),
            };

            let sink = match Sink::try_new(&stream_handle) {
                Ok(s) => s,
                Err(e) => {
                    warn!(error = %e, beep = %beep_name, "sink_creation_failed");
                    continue;
                }
            };

            let cursor = std::io::Cursor::new(data);
            let source = match Decoder::new(cursor) {
                Ok(s) => s,
                Err(e) => {
                    warn!(error = %e, beep = %beep_name, "decode_failed");
                    continue;
                }
            };

            info!(beep = %beep_name, "beep_playing");
            sink.set_volume(0.25); // 25% volume for audible but not jarring beeps
            sink.append(source);
            sink.sleep_until_end();
            info!(beep = %beep_name, "beep_completed");
        }
    });
}

pub fn initialize_beep_player(enabled: bool) {
    BEEP_PLAYER.initialize(enabled);
}

pub fn enable_beep() {
    BEEP_PLAYER.enable();
}

pub fn disable_beep() {
    BEEP_PLAYER.disable();
}

pub fn play_start_beep() {
    if !BEEP_PLAYER.should_play() {
        return;
    }
    if !BEEP_PLAYER.is_enabled() {
        info!("beep_skipped-start_disabled");
        return;
    }
    BEEP_PLAYER.play(BeepMessage::PlayStart);
}

pub fn play_stop_beep() {
    if !BEEP_PLAYER.should_play() {
        return;
    }
    if !BEEP_PLAYER.is_enabled() {
        info!("beep_skipped-stop_disabled");
        return;
    }
    BEEP_PLAYER.play(BeepMessage::PlayStop);
}
