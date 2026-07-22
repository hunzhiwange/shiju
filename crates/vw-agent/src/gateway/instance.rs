//! Gateway 请求到项目实例上下文的解析与绑定工具。
//!
//! HTTP 请求可以通过查询参数或请求头指定工作目录，本模块负责解析该目录、
//! 初始化项目实例，并提供相对路径规范化能力。目录选择会影响后续文件、
//! 会话和工具执行范围，因此解析过程保持显式且可审计。

use std::path::PathBuf;

use axum::http::HeaderMap;
use serde::Deserialize;

use crate::app::agent::gateway::ApiError;
use crate::app::agent::project;
use crate::app::agent::project::instance;

#[derive(Debug, Deserialize)]
/// Gateway 请求中的实例选择查询参数。
///
/// 目前只接受 `directory`，表示本次请求要绑定的项目实例目录。
pub(crate) struct InstanceQuery {
    /// 请求指定的项目目录；为空时会继续检查请求头或当前进程目录。
    pub(crate) directory: Option<String>,
}

/// 解析请求应使用的项目目录。
///
/// # 参数
///
/// * `query` - URL 查询参数，优先级最高。
/// * `headers` - HTTP 请求头，支持 `x-vibewindow-directory` 作为桌面端传入目录。
///
/// # 返回值
///
/// 返回非空目录字符串。非 wasm 目标默认退回当前进程目录，wasm 目标默认退回 `/`。
pub(crate) fn resolve_directory(query: &InstanceQuery, headers: &HeaderMap) -> String {
    // 查询参数是显式调用意图，优先于请求头中的桌面端上下文。
    if let Some(d) = query.directory.as_deref() {
        if !d.trim().is_empty() {
            return d.to_string();
        }
    }
    // 请求头用于前端统一附带当前工作区，空值不能覆盖后面的安全默认值。
    if let Some(d) =
        headers.get("x-vibewindow-directory").and_then(|v| v.to_str().ok()).map(|s| s.to_string())
    {
        if !d.trim().is_empty() {
            return d;
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        display_directory(std::env::current_dir())
    }
    #[cfg(target_arch = "wasm32")]
    {
        "/".to_string()
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn display_directory(current_dir: std::io::Result<PathBuf>) -> String {
    current_dir.unwrap_or_else(|_| PathBuf::from(".")).to_string_lossy().to_string()
}

fn instance_init() -> Box<dyn FnOnce() -> project::BoxFuture<()> + Send + 'static> {
    Box::new(|| {
        Box::pin(async move {
            let wt = instance::worktree();
            if !wt.trim().is_empty() {
                // 只在已有 worktree 时启动实例，避免空路径被解释成进程当前目录。
                project::instance_bootstrap(PathBuf::from(wt)).await;
            }
        })
    })
}

/// 在指定项目目录中执行一个 Gateway 操作。
///
/// # 参数
///
/// * `directory` - 本次请求绑定的项目目录。
/// * `f` - 在项目实例上下文内运行的异步操作。
///
/// # 返回值
///
/// 返回操作本身的结果。
///
/// # 错误处理
///
/// 实例绑定失败会映射为 `400 Bad Request`；操作内部错误由回调自行返回。
pub(crate) async fn with_instance<T>(
    directory: String,
    f: impl FnOnce() -> project::BoxFuture<Result<T, ApiError>> + Send + 'static,
) -> Result<T, ApiError> {
    let provided =
        project::instance::provide(PathBuf::from(directory), Some(instance_init()), f).await;
    let res = match provided {
        Ok(res) => res,
        Err(e) => return Err(ApiError::bad_request(e.to_string())),
    };
    res
}

pub(crate) async fn load_local_project(project_id: &str) -> Result<project::Info, ApiError> {
    let scope = project::project_scope_for_default_tenant(project_id)
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    project::get(&scope)
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?
        .ok_or_else(|| ApiError::not_found("project not found"))
}

/// 将用户输入路径规范化为相对 `root` 的路径。
///
/// # 参数
///
/// * `root` - 允许访问的根目录。
/// * `input` - 客户端传入的路径，可以是相对路径或位于 `root` 下的绝对路径。
///
/// # 返回值
///
/// 返回以 `/` 分隔的相对路径；当绝对路径不在 `root` 下时返回 `None`。
pub(crate) fn normalize_rel_path(root: &PathBuf, input: &str) -> Option<String> {
    if input.is_empty() || input == "/" || input == "." {
        return Some(String::new());
    }
    let p = PathBuf::from(input);
    if p.is_absolute() {
        if let Ok(stripped) = p.strip_prefix(root) {
            // 绝对路径必须位于实例根目录下，防止客户端借由路径参数越界访问。
            let rel = stripped.to_string_lossy().to_string().replace('\\', "/");
            return Some(rel);
        }
        return None;
    }
    Some(input.trim_start_matches('/').to_string())
}

#[cfg(test)]
#[path = "instance_tests.rs"]
mod instance_tests;
