//! FN/Globe key emoji picker blocking for macOS.
//!
//! The blocking logic is integrated directly into the primary shortcut runtime
//! callback (`macos_runner_callback`) in `platform/macos.rs`. This ensures
//! FN release tracking and emoji event blocking happen in the same CGEventTap,
//! eliminating cross-thread race conditions that occurred with a separate
//! TailInsert tap on a different CFRunLoop.
//!
//! Previously, a separate CGEventTap on a different thread would sometimes
//! miss FN release events or see emoji trigger events before seeing the
//! FN release, causing the emoji picker to leak through.
//!
//! This module retains the `FnEmojiBlocker` type as a ZST stub for API
//! compatibility, but it no longer creates a separate event tap.

pub struct FnEmojiBlocker;

impl FnEmojiBlocker {
    pub fn new() -> Self {
        Self
    }
    pub fn start(
        &mut self,
        _event_tx: std::sync::mpsc::Sender<crate::shortcut::platform::RuntimeEvent>,
        _mode: crate::shortcut::platform::RunnerMode,
        _generation: u64,
    ) -> Result<(), String> {
        Ok(())
    }
    pub fn stop(&mut self) {}
    pub fn is_active(&self) -> bool {
        false
    }
}

impl Default for FnEmojiBlocker {
    fn default() -> Self {
        Self::new()
    }
}
