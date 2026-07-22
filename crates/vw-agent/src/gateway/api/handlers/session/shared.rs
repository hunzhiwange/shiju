//! 会话 handler 共享类型与作用域辅助函数。
//!
//! 本模块只放置 session handler 之间复用的窄小结构：错误映射、显式目录判断、
//! UI 请求 DTO 和 session scope 解析。保持这些工具集中，可以避免各 handler
//! 对实例目录和存储错误语义做出不一致判断。

use axum::http::HeaderMap;
use serde::Deserialize;
use vw_api_types::id::TenantScope;
use vw_api_types::session::GatewaySessionCreateBody;

use crate::app::agent::gateway::ApiError;
use crate::app::agent::gateway::instance::InstanceQuery;
use crate::app::agent::gateway::instance::resolve_directory;
use crate::app::agent::gateway::instance::with_instance;
use crate::app::agent::permission::next as permission_next;
use crate::app::agent::session as agent_session;
use crate::app::agent::storage;
use crate::session::ui_store;

/// 将 session 层错误映射为网关 API 错误。
///
/// # 参数
///
/// * `e` - session 模块返回的错误。
///
/// # 返回值
///
/// 存储 NotFound 映射为 404，其他 session 错误映射为 bad request。
pub(super) fn session_api_error(e: agent_session::session::Error) -> ApiError {
    match e {
        agent_session::session::Error::Storage(storage::Error::NotFound(_)) => {
            ApiError::not_found(e.to_string())
        }
        _ => ApiError::bad_request(e.to_string()),
    }
}

/// 判断请求是否显式指定了实例目录。
///
/// # 参数
///
/// * `query` - 查询参数中的目录。
/// * `headers` - 可能包含 `x-vibewindow-directory` 的请求头。
///
/// # 返回值
///
/// 查询参数或请求头任一包含非空目录时返回 `true`。
pub(super) fn has_explicit_directory(query: &InstanceQuery, headers: &HeaderMap) -> bool {
    if query.directory.as_deref().is_some_and(|d| !d.trim().is_empty()) {
        return true;
    }

    headers
        .get("x-vibewindow-directory")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|d| !d.trim().is_empty())
}

/// UI 会话列表查询参数。
#[derive(Debug, Deserialize)]
pub(super) struct UiSessionListQuery {
    /// 可选目录过滤。
    pub(super) directory: Option<String>,
    /// 是否只返回根会话。
    pub(super) roots: Option<bool>,
    /// 只返回更新时间不早于该时间戳的会话。
    pub(super) start: Option<u64>,
    /// 标题搜索词。
    pub(super) search: Option<String>,
    /// 最大返回数量。
    pub(super) limit: Option<usize>,
}

/// UI 创建会话请求体。
#[derive(Debug, Deserialize)]
pub(super) struct UiSessionCreateBody {
    /// session 创建参数，使用 flatten 兼容网关 DTO。
    #[serde(flatten)]
    pub(super) session: GatewaySessionCreateBody,
    /// 可选权限规则集，创建时写入会话上下文。
    pub(super) permission: Option<permission_next::Ruleset>,
}

/// 从请求解析 UI session scope id。
///
/// # 参数
///
/// * `query` - 实例目录查询。
/// * `headers` - 可携带实例目录 header。
///
/// # 返回值
///
/// 返回当前目录对应的 scope id；无法解析时返回 `None`。
pub(super) fn resolve_scope_from_query(
    tenant: &TenantScope,
    query: &InstanceQuery,
    headers: &HeaderMap,
) -> Option<String> {
    let dir = resolve_directory(query, headers);
    crate::session::ui_store::resolve_session_scope_id_for_tenant(tenant, Some(&dir), None)
}

pub(super) async fn with_tenant_instance<T>(
    tenant: TenantScope,
    directory: String,
    f: impl FnOnce() -> crate::app::agent::project::BoxFuture<Result<T, ApiError>> + Send + 'static,
) -> Result<T, ApiError> {
    crate::app::agent::project::from_directory(&tenant, &directory)
        .await
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    ui_store::with_tenant_scope(tenant, with_instance(directory, f)).await
}

#[cfg(test)]
#[path = "shared_tests.rs"]
mod shared_tests;
