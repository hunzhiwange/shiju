use super::ApprovalManager;

pub(crate) fn expire_non_cli_pending_request_for_tests(
    manager: &ApprovalManager,
    request_id: &str,
) {
    let mut pending = manager.pending_non_cli_requests.lock();
    let row = pending.get_mut(request_id).expect("request row");
    row.expires_at = (chrono::Utc::now() - chrono::Duration::minutes(1)).to_rfc3339();
}
