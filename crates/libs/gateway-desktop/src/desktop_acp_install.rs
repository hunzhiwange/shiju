//! 在网关主机安装固定白名单中的 ACP 包。
use axum::{
    Json,
    extract::{Path, Query},
    response::{IntoResponse, Response},
};
use config_acp_install::acp_install_supported;
use gateway_desktop_settings::{AcpInstallInstructions, AcpInstallReport};
use gateway_types::ApiError;
use std::sync::Mutex;
use tokio_util::sync::CancellationToken;

struct ActiveInstall {
    agent: String,
    id: Option<String>,
    cancellation: CancellationToken,
}

static INSTALLING: Mutex<Option<ActiveInstall>> = Mutex::new(None);

struct InstallGuard;

impl Drop for InstallGuard {
    fn drop(&mut self) {
        *INSTALLING.lock().unwrap() = None;
    }
}

pub(super) async fn install_get(Path(agent): Path<String>) -> Json<AcpInstallInstructions> {
    Json(AcpInstallInstructions {
        supported: acp_install_supported(&agent),
        shell: if cfg!(windows) { "PowerShell" } else { "POSIX shell" }.into(),
        commands: acp_agent_registry::manual_install_commands(&agent),
    })
}

#[derive(Default, serde::Deserialize)]
pub(super) struct InstallQuery {
    #[serde(default)]
    report: bool,
    install_id: Option<String>,
}

pub(super) async fn install_post(
    Path(agent): Path<String>,
    Query(query): Query<InstallQuery>,
) -> Result<Response, ApiError> {
    if !acp_install_supported(&agent) {
        return Err(ApiError::bad_request("该 ACP 暂无一键安装命令，请使用官方安装器"));
    }
    let cancellation = CancellationToken::new();
    {
        let mut active = INSTALLING.lock().unwrap();
        if active.is_some() {
            return Err(ApiError::bad_request("网关正在安装 ACP，请稍后重试"));
        }
        *active = Some(ActiveInstall {
            agent: agent.clone(),
            id: query.install_id,
            cancellation: cancellation.clone(),
        });
    }
    let _guard = InstallGuard;
    let mut log = acp_agent_registry::InstallLog::default();
    let result = cancellation
        .run_until_cancelled(acp_agent_registry::install_builtin_agent_with_log(&agent, &mut log))
        .await
        .unwrap_or_else(|| Err(gateway_desktop_settings::ACP_INSTALL_CANCELLED.into()));
    let success_message = || format!("{agent} 安装完成，请测试连接；首次使用可能需要登录");
    if query.report {
        let success = result.is_ok();
        let message = result.map_or_else(|error| error, |()| success_message());
        return Ok(Json(AcpInstallReport { success, message, steps: log.steps }).into_response());
    }
    result.map_err(ApiError::bad_request)?;
    Ok(Json(success_message()).into_response())
}

pub(super) async fn install_delete(
    Path(agent): Path<String>,
    Query(query): Query<InstallQuery>,
) -> Json<bool> {
    let active = INSTALLING.lock().unwrap();
    if let Some(install) = active.as_ref()
        && install.agent == agent
        && query.install_id.is_some()
        && install.id == query.install_id
    {
        install.cancellation.cancel();
        return Json(true);
    }
    Json(false)
}

#[cfg(test)]
#[path = "desktop_acp_install_tests.rs"]
mod tests;
