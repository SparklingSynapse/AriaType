use std::sync::mpsc::Sender;
use std::sync::Arc;

use parking_lot::RwLock;

#[cfg(any(test, target_os = "windows"))]
use crate::shortcut::matcher::MatcherInput;
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
    #[cfg_attr(not(any(test, target_os = "macos")), allow(dead_code))]
    RunnerNeedsRestart {
        mode: RunnerMode,
        generation: u64,
    },
}

pub trait PlatformRunner: Send {
    fn stop(&mut self) -> Result<(), String>;
}

#[cfg(any(test, target_os = "windows"))]
fn should_swallow_windows_input(
    outcome_swallow: bool,
    mode: RunnerMode,
    input: &MatcherInput,
) -> bool {
    outcome_swallow
        && mode == RunnerMode::Main
        && !matches!(
            input,
            MatcherInput::ModifierPressed(_) | MatcherInput::ModifierReleased(_)
        )
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

#[cfg(test)]
mod tests {
    use super::{should_swallow_windows_input, RunnerMode};
    use crate::shortcut::matcher::{MatcherInput, ModifierKey};

    #[test]
    fn windows_main_runner_passes_modifier_events_through() {
        assert!(!should_swallow_windows_input(
            true,
            RunnerMode::Main,
            &MatcherInput::ModifierPressed(ModifierKey::OptLeft)
        ));
        assert!(!should_swallow_windows_input(
            true,
            RunnerMode::Main,
            &MatcherInput::ModifierReleased(ModifierKey::OptLeft)
        ));
    }

    #[test]
    fn windows_main_runner_still_swallows_primary_key_events() {
        assert!(should_swallow_windows_input(
            true,
            RunnerMode::Main,
            &MatcherInput::KeyPressed("Slash".to_string())
        ));
        assert!(should_swallow_windows_input(
            true,
            RunnerMode::Main,
            &MatcherInput::KeyReleased("Slash".to_string())
        ));
    }

    #[test]
    fn windows_capture_runner_does_not_swallow_events() {
        assert!(!should_swallow_windows_input(
            true,
            RunnerMode::CaptureOnly,
            &MatcherInput::KeyPressed("Slash".to_string())
        ));
    }
}
