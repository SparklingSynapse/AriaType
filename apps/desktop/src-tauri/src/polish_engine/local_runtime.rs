use crate::utils::AppPaths;
use reqwest::Url;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Mutex, OnceLock, RwLock};
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};

const LOCAL_POLISH_LLAMA_SERVER_PROVIDER: &str = "llama-server";
const LOCAL_POLISH_SERVER_COMMAND_ENV: &str = "ARIATYPE_LOCAL_POLISH_SERVER_COMMAND";
const LOCAL_POLISH_SERVER_ARGS_JSON_ENV: &str = "ARIATYPE_LOCAL_POLISH_SERVER_ARGS_JSON";
const LOCAL_POLISH_READY_TIMEOUT_SECS_ENV: &str = "ARIATYPE_LOCAL_POLISH_READY_TIMEOUT_SECS";
const LOCAL_POLISH_READY_POLL_INTERVAL: Duration = Duration::from_millis(250);
const LOCAL_POLISH_DEFAULT_READY_TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LocalPolishRuntimeConfig {
    pub provider_type: String,
    pub base_url: String,
    pub api_key: Option<String>,
    pub server_command: Option<String>,
    pub server_args: Option<Vec<String>>,
    pub ready_timeout: Duration,
}

impl LocalPolishRuntimeConfig {
    fn from_env() -> Self {
        Self {
            provider_type: LOCAL_POLISH_LLAMA_SERVER_PROVIDER.to_string(),
            base_url: crate::polish_engine::local_http::local_base_url(),
            api_key: crate::polish_engine::local_http::local_api_key(),
            server_command: env_string(LOCAL_POLISH_SERVER_COMMAND_ENV),
            server_args: env_args_json(LOCAL_POLISH_SERVER_ARGS_JSON_ENV),
            ready_timeout: env_duration_secs(
                LOCAL_POLISH_READY_TIMEOUT_SECS_ENV,
                LOCAL_POLISH_DEFAULT_READY_TIMEOUT,
            ),
        }
    }

    pub(crate) fn from_settings(
        settings: &crate::commands::settings::LocalPolishRuntimeSettings,
    ) -> Result<Self, String> {
        let base_url = settings.base_url.trim();
        if base_url.is_empty() {
            return Err("Local polish base URL is required".to_string());
        }
        Url::parse(base_url)
            .map_err(|e| format!("Invalid local polish base URL ({base_url}): {e}"))?;

        let server_args = parse_server_args_json(&settings.server_args_json)?;
        let ready_timeout_secs = settings.ready_timeout_secs.max(1);

        Ok(Self {
            provider_type: settings.provider_type.trim().to_string(),
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: none_if_empty(&settings.api_key),
            server_command: none_if_empty(&settings.server_command),
            server_args,
            ready_timeout: Duration::from_secs(ready_timeout_secs),
        })
    }
}

impl Default for LocalPolishRuntimeConfig {
    fn default() -> Self {
        Self::from_env()
    }
}

static LOCAL_POLISH_RUNTIME_CONFIG: OnceLock<RwLock<LocalPolishRuntimeConfig>> = OnceLock::new();

pub(crate) fn current_config() -> LocalPolishRuntimeConfig {
    LOCAL_POLISH_RUNTIME_CONFIG
        .get_or_init(|| RwLock::new(LocalPolishRuntimeConfig::from_env()))
        .read()
        .unwrap()
        .clone()
}

fn set_current_config(config: LocalPolishRuntimeConfig) {
    *LOCAL_POLISH_RUNTIME_CONFIG
        .get_or_init(|| RwLock::new(LocalPolishRuntimeConfig::from_env()))
        .write()
        .unwrap() = config;
}

#[derive(Debug, Default)]
struct LocalPolishRuntimeState {
    child: Option<Child>,
    model_id: Option<String>,
    last_used_at: Option<Instant>,
}

#[derive(Debug)]
pub(crate) struct LocalPolishRuntimeManager {
    config: RwLock<LocalPolishRuntimeConfig>,
    state: Mutex<LocalPolishRuntimeState>,
}

impl LocalPolishRuntimeManager {
    pub(crate) fn new() -> Self {
        let config = current_config();
        Self {
            config: RwLock::new(config),
            state: Mutex::new(LocalPolishRuntimeState::default()),
        }
    }

    #[cfg(test)]
    fn with_config(config: LocalPolishRuntimeConfig) -> Self {
        Self {
            config: RwLock::new(config),
            state: Mutex::new(LocalPolishRuntimeState::default()),
        }
    }

    pub(crate) fn configure(&self, config: LocalPolishRuntimeConfig) -> Result<(), String> {
        socket_endpoint_from_base_url(&config.base_url)?;

        let mut current = self.config.write().unwrap();
        if *current == config {
            set_current_config(config);
            return Ok(());
        }

        set_current_config(config.clone());
        *current = config;
        drop(current);

        let mut state = self.state.lock().unwrap();
        stop_child(&mut state);
        info!("local_polish_runtime_configured");
        Ok(())
    }

    pub(crate) fn configure_from_settings(
        &self,
        settings: &crate::commands::settings::LocalPolishRuntimeSettings,
    ) -> Result<(), String> {
        self.configure(LocalPolishRuntimeConfig::from_settings(settings)?)
    }

    pub(crate) fn ensure_ready(&self, model_id: &str, model_filename: &str) -> Result<(), String> {
        let config = self.config.read().unwrap().clone();
        set_current_config(config.clone());
        let endpoint = socket_endpoint_from_base_url(&config.base_url)?;

        {
            let mut state = self.state.lock().unwrap();
            prune_exited_child(&mut state);

            if state.child.is_some() && state.model_id.as_deref() != Some(model_id) {
                info!(
                    old_model_id = state.model_id.as_deref().unwrap_or("unknown"),
                    new_model_id = model_id,
                    "local_polish_runtime_model_switch"
                );
                stop_child(&mut state);
            }

            if runtime_is_ready(&config, Duration::from_millis(500)) {
                state.last_used_at = Some(Instant::now());
                info!(
                    model_id,
                    managed = state.child.is_some(),
                    base_url = %config.base_url,
                    "local_polish_runtime_ready-existing"
                );
                return Ok(());
            }

            if state.child.is_none() {
                let Some(command) = resolve_server_command(&config) else {
                    warn!(
                        model_id,
                        provider_type = %config.provider_type,
                        base_url = %config.base_url,
                        "local_polish_runtime_unavailable-no_server_command"
                    );
                    return Err(runtime_unavailable_message(&config));
                };

                let model_path = AppPaths::models_dir().join(model_filename);
                let args = build_runtime_args(&config, &endpoint, model_id, &model_path)?;
                info!(
                    model_id,
                    command = %command,
                    args = ?args,
                    base_url = %config.base_url,
                    "local_polish_runtime_starting"
                );
                validate_server_command_for_spawn(&command)?;

                let child = spawn_managed_runtime_child(&command, &args).map_err(|e| {
                    error!(
                        model_id,
                        command,
                        error = %e,
                        "local_polish_runtime_spawn_failed"
                    );
                    format!("Failed to start local polish server ({command}): {e}")
                })?;

                state.child = Some(child);
                state.model_id = Some(model_id.to_string());
                state.last_used_at = Some(Instant::now());
            }
        }

        wait_until_runtime_ready(&config, config.ready_timeout).map_err(|e| {
            error!(
                model_id,
                base_url = %config.base_url,
                error = %e,
                "local_polish_runtime_ready_failed"
            );
            format!(
                "Local polish server did not become ready at {}: {}",
                config.base_url, e
            )
        })?;

        info!(
            model_id,
            base_url = %config.base_url,
            "local_polish_runtime_ready-started"
        );
        let mut state = self.state.lock().unwrap();
        if state.model_id.as_deref() == Some(model_id) {
            state.last_used_at = Some(Instant::now());
        }
        Ok(())
    }

    pub(crate) fn is_ready(&self) -> bool {
        let config = self.config.read().unwrap().clone();
        runtime_is_ready(&config, Duration::from_millis(500))
    }

    pub(crate) fn check_config(
        &self,
        config: LocalPolishRuntimeConfig,
        timeout: Duration,
    ) -> Result<(), String> {
        socket_endpoint_from_base_url(&config.base_url)?;
        check_runtime_health(&config, timeout)
    }

    pub(crate) fn stop(&self) {
        let mut state = self.state.lock().unwrap();
        stop_child(&mut state);
    }

    pub(crate) fn stop_if_idle(&self, idle_for: Duration) -> bool {
        if idle_for.is_zero() {
            return false;
        }

        let mut state = self.state.lock().unwrap();
        prune_exited_child(&mut state);
        if state.child.is_none() {
            return false;
        }

        let Some(last_used_at) = state.last_used_at else {
            return false;
        };
        if last_used_at.elapsed() < idle_for {
            return false;
        }

        stop_child(&mut state);
        true
    }

    pub(crate) fn stop_model(&self, model_id: &str) {
        let mut state = self.state.lock().unwrap();
        if state.model_id.as_deref() == Some(model_id) {
            stop_child(&mut state);
        }
    }

    #[cfg(test)]
    fn has_managed_child_for_test(&self) -> bool {
        self.state.lock().unwrap().child.is_some()
    }
}

impl Default for LocalPolishRuntimeManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for LocalPolishRuntimeManager {
    fn drop(&mut self) {
        let mut state = self.state.lock().unwrap();
        stop_child(&mut state);
    }
}

fn env_string(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn env_args_json(name: &str) -> Option<Vec<String>> {
    let raw = env_string(name)?;
    match parse_server_args_json(&raw) {
        Ok(args) => args,
        Err(e) => {
            warn!(env = name, error = %e, "local_polish_runtime_args_json_invalid");
            None
        }
    }
}

fn none_if_empty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn parse_server_args_json(raw: &str) -> Result<Option<Vec<String>>, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    serde_json::from_str::<Vec<String>>(trimmed)
        .map(Some)
        .map_err(|e| format!("Invalid local polish server args JSON: {e}"))
}

fn env_duration_secs(name: &str, default: Duration) -> Duration {
    env_string(name)
        .and_then(|value| value.parse::<u64>().ok())
        .map(Duration::from_secs)
        .filter(|value| *value > Duration::ZERO)
        .unwrap_or(default)
}

fn build_runtime_args(
    config: &LocalPolishRuntimeConfig,
    endpoint: &SocketEndpoint,
    model_id: &str,
    model_path: &Path,
) -> Result<Vec<String>, String> {
    let args = config.server_args.clone().unwrap_or_else(|| {
        if config.provider_type == LOCAL_POLISH_LLAMA_SERVER_PROVIDER {
            default_llama_server_args()
        } else {
            default_python_args()
        }
    });
    let model_path = model_path
        .to_str()
        .ok_or_else(|| format!("Model path is not valid UTF-8: {}", model_path.display()))?;

    Ok(args
        .into_iter()
        .map(|arg| {
            arg.replace("{model_path}", model_path)
                .replace("{model_id}", model_id)
                .replace("{model_alias}", model_id)
                .replace("{host}", &endpoint.host)
                .replace("{port}", &endpoint.port.to_string())
                .replace("{base_url}", &config.base_url)
        })
        .collect())
}

fn default_llama_server_args() -> Vec<String> {
    [
        "--model",
        "{model_path}",
        "--alias",
        "{model_alias}",
        "--host",
        "{host}",
        "--port",
        "{port}",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

fn resolve_server_command(config: &LocalPolishRuntimeConfig) -> Option<String> {
    if let Some(command) = config.server_command.clone() {
        return Some(command);
    }

    if config.provider_type != LOCAL_POLISH_LLAMA_SERVER_PROVIDER {
        return None;
    }

    let command = find_llama_server_command()?;
    info!(
        command = %command,
        provider_type = %config.provider_type,
        "local_polish_runtime_command_autodetected"
    );
    Some(command)
}

fn runtime_unavailable_message(config: &LocalPolishRuntimeConfig) -> String {
    if config.provider_type == LOCAL_POLISH_LLAMA_SERVER_PROVIDER {
        return format!(
            "Local polish server unavailable at {}. Start an OpenAI-compatible local server, bundle or install llama-server, or configure a local polish runtime command.",
            config.base_url
        );
    }

    format!(
        "Local polish server unavailable at {}. Start an OpenAI-compatible local server or configure a local polish runtime command.",
        config.base_url
    )
}

fn find_llama_server_command() -> Option<String> {
    if let Some(command) = find_bundled_llama_server_command() {
        info!(command = %command, "local_polish_runtime_bundled_command_found");
        return Some(command);
    }

    let path_env = std::env::var_os("PATH")?;
    let command = find_command_on_path(
        llama_server_command_candidates(),
        std::env::split_paths(&path_env),
    )?;
    info!(command = %command, "local_polish_runtime_path_command_found");
    Some(command)
}

fn llama_server_command_candidates() -> &'static [&'static str] {
    if cfg!(windows) {
        &["llama-server.exe"]
    } else {
        &["llama-server"]
    }
}

fn find_bundled_llama_server_command() -> Option<String> {
    find_command_in_bundled_roots(
        llama_server_command_candidates(),
        bundled_runtime_roots(),
        bundled_runtime_subdirs(),
    )
}

fn bundled_runtime_roots() -> Vec<PathBuf> {
    if let Ok(exe) = std::env::current_exe() {
        return bundled_runtime_roots_for_exe(&exe);
    }

    dedupe_paths(vec![PathBuf::from(env!("CARGO_MANIFEST_DIR"))])
}

fn bundled_runtime_roots_for_exe(exe: &Path) -> Vec<PathBuf> {
    let mut roots = Vec::new();

    if let Some(exe_dir) = exe.parent() {
        roots.push(exe_dir.to_path_buf());
        roots.push(exe_dir.join("resources"));

        #[cfg(target_os = "macos")]
        if let Some(contents_dir) = exe_dir.parent() {
            roots.push(contents_dir.join("Resources"));
        }

        if let Some(parent_dir) = exe_dir.parent() {
            roots.push(parent_dir.join("resources"));
        }
    }

    roots.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")));

    dedupe_paths(roots)
}

fn bundled_runtime_subdirs() -> &'static [&'static str] {
    #[cfg(windows)]
    {
        &["bin/windows", "bin", ""]
    }
    #[cfg(target_os = "macos")]
    {
        macos_bundled_runtime_subdirs()
    }
    #[cfg(target_os = "linux")]
    {
        &["bin/linux", "bin", ""]
    }
}

#[cfg(target_os = "macos")]
fn macos_bundled_runtime_subdirs() -> &'static [&'static str] {
    macos_bundled_runtime_subdirs_for_arch(std::env::consts::ARCH)
}

#[cfg(target_os = "macos")]
fn macos_bundled_runtime_subdirs_for_arch(arch: &str) -> &'static [&'static str] {
    match arch {
        "aarch64" => &[
            "bin/apple-silicon",
            "bin/universal",
            "bin/macos",
            "bin",
            "bin/intel",
            "",
        ],
        "x86_64" => &[
            "bin/intel",
            "bin/universal",
            "bin/macos",
            "bin",
            "bin/apple-silicon",
            "",
        ],
        _ => &["bin/universal", "bin/macos", "bin", ""],
    }
}

fn find_command_in_bundled_roots<I, R>(
    candidates: &[&str],
    roots: I,
    subdirs: &[&str],
) -> Option<String>
where
    I: IntoIterator<Item = R>,
    R: AsRef<Path>,
{
    for root in roots {
        let root = root.as_ref();
        for subdir in subdirs {
            let dir = if subdir.is_empty() {
                root.to_path_buf()
            } else {
                root.join(subdir)
            };
            for candidate in candidates {
                let path = dir.join(candidate);
                if path.is_file() {
                    return Some(path.to_string_lossy().to_string());
                }
            }
        }
    }

    None
}

fn find_command_on_path<I, P>(candidates: &[&str], paths: I) -> Option<String>
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
{
    for dir in paths {
        let dir = dir.as_ref();
        for candidate in candidates {
            let path = dir.join(candidate);
            if path.is_file() {
                return Some(path.to_string_lossy().to_string());
            }
        }
    }

    None
}

fn validate_server_command_for_spawn(command: &str) -> Result<(), String> {
    #[cfg(windows)]
    {
        validate_windows_server_command_for_spawn(command)?;
    }
    #[cfg(not(windows))]
    {
        let _ = command;
    }

    Ok(())
}

#[cfg(any(windows, test))]
fn validate_windows_server_command_for_spawn(command: &str) -> Result<(), String> {
    let command = command.trim();
    if !command_looks_like_path(command) {
        return Ok(());
    }

    let path = Path::new(command);
    let metadata = std::fs::metadata(path).map_err(|e| {
        format!(
            "Local polish runtime command path is not accessible ({}): {}",
            path.display(),
            e
        )
    })?;

    if !metadata.is_file() {
        return Err(format!(
            "Local polish runtime command must be an executable file, not a directory: {}",
            path.display()
        ));
    }

    if !has_windows_executable_extension(path) {
        return Err(format!(
            "Local polish runtime command must point to an executable file (.exe, .cmd, .bat, or .com): {}",
            path.display()
        ));
    }

    Ok(())
}

#[cfg(any(windows, test))]
fn command_looks_like_path(command: &str) -> bool {
    let command = command.trim();
    Path::new(command).is_absolute()
        || command.contains('\\')
        || command.contains('/')
        || command.as_bytes().get(1) == Some(&b':')
}

#[cfg(any(windows, test))]
fn has_windows_executable_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "exe" | "cmd" | "bat" | "com"
            )
        })
}

fn spawn_managed_runtime_child(command: &str, args: &[String]) -> std::io::Result<Child> {
    let mut child_command = Command::new(command);
    child_command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    apply_managed_runtime_process_options(&mut child_command);
    child_command.spawn()
}

#[cfg(windows)]
fn apply_managed_runtime_process_options(command: &mut Command) {
    use std::os::windows::process::CommandExt;

    command.creation_flags(windows_create_no_window_flag());
}

#[cfg(not(windows))]
fn apply_managed_runtime_process_options(_command: &mut Command) {}

#[cfg(any(windows, test))]
fn windows_create_no_window_flag() -> u32 {
    // CREATE_NO_WINDOW prevents console-subsystem runtimes such as llama-server.exe
    // from showing a terminal when launched by the GUI app.
    0x0800_0000
}

fn dedupe_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut unique = Vec::new();
    for path in paths {
        if !unique.iter().any(|existing| existing == &path) {
            unique.push(path);
        }
    }
    unique
}

fn default_python_args() -> Vec<String> {
    [
        "-m",
        "llama_cpp.server",
        "--model",
        "{model_path}",
        "--model_alias",
        "{model_alias}",
        "--host",
        "{host}",
        "--port",
        "{port}",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SocketEndpoint {
    host: String,
    port: u16,
}

fn socket_endpoint_from_base_url(base_url: &str) -> Result<SocketEndpoint, String> {
    let parsed = Url::parse(base_url)
        .map_err(|e| format!("Invalid local polish base URL ({base_url}): {e}"))?;
    let host = parsed
        .host_str()
        .ok_or_else(|| format!("Invalid local polish base URL without host: {base_url}"))?
        .to_string();
    let port = parsed
        .port_or_known_default()
        .ok_or_else(|| format!("Invalid local polish base URL without port: {base_url}"))?;

    Ok(SocketEndpoint { host, port })
}

fn runtime_is_ready(config: &LocalPolishRuntimeConfig, timeout: Duration) -> bool {
    check_runtime_health(config, timeout).is_ok()
}

fn check_runtime_health(
    config: &LocalPolishRuntimeConfig,
    timeout: Duration,
) -> Result<(), String> {
    let config = config.clone();
    std::thread::spawn(move || check_runtime_health_on_dedicated_thread(&config, timeout))
        .join()
        .map_err(|_| "local polish health check panicked".to_string())?
}

fn check_runtime_health_on_dedicated_thread(
    config: &LocalPolishRuntimeConfig,
    timeout: Duration,
) -> Result<(), String> {
    let url = crate::polish_engine::local_http::local_models_url(&config.base_url);
    let client = reqwest::blocking::Client::builder()
        .timeout(timeout)
        .build()
        .map_err(|e| format!("failed to build local polish health client: {e}"))?;
    let mut request = client.get(&url).header(
        "User-Agent",
        concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION")),
    );

    if let Some(api_key) = &config.api_key {
        request = request.header("Authorization", format!("Bearer {api_key}"));
    }

    let response = request
        .send()
        .map_err(|e| format!("local polish health check failed at {url}: {e}"))?;
    let status = response.status();

    if status.is_success() {
        Ok(())
    } else {
        Err(format!(
            "local polish health check failed at {url}: HTTP {status}"
        ))
    }
}

fn wait_until_runtime_ready(
    config: &LocalPolishRuntimeConfig,
    timeout: Duration,
) -> Result<(), String> {
    let started = Instant::now();
    loop {
        let last_error = match check_runtime_health(config, Duration::from_millis(500)) {
            Ok(()) => return Ok(()),
            Err(e) => e,
        };

        if started.elapsed() >= timeout {
            return Err(format!(
                "timed out after {}s; last error: {}",
                timeout.as_secs(),
                last_error
            ));
        }

        std::thread::sleep(LOCAL_POLISH_READY_POLL_INTERVAL);
    }
}

fn prune_exited_child(state: &mut LocalPolishRuntimeState) {
    let Some(child) = state.child.as_mut() else {
        return;
    };

    match child.try_wait() {
        Ok(Some(status)) => {
            warn!(status = ?status, "local_polish_runtime_child_exited");
            state.child = None;
            state.model_id = None;
            state.last_used_at = None;
        }
        Ok(None) => {}
        Err(e) => {
            warn!(error = %e, "local_polish_runtime_child_status_failed");
            state.child = None;
            state.model_id = None;
            state.last_used_at = None;
        }
    }
}

fn stop_child(state: &mut LocalPolishRuntimeState) {
    let Some(mut child) = state.child.take() else {
        state.model_id = None;
        return;
    };

    if let Err(e) = child.kill() {
        debug!(error = %e, "local_polish_runtime_child_kill_failed");
    }
    if let Err(e) = child.wait() {
        debug!(error = %e, "local_polish_runtime_child_wait_failed");
    }
    state.model_id = None;
    state.last_used_at = None;
    info!("local_polish_runtime_stopped");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::path::PathBuf;
    use std::thread;

    fn test_config(base_url: String) -> LocalPolishRuntimeConfig {
        LocalPolishRuntimeConfig {
            provider_type: LOCAL_POLISH_LLAMA_SERVER_PROVIDER.to_string(),
            base_url,
            api_key: None,
            server_command: None,
            server_args: None,
            ready_timeout: Duration::from_millis(10),
        }
    }

    fn spawn_models_health_server() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buffer = [0_u8; 1024];
            let _ = stream.read(&mut buffer);
            let body = r#"{"data":[]}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).unwrap();
        });

        format!("http://{}:{}/v1", addr.ip(), addr.port())
    }

    #[test]
    fn parses_socket_endpoint_from_base_url() {
        assert_eq!(
            socket_endpoint_from_base_url("http://127.0.0.1:8000/v1").unwrap(),
            SocketEndpoint {
                host: "127.0.0.1".to_string(),
                port: 8000
            }
        );
        assert_eq!(
            socket_endpoint_from_base_url("https://localhost/v1").unwrap(),
            SocketEndpoint {
                host: "localhost".to_string(),
                port: 443
            }
        );
    }

    #[test]
    fn expands_runtime_args_placeholders() {
        let config = LocalPolishRuntimeConfig {
            provider_type: LOCAL_POLISH_LLAMA_SERVER_PROVIDER.to_string(),
            base_url: "http://127.0.0.1:8000/v1".to_string(),
            api_key: None,
            server_command: Some("python3".to_string()),
            server_args: Some(vec![
                "--model".to_string(),
                "{model_path}".to_string(),
                "--alias={model_alias}".to_string(),
                "--listen={host}:{port}".to_string(),
            ]),
            ready_timeout: Duration::from_secs(1),
        };
        let endpoint = SocketEndpoint {
            host: "127.0.0.1".to_string(),
            port: 8000,
        };

        let args = build_runtime_args(
            &config,
            &endpoint,
            "qwen3.5-0.8b",
            &PathBuf::from("/tmp/model.gguf"),
        )
        .unwrap();

        assert_eq!(
            args,
            vec![
                "--model",
                "/tmp/model.gguf",
                "--alias=qwen3.5-0.8b",
                "--listen=127.0.0.1:8000"
            ]
        );
    }

    #[test]
    fn default_llama_server_args_match_native_server_cli() {
        let config = LocalPolishRuntimeConfig {
            provider_type: LOCAL_POLISH_LLAMA_SERVER_PROVIDER.to_string(),
            base_url: "http://127.0.0.1:8000/v1".to_string(),
            api_key: None,
            server_command: Some("llama-server".to_string()),
            server_args: None,
            ready_timeout: Duration::from_secs(1),
        };
        let endpoint = SocketEndpoint {
            host: "127.0.0.1".to_string(),
            port: 8000,
        };

        let args = build_runtime_args(
            &config,
            &endpoint,
            "qwen3.5-0.8b",
            &PathBuf::from("/tmp/model.gguf"),
        )
        .unwrap();

        assert_eq!(
            args,
            vec![
                "--model",
                "/tmp/model.gguf",
                "--alias",
                "qwen3.5-0.8b",
                "--host",
                "127.0.0.1",
                "--port",
                "8000"
            ]
        );
    }

    #[test]
    fn builds_runtime_config_from_settings() {
        let settings = crate::commands::settings::LocalPolishRuntimeSettings {
            provider_type: "custom".to_string(),
            base_url: " http://127.0.0.1:1234/v1/ ".to_string(),
            api_key: " local-key ".to_string(),
            server_command: " python3 ".to_string(),
            server_args_json: r#"["--model","{model_path}"]"#.to_string(),
            ready_timeout_secs: 0,
        };

        let config = LocalPolishRuntimeConfig::from_settings(&settings).unwrap();

        assert_eq!(config.provider_type, "custom");
        assert_eq!(config.base_url, "http://127.0.0.1:1234/v1");
        assert_eq!(config.api_key.as_deref(), Some("local-key"));
        assert_eq!(config.server_command.as_deref(), Some("python3"));
        assert_eq!(
            config.server_args.unwrap(),
            vec!["--model".to_string(), "{model_path}".to_string()]
        );
        assert_eq!(config.ready_timeout, Duration::from_secs(1));
    }

    #[test]
    fn rejects_invalid_runtime_args_json() {
        let settings = crate::commands::settings::LocalPolishRuntimeSettings {
            server_args_json: "not json".to_string(),
            ..Default::default()
        };

        let err = LocalPolishRuntimeConfig::from_settings(&settings).unwrap_err();

        assert!(err.contains("Invalid local polish server args JSON"));
    }

    #[test]
    fn explicit_runtime_command_takes_priority() {
        let config = LocalPolishRuntimeConfig {
            provider_type: LOCAL_POLISH_LLAMA_SERVER_PROVIDER.to_string(),
            base_url: "http://127.0.0.1:8000/v1".to_string(),
            api_key: None,
            server_command: Some("custom-server".to_string()),
            server_args: None,
            ready_timeout: Duration::from_secs(1),
        };

        assert_eq!(
            resolve_server_command(&config).as_deref(),
            Some("custom-server")
        );
    }

    #[test]
    fn finds_llama_server_on_path_for_default_provider() {
        let dir = tempfile::tempdir().unwrap();
        let binary_name = llama_server_command_candidates()[0];
        let binary_path = dir.path().join(binary_name);
        std::fs::write(&binary_path, "").unwrap();

        let command = find_command_on_path(
            llama_server_command_candidates(),
            std::iter::once(dir.path().to_path_buf()),
        )
        .unwrap();

        assert_eq!(command, binary_path.to_string_lossy());
    }

    #[test]
    fn detects_path_like_runtime_commands() {
        assert!(command_looks_like_path("/usr/local/bin/llama-server"));
        assert!(command_looks_like_path(r"C:\Tools\llama-server.exe"));
        assert!(command_looks_like_path("D:llama-server.exe"));

        assert!(!command_looks_like_path("llama-server"));
        assert!(!command_looks_like_path("python3"));
    }

    #[test]
    fn recognizes_windows_executable_extensions() {
        assert!(has_windows_executable_extension(Path::new(
            "llama-server.exe"
        )));
        assert!(has_windows_executable_extension(Path::new(
            "start-runtime.CMD"
        )));
        assert!(has_windows_executable_extension(Path::new(
            "start-runtime.bat"
        )));
        assert!(has_windows_executable_extension(Path::new("runtime.com")));

        assert!(!has_windows_executable_extension(Path::new("model.gguf")));
        assert!(!has_windows_executable_extension(Path::new(
            "settings.json"
        )));
        assert!(!has_windows_executable_extension(Path::new("llama-server")));
    }

    #[test]
    fn managed_windows_runtime_processes_use_hidden_console_flag() {
        assert_eq!(windows_create_no_window_flag(), 0x0800_0000);
    }

    #[test]
    fn rejects_windows_runtime_command_path_that_is_not_executable() {
        let root = tempfile::tempdir().unwrap();
        let model_path = root.path().join("model.gguf");
        std::fs::write(&model_path, "not an executable").unwrap();

        let err =
            validate_windows_server_command_for_spawn(&model_path.to_string_lossy()).unwrap_err();

        assert!(err.contains("must point to an executable file"));
    }

    #[test]
    fn rejects_windows_runtime_command_path_that_is_directory() {
        let root = tempfile::tempdir().unwrap();

        let err =
            validate_windows_server_command_for_spawn(&root.path().to_string_lossy()).unwrap_err();

        assert!(err.contains("not a directory"));
    }

    #[test]
    fn finds_bundled_llama_server_under_resource_subdir() {
        let root = tempfile::tempdir().unwrap();
        let binary_path = root.path().join("bin/test-runtime/llama-server");
        std::fs::create_dir_all(binary_path.parent().unwrap()).unwrap();
        std::fs::write(&binary_path, "").unwrap();

        let command = find_command_in_bundled_roots(
            &["llama-server"],
            std::iter::once(root.path().to_path_buf()),
            &["bin/test-runtime"],
        )
        .unwrap();

        assert_eq!(command, binary_path.to_string_lossy());
    }

    #[test]
    fn bundled_runtime_roots_include_tauri_resource_locations() {
        let root = tempfile::tempdir().unwrap();
        let exe = root.path().join("app").join("ariatype.exe");

        let roots = bundled_runtime_roots_for_exe(&exe);

        assert!(roots.contains(&root.path().join("app")));
        assert!(roots.contains(&root.path().join("app").join("resources")));
        assert!(roots.contains(&root.path().join("resources")));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_bundled_runtime_subdirs_prefer_current_architecture() {
        assert_eq!(
            macos_bundled_runtime_subdirs_for_arch("aarch64")
                .first()
                .copied(),
            Some("bin/apple-silicon")
        );
        assert!(
            macos_bundled_runtime_subdirs_for_arch("aarch64")
                .iter()
                .position(|path| *path == "bin/apple-silicon")
                < macos_bundled_runtime_subdirs_for_arch("aarch64")
                    .iter()
                    .position(|path| *path == "bin/intel")
        );

        assert_eq!(
            macos_bundled_runtime_subdirs_for_arch("x86_64")
                .first()
                .copied(),
            Some("bin/intel")
        );
        assert!(
            macos_bundled_runtime_subdirs_for_arch("x86_64")
                .iter()
                .position(|path| *path == "bin/intel")
                < macos_bundled_runtime_subdirs_for_arch("x86_64")
                    .iter()
                    .position(|path| *path == "bin/apple-silicon")
        );
    }

    #[test]
    fn non_llama_server_provider_does_not_autodetect_command() {
        let config = LocalPolishRuntimeConfig {
            provider_type: "lm-studio".to_string(),
            base_url: "http://127.0.0.1:1234/v1".to_string(),
            api_key: None,
            server_command: None,
            server_args: None,
            ready_timeout: Duration::from_secs(1),
        };

        assert!(resolve_server_command(&config).is_none());
    }

    #[test]
    fn runtime_unavailable_message_mentions_bundled_llama_server_for_default_provider() {
        let config = LocalPolishRuntimeConfig {
            provider_type: LOCAL_POLISH_LLAMA_SERVER_PROVIDER.to_string(),
            base_url: "http://127.0.0.1:8000/v1".to_string(),
            api_key: None,
            server_command: None,
            server_args: None,
            ready_timeout: Duration::from_secs(1),
        };

        let message = runtime_unavailable_message(&config);

        assert!(message.contains("bundle or install llama-server"));
    }

    #[test]
    fn runtime_unavailable_message_is_provider_neutral_for_custom_runtime() {
        let config = LocalPolishRuntimeConfig {
            provider_type: "custom".to_string(),
            base_url: "http://127.0.0.1:9000/v1".to_string(),
            api_key: None,
            server_command: None,
            server_args: None,
            ready_timeout: Duration::from_secs(1),
        };

        let message = runtime_unavailable_message(&config);

        assert!(!message.contains("llama-server"));
        assert!(message.contains("configure a local polish runtime command"));
    }

    #[test]
    fn treats_existing_listener_as_ready_without_spawn_command() {
        let manager =
            LocalPolishRuntimeManager::with_config(test_config(spawn_models_health_server()));

        assert!(manager
            .ensure_ready("qwen3.5-0.8b", "Qwen3.5-0.8B-Q5_K_M.gguf")
            .is_ok());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn runtime_status_probe_is_safe_inside_async_context() {
        let manager =
            LocalPolishRuntimeManager::with_config(test_config(spawn_models_health_server()));

        assert!(manager.is_ready());
    }

    #[cfg(unix)]
    #[test]
    fn ensure_ready_spawns_configured_runtime_and_waits_for_health() {
        use std::os::unix::fs::PermissionsExt;
        use std::time::Duration;

        let port_probe = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = port_probe.local_addr().unwrap();
        drop(port_probe);

        let root = tempfile::tempdir().unwrap();
        let command_path = root.path().join("fake-llama-server");
        let args_path = root.path().join("spawned.args");
        let args_path_literal = args_path.to_string_lossy().replace('\'', "'\\''");
        std::fs::write(
            &command_path,
            format!(
                "#!/bin/sh\nprintf '%s\\n' \"$@\" > '{}'\nwhile true; do sleep 1; done\n",
                args_path_literal
            ),
        )
        .unwrap();
        let mut permissions = std::fs::metadata(&command_path).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&command_path, permissions).unwrap();

        let health_args_path = args_path.clone();
        thread::spawn(move || {
            for _ in 0..100 {
                if health_args_path.exists() {
                    break;
                }
                thread::sleep(Duration::from_millis(10));
            }

            let listener = TcpListener::bind(addr).unwrap();
            let (mut stream, _) = listener.accept().unwrap();
            let mut buffer = [0_u8; 1024];
            let _ = stream.read(&mut buffer);
            let body = r#"{"data":[]}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).unwrap();
        });

        let config = LocalPolishRuntimeConfig {
            provider_type: LOCAL_POLISH_LLAMA_SERVER_PROVIDER.to_string(),
            base_url: format!("http://{}:{}/v1", addr.ip(), addr.port()),
            api_key: None,
            server_command: Some(command_path.to_string_lossy().to_string()),
            server_args: None,
            ready_timeout: Duration::from_secs(2),
        };
        let manager = LocalPolishRuntimeManager::with_config(config);

        manager
            .ensure_ready("qwen3.5-0.8b", "Qwen3.5-0.8B-Q5_K_M.gguf")
            .unwrap();

        let spawned_args = std::fs::read_to_string(args_path).unwrap();
        assert!(spawned_args.contains("--model\n"));
        assert!(spawned_args.contains("Qwen3.5-0.8B-Q5_K_M.gguf"));
        assert!(spawned_args.contains("--alias\nqwen3.5-0.8b"));
        assert!(spawned_args.contains(&format!("--port\n{}", addr.port())));
    }

    #[test]
    fn reports_missing_runtime_when_no_listener_or_command_exists() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);

        let mut config = test_config(format!("http://{}:{}/v1", addr.ip(), addr.port()));
        config.provider_type = "custom".to_string();
        let manager = LocalPolishRuntimeManager::with_config(config);

        let err = manager
            .ensure_ready("qwen3.5-0.8b", "Qwen3.5-0.8B-Q5_K_M.gguf")
            .unwrap_err();

        assert!(err.contains("Local polish server unavailable"));
        assert!(err.contains("configure a local polish runtime command"));
        assert!(!err.contains("llama-server"));
    }

    #[cfg(unix)]
    #[test]
    fn stop_if_idle_stops_managed_child_after_idle_window() {
        let manager = LocalPolishRuntimeManager::with_config(test_config(
            "http://127.0.0.1:1/v1".to_string(),
        ));
        let child = Command::new("sleep").arg("60").spawn().unwrap();

        {
            let mut state = manager.state.lock().unwrap();
            state.child = Some(child);
            state.model_id = Some("qwen3.5-0.8b".to_string());
            state.last_used_at = Some(Instant::now() - Duration::from_secs(120));
        }

        assert!(manager.stop_if_idle(Duration::from_secs(60)));
        assert!(!manager.has_managed_child_for_test());
    }

    #[cfg(unix)]
    #[test]
    fn stop_if_idle_keeps_recent_managed_child_running() {
        let manager = LocalPolishRuntimeManager::with_config(test_config(
            "http://127.0.0.1:1/v1".to_string(),
        ));
        let child = Command::new("sleep").arg("60").spawn().unwrap();

        {
            let mut state = manager.state.lock().unwrap();
            state.child = Some(child);
            state.model_id = Some("qwen3.5-0.8b".to_string());
            state.last_used_at = Some(Instant::now());
        }

        assert!(!manager.stop_if_idle(Duration::from_secs(60)));
        assert!(manager.has_managed_child_for_test());
    }
}
