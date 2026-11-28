//! Install built-in recommended MCP servers on the local gateway host.
mod browse;
mod rustcodegraph;

use acp_agent_registry::{InstallLog, InstallProgress, run_installer};
use agent_config::runtime::{get_global_for_tenant_without_env, update_global_for_tenant};
use axum::{
    Extension, Json,
    extract::{ConnectInfo, Path, Query},
    http::{HeaderMap, StatusCode},
};
use gateway_desktop_settings::{
    AcpInstallReport, McpExtension, McpInstallAction, McpInstallInstructions, McpInstallStatus,
};
use gateway_types::ApiError;
use semver::Version;
use serde_json::{Value, json};
use std::{
    net::SocketAddr,
    path::{Path as FilePath, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};
use vw_api_transport::id::TenantScope;

const CHROME_VERSION: &str = "1.14.0";
const CHROME_PACKAGE: &str = "chrome-devtools-mcp";
const CHROME_FLAGS: [&str; 3] = ["--isolated", "--no-usage-statistics", "--no-performance-crux"];
const CUSTOM_SERVER: &str = "已有同名的自定义 MCP 服务器，请使用手动安装命令与配置";
const DOWNGRADE: &str = "已注册的 MCP 版本高于目标版本";

struct Active {
    extension: McpExtension,
    tenant: TenantScope,
    id: String,
    progress: Arc<Mutex<InstallProgress>>,
}

static ACTIVE: Mutex<Option<Active>> = Mutex::new(None);

struct InstallGuard;

impl Drop for InstallGuard {
    fn drop(&mut self) {
        *ACTIVE.lock().unwrap() = None;
    }
}

fn admit(peer: SocketAddr, headers: &HeaderMap, extension: &str) -> Result<McpExtension, ApiError> {
    if !peer.ip().is_loopback()
        || headers.contains_key(axum::http::header::ORIGIN)
        || headers
            .keys()
            .any(|key| key.as_str() == "forwarded" || key.as_str().starts_with("x-forwarded-"))
    {
        return Err(ApiError {
            status: StatusCode::FORBIDDEN,
            message: "MCP 扩展安装需要本机网关".into(),
        });
    }
    McpExtension::from_id(extension).ok_or_else(|| ApiError::bad_request("未知的 MCP 扩展"))
}

#[derive(serde::Deserialize)]
pub(super) struct InstallQuery {
    install_id: String,
}

#[derive(serde::Deserialize)]
pub(super) struct InstallRequest {
    target_version: Option<String>,
}

pub(super) async fn instructions(
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(extension): Path<String>,
) -> Result<Json<McpInstallInstructions>, ApiError> {
    let extension = admit(peer, &headers, &extension)?;
    manual(extension).await.map(Json).map_err(ApiError::bad_request)
}

pub(super) async fn status(
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    Path(extension): Path<String>,
) -> Result<Json<McpInstallStatus>, ApiError> {
    let extension = admit(peer, &headers, &extension)?;
    let current = current_registration(extension, &tenant).await.map_err(ApiError::bad_request)?;
    let path = match install_path(extension) {
        Ok(path) => path,
        Err(warning) => {
            return Ok(Json(McpInstallStatus { warning: Some(warning), ..Default::default() }));
        }
    };
    let registered = match registered_version(current.as_ref(), extension, &path).await {
        Ok(registered) => registered,
        Err(warning) => {
            return Ok(Json(McpInstallStatus { warning: Some(warning), ..Default::default() }));
        }
    };
    // A registered npx launcher is installable on demand regardless of npm's disposable cache.
    let installed = registered.is_some()
        && (extension == McpExtension::ChromeDevTools
            || current
                .as_ref()
                .and_then(|current| current["command"].as_str())
                .is_some_and(|command| FilePath::new(command).is_file()));
    let target = match extension {
        McpExtension::RustCodeGraph => {
            rustcodegraph::latest_release().await.map(|release| release.version)
        }
        McpExtension::ChromeDevTools => Ok(chrome_version()),
        McpExtension::OpenCodeBrowse => {
            browse::latest_release().await.map(|release| release.version)
        }
    };
    Ok(Json(install_status(registered, installed, target)))
}

fn install_status(
    registered: Option<Version>,
    installed: bool,
    target: Result<Version, String>,
) -> McpInstallStatus {
    let installed_version = registered.as_ref().filter(|_| installed).map(Version::to_string);
    let mut status = McpInstallStatus { installed_version, ..Default::default() };
    let target = match target {
        Ok(target) => target,
        Err(warning) => {
            status.warning = Some(warning);
            return status;
        }
    };
    let version = target.to_string();
    if registered.as_ref().is_some_and(|current| current.cmp_precedence(&target).is_gt()) {
        if !installed {
            status.warning = Some(DOWNGRADE.into());
        }
    } else if !installed {
        status.action = Some(McpInstallAction::Install { version });
    } else if registered.is_some_and(|current| current.cmp_precedence(&target).is_lt()) {
        status.action = Some(McpInstallAction::Upgrade { version });
    }
    status
}

pub(super) async fn progress(
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    Path(extension): Path<String>,
    Query(query): Query<InstallQuery>,
) -> Result<Json<Option<InstallProgress>>, ApiError> {
    let extension = admit(peer, &headers, &extension)?;
    let active = ACTIVE.lock().unwrap();
    Ok(Json(
        active
            .as_ref()
            .filter(|active| {
                active.extension == extension
                    && active.tenant == tenant
                    && active.id == query.install_id
            })
            .map(|active| active.progress.lock().unwrap().clone()),
    ))
}

pub(super) async fn start(
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    Path(extension): Path<String>,
    Query(query): Query<InstallQuery>,
    Json(request): Json<InstallRequest>,
) -> Result<Json<AcpInstallReport>, ApiError> {
    let extension = admit(peer, &headers, &extension)?;
    if query.install_id.is_empty() || query.install_id.len() > 80 {
        return Err(ApiError::bad_request("无效的 MCP 安装任务编号"));
    }
    let mut log = InstallLog::default();
    {
        let mut active = ACTIVE.lock().unwrap();
        if active.is_some() {
            return Err(ApiError::bad_request("网关正在安装 MCP 扩展，请稍后重试"));
        }
        *active = Some(Active {
            extension,
            tenant: tenant.clone(),
            id: query.install_id,
            progress: Arc::clone(&log.progress),
        });
    }
    let guard = InstallGuard;
    // Keep ownership of the installation if the HTTP connection is interrupted.
    let report = tokio::spawn(async move {
        let _guard = guard;
        let result = install(extension, &tenant, request.target_version.as_deref(), &mut log).await;
        AcpInstallReport {
            success: result.is_ok(),
            message: result.err().unwrap_or_else(|| "MCP 扩展已安装，请测试连接".into()),
            steps: log.steps,
        }
    })
    .await
    .map_err(|_| ApiError::bad_request("MCP 扩展安装任务中断"))?;
    Ok(Json(report))
}

enum Installer {
    RustCodeGraph(rustcodegraph::Release),
    ChromeDevTools,
    OpenCodeBrowse(browse::Release),
}

fn chrome_version() -> Version {
    Version::parse(CHROME_VERSION).expect("bundled version must be valid")
}

fn chrome_args() -> Vec<String> {
    ["-y".to_owned(), format!("{CHROME_PACKAGE}@{CHROME_VERSION}")]
        .into_iter()
        .chain(CHROME_FLAGS.map(str::to_owned))
        .collect()
}

fn npx_name() -> &'static str {
    if cfg!(windows) { "npx.cmd" } else { "npx" }
}

async fn manual(extension: McpExtension) -> Result<McpInstallInstructions, String> {
    let (version, command, configuration) = match extension {
        McpExtension::RustCodeGraph => {
            let release = rustcodegraph::latest_release().await?;
            let binary = rustcodegraph::binary_at(&rustcodegraph::install_root()?, &release.tag);
            let command = rustcodegraph::manual_command(&binary, &release)?;
            let config = json!({
                "name": extension.id(), "type": "stdio", "command": executable(&binary)?,
                "args": rustcodegraph::ARGS, "enabled": true, "required": false,
            });
            (release.tag, command, config)
        }
        McpExtension::ChromeDevTools => {
            let args = chrome_args();
            let command = format!("{} {} --help", npx_name(), args.join(" "));
            let config = json!({
                "name": extension.id(), "type": "stdio", "command": npx_name(),
                "args": args, "enabled": true, "required": false, "timeout": 60,
            });
            (CHROME_VERSION.to_owned(), command, config)
        }
        McpExtension::OpenCodeBrowse => {
            let release = browse::latest_release().await?;
            let binary = browse::binary_at(&browse::install_root()?, &release.version);
            let command = browse::manual_command(&binary, &release)?;
            let mut config = json!({
                "name": extension.id(), "type": "stdio", "command": executable(&binary)?,
                "args": browse::ARGS, "enabled": true, "required": false,
            });
            browse::set_managed_environment(&mut config)?;
            (release.version.to_string(), command, config)
        }
    };
    Ok(McpInstallInstructions {
        version,
        shell: if cfg!(windows) { "PowerShell" } else { "POSIX shell" }.into(),
        commands: vec![command],
        configuration: serde_json::to_string_pretty(&configuration)
            .map_err(|error| error.to_string())?,
    })
}

fn executable(path: &FilePath) -> Result<&str, String> {
    path.to_str().ok_or_else(|| "安装路径包含无效字符".into())
}

fn install_path(extension: McpExtension) -> Result<PathBuf, String> {
    match extension {
        McpExtension::RustCodeGraph => rustcodegraph::install_root(),
        McpExtension::ChromeDevTools => shell_command::resolve_executable(npx_name())
            .ok_or_else(|| "找不到 npx，请安装 Node.js LTS 和 npm".into()),
        McpExtension::OpenCodeBrowse => browse::install_root(),
    }
}

async fn current_registration(
    extension: McpExtension,
    tenant: &TenantScope,
) -> Result<Option<Value>, String> {
    let config =
        get_global_for_tenant_without_env(tenant).await.map_err(|_| "无法读取 MCP 全局配置")?;
    Ok(config
        .mcp_servers
        .into_iter()
        .find(|entry| entry["name"].as_str().map(str::trim) == Some(extension.id())))
}

fn managed_version(current: &Value, extension: McpExtension, path: &FilePath) -> Option<Version> {
    if current["type"].as_str().unwrap_or("stdio") != "stdio" {
        return None;
    }
    let command = current["command"].as_str()?;
    let version = match extension {
        McpExtension::RustCodeGraph => {
            if current["args"] != json!(rustcodegraph::ARGS) {
                return None;
            }
            let binary = FilePath::new(command);
            let version = binary.parent()?.file_name()?.to_str()?;
            if binary != rustcodegraph::binary_at(path, version).as_path() {
                return None;
            }
            version.strip_prefix('v')?
        }
        McpExtension::ChromeDevTools => {
            if command != npx_name() && FilePath::new(command) != path {
                return None;
            }
            let args = current["args"].as_array()?;
            if args.len() != CHROME_FLAGS.len() + 2
                || args[0] != "-y"
                || !args[2..]
                    .iter()
                    .zip(CHROME_FLAGS)
                    .all(|(value, flag)| value.as_str() == Some(flag))
            {
                return None;
            }
            args[1].as_str()?.strip_prefix(CHROME_PACKAGE)?.strip_prefix('@')?
        }
        McpExtension::OpenCodeBrowse => {
            if current["args"] != json!(browse::ARGS) {
                return None;
            }
            return browse::managed_version(path, FilePath::new(command));
        }
    };
    Version::parse(version).ok()
}

async fn registered_version(
    current: Option<&Value>,
    extension: McpExtension,
    path: &FilePath,
) -> Result<Option<Version>, String> {
    let Some(current) = current else { return Ok(None) };
    if let Some(version) = managed_version(current, extension, path) {
        return Ok(Some(version));
    }
    if extension == McpExtension::OpenCodeBrowse
        && current["type"].as_str().unwrap_or("stdio") == "stdio"
        && current["args"] == json!(browse::ARGS)
        && let Some(binary) = current["command"].as_str().map(FilePath::new)
        && browse::is_official_path(binary)
    {
        return browse::installed_version(binary).await;
    }
    Err(CUSTOM_SERVER.into())
}

async fn install(
    extension: McpExtension,
    tenant: &TenantScope,
    expected_version: Option<&str>,
    log: &mut InstallLog,
) -> Result<(), String> {
    log.step("检查网关平台与安装方式");
    let path = install_path(extension)?;
    let (installer, version) = match extension {
        McpExtension::RustCodeGraph => {
            log.step("查询 RustCodeGraph 最新稳定版");
            let release = rustcodegraph::latest_release().await?;
            log.steps.push(format!("RustCodeGraph {}", release.tag));
            let version = release.version.clone();
            (Installer::RustCodeGraph(release), version)
        }
        McpExtension::ChromeDevTools => (Installer::ChromeDevTools, chrome_version()),
        McpExtension::OpenCodeBrowse => {
            log.step("查询 OpenCode Browse 最新稳定版");
            let release = browse::latest_release().await?;
            let version = release.version.clone();
            (Installer::OpenCodeBrowse(release), version)
        }
    };
    if expected_version.is_some_and(|expected| expected != version.to_string()) {
        return Err("MCP 目标版本已变化，请查看更新后的版本再试".into());
    }
    let current = current_registration(extension, tenant).await?;
    if registered_version(current.as_ref(), extension, &path)
        .await?
        .is_some_and(|installed| installed.cmp_precedence(&version).is_gt())
    {
        return Err(DOWNGRADE.into());
    }

    let mut replacement = current.clone().unwrap_or_else(
        || json!({"name": extension.id(), "type": "stdio", "enabled": true, "required": false}),
    );
    match installer {
        Installer::RustCodeGraph(release) => {
            let binary = rustcodegraph::binary_at(&path, &release.tag);
            replacement["command"] = json!(executable(&binary)?);
            replacement["args"] = json!(rustcodegraph::ARGS);
            install_versioned_binary(
                log,
                binary,
                release,
                rustcodegraph::install_binary,
                "RustCodeGraph 安装任务中断",
            )
            .await?;
        }
        Installer::ChromeDevTools => {
            let command = executable(&path)?;
            let args = chrome_args();
            replacement["command"] = json!(command);
            replacement["args"] = json!(&args);
            if current.is_none() {
                replacement["timeout"] = json!(60);
                if let Some(path) = shell_command::effective_path_env() {
                    replacement["env"] = json!([{"name": "PATH", "value": path}]);
                }
            }
            let mut npm_args = args.iter().map(String::as_str).collect::<Vec<_>>();
            npm_args.push("--help");
            tokio::time::timeout(
                Duration::from_secs(120),
                run_installer(log, "通过 npm 下载并验证 Chrome DevTools MCP", command, &npm_args),
            )
            .await
            .map_err(|_| "Chrome DevTools MCP 下载超时")??;
        }
        Installer::OpenCodeBrowse(release) => {
            let binary = browse::binary_at(&path, &release.version);
            replacement["command"] = json!(executable(&binary)?);
            replacement["args"] = json!(browse::ARGS);
            browse::set_managed_environment(&mut replacement)?;
            install_versioned_binary(
                log,
                binary,
                release,
                browse::install_binary,
                "OpenCode Browse 安装任务中断",
            )
            .await?;
        }
    }

    log.step("保存 MCP 服务器配置");
    let patch = match current {
        Some(previous) if previous == replacement => return Ok(()),
        Some(previous) => {
            json!({"mcpServerToReplace": {"previous": previous, "replacement": replacement}})
        }
        None => json!({"mcpServerToAdd": replacement}),
    };
    update_global_for_tenant(tenant, patch)
        .await
        .map_err(|_| "无法保存 MCP 配置，请检查配置是否在安装期间发生变更".into())
}

async fn install_versioned_binary<R: Send + 'static>(
    log: &mut InstallLog,
    binary: PathBuf,
    release: R,
    install: fn(&FilePath, &R, &mut InstallLog) -> Result<(), String>,
    interrupted: &'static str,
) -> Result<(), String> {
    let progress = Arc::clone(&log.progress);
    let (steps, result) = tokio::task::spawn_blocking(move || {
        let mut binary_log = InstallLog { steps: Vec::new(), progress };
        let result = install(&binary, &release, &mut binary_log);
        (binary_log.steps, result)
    })
    .await
    .map_err(|_| interrupted)?;
    log.steps.extend(steps);
    result
}

#[cfg(test)]
mod tests;
