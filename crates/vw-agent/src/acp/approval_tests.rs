//! 测试 ACP 审批桥接的请求映射、决议传播与敏感参数清理。
//! 通过真实异步通道覆盖未知选项、断连、取消和超时一律拒绝的安全默认值。

use std::sync::Arc;
use std::time::Duration;

use serde_json::json;
use tokio_util::sync::CancellationToken;

use super::approval::ApprovalBridge;
use crate::agent::loop_::approval::NonCliApprovalPrompt;
use crate::app::agent::approval::{ApprovalManager, ApprovalResponse};
use crate::app::agent::config::AutonomyConfig;

// 构造仍处于等待态的原生请求，使各测试只改变外部决议或终止条件。
fn pending_prompt(manager: &ApprovalManager, arguments: serde_json::Value) -> NonCliApprovalPrompt {
    let request = manager.create_non_cli_pending_request(
        "shell",
        "session-1",
        "acp",
        "session-1",
        Some("sensitive command".to_string()),
        arguments.clone(),
        Some("message-1".to_string()),
        Some("call-1".to_string()),
    );
    NonCliApprovalPrompt {
        request_id: request.request_id,
        tool_name: request.tool_name,
        arguments,
        call_id: request.call_id,
        sender: request.requested_by,
        channel: request.requested_channel,
        reply_target: request.requested_reply_target,
    }
}

async fn wait_for_resolution(manager: &ApprovalManager, request_id: &str) -> ApprovalResponse {
    for _ in 0..20 {
        if let Some(decision) = manager.take_non_cli_pending_resolution(request_id) {
            return decision;
        }
        tokio::task::yield_now().await;
    }
    panic!("approval decision was not resolved");
}

#[tokio::test]
async fn approval_prompt_maps_to_single_acp_permission_request() {
    let manager = Arc::new(ApprovalManager::from_config(&AutonomyConfig::default()));
    let prompt = pending_prompt(&manager, json!({"command": "pwd"}));
    let request_id = prompt.request_id.clone();
    let (prompt_tx, prompt_rx) = tokio::sync::mpsc::unbounded_channel();
    let (permission_tx, mut permission_rx) = tokio::sync::mpsc::unbounded_channel();
    let cancellation = CancellationToken::new();
    let bridge =
        ApprovalBridge::new(Arc::clone(&manager), prompt_rx, permission_tx, cancellation.clone());
    let task = tokio::spawn(bridge.run());

    prompt_tx.send(prompt).expect("approval prompt");
    let request = permission_rx.recv().await.expect("permission request");
    assert_eq!(request.request_id, request_id);
    assert_eq!(request.call_id, "call-1");
    assert_eq!(request.kind, agent_client_protocol::ToolKind::Execute);
    assert_eq!(request.options.len(), 2);
    assert!(permission_rx.try_recv().is_err());

    request.response.send(Some("reject_once".to_string())).expect("permission response");
    assert_eq!(wait_for_resolution(&manager, &request_id).await, ApprovalResponse::No);
    cancellation.cancel();
    task.await.expect("bridge task");
}

#[tokio::test]
async fn approval_response_resolves_matching_native_request() {
    let manager = Arc::new(ApprovalManager::from_config(&AutonomyConfig::default()));
    let prompt = pending_prompt(&manager, json!({"command": "pwd"}));
    let request_id = prompt.request_id.clone();
    let (prompt_tx, prompt_rx) = tokio::sync::mpsc::unbounded_channel();
    let (permission_tx, mut permission_rx) = tokio::sync::mpsc::unbounded_channel();
    let cancellation = CancellationToken::new();
    let task = tokio::spawn(
        ApprovalBridge::new(Arc::clone(&manager), prompt_rx, permission_tx, cancellation.clone())
            .run(),
    );

    prompt_tx.send(prompt).expect("approval prompt");
    let request = permission_rx.recv().await.expect("permission request");
    request.response.send(Some("allow_once".to_string())).expect("permission response");

    assert_eq!(wait_for_resolution(&manager, &request_id).await, ApprovalResponse::Yes);
    assert!(!manager.has_non_cli_pending_request(&request_id));
    cancellation.cancel();
    task.await.expect("bridge task");
}

async fn denied_for_response(response: Option<String>) {
    let manager = Arc::new(ApprovalManager::from_config(&AutonomyConfig::default()));
    let prompt = pending_prompt(&manager, json!({"command": "pwd"}));
    let request_id = prompt.request_id.clone();
    let (prompt_tx, prompt_rx) = tokio::sync::mpsc::unbounded_channel();
    let (permission_tx, mut permission_rx) = tokio::sync::mpsc::unbounded_channel();
    let cancellation = CancellationToken::new();
    let task = tokio::spawn(
        ApprovalBridge::new(Arc::clone(&manager), prompt_rx, permission_tx, cancellation.clone())
            .run(),
    );
    prompt_tx.send(prompt).expect("approval prompt");
    permission_rx
        .recv()
        .await
        .expect("permission request")
        .response
        .send(response)
        .expect("permission response");
    assert_eq!(wait_for_resolution(&manager, &request_id).await, ApprovalResponse::No);
    assert!(!manager.has_non_cli_pending_request(&request_id));
    cancellation.cancel();
    task.await.expect("bridge task");
}

#[tokio::test(start_paused = true)]
async fn approval_disconnect_cancel_timeout_and_unknown_option_deny() {
    denied_for_response(Some("unknown".to_string())).await;
    denied_for_response(None).await;

    let manager = Arc::new(ApprovalManager::from_config(&AutonomyConfig::default()));
    let prompt = pending_prompt(&manager, json!({"command": "pwd"}));
    let request_id = prompt.request_id.clone();
    let (prompt_tx, prompt_rx) = tokio::sync::mpsc::unbounded_channel();
    let (permission_tx, permission_rx) = tokio::sync::mpsc::unbounded_channel();
    drop(permission_rx);
    let cancellation = CancellationToken::new();
    let task = tokio::spawn(
        ApprovalBridge::new(Arc::clone(&manager), prompt_rx, permission_tx, cancellation.clone())
            .run(),
    );
    prompt_tx.send(prompt).expect("approval prompt");
    assert_eq!(wait_for_resolution(&manager, &request_id).await, ApprovalResponse::No);
    assert!(!manager.has_non_cli_pending_request(&request_id));
    cancellation.cancel();
    task.await.expect("bridge task");

    let manager = Arc::new(ApprovalManager::from_config(&AutonomyConfig::default()));
    let prompt = pending_prompt(&manager, json!({"command": "pwd"}));
    let request_id = prompt.request_id.clone();
    let (prompt_tx, prompt_rx) = tokio::sync::mpsc::unbounded_channel();
    let (permission_tx, mut permission_rx) = tokio::sync::mpsc::unbounded_channel();
    let cancellation = CancellationToken::new();
    let task = tokio::spawn(
        ApprovalBridge::new(Arc::clone(&manager), prompt_rx, permission_tx, cancellation.clone())
            .with_timeout(Duration::from_secs(1))
            .run(),
    );
    prompt_tx.send(prompt).expect("approval prompt");
    let _request = permission_rx.recv().await.expect("permission request");
    tokio::time::advance(Duration::from_secs(1)).await;
    assert_eq!(wait_for_resolution(&manager, &request_id).await, ApprovalResponse::No);
    assert!(!manager.has_non_cli_pending_request(&request_id));
    cancellation.cancel();
    task.await.expect("bridge task");

    let manager = Arc::new(ApprovalManager::from_config(&AutonomyConfig::default()));
    let prompt = pending_prompt(&manager, json!({"command": "pwd"}));
    let request_id = prompt.request_id.clone();
    let (prompt_tx, prompt_rx) = tokio::sync::mpsc::unbounded_channel();
    let (permission_tx, mut permission_rx) = tokio::sync::mpsc::unbounded_channel();
    let cancellation = CancellationToken::new();
    let task = tokio::spawn(
        ApprovalBridge::new(Arc::clone(&manager), prompt_rx, permission_tx, cancellation.clone())
            .run(),
    );
    prompt_tx.send(prompt).expect("approval prompt");
    let _request = permission_rx.recv().await.expect("permission request");
    cancellation.cancel();
    assert_eq!(wait_for_resolution(&manager, &request_id).await, ApprovalResponse::No);
    assert!(!manager.has_non_cli_pending_request(&request_id));
    task.await.expect("bridge task");
}

#[tokio::test]
async fn approval_payload_uses_sanitized_tool_context() {
    let manager = Arc::new(ApprovalManager::from_config(&AutonomyConfig::default()));
    let sanitized = json!({"command": "curl", "authorization": "[REDACTED]"});
    let prompt = pending_prompt(&manager, sanitized.clone());
    let request_id = prompt.request_id.clone();
    let (prompt_tx, prompt_rx) = tokio::sync::mpsc::unbounded_channel();
    let (permission_tx, mut permission_rx) = tokio::sync::mpsc::unbounded_channel();
    let cancellation = CancellationToken::new();
    let task = tokio::spawn(
        ApprovalBridge::new(Arc::clone(&manager), prompt_rx, permission_tx, cancellation.clone())
            .run(),
    );

    prompt_tx.send(prompt).expect("approval prompt");
    let request = permission_rx.recv().await.expect("permission request");
    assert_eq!(request.raw_input, sanitized);
    assert!(!request.raw_input.to_string().contains("Bearer "));
    request.response.send(Some("reject_once".to_string())).expect("permission response");
    assert_eq!(wait_for_resolution(&manager, &request_id).await, ApprovalResponse::No);
    cancellation.cancel();
    task.await.expect("bridge task");
}
