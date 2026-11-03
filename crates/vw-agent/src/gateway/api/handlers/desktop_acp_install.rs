//! 在网关主机安装固定白名单中的 ACP 包。
use crate::app::agent::gateway::ApiError;
use axum::{Json, extract::Path};
use std::sync::atomic::{AtomicBool, Ordering};
use vw_config_types::acp_install::acp_install_command;

static INSTALLING: AtomicBool = AtomicBool::new(false);
struct InstallGuard;
impl Drop for InstallGuard {
    fn drop(&mut self) {
        INSTALLING.store(false, Ordering::Release);
    }
}

pub(super) async fn install_post(Path(agent): Path<String>) -> Result<Json<String>, ApiError> {
    if acp_install_command(&agent).is_none() {
        return Err(ApiError::bad_request("该 ACP 暂无一键安装命令，请使用官方安装器"));
    }
    if INSTALLING.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire).is_err() {
        return Err(ApiError::bad_request("网关正在安装 ACP，请稍后重试"));
    }
    let _guard = InstallGuard;
    vw_acp::install_builtin_agent(&agent).await.map_err(ApiError::bad_request)?;
    Ok(Json(format!("{agent} 安装完成，请测试连接；首次使用可能需要登录")))
}

#[cfg(test)]
#[path = "desktop_acp_install_tests.rs"]
mod tests;
