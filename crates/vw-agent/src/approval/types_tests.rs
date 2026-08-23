//! 测试审批领域类型对决议、摘要和请求身份字段的保留。
//! 往返序列化锁定审计和待审批记录恢复时所需的全部身份信息。

use super::{
    ApprovalLogEntry, ApprovalResponse, PendingApprovalError, PendingNonCliApprovalRequest,
};

#[test]
fn approval_log_entry_preserves_decision_and_summary() {
    let entry = ApprovalLogEntry {
        timestamp: "2026-05-24T00:00:00Z".to_string(),
        tool_name: "shell".to_string(),
        arguments_summary: "command: pwd".to_string(),
        decision: ApprovalResponse::Yes,
        channel: "cli".to_string(),
    };
    let decoded: ApprovalLogEntry =
        serde_json::from_str(&serde_json::to_string(&entry).unwrap()).unwrap();

    assert_eq!(decoded.tool_name, "shell");
    assert_eq!(decoded.decision, ApprovalResponse::Yes);
}

#[test]
fn pending_request_serializes_all_identity_fields() {
    let request = PendingNonCliApprovalRequest {
        request_id: "apr-123".to_string(),
        tool_name: "file_write".to_string(),
        arguments: serde_json::json!({"path": "a"}),
        message_id: Some("msg".to_string()),
        call_id: Some("call".to_string()),
        requested_by: "alice".to_string(),
        requested_channel: "discord".to_string(),
        requested_reply_target: "chan".to_string(),
        reason: None,
        created_at: "now".to_string(),
        expires_at: "later".to_string(),
    };
    let decoded: PendingNonCliApprovalRequest =
        serde_json::from_str(&serde_json::to_string(&request).unwrap()).unwrap();

    assert_eq!(decoded, request);
    assert_eq!(PendingApprovalError::NotFound, PendingApprovalError::NotFound);
}
