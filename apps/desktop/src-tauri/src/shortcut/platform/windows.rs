#[cfg(target_os = "windows")]
use std::ptr;
#[cfg(target_os = "windows")]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(target_os = "windows")]
use std::sync::mpsc::Sender;
#[cfg(target_os = "windows")]
use std::sync::{Arc, LazyLock, Mutex};
#[cfg(target_os = "windows")]
use std::thread::{self, JoinHandle};
#[cfg(target_os = "windows")]
use std::time::Duration;

#[cfg(target_os = "windows")]
use crate::shortcut::hotkey_codec::key_token_from_rdev_key;
#[cfg(target_os = "windows")]
use crate::shortcut::matcher::{handle_input, MatcherInput, MatcherState, ModifierKey};

#[cfg(target_os = "windows")]
use super::{PlatformRunner, RunnerMode, RuntimeEvent, SharedMatcherSnapshot};

#[cfg(target_os = "windows")]
static CALLBACK_STATE: LazyLock<Mutex<Option<Arc<WindowsCallbackState>>>> =
    LazyLock::new(|| Mutex::new(None));

#[cfg(target_os = "windows")]
pub struct WindowsRunner {
    thread_handle: Option<JoinHandle<()>>,
    running: Arc<AtomicBool>,
    active: Arc<AtomicBool>,
}

#[cfg(target_os = "windows")]
struct WindowsCallbackState {
    snapshot: SharedMatcherSnapshot,
    matcher_state: Mutex<MatcherState>,
    event_tx: Sender<RuntimeEvent>,
    mode: RunnerMode,
}

#[cfg(target_os = "windows")]
pub fn start_runner(
    mode: RunnerMode,
    snapshot: SharedMatcherSnapshot,
    event_tx: Sender<RuntimeEvent>,
    generation: u64,
) -> Result<WindowsRunner, String> {
    let _ = generation;
    let running = Arc::new(AtomicBool::new(true));
    let active = Arc::new(AtomicBool::new(true));
    let thread_running = Arc::clone(&running);
    let thread_active = Arc::clone(&active);

    let callback_state = Arc::new(WindowsCallbackState {
        snapshot,
        matcher_state: Mutex::new(MatcherState::default()),
        event_tx,
        mode,
    });

    let handle = thread::spawn(move || {
        run_windows_runner(thread_running, thread_active, callback_state);
    });

    Ok(WindowsRunner {
        thread_handle: Some(handle),
        running,
        active,
    })
}

#[cfg(target_os = "windows")]
impl PlatformRunner for WindowsRunner {
    fn stop(&mut self) -> Result<(), String> {
        self.running.store(false, Ordering::SeqCst);
        self.active.store(false, Ordering::SeqCst);

        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }

        Ok(())
    }
}

#[cfg(target_os = "windows")]
fn run_windows_runner(
    running: Arc<AtomicBool>,
    active: Arc<AtomicBool>,
    callback_state: Arc<WindowsCallbackState>,
) {
    use winapi::shared::minwindef::{LPARAM, LRESULT, UINT, WPARAM};
    use winapi::um::libloaderapi::GetModuleHandleW;
    use winapi::um::winuser::{
        CallNextHookEx, DispatchMessageW, PeekMessageW, SetWindowsHookExW, TranslateMessage,
        UnhookWindowsHookEx, KBDLLHOOKSTRUCT, MSG, PM_REMOVE, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP,
        WM_SYSKEYDOWN, WM_SYSKEYUP,
    };

    extern "system" fn keyboard_proc(code: i32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
        use winapi::um::winuser::HC_ACTION;

        if code != HC_ACTION {
            return unsafe { CallNextHookEx(ptr::null_mut(), code, w_param, l_param) };
        }

        let Some(shared) = CALLBACK_STATE.lock().ok().and_then(|state| state.clone()) else {
            return unsafe { CallNextHookEx(ptr::null_mut(), code, w_param, l_param) };
        };

        let keyboard = unsafe { &*(l_param as *const KBDLLHOOKSTRUCT) };
        let Some(input) = matcher_input_from_vk(w_param as UINT, keyboard.vkCode as u16) else {
            return unsafe { CallNextHookEx(ptr::null_mut(), code, w_param, l_param) };
        };

        let snapshot = shared.snapshot.read().clone();
        let mut matcher_state = shared
            .matcher_state
            .lock()
            .expect("windows matcher state poisoned");
        let outcome = handle_input(&mut matcher_state, &snapshot, input);
        drop(matcher_state);

        for matcher_event in outcome.events {
            let _ = shared.event_tx.send(RuntimeEvent::Matcher(matcher_event));
        }

        if outcome.swallow && shared.mode == RunnerMode::Main {
            1
        } else {
            unsafe { CallNextHookEx(ptr::null_mut(), code, w_param, l_param) }
        }
    }

    if let Ok(mut state) = CALLBACK_STATE.lock() {
        *state = Some(callback_state);
    }

    let module = unsafe { GetModuleHandleW(ptr::null()) };
    let hook = unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), module, 0) };

    if hook.is_null() {
        running.store(false, Ordering::SeqCst);
        active.store(false, Ordering::SeqCst);
        if let Ok(mut state) = CALLBACK_STATE.lock() {
            *state = None;
        }
        return;
    }

    let mut message: MSG = unsafe { std::mem::zeroed() };
    while running.load(Ordering::SeqCst) {
        while unsafe { PeekMessageW(&mut message, ptr::null_mut(), 0, 0, PM_REMOVE) } != 0 {
            unsafe {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        thread::sleep(Duration::from_millis(10));
    }

    unsafe {
        UnhookWindowsHookEx(hook);
    }
    if let Ok(mut state) = CALLBACK_STATE.lock() {
        *state = None;
    }
    running.store(false, Ordering::SeqCst);
    active.store(false, Ordering::SeqCst);
}

#[cfg(target_os = "windows")]
fn matcher_input_from_vk(message: u32, vk_code: u16) -> Option<MatcherInput> {
    use rdev::Key;
    use winapi::um::winuser::{WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP};

    let key = match vk_code {
        160 => {
            return Some(if matches!(message, WM_KEYDOWN | WM_SYSKEYDOWN) {
                MatcherInput::ModifierPressed(ModifierKey::ShiftLeft)
            } else {
                MatcherInput::ModifierReleased(ModifierKey::ShiftLeft)
            })
        }
        161 => {
            return Some(if matches!(message, WM_KEYDOWN | WM_SYSKEYDOWN) {
                MatcherInput::ModifierPressed(ModifierKey::ShiftRight)
            } else {
                MatcherInput::ModifierReleased(ModifierKey::ShiftRight)
            })
        }
        162 => {
            return Some(if matches!(message, WM_KEYDOWN | WM_SYSKEYDOWN) {
                MatcherInput::ModifierPressed(ModifierKey::CtrlLeft)
            } else {
                MatcherInput::ModifierReleased(ModifierKey::CtrlLeft)
            })
        }
        163 => {
            return Some(if matches!(message, WM_KEYDOWN | WM_SYSKEYDOWN) {
                MatcherInput::ModifierPressed(ModifierKey::CtrlRight)
            } else {
                MatcherInput::ModifierReleased(ModifierKey::CtrlRight)
            })
        }
        164 => {
            return Some(if matches!(message, WM_KEYDOWN | WM_SYSKEYDOWN) {
                MatcherInput::ModifierPressed(ModifierKey::OptLeft)
            } else {
                MatcherInput::ModifierReleased(ModifierKey::OptLeft)
            })
        }
        165 => {
            return Some(if matches!(message, WM_KEYDOWN | WM_SYSKEYDOWN) {
                MatcherInput::ModifierPressed(ModifierKey::OptRight)
            } else {
                MatcherInput::ModifierReleased(ModifierKey::OptRight)
            })
        }
        91 => {
            return Some(if matches!(message, WM_KEYDOWN | WM_SYSKEYDOWN) {
                MatcherInput::ModifierPressed(ModifierKey::CmdLeft)
            } else {
                MatcherInput::ModifierReleased(ModifierKey::CmdLeft)
            })
        }
        92 => {
            return Some(if matches!(message, WM_KEYDOWN | WM_SYSKEYDOWN) {
                MatcherInput::ModifierPressed(ModifierKey::CmdRight)
            } else {
                MatcherInput::ModifierReleased(ModifierKey::CmdRight)
            })
        }
        8 => Key::Backspace,
        46 => Key::Delete,
        40 => Key::DownArrow,
        35 => Key::End,
        27 => Key::Escape,
        112 => Key::F1,
        113 => Key::F2,
        114 => Key::F3,
        115 => Key::F4,
        116 => Key::F5,
        117 => Key::F6,
        118 => Key::F7,
        119 => Key::F8,
        120 => Key::F9,
        121 => Key::F10,
        122 => Key::F11,
        123 => Key::F12,
        36 => Key::Home,
        37 => Key::LeftArrow,
        34 => Key::PageDown,
        33 => Key::PageUp,
        13 => Key::Return,
        39 => Key::RightArrow,
        32 => Key::Space,
        9 => Key::Tab,
        38 => Key::UpArrow,
        192 => Key::BackQuote,
        49 => Key::Num1,
        50 => Key::Num2,
        51 => Key::Num3,
        52 => Key::Num4,
        53 => Key::Num5,
        54 => Key::Num6,
        55 => Key::Num7,
        56 => Key::Num8,
        57 => Key::Num9,
        48 => Key::Num0,
        189 => Key::Minus,
        187 => Key::Equal,
        81 => Key::KeyQ,
        87 => Key::KeyW,
        69 => Key::KeyE,
        82 => Key::KeyR,
        84 => Key::KeyT,
        89 => Key::KeyY,
        85 => Key::KeyU,
        73 => Key::KeyI,
        79 => Key::KeyO,
        80 => Key::KeyP,
        219 => Key::LeftBracket,
        221 => Key::RightBracket,
        65 => Key::KeyA,
        83 => Key::KeyS,
        68 => Key::KeyD,
        70 => Key::KeyF,
        71 => Key::KeyG,
        72 => Key::KeyH,
        74 => Key::KeyJ,
        75 => Key::KeyK,
        76 => Key::KeyL,
        186 => Key::SemiColon,
        222 => Key::Quote,
        220 => Key::BackSlash,
        226 => Key::IntlBackslash,
        90 => Key::KeyZ,
        88 => Key::KeyX,
        67 => Key::KeyC,
        86 => Key::KeyV,
        66 => Key::KeyB,
        78 => Key::KeyN,
        77 => Key::KeyM,
        188 => Key::Comma,
        190 => Key::Dot,
        191 => Key::Slash,
        45 => Key::Insert,
        109 => Key::KpMinus,
        107 => Key::KpPlus,
        106 => Key::KpMultiply,
        111 => Key::KpDivide,
        96 => Key::Kp0,
        97 => Key::Kp1,
        98 => Key::Kp2,
        99 => Key::Kp3,
        100 => Key::Kp4,
        101 => Key::Kp5,
        102 => Key::Kp6,
        103 => Key::Kp7,
        104 => Key::Kp8,
        105 => Key::Kp9,
        110 => Key::KpDelete,
        _ => return None,
    };

    let token = key_token_from_rdev_key(key)?;
    if matches!(message, WM_KEYDOWN | WM_SYSKEYDOWN) {
        Some(MatcherInput::KeyPressed(token))
    } else if matches!(message, WM_KEYUP | WM_SYSKEYUP) {
        Some(MatcherInput::KeyReleased(token))
    } else {
        None
    }
}

#[cfg(not(target_os = "windows"))]
pub struct WindowsRunner;

#[cfg(not(target_os = "windows"))]
pub fn start_runner(
    _mode: RunnerMode,
    _snapshot: SharedMatcherSnapshot,
    _event_tx: Sender<RuntimeEvent>,
) -> Result<WindowsRunner, String> {
    Err("windows runner unavailable on this platform".to_string())
}

#[cfg(not(target_os = "windows"))]
impl PlatformRunner for WindowsRunner {
    fn stop(&mut self) -> Result<(), String> {
        Ok(())
    }
}
