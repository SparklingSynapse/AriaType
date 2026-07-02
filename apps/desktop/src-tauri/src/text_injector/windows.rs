#![cfg_attr(all(test, not(target_os = "windows")), allow(dead_code))]

use enigo::{Direction, Enigo, Key, Keyboard, Settings};
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
        let mut enigo =
            Enigo::new(&Settings::default()).map_err(|e| format!("Failed to create Enigo: {e}"))?;

        std::thread::sleep(std::time::Duration::from_millis(20));

        send_clipboard_paste_shortcut(&mut enigo)?;
        info!("clipboard_paste_ctrlv_sent");
        Ok(())
    }
}

trait KeyboardDriver {
    fn key(&mut self, key: Key, direction: Direction) -> Result<(), String>;
}

impl KeyboardDriver for Enigo {
    fn key(&mut self, key: Key, direction: Direction) -> Result<(), String> {
        Keyboard::key(self, key, direction).map_err(|error| error.to_string())
    }
}

fn send_clipboard_paste_shortcut(keyboard: &mut dyn KeyboardDriver) -> Result<(), String> {
    let mut errors = Vec::new();

    if let Err(error) = keyboard.key(Key::Control, Direction::Press) {
        errors.push(format!("ctrl_press_failed: {error}"));
        errors.extend(release_keyboard_modifiers(keyboard));
        return Err(errors.join("; "));
    }

    if let Err(error) = keyboard.key(paste_shortcut_key(), Direction::Click) {
        errors.push(format!("v_click_failed: {error}"));
    }

    if let Err(error) = keyboard.key(Key::Control, Direction::Release) {
        errors.push(format!("ctrl_release_failed: {error}"));
    }

    errors.extend(release_keyboard_modifiers(keyboard));

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

fn paste_shortcut_key() -> Key {
    #[cfg(target_os = "windows")]
    {
        Key::V
    }
    #[cfg(not(target_os = "windows"))]
    {
        Key::Unicode('v')
    }
}

fn release_keyboard_modifiers(keyboard: &mut dyn KeyboardDriver) -> Vec<String> {
    [
        Key::Control,
        Key::LControl,
        Key::RControl,
        Key::Shift,
        Key::LShift,
        Key::RShift,
        Key::Alt,
        Key::Option,
        Key::Meta,
    ]
    .into_iter()
    .filter_map(|key| {
        keyboard
            .key(key, Direction::Release)
            .err()
            .map(|error| format!("modifier_release_failed({key:?}): {error}"))
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        paste_shortcut_key, release_keyboard_modifiers, send_clipboard_paste_shortcut,
        KeyboardDriver,
    };
    use enigo::{Direction, Key};

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct KeyEvent {
        key: Key,
        direction: Direction,
    }

    #[derive(Default)]
    struct FakeKeyboard {
        events: Vec<KeyEvent>,
        failures: Vec<(Key, Direction, &'static str)>,
    }

    impl FakeKeyboard {
        fn fail_on(mut self, key: Key, direction: Direction, message: &'static str) -> Self {
            self.failures.push((key, direction, message));
            self
        }
    }

    impl KeyboardDriver for FakeKeyboard {
        fn key(&mut self, key: Key, direction: Direction) -> Result<(), String> {
            self.events.push(KeyEvent { key, direction });
            if let Some((_, _, message)) =
                self.failures
                    .iter()
                    .find(|(failed_key, failed_direction, _)| {
                        *failed_key == key && *failed_direction == direction
                    })
            {
                return Err((*message).to_string());
            }

            Ok(())
        }
    }

    #[test]
    fn paste_shortcut_releases_control_when_v_click_fails() {
        let mut keyboard = FakeKeyboard::default().fail_on(
            paste_shortcut_key(),
            Direction::Click,
            "v was blocked",
        );

        let error = send_clipboard_paste_shortcut(&mut keyboard).unwrap_err();

        assert!(error.contains("v_click_failed: v was blocked"));
        assert!(keyboard.events.starts_with(&[
            KeyEvent {
                key: Key::Control,
                direction: Direction::Press,
            },
            KeyEvent {
                key: paste_shortcut_key(),
                direction: Direction::Click,
            },
            KeyEvent {
                key: Key::Control,
                direction: Direction::Release,
            },
        ]));
    }

    #[test]
    fn paste_shortcut_reports_control_release_failure_after_cleanup_attempt() {
        let mut keyboard = FakeKeyboard::default().fail_on(
            Key::Control,
            Direction::Release,
            "ctrl release blocked",
        );

        let error = send_clipboard_paste_shortcut(&mut keyboard).unwrap_err();

        assert!(error.contains("ctrl_release_failed: ctrl release blocked"));
        assert!(error.contains("modifier_release_failed(Control): ctrl release blocked"));
    }

    #[test]
    fn modifier_cleanup_releases_common_modifier_keys() {
        let mut keyboard = FakeKeyboard::default();

        let errors = release_keyboard_modifiers(&mut keyboard);

        assert!(errors.is_empty());
        assert_eq!(
            keyboard.events,
            vec![
                KeyEvent {
                    key: Key::Control,
                    direction: Direction::Release,
                },
                KeyEvent {
                    key: Key::LControl,
                    direction: Direction::Release,
                },
                KeyEvent {
                    key: Key::RControl,
                    direction: Direction::Release,
                },
                KeyEvent {
                    key: Key::Shift,
                    direction: Direction::Release,
                },
                KeyEvent {
                    key: Key::LShift,
                    direction: Direction::Release,
                },
                KeyEvent {
                    key: Key::RShift,
                    direction: Direction::Release,
                },
                KeyEvent {
                    key: Key::Alt,
                    direction: Direction::Release,
                },
                KeyEvent {
                    key: Key::Option,
                    direction: Direction::Release,
                },
                KeyEvent {
                    key: Key::Meta,
                    direction: Direction::Release,
                },
            ]
        );
    }
}
