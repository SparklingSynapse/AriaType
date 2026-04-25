use std::sync::mpsc::Sender;
use std::sync::Arc;

use parking_lot::RwLock;

use crate::shortcut::matcher::{MatcherEvent, MatcherSnapshot};

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RunnerMode {
    Main,
    CaptureOnly,
}

pub type SharedMatcherSnapshot = Arc<RwLock<MatcherSnapshot>>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeEvent {
    Matcher(MatcherEvent),
    RunnerNeedsRestart { mode: RunnerMode, generation: u64 },
}

pub trait PlatformRunner: Send {
    fn stop(&mut self) -> Result<(), String>;
}

pub fn start_platform_runner(
    mode: RunnerMode,
    snapshot: SharedMatcherSnapshot,
    event_tx: Sender<RuntimeEvent>,
    generation: u64,
) -> Result<Box<dyn PlatformRunner>, String> {
    #[cfg(target_os = "macos")]
    {
        macos::start_runner(mode, snapshot, event_tx, generation)
            .map(|runner| Box::new(runner) as Box<dyn PlatformRunner>)
    }
    #[cfg(target_os = "windows")]
    {
        windows::start_runner(mode, snapshot, event_tx, generation)
            .map(|runner| Box::new(runner) as Box<dyn PlatformRunner>)
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = mode;
        let _ = snapshot;
        let _ = event_tx;
        let _ = generation;
        Err("shortcut platform runner unsupported on this platform".to_string())
    }
}
