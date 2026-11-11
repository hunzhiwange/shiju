//! 网关浏览器安装入口：白名单与进程内互斥，复用现有认证路由。
use crate::app::agent::gateway::ApiError;
use axum::{Json, extract::Path};
use std::sync::atomic::{AtomicBool, Ordering};

static INSTALLING: AtomicBool = AtomicBool::new(false);
struct InstallGuard;
impl Drop for InstallGuard {
    fn drop(&mut self) {
        INSTALLING.store(false, Ordering::Release);
    }
}

pub(super) async fn install_post(Path(backend): Path<String>) -> Result<Json<String>, ApiError> {
    if !matches!(backend.as_str(), "agent_browser" | "auto") {
        return Err(ApiError::bad_request("该后端需要手工准备服务，请查看安装帮助"));
    }
    if INSTALLING.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire).is_err() {
        return Err(ApiError::bad_request("网关正在安装浏览器，请稍后重试"));
    }
    let _guard = InstallGuard;
    crate::app::agent::tools::browser::installation::install()
        .await
        .map_err(ApiError::bad_request)?;
    Ok(Json("代理浏览器与 Chrome 已安装并验证，可直接使用".into()))
}

#[cfg(test)]
#[path = "desktop_browser_install_tests.rs"]
mod tests;
