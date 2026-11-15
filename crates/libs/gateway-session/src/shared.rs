//! 会话 handler 共享类型与作用域辅助函数。
//!
//! 本模块只放置 session handler 之间复用的窄小结构：错误映射、显式目录判断、
//! UI 请求 DTO 和 session scope 解析。保持这些工具集中，可以避免各 handler
//! 对实例目录和存储错误语义做出不一致判断。

use axum::http::HeaderMap;
use serde::Deserialize;
use vw_api_transport::id::TenantScope;
use vw_api_session::session::GatewaySessionCreateBody;

use gateway_types::ApiError;
use gateway_instance::host::InstanceQuery;
use crate::session_host::SessionHost;

use permission_types as permission_next;
use session_info as session;
use storage_types as storage;
use session_port::LifecycleError;


/// 将 session 层错误映射为网关 API 错误。
///
/// # 参数
///
/// * `e` - session 模块返回的错误。
///
/// # 返回值
///
/// 存储 NotFound 映射为 404，其他 session 错误映射为 bad request。
pub fn session_api_error(e: session::Error) -> ApiError {
    match e {
        session::Error::Storage(storage::Error::NotFound(_)) => {
            ApiError::not_found(e.to_string())
        }
        _ => ApiError::bad_request(e.to_string()),
    }
}

/// 将共享会话生命周期错误映射为现有 Gateway 状态码。
pub fn lifecycle_api_error(error: LifecycleError) -> ApiError {
    match &error {
        LifecycleError::SessionNotFound { .. }
        | LifecycleError::SessionOwnershipMismatch { .. }
        | LifecycleError::Session(session::Error::Storage(
            storage::Error::NotFound(_),
        )) => ApiError::not_found(error.to_string()),
        _ => ApiError::bad_request(error.to_string()),
    }
}

/// 判断请求是否显式指定了实例目录。
///
/// # 参数
///
/// * `query` - 查询参数中的目录。
/// * `headers` - 可能包含 `x-directory` 的请求头。
///
/// # 返回值
///
/// 查询参数或请求头任一包含非空目录时返回 `true`。
pub fn has_explicit_directory(query: &InstanceQuery, headers: &HeaderMap) -> bool {
    if query.directory.as_deref().is_some_and(|d| !d.trim().is_empty()) {
        return true;
    }

    headers.get("x-directory").and_then(|v| v.to_str().ok()).is_some_and(|d| !d.trim().is_empty())
}

/// UI 会话列表查询参数。
#[derive(Debug, Deserialize)]
pub struct UiSessionListQuery {
    /// 可选目录过滤。
    pub directory: Option<String>,
    /// 可选项目过滤，涵盖项目的全部工作树。
    #[serde(rename = "projectID")]
    pub project_id: Option<vw_api_transport::ProjectId>,
    /// 是否只返回根会话。
    pub roots: Option<bool>,
    /// 只返回更新时间不早于该时间戳的会话。
    pub start: Option<u64>,
    /// 标题搜索词。
    pub search: Option<String>,
    /// 最大返回数量。
    pub limit: Option<usize>,
}

/// UI 创建会话请求体。
#[derive(Debug, Deserialize)]
pub struct UiSessionCreateBody {
    /// session 创建参数，使用 flatten 兼容网关 DTO。
    #[serde(flatten)]
    pub session: GatewaySessionCreateBody,
    /// 可选权限规则集，创建时写入会话上下文。
    pub permission: Option<permission_next::Ruleset>,
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
pub fn resolve_scope_from_query<H: SessionHost>(
    tenant: &TenantScope,
    query: &InstanceQuery,
    headers: &HeaderMap,
) -> Option<String> {
    let dir = H::resolve_directory(query, headers);
    H::resolve_scope(tenant, Some(&dir), None)
}
