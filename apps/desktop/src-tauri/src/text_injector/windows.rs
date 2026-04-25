use enigo::{Enigo, Keyboard, Settings};
use tracing::{info, warn};

pub struct WindowsInjector;

const CHUNK_SIZE: usize = 100;
const CHUNK_DELAY_MS: u64 = 50;

impl super::TextInjector for WindowsInjector {
    fn insert(&self, text: &str, write_clipboard: &dyn Fn()) {
        let grapheme_count = text.chars().count();
        info!(
            text_len = text.len(),
            grapheme_count, "text_injection_started"
        );

        // For long text, use clipboard paste (more reliable)
        if grapheme_count > 400 {
            info!(grapheme_count, "text_injection_clipboard_mode-long_text");
            write_clipboard();
            if let Err(e) = self.paste_from_clipboard() {
                warn!(error = %e, "clipboard_paste_failed");
            }
            return;
        }

        // Try keyboard simulation first
        if self.try_enigo_key_sequence(text) {
            info!("text_injection_completed-enigo");
            return;
        }

        // Fallback to clipboard paste
        info!("text_injection_fallback-clipboard");
        write_clipboard();
        if let Err(e) = self.paste_from_clipboard() {
            warn!(error = %e, "clipboard_paste_failed");
        }
    }
}

impl WindowsInjector {
    fn try_enigo_key_sequence(&self, text: &str) -> bool {
        let mut enigo = match Enigo::new(&Settings::default()) {
            Ok(e) => e,
            Err(e) => {
                warn!(error = %e, "enigo_creation_failed");
                return false;
            }
        };

        let char_count = text.chars().count();

        if char_count <= CHUNK_SIZE {
            match enigo.text(text) {
                Ok(_) => {
                    info!("text_injection_enigo_succeeded-single_chunk");
                    true
                }
                Err(e) => {
                    warn!(error = %e, "text_injection_enigo_failed");
                    false
                }
            }
        } else {
            // Split into chunks to avoid IME issues
            let chars: Vec<char> = text.chars().collect();
            let chunk_count = char_count.div_ceil(CHUNK_SIZE);
            info!(chunk_count, "text_injection_chunking_started");

            for (i, chunk) in chars.chunks(CHUNK_SIZE).enumerate() {
                let chunk_str: String = chunk.iter().collect();
                match enigo.text(&chunk_str) {
                    Ok(_) => {
                        info!(
                            chunk_index = i + 1,
                            chunk_chars = chunk.len(),
                            "text_injection_chunk_injected"
                        );
                    }
                    Err(e) => {
                        warn!(chunk_index = i + 1, error = %e, "text_injection_chunk_failed");
                        return false;
                    }
                }

                if i < chunk_count - 1 {
                    std::thread::sleep(std::time::Duration::from_millis(CHUNK_DELAY_MS));
                }
            }

            info!("text_injection_enigo_succeeded-chunked");
            true
        }
    }

    fn paste_from_clipboard(&self) -> Result<(), String> {
        use enigo::{Key, Keyboard, Settings};

        let mut enigo =
            Enigo::new(&Settings::default()).map_err(|e| format!("Failed to create Enigo: {e}"))?;

        std::thread::sleep(std::time::Duration::from_millis(20));

        enigo
            .key(Key::Control, enigo::Direction::Press)
            .map_err(|e| format!("ctrl_press_failed: {e}"))?;

        enigo
            .key(Key::Unicode('v'), enigo::Direction::Click)
            .map_err(|e| format!("v_click_failed: {e}"))?;

        enigo
            .key(Key::Control, enigo::Direction::Release)
            .map_err(|e| format!("ctrl_release_failed: {e}"))?;

        info!("clipboard_paste_ctrlv_sent");
        Ok(())
    }
}
