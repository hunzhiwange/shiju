//! 桌面启动阶段的 CLI 与本机服务引导逻辑。
//!
//! 这里不依赖 gateway，因为它的职责正是确保 gateway 所在的本机服务可用。

#[cfg(not(target_arch = "wasm32"))]
use std::fs;
#[cfg(not(target_arch = "wasm32"))]
use std::io::{Read, Write};
#[cfg(not(target_arch = "wasm32"))]
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
#[cfg(not(target_arch = "wasm32"))]
use std::path::{Path, PathBuf};
#[cfg(not(target_arch = "wasm32"))]
use std::process::Command;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;
#[cfg(all(not(target_arch = "wasm32"), any(not(windows), test)))]
use std::time::Instant;
#[cfg(not(target_arch = "wasm32"))]
use vw_config_types::gateway::DEFAULT_GATEWAY_PORT;
#[cfg(not(target_arch = "wasm32"))]
use vw_config_types::ui::{GatewayClientServerConfig, GatewayClientSystemSettingsConfig};
#[cfg(not(target_arch = "wasm32"))]
use vw_gateway_client::vw_api_types::id::TenantScope;

#[cfg(all(not(target_arch = "wasm32"), target_os = "macos"))]
const MACOS_SERVICE_LABEL: &str = "com.vibewindow.daemon";
#[cfg(not(target_arch = "wasm32"))]
const GATEWAY_HEALTH_PATH: &str = "/v1/health";
#[cfg(not(target_arch = "wasm32"))]
const GATEWAY_HEALTH_CONNECT_TIMEOUT: Duration = Duration::from_millis(250);
#[cfg(not(target_arch = "wasm32"))]
const GATEWAY_STARTUP_POLL_INTERVAL: Duration = Duration::from_millis(200);
#[cfg(not(target_arch = "wasm32"))]
const GATEWAY_STARTUP_TIMEOUT: Duration = Duration::from_secs(10);
#[cfg(all(not(target_arch = "wasm32"), any(windows, test)))]
#[path = "config_cli_bootstrap_windows.rs"]
mod windows_bootstrap;

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone, PartialEq, Eq)]
struct GatewayRuntimeEndpoint {
    host: String,
    port: u16,
    tenant_scope: TenantScope,
}

#[cfg(not(target_arch = "wasm32"))]
impl GatewayRuntimeEndpoint {
    fn new(host: impl Into<String>, port: u16) -> Self {
        let host = host.into();
        let host = if host.trim().is_empty() { "127.0.0.1".to_string() } else { host };
        Self { host, port: port.max(1), tenant_scope: TenantScope::default() }
    }

    fn with_tenant_scope(mut self, tenant_scope: TenantScope) -> Self {
        self.tenant_scope = tenant_scope;
        self
    }

    fn connect_host(&self) -> String {
        match self.host.trim().trim_matches(|ch| ch == '[' || ch == ']') {
            "" | "0.0.0.0" => "127.0.0.1".to_string(),
            "::" => "::1".to_string(),
            host => host.to_string(),
        }
    }

    fn describe(&self) -> String {
        format!("{}:{}", self.connect_host(), self.port)
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn bootstrap_cli_service_async() -> Result<(), String> {
    tokio::task::spawn_blocking(|| {
        #[cfg(windows)]
        windows_bootstrap::record_startup("begin Windows direct-daemon-v3 log-failure-nonfatal");
        let result = bootstrap_cli_service();
        #[cfg(windows)]
        match &result {
            Ok(()) => windows_bootstrap::record_startup("ready"),
            Err(error) => windows_bootstrap::record_startup(&format!("failed: {error}")),
        }
        result
    })
        .await
        .map_err(|err| format!("CLI 服务引导任务失败: {err}"))?
}

#[cfg(not(target_arch = "wasm32"))]
fn bootstrap_cli_service() -> Result<(), String> {
    // 开发环境由开发者运行 gateway，不能安装 CLI 或注册常驻服务抢占端口。
    if cfg!(debug_assertions) {
        tracing::debug!(target: "vw_desktop", "development build uses a manually started gateway");
        return Ok(());
    }

    let home = resolve_home_dir()?;
    let config_dir = vw_config_types::paths::home_config_dir(&home);
    let cli = ensure_cli_installed(&home)?;
    let cli_build_id = vw_shared::build_identity::executable_build_id(&cli)
        .map_err(|error| format!("计算 CLI 构建标识失败 {}: {error}", cli.display()))?;
    let endpoint = sync_gateway_client_bootstrap_from_runtime_config(&home)?;
    #[cfg(windows)]
    return windows_bootstrap::bootstrap(&cli, &config_dir, &endpoint, &cli_build_id);

    #[cfg(not(windows))]
    {
        let installed = service_installed(&home);
        let health = gateway_health(&endpoint);
        let running = service_running(&home);
        let running_build_id = health
            .as_ref()
            .and_then(|body| body.get("build_id"))
            .and_then(serde_json::Value::as_str);
        let service_outdated =
            installed && running && running_build_id != Some(cli_build_id.as_str());

        if service_install_required(service_outdated, installed) {
            if service_outdated {
                tracing::info!(
                    target: "vw_desktop",
                    running_build_id = ?running_build_id,
                    cli_build_id = %cli_build_id,
                    "local service is unavailable or uses a different build; reinstalling the local service"
                );
            }
            run_cli_command(&cli, &config_dir, &["service", "install"])?;
        }

        if service_outdated || !running || health.is_none() {
            run_cli_command(&cli, &config_dir, &["service", "start"])?;
        }

        wait_for_gateway_build(&endpoint, &cli_build_id, GATEWAY_STARTUP_TIMEOUT)
            .then_some(())
            .ok_or_else(|| format!("Gateway 服务启动后构建版本未就绪: {}", endpoint.describe()))
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn ensure_cli_installed(home: &Path) -> Result<PathBuf, String> {
    let install_dir = vw_config_types::paths::home_config_dir(home).join("bin");
    let target_bin = install_dir.join(cli_binary_name());
    let source_bin = resolve_agent_binary_path()
        .map_err(|error| format!("{error}。请重新安装包含 shiju 的完整安装包。"))?;
    if installed_cli_matches_source(&target_bin, &source_bin) {
        return Ok(target_bin);
    }

    fs::create_dir_all(&install_dir).map_err(|err| format!("创建 CLI 安装目录失败: {err}"))?;
    // Windows 不允许替换仍被常驻服务占用的可执行文件。
    #[cfg(windows)]
    if target_bin.is_file() {
        run_cli_command(
            &target_bin,
            &vw_config_types::paths::home_config_dir(home),
            &["service", "stop"],
        )?;
    }
    install_cli_from_source(&source_bin, &target_bin)?;
    let _ = ensure_path_profiles(home, &install_dir).map_err(|err| {
        tracing::warn!(target: "vw_desktop", error = %err, "failed to persist CLI PATH profile");
        err
    });
    verify_cli_binary(&target_bin)?;
    Ok(target_bin)
}

#[cfg(not(target_arch = "wasm32"))]
fn install_cli_from_source(source: &Path, target: &Path) -> Result<(), String> {
    let file_name = target
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| format!("CLI 安装路径无效: {}", target.display()))?;
    let staged = target.with_file_name(format!(".{file_name}.installing-{}", std::process::id()));
    let _ = fs::remove_file(&staged);

    let install_result = (|| {
        fs::copy(source, &staged).map_err(|err| {
            format!("复制 CLI 可执行文件失败: {} -> {}: {err}", source.display(), staged.display())
        })?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&staged, fs::Permissions::from_mode(0o755))
                .map_err(|err| format!("设置 CLI 临时文件权限失败: {err}"))?;
        }

        verify_cli_binary(&staged)?;
        replace_staged_cli(&staged, target)
    })();

    if install_result.is_err() {
        let _ = fs::remove_file(&staged);
    }
    install_result
}

#[cfg(all(not(target_arch = "wasm32"), not(windows)))]
fn replace_staged_cli(staged: &Path, target: &Path) -> Result<(), String> {
    fs::rename(staged, target)
        .map_err(|err| format!("原子替换 CLI 可执行文件失败 {}: {err}", target.display()))
}

#[cfg(all(not(target_arch = "wasm32"), windows))]
fn replace_staged_cli(staged: &Path, target: &Path) -> Result<(), String> {
    if target.exists() {
        fs::remove_file(target)
            .map_err(|err| format!("移除旧 CLI 可执行文件失败 {}: {err}", target.display()))?;
    }
    fs::rename(staged, target)
        .map_err(|err| format!("替换 CLI 可执行文件失败 {}: {err}", target.display()))
}

#[cfg(all(not(target_arch = "wasm32"), any(not(windows), test)))]
fn service_install_required(service_outdated: bool, service_installed: bool) -> bool {
    service_outdated || !service_installed
}

#[cfg(not(target_arch = "wasm32"))]
fn verify_cli_binary(path: &Path) -> Result<(), String> {
    cli_version(path).map(|_| ())
}

#[cfg(not(target_arch = "wasm32"))]
fn installed_cli_matches_source(target: &Path, source: &Path) -> bool {
    let Ok(target_version) = cli_version(target) else {
        return false;
    };
    let Ok(source_version) = cli_version(source) else {
        return false;
    };
    target_version == source_version && same_file_contents(target, source).unwrap_or(false)
}

#[cfg(not(target_arch = "wasm32"))]
fn same_file_contents(left: &Path, right: &Path) -> Result<bool, String> {
    let left_meta = fs::metadata(left)
        .map_err(|err| format!("读取文件元数据失败 {}: {err}", left.display()))?;
    let right_meta = fs::metadata(right)
        .map_err(|err| format!("读取文件元数据失败 {}: {err}", right.display()))?;
    if left_meta.len() != right_meta.len() {
        return Ok(false);
    }

    let left_bytes =
        fs::read(left).map_err(|err| format!("读取文件失败 {}: {err}", left.display()))?;
    let right_bytes =
        fs::read(right).map_err(|err| format!("读取文件失败 {}: {err}", right.display()))?;
    Ok(left_bytes == right_bytes)
}

/// 后台引导命令不应在 Windows 桌面启动期间创建控制台窗口。
#[cfg(not(target_arch = "wasm32"))]
fn background_command(program: impl AsRef<std::ffi::OsStr>) -> Command {
    let command = Command::new(program);
    #[cfg(windows)]
    let command = {
        use std::os::windows::process::CommandExt;
        let mut command = command;
        command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        command
    };
    command
}

#[cfg(not(target_arch = "wasm32"))]
fn cli_version(path: &Path) -> Result<String, String> {
    let output = background_command(path)
        .arg("--version")
        .output()
        .map_err(|err| format!("执行 {} --version 失败: {err}", path.display()))?;
    if !output.status.success() {
        return Err(command_failure_message(path, &["--version"], output));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn run_cli_command(cli: &Path, config_dir: &Path, args: &[&str]) -> Result<String, String> {
    let output = background_command(cli)
        .arg("--config-dir")
        .arg(config_dir)
        .args(args)
        .output()
        .map_err(|err| format!("执行 {} 失败: {err}", format_cli_command(cli, args)))?;
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        return Ok(stdout);
    }
    Err(command_failure_message(cli, args, output))
}

#[cfg(not(target_arch = "wasm32"))]
fn command_failure_message(cli: &Path, args: &[&str], output: std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let detail = if stderr.is_empty() { stdout } else { stderr };
    if detail.is_empty() {
        format!("{} 退出状态异常: {}", format_cli_command(cli, args), output.status)
    } else {
        format!("{} 失败: {detail}", format_cli_command(cli, args))
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn format_cli_command(cli: &Path, args: &[&str]) -> String {
    let mut command = cli.display().to_string();
    for arg in args {
        command.push(' ');
        command.push_str(arg);
    }
    command
}

#[cfg(not(target_arch = "wasm32"))]
fn sync_gateway_client_bootstrap_from_runtime_config(
    home: &Path,
) -> Result<GatewayRuntimeEndpoint, String> {
    let endpoint = load_gateway_runtime_endpoint(home)?;
    let mut cfg = super::system_settings::load_gateway_client_bootstrap_config();
    let active = cfg.active_server();

    if !should_refresh_gateway_client_bootstrap(&active, &endpoint) {
        return Ok(endpoint);
    }

    let mut next = active.clone();
    if next.id.trim().is_empty() {
        next.id = "local".to_string();
    }
    if next.name.trim().is_empty() {
        next.name = "本地网关".to_string();
    }
    next.host = endpoint.host.clone();
    next.port = endpoint.port;

    let mut servers = cfg.normalized_servers();
    let mut replaced = false;
    for server in &mut servers {
        if server.id == next.id {
            *server = next.clone();
            replaced = true;
        } else if server.id == "local" && is_local_gateway_host(&server.host) {
            server.host = endpoint.host.clone();
            server.port = endpoint.port;
        }
    }
    if !replaced {
        servers.insert(0, next.clone());
    }

    cfg.set_servers(servers, next.id);
    super::system_settings::save_gateway_client_bootstrap_config(&cfg);
    Ok(endpoint)
}

#[cfg(not(target_arch = "wasm32"))]
fn should_refresh_gateway_client_bootstrap(
    active: &GatewayClientServerConfig,
    endpoint: &GatewayRuntimeEndpoint,
) -> bool {
    if !is_local_gateway_host(&active.host) {
        return false;
    }

    let tenant_scope = TenantScope::new(active.platform_id.clone(), active.company_id.clone())
        .expect("gateway client tenant IDs are validated when loading bootstrap config");
    let active_endpoint = GatewayRuntimeEndpoint::new(active.host.clone(), active.port)
        .with_tenant_scope(tenant_scope);
    active_endpoint != *endpoint && !gateway_health_ready(&active_endpoint)
}

#[cfg(not(target_arch = "wasm32"))]
fn load_gateway_runtime_endpoint(home: &Path) -> Result<GatewayRuntimeEndpoint, String> {
    let config_dir = vw_config_types::paths::home_config_dir(home);
    let config_path = config_dir.join("gateway-client-bootstrap.json");
    let config = fs::read_to_string(config_path)
        .ok()
        .and_then(|content| {
            serde_json::from_str::<GatewayClientSystemSettingsConfig>(&content).ok()
        })
        .unwrap_or_default();
    let servers = config.normalized_servers();
    let local = servers
        .iter()
        .find(|server| server.id == "local" && is_local_gateway_host(&server.host))
        .or_else(|| servers.iter().find(|server| is_local_gateway_host(&server.host)))
        .cloned()
        .unwrap_or_default();
    let port = if local.port == 0 { DEFAULT_GATEWAY_PORT } else { local.port };
    let tenant_scope = TenantScope::new(local.platform_id, local.company_id)
        .expect("gateway client tenant IDs are validated when loading bootstrap config");
    Ok(GatewayRuntimeEndpoint::new(local.host, port).with_tenant_scope(tenant_scope))
}

#[cfg(not(target_arch = "wasm32"))]
fn is_local_gateway_host(host: &str) -> bool {
    matches!(
        host.trim().trim_matches(|ch| ch == '[' || ch == ']').to_ascii_lowercase().as_str(),
        "" | "127.0.0.1" | "localhost" | "::1" | "0.0.0.0" | "::"
    )
}

#[cfg(not(target_arch = "wasm32"))]
fn gateway_health_ready(endpoint: &GatewayRuntimeEndpoint) -> bool {
    gateway_health(endpoint).is_some()
}

#[cfg(all(not(target_arch = "wasm32"), any(not(windows), test)))]
fn gateway_build_id(endpoint: &GatewayRuntimeEndpoint) -> Option<String> {
    gateway_health(endpoint)?
        .get("build_id")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
}

#[cfg(all(not(target_arch = "wasm32"), any(not(windows), test)))]
fn wait_for_gateway_build(
    endpoint: &GatewayRuntimeEndpoint,
    expected_build_id: &str,
    timeout: Duration,
) -> bool {
    let started = Instant::now();
    while started.elapsed() < timeout {
        if gateway_build_id(endpoint).as_deref() == Some(expected_build_id) {
            return true;
        }
        std::thread::sleep(GATEWAY_STARTUP_POLL_INTERVAL);
    }
    false
}

#[cfg(not(target_arch = "wasm32"))]
fn gateway_health(endpoint: &GatewayRuntimeEndpoint) -> Option<serde_json::Value> {
    let address = match resolve_gateway_socket_address(endpoint) {
        Some(address) => address,
        None => return None,
    };

    let mut stream = match TcpStream::connect_timeout(&address, GATEWAY_HEALTH_CONNECT_TIMEOUT) {
        Ok(stream) => stream,
        Err(_) => return None,
    };
    let _ = stream.set_read_timeout(Some(GATEWAY_HEALTH_CONNECT_TIMEOUT));
    let _ = stream.set_write_timeout(Some(GATEWAY_HEALTH_CONNECT_TIMEOUT));

    let request = format!(
        "GET {GATEWAY_HEALTH_PATH} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        endpoint.describe(),
    );
    if stream.write_all(request.as_bytes()).is_err() {
        return None;
    }

    let mut response = String::new();
    if stream.read_to_string(&mut response).is_err() {
        return None;
    }

    let Some(status_line) = response.lines().next() else {
        return None;
    };
    if !status_line.contains(" 200 ") {
        return None;
    }
    let (_, body) = response.split_once("\r\n\r\n")?;
    let body = serde_json::from_str::<serde_json::Value>(body).ok()?;
    (body.get("status").and_then(serde_json::Value::as_str) == Some("ok")).then_some(body)
}

#[cfg(not(target_arch = "wasm32"))]
fn resolve_gateway_socket_address(endpoint: &GatewayRuntimeEndpoint) -> Option<SocketAddr> {
    format!("{}:{}", endpoint.connect_host(), endpoint.port)
        .to_socket_addrs()
        .ok()?
        .find(|addr| addr.is_ipv4() || addr.is_ipv6())
}

#[cfg(all(not(target_arch = "wasm32"), not(windows)))]
fn service_installed(_home: &Path) -> bool {
    #[cfg(target_os = "macos")]
    {
        return _home
            .join("Library")
            .join("LaunchAgents")
            .join(format!("{MACOS_SERVICE_LABEL}.plist"))
            .exists();
    }

    #[cfg(target_os = "linux")]
    {
        return _home
            .join(".config")
            .join("systemd")
            .join("user")
            .join("vibewindow.service")
            .exists()
            || Path::new("/etc/init.d/vibewindow").exists();
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        let _ = _home;
        false
    }
}

#[cfg(all(not(target_arch = "wasm32"), not(target_os = "windows")))]
fn service_running(home: &Path) -> bool {
    #[cfg(target_os = "macos")]
    {
        let _ = home;
        return background_command("launchctl")
            .arg("list")
            .output()
            .ok()
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .is_some_and(|text| text.lines().any(|line| line.contains(MACOS_SERVICE_LABEL)));
    }

    #[cfg(target_os = "linux")]
    {
        let systemd_active = background_command("systemctl")
            .args(["--user", "is-active", "--quiet", "vibewindow.service"])
            .status()
            .is_ok_and(|status| status.success());
        if systemd_active {
            return true;
        }
        if Path::new("/etc/init.d/vibewindow").exists() {
            return background_command("rc-service")
                .args(["vibewindow", "status"])
                .status()
                .is_ok_and(|status| status.success());
        }
        let _ = home;
        false
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        let _ = home;
        false
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn resolve_home_dir() -> Result<PathBuf, String> {
    if let Some(user_dirs) = directories::UserDirs::new() {
        return Ok(user_dirs.home_dir().to_path_buf());
    }
    if let Ok(home) = std::env::var("HOME")
        && !home.trim().is_empty()
    {
        return Ok(PathBuf::from(home));
    }
    if let Ok(home) = std::env::var("USERPROFILE")
        && !home.trim().is_empty()
    {
        return Ok(PathBuf::from(home));
    }
    Err("无法定位用户主目录".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn cli_binary_name() -> &'static str {
    if cfg!(windows) { "shiju.exe" } else { "shiju" }
}

#[cfg(not(target_arch = "wasm32"))]
fn resolve_agent_binary_path() -> Result<PathBuf, String> {
    let current_exe =
        std::env::current_exe().map_err(|err| format!("获取当前可执行文件失败: {err}"))?;
    let exe_dir = current_exe.parent().ok_or_else(|| "无法定位当前可执行文件目录".to_string())?;
    let agent_names =
        if cfg!(windows) { ["shiju.exe", "vibe-agent.exe"] } else { ["shiju", "vibe-agent"] };
    let candidates = agent_names.iter().map(|name| exe_dir.join(name)).collect::<Vec<_>>();

    for candidate in candidates {
        if candidate.exists() && candidate.is_file() {
            return Ok(candidate);
        }
    }
    Err("未找到内置 CLI 可执行文件 shiju（兼容名 vibe-agent）".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn ensure_path_profiles(home: &Path, install_dir: &Path) -> Result<Vec<String>, String> {
    #[cfg(windows)]
    let profiles = vec![
        home.join("Documents").join("WindowsPowerShell").join("Microsoft.PowerShell_profile.ps1"),
        home.join("Documents").join("PowerShell").join("Microsoft.PowerShell_profile.ps1"),
    ];
    #[cfg(not(windows))]
    let profiles = vec![home.join(".zshrc"), home.join(".bashrc"), home.join(".profile")];

    let mut updated = Vec::new();
    for profile in profiles {
        if ensure_profile_contains_path(&profile, install_dir)? {
            updated.push(profile.display().to_string());
        }
    }
    Ok(updated)
}

#[cfg(not(target_arch = "wasm32"))]
fn ensure_profile_contains_path(profile: &Path, install_dir: &Path) -> Result<bool, String> {
    #[cfg(windows)]
    let export_line = format!(r#"$env:Path = "{};$env:Path""#, install_dir.display());
    #[cfg(not(windows))]
    let export_line = format!("export PATH={}:$PATH", install_dir.display());

    if let Some(parent) = profile.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("创建配置文件目录失败 {}: {err}", parent.display()))?;
    }

    let install_path_text = install_dir.to_string_lossy();
    let existing = fs::read_to_string(profile).unwrap_or_default();
    if existing.contains(export_line.as_str()) || existing.contains(install_path_text.as_ref()) {
        return Ok(false);
    }

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(profile)
        .map_err(|err| format!("打开配置文件失败 {}: {err}", profile.display()))?;

    if !existing.ends_with('\n') && !existing.is_empty() {
        file.write_all(b"\n")
            .map_err(|err| format!("写入配置文件失败 {}: {err}", profile.display()))?;
    }
    file.write_all(export_line.as_bytes())
        .map_err(|err| format!("写入配置文件失败 {}: {err}", profile.display()))?;
    file.write_all(b"\n")
        .map_err(|err| format!("写入配置文件失败 {}: {err}", profile.display()))?;
    Ok(true)
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[cfg(unix)]
    fn write_test_cli(path: &Path, version: &str, marker: &str) {
        use std::os::unix::fs::PermissionsExt;

        fs::write(path, format!("#!/bin/sh\n# {marker}\nprintf '%s\\n' '{version}'\n")).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn installed_cli_matches_source_requires_same_version() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("target-cli");
        let source = tmp.path().join("source-cli");
        write_test_cli(&target, "vibewindow 1.0.0", "same-build");
        write_test_cli(&source, "vibewindow 1.0.0", "same-build");

        assert!(installed_cli_matches_source(&target, &source));

        write_test_cli(&target, "vibewindow 1.0.0", "old-build");

        assert!(!installed_cli_matches_source(&target, &source));

        write_test_cli(&target, "vibewindow 0.9.0", "same-build");

        assert!(!installed_cli_matches_source(&target, &source));
    }

    #[test]
    fn load_gateway_runtime_endpoint_reads_bootstrap_cache() {
        let tmp = tempfile::tempdir().unwrap();
        let config_dir = vw_config_types::paths::home_config_dir(tmp.path());
        fs::create_dir_all(&config_dir).unwrap();
        let config = GatewayClientSystemSettingsConfig {
            host: "0.0.0.0".to_string(),
            port: 14682,
            servers: Vec::new(),
            ..GatewayClientSystemSettingsConfig::default()
        };
        fs::write(
            config_dir.join("gateway-client-bootstrap.json"),
            serde_json::to_string(&config).unwrap(),
        )
        .unwrap();

        let endpoint = load_gateway_runtime_endpoint(tmp.path()).unwrap();

        assert_eq!(endpoint.host, "0.0.0.0");
        assert_eq!(endpoint.connect_host(), "127.0.0.1");
        assert_eq!(endpoint.port, 14682);
    }

    #[test]
    fn refresh_bootstrap_keeps_remote_gateway_client_server() {
        let active = GatewayClientServerConfig {
            host: "gateway.internal".to_string(),
            port: 53607,
            ..GatewayClientServerConfig::default()
        };
        let endpoint = GatewayRuntimeEndpoint::new("127.0.0.1", 42617);

        assert!(!should_refresh_gateway_client_bootstrap(&active, &endpoint));
    }

    #[test]
    fn refresh_bootstrap_replaces_unhealthy_loopback_server() {
        let active = GatewayClientServerConfig {
            host: "127.0.0.1".to_string(),
            port: 1,
            ..GatewayClientServerConfig::default()
        };
        let endpoint = GatewayRuntimeEndpoint::new("127.0.0.1", 42617);

        assert!(should_refresh_gateway_client_bootstrap(&active, &endpoint));
    }

    #[test]
    fn gateway_health_probe_omits_tenant_headers() {
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = Vec::new();
            let mut buffer = [0_u8; 512];
            while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                let read = stream.read(&mut buffer).unwrap();
                if read == 0 {
                    break;
                }
                request.extend_from_slice(&buffer[..read]);
            }
            stream
                .write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Length: 15\r\nConnection: close\r\n\r\n{\"status\":\"ok\"}",
                )
                .unwrap();
            String::from_utf8(request).unwrap()
        });
        let endpoint = GatewayRuntimeEndpoint::new("127.0.0.1", port).with_tenant_scope(
            vw_gateway_client::vw_api_types::id::TenantScope::new("20000", "200000001").unwrap(),
        );

        assert!(gateway_health_ready(&endpoint));
        let request = server.join().unwrap();
        assert!(!request.contains("X-Platform-ID:"));
        assert!(!request.contains("X-Company-ID:"));
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "config_cli_bootstrap_tests.rs"]
mod config_cli_bootstrap_tests;
