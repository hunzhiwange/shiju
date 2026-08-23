//! 提供审批测试专用状态操作，使非命令行待处理请求可确定性地进入过期分支。

use super::ApprovalManager;

/// 将指定待处理请求的过期时间移到过去；请求不存在时测试应立即失败。
pub(crate) fn expire_non_cli_pending_request_for_tests(
    manager: &ApprovalManager,
    request_id: &str,
) {
    let mut pending = manager.pending_non_cli_requests.lock();
    let row = pending.get_mut(request_id).expect("request row");
    row.expires_at = (chrono::Utc::now() - chrono::Duration::minutes(1)).to_rfc3339();
}
