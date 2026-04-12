//! FN/Globe key event blocker for macOS.
//!
//! macOS has a hidden behavior: short-pressing FN/Globe key (<~0.2s) triggers
//! the emoji picker or input source switcher via hidden NX_SYSDEFINED events
//! with keyCode 179, sent AFTER the FN key is released.
//!
//! Additionally, the `handy-keys` library only blocks FN press events (when
//! FN is in blocking_hotkeys), but NOT FN release events. This causes a
//! mismatch where the system sees an orphaned FN release, triggering system
//! FN shortcuts like input source switching.
//!
//! This module creates a dedicated CGEventTap to:
//! 1. Track FN key FlagsChanged events to detect release
//! 2. Block KeyDown/KeyUp events for keyCode 179 and NX_SYSDEFINED events
//!    within 200ms after FN release
//!
//! References:
//! - Typeless (proprietary app) handles this cleanly
//! - https://macos-defaults.com/keyboard/applefnusagetype.html

#[cfg(target_os = "macos")]
use std::ffi::c_void;
#[cfg(target_os = "macos")]
use std::ptr::NonNull;
#[cfg(target_os = "macos")]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(target_os = "macos")]
use std::sync::Arc;
#[cfg(target_os = "macos")]
use std::thread::{self, JoinHandle};
#[cfg(target_os = "macos")]
use std::time::Instant;

#[cfg(target_os = "macos")]
use parking_lot::Mutex;

/// NX_SYSDEFINED event type constant (same as NSSystemDefined).
/// These events contain special system-defined data like media keys and FN emoji trigger.
#[cfg(target_os = "macos")]
const NX_SYSDEFINED: u32 = 14;

/// Window duration after FN release to block NX_SYSDEFINED events.
/// macOS sends emoji picker (keyCode 179) quickly after FN release (~50ms),
/// but input source switching may take longer (~100-200ms).
/// Extended window ensures all FN-triggered system events are blocked.
#[cfg(target_os = "macos")]
const FN_RELEASE_BLOCK_WINDOW_MS: u64 = 200;

/// KeyCode for the FN/Globe key itself.
#[cfg(target_os = "macos")]
const KEYCODE_FN: u16 = 0x3F;

/// KeyCode for the hidden events macOS sends to trigger emoji picker / input source switch.
#[cfg(target_os = "macos")]
const KEYCODE_FN_HIDDEN_TRIGGER: u16 = 179;

// CoreGraphics Event Type Constants (as u32 for matching against event_type.0)
#[cfg(target_os = "macos")]
const EVENT_TYPE_KEY_DOWN: u32 = 10;
#[cfg(target_os = "macos")]
const EVENT_TYPE_KEY_UP: u32 = 11;
#[cfg(target_os = "macos")]
const EVENT_TYPE_FLAGS_CHANGED: u32 = 12;
#[cfg(target_os = "macos")]
const EVENT_TYPE_TAP_DISABLED_BY_TIMEOUT: u32 = 15;
#[cfg(target_os = "macos")]
const EVENT_TYPE_TAP_DISABLED_BY_USER_INPUT: u32 = 16;

/// State for the FN emoji blocker event tap.
#[cfg(target_os = "macos")]
struct FnEmojiBlockerState {
    /// Timestamp of FN key release (if recently released).
    fn_release_time: Mutex<Option<Instant>>,
}

#[cfg(target_os = "macos")]
impl FnEmojiBlockerState {
    /// Checks if the current time is within the blocking window after an FN release.
    fn is_in_block_window(&self, now: Instant) -> bool {
        if let Some(release_time) = *self.fn_release_time.lock() {
            let elapsed_ms = (now - release_time).as_millis() as u64;
            elapsed_ms < FN_RELEASE_BLOCK_WINDOW_MS
        } else {
            false
        }
    }
}

/// FN/Globe key emoji picker blocker.
///
/// Creates a CGEventTap that intercepts NX_SYSDEFINED events and blocks
/// the hidden keyCode 179 events that macOS sends to trigger emoji picker
/// after a short FN key press.
#[cfg(target_os = "macos")]
pub struct FnEmojiBlocker {
    thread_handle: Option<JoinHandle<()>>,
    running: Arc<AtomicBool>,
    active: Arc<AtomicBool>,
}

#[cfg(target_os = "macos")]
impl FnEmojiBlocker {
    /// Create a new FN emoji blocker (not yet started).
    pub fn new() -> Self {
        Self {
            thread_handle: None,
            running: Arc::new(AtomicBool::new(false)),
            active: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Start the emoji blocker.
    ///
    /// This creates a CGEventTap that monitors NX_SYSDEFINED events and
    /// blocks keyCode 179 events within 50ms of FN release.
    pub fn start(&mut self) -> Result<(), String> {
        if self.running.load(Ordering::SeqCst) {
            return Err("FN emoji blocker already running".to_string());
        }

        self.running.store(true, Ordering::SeqCst);
        self.active.store(true, Ordering::SeqCst);

        let running = Arc::clone(&self.running);
        let active = Arc::clone(&self.active);

        // Check accessibility permissions first
        if !handy_keys::check_accessibility() {
            return Err("Accessibility permission not granted".to_string());
        }

        let handle = thread::spawn(move || {
            run_fn_emoji_blocker_tap(running, active);
        });

        self.thread_handle = Some(handle);
        tracing::info!("fn_emoji_blocker_started");
        Ok(())
    }

    /// Stop the emoji blocker.
    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        self.active.store(false, Ordering::SeqCst);

        if let Some(handle) = self.thread_handle.take() {
            // Do not sleep here; it delays the calling thread. The thread will exit soon anyway.
            let _ = handle.join();
        }
        tracing::info!("fn_emoji_blocker_stopped");
    }

    /// Check if blocker is active.
    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::SeqCst)
    }
}

#[cfg(target_os = "macos")]
impl Default for FnEmojiBlocker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_os = "macos")]
impl Drop for FnEmojiBlocker {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Run the FN emoji blocker CGEventTap.
///
/// Monitors KeyDown/KeyUp events and blocks keyCode 179 events within
/// the block window after FN release.
///
/// Uses TailInsertEventTap to process events AFTER handy-keys' HeadInsertEventTap.
/// This allows handy-keys to see FN press events (for hotkey triggering) while
/// we catch FN release events that handy-keys didn't block.
#[cfg(target_os = "macos")]
fn run_fn_emoji_blocker_tap(running: Arc<AtomicBool>, active: Arc<AtomicBool>) {
    use objc2_core_foundation::{CFMachPort, CFRetained, CFRunLoop, CFRunLoopSource};
    use objc2_core_graphics::{
        CGEvent, CGEventMask, CGEventTapCallBack, CGEventTapLocation, CGEventTapOptions,
        CGEventTapPlacement, CGEventType,
    };

    let state = Arc::new(FnEmojiBlockerState {
        fn_release_time: Mutex::new(None),
    });

    // We need to monitor:
    // 1. FlagsChanged - to detect FN release (event type 12)
    // 2. KeyDown/KeyUp - to catch the hidden keyCode 179 events
    // 3. NX_SYSDEFINED - to block other potential emoji triggers (event type 14)
    let event_mask: CGEventMask = (1 << CGEventType::FlagsChanged.0)
        | (1 << CGEventType::KeyDown.0)
        | (1 << CGEventType::KeyUp.0)
        | (1 << NX_SYSDEFINED);

    let state_ptr = Arc::into_raw(Arc::clone(&state)) as *mut c_void;

    // Pack state pointer into user_info for the callback
    let callback: CGEventTapCallBack = Some(fn_emoji_blocker_callback);

    // Use TailInsertEventTap (value 1) to process events AFTER handy-keys' HeadInsertEventTap.
    // This allows handy-keys to see FN press events (for hotkey triggering),
    // while we catch and block the hidden keyCode 179 events macOS sends after FN release.
    let tail_insert_tap = CGEventTapPlacement(1);

    let tap: Option<CFRetained<CFMachPort>> = unsafe {
        CGEvent::tap_create(
            CGEventTapLocation::SessionEventTap, // Session tap is allowed without root
            tail_insert_tap, // TailInsert
            CGEventTapOptions::Default, // Active tap
            event_mask,
            callback,
            state_ptr,
        )
    };

    let tap = match tap {
        Some(t) => t,
        None => {
            unsafe {
                let _ = Arc::from_raw(state_ptr as *const FnEmojiBlockerState);
            }
            tracing::error!("fn_emoji_blocker_tap_creation_failed");
            active.store(false, Ordering::SeqCst);
            return;
        }
    };

    // Create run loop source
    let source: Option<CFRetained<CFRunLoopSource>> =
        CFMachPort::new_run_loop_source(None, Some(&tap), 0);

    let source = match source {
        Some(s) => s,
        None => {
            unsafe {
                CFMachPort::invalidate(&tap);
                let _ = Arc::from_raw(state_ptr as *const FnEmojiBlockerState);
            }
            tracing::error!("fn_emoji_blocker_runloop_source_failed");
            active.store(false, Ordering::SeqCst);
            return;
        }
    };

    let run_loop = match CFRunLoop::current() {
        Some(rl) => rl,
        None => {
            unsafe {
                CFMachPort::invalidate(&tap);
                let _ = Arc::from_raw(state_ptr as *const FnEmojiBlockerState);
            }
            tracing::error!("fn_emoji_blocker_runloop_failed");
            active.store(false, Ordering::SeqCst);
            return;
        }
    };

    run_loop.add_source(Some(&source), unsafe {
        objc2_core_foundation::kCFRunLoopCommonModes
    });
    CGEvent::tap_enable(&tap, true);

    tracing::info!("fn_emoji_blocker_tap_enabled");

    // Run the loop
    while running.load(Ordering::SeqCst) {
        CFRunLoop::run_in_mode(
            unsafe { objc2_core_foundation::kCFRunLoopDefaultMode },
            0.1, // 100ms timeout
            true,
        );

        // Re-enable tap if macOS disabled it
        if !CGEvent::tap_is_enabled(&tap) {
            CGEvent::tap_enable(&tap, true);
            tracing::debug!("fn_emoji_blocker_tap_re-enabled");
        }
    }

    // Cleanup
    run_loop.remove_source(Some(&source), unsafe {
        objc2_core_foundation::kCFRunLoopCommonModes
    });
    CGEvent::tap_enable(&tap, false);
    CFMachPort::invalidate(&tap);
    unsafe {
        let _ = Arc::from_raw(state_ptr as *const FnEmojiBlockerState);
    }
    active.store(false, Ordering::SeqCst);
}

/// CGEventTap callback for FN emoji blocker.
///
/// Returns NULL to block events that match the FN emoji trigger pattern.
///
/// Blocking strategy:
/// 1. FN key FlagsChanged events (keycode 0x3F) - Allowed through for hotkey trigger.
/// 2. KeyDown/KeyUp events for keyCode 179 within 200ms of FN release - BLOCKED
///    macOS sends these hidden events after a short FN press to trigger the emoji
///    picker or input source switcher. Blocking them prevents the system UI.
/// 3. NX_SYSDEFINED events within 200ms of FN release - BLOCKED (fallback).
#[cfg(target_os = "macos")]
unsafe extern "C-unwind" fn fn_emoji_blocker_callback(
    _proxy: objc2_core_graphics::CGEventTapProxy,
    event_type: objc2_core_graphics::CGEventType,
    event: NonNull<objc2_core_graphics::CGEvent>,
    user_info: *mut c_void,
) -> *mut objc2_core_graphics::CGEvent {
    use objc2_core_graphics::{
        CGEvent, CGEventField, CGEventFlags,
    };

    let state = &*(user_info as *const FnEmojiBlockerState);
    let cg_event = event.as_ref();
    let now = Instant::now();

    let should_block = match event_type.0 {
        // FlagsChanged (12) - track FN release
        EVENT_TYPE_FLAGS_CHANGED => {
            let flags = CGEvent::flags(Some(cg_event));
            let has_fn = flags.contains(CGEventFlags::MaskSecondaryFn);

            let keycode =
                CGEvent::integer_value_field(Some(cg_event), CGEventField::KeyboardEventKeycode)
                    as u16;

            // keycode 0x3F (63) is FN key itself
            if keycode == KEYCODE_FN {
                if !has_fn {
                    // FN released - start the block window for hidden events
                    *state.fn_release_time.lock() = Some(now);
                    tracing::debug!("fn_key_released_block_window_started");
                } else {
                    // FN pressed - clear any previous release time
                    *state.fn_release_time.lock() = None;
                    tracing::debug!("fn_key_pressed_tracking_started");
                }
                
                // Do NOT block the FN event itself, let handy-keys process it
                false
            } else {
                false // Don't block other modifier FlagsChanged events
            }
        }
        // KeyDown (10) / KeyUp (11) - check for hidden keyCode 179
        EVENT_TYPE_KEY_DOWN | EVENT_TYPE_KEY_UP => {
            let keycode =
                CGEvent::integer_value_field(Some(cg_event), CGEventField::KeyboardEventKeycode)
                    as u16;

            if keycode == KEYCODE_FN_HIDDEN_TRIGGER && state.is_in_block_window(now) {
                tracing::debug!(
                    event_type = event_type.0,
                    "blocking_hidden_keycode_179_after_fn_release"
                );
                return std::ptr::null_mut(); // Block this event immediately
            }
            false
        }
        // NX_SYSDEFINED (14) - check for emoji trigger just in case
        NX_SYSDEFINED => {
            // Check if we're within the block window after FN release
            if state.is_in_block_window(now) {
                tracing::debug!("blocking_nx_sysdefined_after_fn_release");
                return std::ptr::null_mut(); // Block this event immediately
            }
            false
        }
        // TapDisabledByTimeout (15) or TapDisabledByUserInput (16)
        EVENT_TYPE_TAP_DISABLED_BY_TIMEOUT | EVENT_TYPE_TAP_DISABLED_BY_USER_INPUT => {
            tracing::debug!("fn_emoji_blocker_tap_disabled");
            false
        }
        _ => false,
    };

    if should_block {
        std::ptr::null_mut() // Block the event
    } else {
        event.as_ptr() // Pass through
    }
}

#[cfg(not(target_os = "macos"))]
pub struct FnEmojiBlocker;

#[cfg(not(target_os = "macos"))]
impl FnEmojiBlocker {
    pub fn new() -> Self {
        Self
    }
    pub fn start(&mut self) -> Result<(), String> {
        Ok(())
    }
    pub fn stop(&mut self) {}
    pub fn is_active(&self) -> bool {
        false
    }
}
