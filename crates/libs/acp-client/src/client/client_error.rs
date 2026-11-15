//! ACP 客户端回调错误映射。

use super::*;

/// 将客户端侧错误映射为 ACP 错误并维护权限统计。
///
/// 权限拒绝和权限提示不可用会分别计入 denied/cancelled，其余错误作为内部错误
/// 返回给代理。
pub(super) fn map_client_error(
    err: Box<dyn StdError + Send + Sync + 'static>,
    permission_stats: &Arc<Mutex<PermissionStats>>,
) -> acp::Error {
    if err.is::<crate::errors::PermissionDeniedError>() {
        permission_stats.lock().denied += 1;
    } else if err.is::<crate::errors::PermissionPromptUnavailableError>() {
        permission_stats.lock().cancelled += 1;
    }
    acp::Error::internal_error().data(err.to_string())
}

/// 构造 ACP 权限请求的取消响应。
///
/// 该响应用于本地取消中的会话，避免代理继续等待权限确认。
pub(super) fn cancelled_permission_response() -> acp::RequestPermissionResponse {
    serde_json::from_value(serde_json::json!({
        "outcome": {
            "outcome": "cancelled"
        }
    }))
    .expect("valid ACP permission cancellation response")
}
