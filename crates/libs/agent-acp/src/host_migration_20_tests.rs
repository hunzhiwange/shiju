use crate::prompt_approval::PendingPromptApproval;
use agent_approval::{ApprovalManager, ApprovalResponse, ACP_APPROVAL_CHANNEL};

fn manager() -> ApprovalManager {
    ApprovalManager::from_config(&Default::default())
}

#[test]
fn pending_prompt_drop_rejects_request_and_clears_resolution() {
    let manager = manager();
    let request = manager.create_non_cli_pending_request(
        "ACP", "sender", ACP_APPROVAL_CHANNEL, "session", None,
        serde_json::json!({}), None, None,
    );
    let id = request.request_id.clone();
    let pending = PendingPromptApproval::new(&manager, request);
    assert_eq!(pending.request().request_id, id);
    assert!(manager.has_non_cli_pending_request(&id));
    drop(pending);
    assert!(!manager.has_non_cli_pending_request(&id));
    assert_eq!(manager.take_non_cli_pending_resolution(&id), None);
}

#[test]
fn pending_prompt_drop_clears_already_resolved_request() {
    let manager = manager();
    let request = manager.create_non_cli_pending_request(
        "ACP", "sender", ACP_APPROVAL_CHANNEL, "session", None,
        serde_json::json!({}), None, None,
    );
    let id = request.request_id.clone();
    let pending = PendingPromptApproval::new(&manager, request);
    manager.confirm_non_cli_pending_request(&id, "sender", ACP_APPROVAL_CHANNEL, "session").unwrap();
    manager.record_non_cli_pending_resolution(&id, ApprovalResponse::Yes);
    drop(pending);
    assert!(!manager.has_non_cli_pending_request(&id));
    assert_eq!(manager.take_non_cli_pending_resolution(&id), None);
}
