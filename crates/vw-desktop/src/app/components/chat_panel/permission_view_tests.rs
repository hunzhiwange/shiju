//! 验证聊天面板权限视图。
//! 测试确保权限请求、拒绝和错误状态都能以明确 UI 呈现。

use super::tools::{
    pending_permission_badge_label, pending_permission_request_badge_label,
    pending_permission_request_for_message, pending_permission_request_for_tool_call,
    pending_permission_targets_message, pending_permission_targets_tool_call,
    tool_call_id_from_raw, tool_permission_target_summary,
};

#[test]
fn pending_permission_targets_message_matches_tool_message_id() {
    let request = vw_gateway_client::PendingPermissionRequestDto {
        id: "perm-1".to_string(),
        session_id: "session-1".to_string(),
        permission: "shell".to_string(),
        patterns: Vec::new(),
        metadata: serde_json::Map::new(),
        always: Vec::new(),
        tool: Some(vw_gateway_client::PendingPermissionToolDto {
            message_id: "msg-1".to_string(),
            call_id: "call-1".to_string(),
        }),
    };

    assert!(pending_permission_targets_message(Some(&request), Some("msg-1")));
    assert!(!pending_permission_targets_message(Some(&request), Some("msg-2")));
    assert!(!pending_permission_targets_message(Some(&request), None));
}

#[test]
fn tool_call_id_from_raw_reads_supported_keys() {
    let raw = concat!(
        "tool bash\n",
        "{\"status\":\"running\",\"tool_call_id\":\"call-top\",\"input\":\"echo hi\"}"
    );
    let nested_raw = concat!(
        "tool write\n",
        "{\"status\":\"running\",\"result\":{\"metadata\":{\"callId\":\"call-nested\"}}}"
    );
    let dto_raw = concat!(
        "tool bash\n",
        "{\"status\":\"running\",\"tool_use_id\":\"call-dto\",\"input\":\"pwd\"}"
    );

    assert_eq!(tool_call_id_from_raw(raw).as_deref(), Some("call-top"));
    assert_eq!(tool_call_id_from_raw(nested_raw).as_deref(), Some("call-nested"));
    assert_eq!(tool_call_id_from_raw(dto_raw).as_deref(), Some("call-dto"));
}

#[test]
fn pending_permission_targets_tool_call_matches_call_id_and_message_id() {
    let request = vw_gateway_client::PendingPermissionRequestDto {
        id: "perm-1".to_string(),
        session_id: "session-1".to_string(),
        permission: "shell".to_string(),
        patterns: Vec::new(),
        metadata: serde_json::Map::new(),
        always: Vec::new(),
        tool: Some(vw_gateway_client::PendingPermissionToolDto {
            message_id: "msg-1".to_string(),
            call_id: "call-1".to_string(),
        }),
    };
    let raw = concat!(
        "tool bash\n",
        "{\"status\":\"running\",\"toolCallId\":\"call-1\",\"input\":\"pwd\"}"
    );

    assert!(pending_permission_targets_tool_call(Some(&request), Some("msg-1"), raw));
    assert!(!pending_permission_targets_tool_call(Some(&request), Some("msg-2"), raw));
    assert!(!pending_permission_targets_tool_call(Some(&request), Some("msg-1"), "tool bash\n{}"));
}

#[test]
fn pending_permission_badge_label_shows_position_for_multiple_requests() {
    let requests = vec![
        vw_gateway_client::PendingPermissionRequestDto {
            id: "perm-1".to_string(),
            session_id: "session-1".to_string(),
            permission: "shell".to_string(),
            patterns: Vec::new(),
            metadata: serde_json::Map::new(),
            always: Vec::new(),
            tool: None,
        },
        vw_gateway_client::PendingPermissionRequestDto {
            id: "perm-2".to_string(),
            session_id: "session-1".to_string(),
            permission: "shell".to_string(),
            patterns: Vec::new(),
            metadata: serde_json::Map::new(),
            always: Vec::new(),
            tool: None,
        },
    ];

    assert_eq!(
        pending_permission_badge_label(
            crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
            &requests,
            Some("perm-2")
        ),
        "当前审批 2/2"
    );
    assert_eq!(
        pending_permission_badge_label(
            crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
            &requests,
            Some("missing")
        ),
        "当前审批 1/2"
    );
    assert_eq!(
        pending_permission_request_badge_label(
            crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
            &requests,
            "perm-1"
        ),
        "待审批 1/2"
    );
}

#[test]
fn pending_permission_request_for_tool_call_finds_matching_request() {
    let requests = vec![
        vw_gateway_client::PendingPermissionRequestDto {
            id: "perm-1".to_string(),
            session_id: "session-1".to_string(),
            permission: "shell".to_string(),
            patterns: Vec::new(),
            metadata: serde_json::Map::new(),
            always: Vec::new(),
            tool: Some(vw_gateway_client::PendingPermissionToolDto {
                message_id: "msg-1".to_string(),
                call_id: "call-1".to_string(),
            }),
        },
        vw_gateway_client::PendingPermissionRequestDto {
            id: "perm-2".to_string(),
            session_id: "session-1".to_string(),
            permission: "shell".to_string(),
            patterns: Vec::new(),
            metadata: serde_json::Map::new(),
            always: Vec::new(),
            tool: Some(vw_gateway_client::PendingPermissionToolDto {
                message_id: "msg-1".to_string(),
                call_id: "call-2".to_string(),
            }),
        },
    ];
    let raw = concat!(
        "tool bash\n",
        "{\"status\":\"running\",\"toolCallId\":\"call-2\",\"input\":\"pwd\"}"
    );

    let matched = pending_permission_request_for_tool_call(&requests, Some("msg-1"), raw)
        .map(|request| request.id.as_str());

    assert_eq!(matched, Some("perm-2"));
}

#[test]
fn pending_permission_request_for_message_requires_unique_match() {
    let first = vw_gateway_client::PendingPermissionRequestDto {
        id: "perm-1".to_string(),
        session_id: "session-1".to_string(),
        permission: "shell".to_string(),
        patterns: Vec::new(),
        metadata: serde_json::Map::new(),
        always: Vec::new(),
        tool: Some(vw_gateway_client::PendingPermissionToolDto {
            message_id: "msg-1".to_string(),
            call_id: "call-1".to_string(),
        }),
    };
    let second = vw_gateway_client::PendingPermissionRequestDto {
        id: "perm-2".to_string(),
        session_id: "session-1".to_string(),
        permission: "shell".to_string(),
        patterns: Vec::new(),
        metadata: serde_json::Map::new(),
        always: Vec::new(),
        tool: Some(vw_gateway_client::PendingPermissionToolDto {
            message_id: "msg-2".to_string(),
            call_id: "call-2".to_string(),
        }),
    };
    let duplicate = vw_gateway_client::PendingPermissionRequestDto {
        id: "perm-3".to_string(),
        session_id: "session-1".to_string(),
        permission: "shell".to_string(),
        patterns: Vec::new(),
        metadata: serde_json::Map::new(),
        always: Vec::new(),
        tool: Some(vw_gateway_client::PendingPermissionToolDto {
            message_id: "msg-1".to_string(),
            call_id: "call-3".to_string(),
        }),
    };

    let unique_requests = [first.clone(), second.clone()];
    let unique_match = pending_permission_request_for_message(&unique_requests, Some("msg-2"))
        .map(|request| request.id.as_str());
    let ambiguous_requests = [first, duplicate, second];
    let ambiguous_match =
        pending_permission_request_for_message(&ambiguous_requests, Some("msg-1"))
            .map(|request| request.id.as_str());

    assert_eq!(unique_match, Some("perm-2"));
    assert_eq!(ambiguous_match, None);
}

#[test]
fn permission_target_summary_uses_file_write_path() {
    let value = serde_json::json!({
        "status": "denied",
        "permission_request": {
            "updated_input": {
                "path": "src/main.rs"
            }
        }
    });

    assert_eq!(
        tool_permission_target_summary("file_write", &value).as_deref(),
        Some("src/main.rs")
    );
}

#[test]
fn permission_target_summary_uses_bash_command() {
    let value = serde_json::json!({
        "status": "denied",
        "permission_request": {
            "updated_input": {
                "command": "git status --short"
            }
        }
    });

    assert_eq!(
        tool_permission_target_summary("bash", &value).as_deref(),
        Some("git status --short")
    );
}
