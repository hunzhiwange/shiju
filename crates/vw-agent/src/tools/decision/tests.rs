//! 验证工具决策合并、权限检查和拒绝输入快照行为。
//! 交互批准、非命令行旁路和工具自身权限错误分别归类，只有显式快照的拒绝才暴露更新输入。

use super::*;
use crate::agent::loop_::approval::NonCliApprovalContext;
use crate::app::agent::approval::{ApprovalManager, ApprovalResponse};
use crate::app::agent::config::AutonomyConfig;
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn request_interactive_approval_denies_when_manager_is_missing_or_user_declines() {
    let pending = PermissionDecision::ask("needs approval", json!({"command": "python script.py"}))
        .with_warning(Some("review".to_string()));
    let no_manager =
        request_interactive_approval("shell", pending.clone(), &ToolUseContext::new("s", None))
            .await;

    assert!(matches!(no_manager, PermissionDecision::Deny { .. }));
    assert!(no_manager.reason().unwrap_or_default().contains("unavailable"));
    assert_eq!(no_manager.warning(), Some("review"));
    assert!(no_manager.updated_input().is_some());

    let approval = Arc::new(ApprovalManager::from_config(&AutonomyConfig::default()));
    let context = ToolUseContext::new("s", None).with_approval(approval).with_channel("slack");
    let declined = request_interactive_approval("shell", pending, &context).await;

    assert_eq!(declined.reason(), Some("Denied by user."));
    assert!(declined.updated_input().is_some());
}

#[tokio::test]
// 非命令行旁路仍需写入批准记录，不能直接把待确认决策改写为无痕允许。
async fn non_cli_bypass_records_approval_and_allows_pending_decision() {
    let approval = Arc::new(ApprovalManager::from_config(&AutonomyConfig::default()));
    let context = ToolUseContext::new("s", None)
        .with_approval(approval.clone())
        .with_channel("slack")
        .with_bypass_non_cli_approval_for_turn(true);

    let decision = request_interactive_approval(
        "write",
        PermissionDecision::allow(json!({"path": "safe.txt"})),
        &context,
    )
    .await;

    assert!(matches!(decision, PermissionDecision::Allow { .. }));
}

#[tokio::test]
async fn non_cli_approval_context_sends_prompt_and_waits_for_resolution() {
    let approval = Arc::new(ApprovalManager::from_config(&AutonomyConfig::default()));
    let (prompt_tx, mut prompt_rx) = tokio::sync::mpsc::unbounded_channel();
    let context = ToolUseContext::new("s", None)
        .with_approval(approval.clone())
        .with_channel("telegram")
        .with_message_id("msg-1")
        .with_tool_call_id("call-1")
        .with_non_cli_approval_context(NonCliApprovalContext {
            sender: "alice".to_string(),
            reply_target: "chat-1".to_string(),
            prompt_tx,
        });
    let approver = approval.clone();
    let approval_task = tokio::spawn(async move {
        let prompt = prompt_rx.recv().await.expect("prompt");
        approver.record_non_cli_pending_resolution(&prompt.request_id, ApprovalResponse::Yes);
    });

    let decision = request_interactive_approval(
        "shell",
        PermissionDecision::ask("needs approval", json!({"command": "date"})),
        &context,
    )
    .await;

    approval_task.await.expect("approval task");
    assert!(matches!(decision, PermissionDecision::Allow { .. }));
    assert_eq!(decision.updated_input().unwrap()["approved"], json!(true));
}
