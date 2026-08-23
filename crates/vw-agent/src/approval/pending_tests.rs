//! 测试待审批请求的去重、身份匹配、过期、清理与缓存上限。
//! 固定请求者上下文和过期时间用于验证决议不能被其他渠道或回复目标冒领。

use chrono::{Duration, Utc};
use std::collections::HashMap;

use crate::app::agent::config::AutonomyConfig;

use super::{
    ApprovalManager, ApprovalResponse, PendingApprovalError, PendingNonCliApprovalRequest,
};

#[test]
fn pending_request_deduplicates_and_resolves_by_requester_context() {
    let manager = ApprovalManager::from_config(&AutonomyConfig::default());
    let args = serde_json::json!({"path": "README.md"});
    let first = manager.create_non_cli_pending_request(
        "file_read",
        "alice",
        "discord",
        "chan-1",
        Some("need approval".to_string()),
        args.clone(),
        Some("msg-1".to_string()),
        Some("call-1".to_string()),
    );
    let second = manager.create_non_cli_pending_request(
        "file_read",
        "alice",
        "discord",
        "chan-1",
        Some("need approval".to_string()),
        args,
        Some("msg-1".to_string()),
        Some("call-1".to_string()),
    );

    assert_eq!(first.request_id, second.request_id);
    assert!(manager.has_non_cli_pending_request(&first.request_id));
    assert_eq!(
        manager
            .confirm_non_cli_pending_request(&first.request_id, "bob", "discord", "chan-1")
            .unwrap_err(),
        PendingApprovalError::RequesterMismatch
    );
    assert!(
        manager
            .confirm_non_cli_pending_request(&first.request_id, "alice", "discord", "chan-1")
            .is_ok()
    );
    assert!(!manager.has_non_cli_pending_request(&first.request_id));
}

#[test]
fn pending_resolution_only_records_yes_or_no() {
    let manager = ApprovalManager::from_config(&AutonomyConfig::default());

    manager.record_non_cli_pending_resolution("apr-1", ApprovalResponse::Always);
    assert_eq!(manager.take_non_cli_pending_resolution("apr-1"), None);

    manager.record_non_cli_pending_resolution("apr-1", ApprovalResponse::No);
    assert_eq!(manager.take_non_cli_pending_resolution("apr-1"), Some(ApprovalResponse::No));
}

#[test]
fn expired_pending_request_is_detected() {
    let request = PendingNonCliApprovalRequest {
        request_id: "apr-old".to_string(),
        tool_name: "shell".to_string(),
        arguments: serde_json::json!({}),
        message_id: None,
        call_id: None,
        requested_by: "alice".to_string(),
        requested_channel: "discord".to_string(),
        requested_reply_target: "chan".to_string(),
        reason: None,
        created_at: (Utc::now() - Duration::minutes(40)).to_rfc3339(),
        expires_at: (Utc::now() - Duration::minutes(1)).to_rfc3339(),
    };

    assert!(super::pending::is_pending_request_expired(&request));
}

#[test]
fn pending_request_rejects_missing_and_mismatched_resolution_then_allows_matching_reject() {
    let manager = ApprovalManager::from_config(&AutonomyConfig::default());

    assert_eq!(
        manager
            .confirm_non_cli_pending_request("apr-missing", "alice", "discord", "chan")
            .unwrap_err(),
        PendingApprovalError::NotFound
    );

    let req = manager.create_non_cli_pending_request(
        "shell",
        "alice",
        "discord",
        "chan",
        None,
        serde_json::json!({"command": "pwd"}),
        None,
        None,
    );

    assert_eq!(
        manager
            .reject_non_cli_pending_request(&req.request_id, "alice", "slack", "chan")
            .unwrap_err(),
        PendingApprovalError::RequesterMismatch
    );
    assert!(manager.has_non_cli_pending_request(&req.request_id));

    let rejected = manager
        .reject_non_cli_pending_request(&req.request_id, "alice", "discord", "chan")
        .expect("matching requester should resolve request");
    assert_eq!(rejected.request_id, req.request_id);
    assert!(!manager.has_non_cli_pending_request(&req.request_id));
}

#[test]
fn pending_list_filters_by_requester_channel_and_reply_target() {
    let manager = ApprovalManager::from_config(&AutonomyConfig::default());

    let first = manager.create_non_cli_pending_request(
        "file_read",
        "alice",
        "discord",
        "chan-1",
        None,
        serde_json::json!({"path": "a"}),
        None,
        None,
    );
    let second = manager.create_non_cli_pending_request(
        "shell",
        "alice",
        "slack",
        "chan-1",
        None,
        serde_json::json!({"command": "pwd"}),
        None,
        None,
    );
    let third = manager.create_non_cli_pending_request(
        "file_write",
        "bob",
        "discord",
        "chan-2",
        None,
        serde_json::json!({"path": "b"}),
        None,
        None,
    );

    let alice = manager.list_non_cli_pending_requests(Some("alice"), None, None);
    assert_eq!(alice.len(), 2);
    assert!(alice.iter().any(|req| req.request_id == first.request_id));
    assert!(alice.iter().any(|req| req.request_id == second.request_id));

    let discord_chan_2 =
        manager.list_non_cli_pending_requests(None, Some("discord"), Some("chan-2"));
    assert_eq!(discord_chan_2, vec![third]);
}

#[test]
fn clear_pending_requests_for_tool_removes_pending_and_resolved_entries() {
    let manager = ApprovalManager::from_config(&AutonomyConfig::default());
    let shell = manager.create_non_cli_pending_request(
        "shell",
        "alice",
        "discord",
        "chan",
        None,
        serde_json::json!({"command": "pwd"}),
        None,
        None,
    );
    let read = manager.create_non_cli_pending_request(
        "file_read",
        "alice",
        "discord",
        "chan",
        None,
        serde_json::json!({"path": "README.md"}),
        None,
        None,
    );
    manager.record_non_cli_pending_resolution(&shell.request_id, ApprovalResponse::Yes);
    manager.record_non_cli_pending_resolution(&read.request_id, ApprovalResponse::No);

    assert_eq!(manager.clear_non_cli_pending_requests_for_tool("shell"), 1);

    assert!(!manager.has_non_cli_pending_request(&shell.request_id));
    assert!(manager.has_non_cli_pending_request(&read.request_id));
    assert_eq!(manager.take_non_cli_pending_resolution(&shell.request_id), None);
    assert_eq!(
        manager.take_non_cli_pending_resolution(&read.request_id),
        Some(ApprovalResponse::No)
    );
}

#[test]
fn pending_resolution_cache_is_capped() {
    let manager = ApprovalManager::from_config(&AutonomyConfig::default());

    for idx in 0..1030 {
        manager.record_non_cli_pending_resolution(&format!("apr-{idx}"), ApprovalResponse::Yes);
    }

    assert_eq!(manager.resolved_non_cli_requests.lock().len(), 1024);
}

#[test]
fn prune_expired_pending_requests_removes_expired_and_invalid_entries() {
    let mut pending = HashMap::new();
    let active = PendingNonCliApprovalRequest {
        request_id: "apr-active".to_string(),
        tool_name: "shell".to_string(),
        arguments: serde_json::json!({}),
        message_id: None,
        call_id: None,
        requested_by: "alice".to_string(),
        requested_channel: "discord".to_string(),
        requested_reply_target: "chan".to_string(),
        reason: None,
        created_at: Utc::now().to_rfc3339(),
        expires_at: (Utc::now() + Duration::minutes(1)).to_rfc3339(),
    };
    let expired = PendingNonCliApprovalRequest {
        request_id: "apr-expired".to_string(),
        expires_at: (Utc::now() - Duration::minutes(1)).to_rfc3339(),
        ..active.clone()
    };
    let invalid = PendingNonCliApprovalRequest {
        request_id: "apr-invalid".to_string(),
        expires_at: "not a timestamp".to_string(),
        ..active.clone()
    };
    pending.insert(active.request_id.clone(), active);
    pending.insert(expired.request_id.clone(), expired);
    pending.insert(invalid.request_id.clone(), invalid);

    assert_eq!(super::pending::prune_expired_pending_requests(&mut pending), 2);

    assert_eq!(pending.len(), 1);
    assert!(pending.contains_key("apr-active"));
}
