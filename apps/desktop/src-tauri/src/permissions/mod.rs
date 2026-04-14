use std::sync::{LazyLock, Mutex};

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PermissionKind {
    Accessibility,
    InputMonitoring,
    Microphone,
}

impl PermissionKind {
    pub const ALL: [Self; 3] = [Self::Accessibility, Self::InputMonitoring, Self::Microphone];

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "accessibility" => Some(Self::Accessibility),
            "input_monitoring" => Some(Self::InputMonitoring),
            "microphone" => Some(Self::Microphone),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Accessibility => "accessibility",
            Self::InputMonitoring => "input_monitoring",
            Self::Microphone => "microphone",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PermissionStatus {
    Granted,
    Denied,
    NotDetermined,
}

impl PermissionStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Granted => "granted",
            Self::Denied => "denied",
            Self::NotDetermined => "not_determined",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PermissionSnapshot {
    pub accessibility: PermissionStatus,
    pub input_monitoring: PermissionStatus,
    pub microphone: PermissionStatus,
}

impl PermissionSnapshot {
    pub fn status_for(&self, kind: PermissionKind) -> PermissionStatus {
        match kind {
            PermissionKind::Accessibility => self.accessibility,
            PermissionKind::InputMonitoring => self.input_monitoring,
            PermissionKind::Microphone => self.microphone,
        }
    }
}

pub struct PermissionDefinition {
    pub kind: PermissionKind,
    pub capability: &'static str,
    pub check_method: &'static str,
    pub core_flow: bool,
}

const PERMISSION_DEFINITIONS: [PermissionDefinition; 3] = [
    PermissionDefinition {
        kind: PermissionKind::Accessibility,
        capability: "global_hotkey",
        check_method: "ax_is_process_trusted",
        core_flow: true,
    },
    PermissionDefinition {
        kind: PermissionKind::InputMonitoring,
        capability: "global_key_capture",
        check_method: "iohid_check_access",
        core_flow: true,
    },
    PermissionDefinition {
        kind: PermissionKind::Microphone,
        capability: "audio_capture",
        check_method: "av_capture_device_authorization",
        core_flow: true,
    },
];

pub trait PermissionProvider: Send + Sync {
    fn check_accessibility(&self) -> PermissionStatus;
    fn check_input_monitoring(&self) -> PermissionStatus;
    fn check_microphone(&self) -> PermissionStatus;

    fn apply_accessibility(&self) -> Result<(), String>;
    fn apply_input_monitoring(&self) -> Result<(), String>;
    fn apply_microphone(&self) -> Result<(), String>;
}

struct PermissionReporter {
    last_snapshot: Mutex<Option<PermissionSnapshot>>,
}

static PERMISSION_REPORTER: LazyLock<PermissionReporter> = LazyLock::new(|| PermissionReporter {
    last_snapshot: Mutex::new(None),
});

fn provider() -> Box<dyn PermissionProvider> {
    #[cfg(target_os = "macos")]
    return Box::new(macos::MacosPermissions);
    #[cfg(target_os = "windows")]
    return Box::new(windows::WindowsPermissions);
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    compile_error!("permissions: unsupported platform");
}

fn permission_reporter() -> &'static PermissionReporter {
    &PERMISSION_REPORTER
}

pub fn read_permission_snapshot() -> PermissionSnapshot {
    let provider = provider();
    PermissionSnapshot {
        accessibility: provider.check_accessibility(),
        input_monitoring: provider.check_input_monitoring(),
        microphone: provider.check_microphone(),
    }
}

pub fn check_permission(kind: PermissionKind) -> PermissionStatus {
    read_permission_snapshot().status_for(kind)
}

pub fn apply_permission(kind: PermissionKind) -> Result<(), String> {
    let provider = provider();
    match kind {
        PermissionKind::Accessibility => provider.apply_accessibility(),
        PermissionKind::InputMonitoring => provider.apply_input_monitoring(),
        PermissionKind::Microphone => provider.apply_microphone(),
    }
}

pub fn report_startup_permission_snapshot() -> PermissionSnapshot {
    let snapshot = read_permission_snapshot();
    permission_reporter().report_snapshot("startup", &snapshot, true);
    snapshot
}

pub fn report_permission_snapshot_if_changed(trigger: &'static str) -> PermissionSnapshot {
    let snapshot = read_permission_snapshot();
    permission_reporter().report_snapshot(trigger, &snapshot, false);
    snapshot
}

impl PermissionReporter {
    fn report_snapshot(
        &self,
        trigger: &'static str,
        snapshot: &PermissionSnapshot,
        force_log: bool,
    ) {
        let mut guard = self
            .last_snapshot
            .lock()
            .expect("permission reporter poisoned");
        let previous = guard.clone();
        let changed = previous.as_ref() != Some(snapshot);

        if force_log || changed {
            log_permission_snapshot(trigger, snapshot, previous.as_ref());
            *guard = Some(snapshot.clone());
        } else {
            tracing::debug!(trigger, "app_permission_snapshot_unchanged");
        }
    }
}

fn log_permission_snapshot(
    trigger: &'static str,
    snapshot: &PermissionSnapshot,
    previous: Option<&PermissionSnapshot>,
) {
    tracing::info!(
        trigger,
        accessibility = snapshot.accessibility.as_str(),
        input_monitoring = snapshot.input_monitoring.as_str(),
        microphone = snapshot.microphone.as_str(),
        permission_count = PERMISSION_DEFINITIONS.len(),
        "app_permission_snapshot"
    );

    for definition in PERMISSION_DEFINITIONS {
        let status = snapshot.status_for(definition.kind);
        tracing::info!(
            trigger,
            permission = definition.kind.as_str(),
            status = status.as_str(),
            capability = definition.capability,
            check_method = definition.check_method,
            core_flow = definition.core_flow,
            "app_permission_status"
        );

        if let Some(previous_snapshot) = previous {
            let previous_status = previous_snapshot.status_for(definition.kind);
            if previous_status != status {
                tracing::info!(
                    trigger,
                    permission = definition.kind.as_str(),
                    from = previous_status.as_str(),
                    to = status.as_str(),
                    "app_permission_status_changed"
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{PermissionKind, PermissionSnapshot, PermissionStatus};

    #[test]
    fn permission_kind_round_trip_matches_ipc_contract() {
        for kind in PermissionKind::ALL {
            let encoded = kind.as_str();
            assert_eq!(PermissionKind::parse(encoded), Some(kind));
        }
    }

    #[test]
    fn snapshot_returns_status_for_each_kind() {
        let snapshot = PermissionSnapshot {
            accessibility: PermissionStatus::Granted,
            input_monitoring: PermissionStatus::Denied,
            microphone: PermissionStatus::NotDetermined,
        };

        assert_eq!(
            snapshot.status_for(PermissionKind::Accessibility),
            PermissionStatus::Granted
        );
        assert_eq!(
            snapshot.status_for(PermissionKind::InputMonitoring),
            PermissionStatus::Denied
        );
        assert_eq!(
            snapshot.status_for(PermissionKind::Microphone),
            PermissionStatus::NotDetermined
        );
    }
}
