//! 测试非命令行审批上下文的决议等待、取消、超时与清理行为。
//! 轮询夹具刻意覆盖决议晚到以及请求消失，确认清理动作不会遗留待处理状态。

use super::*;
use crate::app::agent::config::AutonomyConfig;
use tokio_util::sync::CancellationToken;

#[test]
fn non_cli_approval_context_keeps_sender_and_reply_target() {
    let (prompt_tx, mut prompt_rx) = tokio::sync::mpsc::unbounded_channel();
    let context = NonCliApprovalContext {
        sender: "alice".to_string(),
        reply_target: "thread-1".to_string(),
        prompt_tx,
    };

    context
        .prompt_tx
        .send(NonCliApprovalPrompt {
            request_id: "req-1".to_string(),
            tool_name: "shell".to_string(),
            arguments: serde_json::json!({"command": "pwd"}),
            call_id: Some("call-1".to_string()),
            sender: "alice".to_string(),
            channel: "acp".to_string(),
            reply_target: "thread-1".to_string(),
        })
        .unwrap();

    let prompt = prompt_rx.try_recv().unwrap();
    assert_eq!(context.sender, "alice");
    assert_eq!(context.reply_target, "thread-1");
    assert_eq!(prompt.request_id, "req-1");
    assert_eq!(prompt.tool_name, "shell");
    assert_eq!(prompt.arguments["command"], "pwd");
    assert_eq!(prompt.call_id.as_deref(), Some("call-1"));
}

// 返回请求编号而不暴露内部待处理表，测试始终通过公开决议入口驱动状态。
fn approval_manager_with_request() -> (ApprovalManager, String) {
    let manager = ApprovalManager::from_config(&AutonomyConfig::default());
    let request = manager.create_non_cli_pending_request(
        "shell",
        "alice",
        "telegram",
        "chat-1",
        Some("needs approval".to_string()),
        serde_json::json!({"command": "pwd"}),
        Some("msg-1".to_string()),
        Some("call-1".to_string()),
    );

    (manager, request.request_id)
}

#[tokio::test]
async fn await_non_cli_approval_decision_returns_recorded_decision() {
    let (manager, request_id) = approval_manager_with_request();
    manager.record_non_cli_pending_resolution(&request_id, ApprovalResponse::Yes);

    let decision =
        await_non_cli_approval_decision(&manager, &request_id, "alice", "telegram", "chat-1", None)
            .await;

    assert_eq!(decision, ApprovalResponse::Yes);
    assert_eq!(manager.take_non_cli_pending_resolution(&request_id), None);
}

#[tokio::test(start_paused = true)]
async fn await_non_cli_approval_decision_returns_decision_recorded_after_poll() {
    let (manager, request_id) = approval_manager_with_request();
    let decision =
        await_non_cli_approval_decision(&manager, &request_id, "alice", "telegram", "chat-1", None);
    tokio::pin!(decision);

    tokio::select! {
        resolved = &mut decision => panic!("approval resolved before decision: {resolved:?}"),
        _ = tokio::time::sleep(Duration::from_millis(1)) => {}
    }

    manager.record_non_cli_pending_resolution(&request_id, ApprovalResponse::No);
    tokio::time::advance(Duration::from_millis(NON_CLI_APPROVAL_POLL_INTERVAL_MS)).await;

    assert_eq!(decision.await, ApprovalResponse::No);
    assert_eq!(manager.take_non_cli_pending_resolution(&request_id), None);
}

#[tokio::test]
async fn await_non_cli_approval_decision_denies_missing_request() {
    let manager = ApprovalManager::from_config(&AutonomyConfig::default());

    let decision = await_non_cli_approval_decision(
        &manager,
        "apr-missing",
        "alice",
        "telegram",
        "chat-1",
        None,
    )
    .await;

    assert_eq!(decision, ApprovalResponse::No);
}

#[tokio::test]
async fn await_non_cli_approval_decision_denies_cancelled_request() {
    let (manager, request_id) = approval_manager_with_request();
    let cancellation_token = CancellationToken::new();
    cancellation_token.cancel();

    let decision = await_non_cli_approval_decision(
        &manager,
        &request_id,
        "alice",
        "telegram",
        "chat-1",
        Some(&cancellation_token),
    )
    .await;

    assert_eq!(decision, ApprovalResponse::No);
    assert!(!manager.has_non_cli_pending_request(&request_id));
}

#[tokio::test(start_paused = true)]
async fn await_non_cli_approval_decision_denies_and_cleans_up_on_timeout() {
    let (manager, request_id) = approval_manager_with_request();

    let decision =
        await_non_cli_approval_decision(&manager, &request_id, "alice", "telegram", "chat-1", None)
            .await;

    assert_eq!(decision, ApprovalResponse::No);
    assert!(!manager.has_non_cli_pending_request(&request_id));
    assert_eq!(manager.take_non_cli_pending_resolution(&request_id), None);
}
