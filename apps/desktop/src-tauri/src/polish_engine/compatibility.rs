use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceProfile {
    pub platform: &'static str,
    pub arch: &'static str,
    pub logical_cpu_count: usize,
    pub total_memory_mb: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PolishModelCompatibilityLevel {
    Smooth,
    Limited,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PolishModelCompatibility {
    pub level: PolishModelCompatibilityLevel,
    pub code: &'static str,
    pub minimum_memory_mb: u64,
    pub recommended_memory_mb: u64,
    pub device_memory_mb: Option<u64>,
    pub logical_cpu_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PolishModelRequirement {
    minimum_memory_mb: u64,
    recommended_memory_mb: u64,
}

impl DeviceProfile {
    pub fn current() -> Self {
        Self {
            platform: current_platform(),
            arch: std::env::consts::ARCH,
            logical_cpu_count: std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1),
            total_memory_mb: total_memory_mb(),
        }
    }
}

pub fn assess_polish_model_compatibility(
    model_id: &str,
    device: &DeviceProfile,
) -> PolishModelCompatibility {
    let requirement = requirement_for_model(model_id);
    let (level, code) = match device.total_memory_mb {
        None => (PolishModelCompatibilityLevel::Limited, "memory_unknown"),
        Some(memory_mb) if memory_mb < requirement.minimum_memory_mb => (
            PolishModelCompatibilityLevel::Unsupported,
            "memory_below_minimum",
        ),
        Some(memory_mb) if memory_mb < requirement.recommended_memory_mb => (
            PolishModelCompatibilityLevel::Limited,
            "memory_below_recommended",
        ),
        Some(_) if device.logical_cpu_count < 4 => {
            (PolishModelCompatibilityLevel::Limited, "cpu_threads_low")
        }
        Some(_) => (PolishModelCompatibilityLevel::Smooth, "smooth"),
    };

    PolishModelCompatibility {
        level,
        code,
        minimum_memory_mb: requirement.minimum_memory_mb,
        recommended_memory_mb: requirement.recommended_memory_mb,
        device_memory_mb: device.total_memory_mb,
        logical_cpu_count: device.logical_cpu_count,
    }
}

fn requirement_for_model(model_id: &str) -> PolishModelRequirement {
    match model_id {
        "qwen3.5-0.8b" => PolishModelRequirement {
            minimum_memory_mb: 4 * 1024,
            recommended_memory_mb: 8 * 1024,
        },
        "lfm2.5-1.2b" => PolishModelRequirement {
            minimum_memory_mb: 6 * 1024,
            recommended_memory_mb: 8 * 1024,
        },
        "qwen3.5-2b" | "gemma-2b-it" | "gemma-4-e2b" | "lfm2-2.6b" => PolishModelRequirement {
            minimum_memory_mb: 8 * 1024,
            recommended_memory_mb: 16 * 1024,
        },
        "qwen3-4b" => PolishModelRequirement {
            minimum_memory_mb: 16 * 1024,
            recommended_memory_mb: 32 * 1024,
        },
        "glm-4.7-flash-reap-23b-a3b" => PolishModelRequirement {
            minimum_memory_mb: 32 * 1024,
            recommended_memory_mb: 64 * 1024,
        },
        _ => PolishModelRequirement {
            minimum_memory_mb: 8 * 1024,
            recommended_memory_mb: 16 * 1024,
        },
    }
}

#[cfg(target_os = "windows")]
fn total_memory_mb() -> Option<u64> {
    use winapi::um::sysinfoapi::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

    let mut status = MEMORYSTATUSEX {
        dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
        dwMemoryLoad: 0,
        ullTotalPhys: 0,
        ullAvailPhys: 0,
        ullTotalPageFile: 0,
        ullAvailPageFile: 0,
        ullTotalVirtual: 0,
        ullAvailVirtual: 0,
        ullAvailExtendedVirtual: 0,
    };

    let ok = unsafe { GlobalMemoryStatusEx(&mut status) };
    (ok != 0).then_some(status.ullTotalPhys / 1024 / 1024)
}

#[cfg(target_os = "macos")]
fn total_memory_mb() -> Option<u64> {
    let output = std::process::Command::new("/usr/sbin/sysctl")
        .args(["-n", "hw.memsize"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let bytes = String::from_utf8(output.stdout)
        .ok()?
        .trim()
        .parse::<u64>()
        .ok()?;
    Some(bytes / 1024 / 1024)
}

#[cfg(target_os = "linux")]
fn total_memory_mb() -> Option<u64> {
    let content = std::fs::read_to_string("/proc/meminfo").ok()?;
    let line = content.lines().find(|line| line.starts_with("MemTotal:"))?;
    let kb = line
        .split_whitespace()
        .nth(1)
        .and_then(|value| value.parse::<u64>().ok())?;
    Some(kb / 1024)
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
fn total_memory_mb() -> Option<u64> {
    None
}

fn current_platform() -> &'static str {
    #[cfg(target_os = "macos")]
    return "macos";
    #[cfg(target_os = "windows")]
    return "windows";
    #[cfg(target_os = "linux")]
    return "linux";
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    return "unknown";
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(memory_mb: Option<u64>, logical_cpu_count: usize) -> DeviceProfile {
        DeviceProfile {
            platform: "test",
            arch: "x86_64",
            logical_cpu_count,
            total_memory_mb: memory_mb,
        }
    }

    #[test]
    fn marks_small_model_smooth_on_recommended_memory() {
        let compatibility =
            assess_polish_model_compatibility("qwen3.5-0.8b", &profile(Some(8 * 1024), 8));

        assert_eq!(compatibility.level, PolishModelCompatibilityLevel::Smooth);
        assert_eq!(compatibility.code, "smooth");
    }

    #[test]
    fn marks_model_limited_below_recommended_memory() {
        let compatibility =
            assess_polish_model_compatibility("qwen3.5-2b", &profile(Some(12 * 1024), 8));

        assert_eq!(compatibility.level, PolishModelCompatibilityLevel::Limited);
        assert_eq!(compatibility.code, "memory_below_recommended");
    }

    #[test]
    fn marks_model_unsupported_below_minimum_memory() {
        let compatibility =
            assess_polish_model_compatibility("qwen3-4b", &profile(Some(8 * 1024), 8));

        assert_eq!(
            compatibility.level,
            PolishModelCompatibilityLevel::Unsupported
        );
        assert_eq!(compatibility.code, "memory_below_minimum");
    }

    #[test]
    fn marks_low_cpu_threads_as_limited_after_memory_passes() {
        let compatibility =
            assess_polish_model_compatibility("qwen3.5-0.8b", &profile(Some(16 * 1024), 2));

        assert_eq!(compatibility.level, PolishModelCompatibilityLevel::Limited);
        assert_eq!(compatibility.code, "cpu_threads_low");
    }

    #[test]
    fn marks_unknown_memory_as_limited() {
        let compatibility = assess_polish_model_compatibility("qwen3.5-0.8b", &profile(None, 8));

        assert_eq!(compatibility.level, PolishModelCompatibilityLevel::Limited);
        assert_eq!(compatibility.code, "memory_unknown");
    }

    #[test]
    fn marks_glm_limited_on_32gb_memory() {
        let compatibility = assess_polish_model_compatibility(
            "glm-4.7-flash-reap-23b-a3b",
            &profile(Some(32 * 1024), 12),
        );

        assert_eq!(compatibility.level, PolishModelCompatibilityLevel::Limited);
        assert_eq!(compatibility.code, "memory_below_recommended");
        assert_eq!(compatibility.minimum_memory_mb, 32 * 1024);
        assert_eq!(compatibility.recommended_memory_mb, 64 * 1024);
    }

    #[test]
    fn marks_glm_unsupported_below_32gb_memory() {
        let compatibility = assess_polish_model_compatibility(
            "glm-4.7-flash-reap-23b-a3b",
            &profile(Some(16 * 1024), 12),
        );

        assert_eq!(
            compatibility.level,
            PolishModelCompatibilityLevel::Unsupported
        );
        assert_eq!(compatibility.code, "memory_below_minimum");
    }
}
