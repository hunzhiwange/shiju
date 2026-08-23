//! 验证工具权限请求匹配、审批状态、拒绝详情与徽标聚合逻辑。
//! 匹配优先使用调用标识，再回退消息标识，以避免同一消息内多工具请求串位。

use super::tool_permission::{
    ToolPermissionState, pending_permission_badge_label, pending_permission_request_badge_label,
    pending_permission_request_for_message, pending_permission_request_for_tool_call,
    pending_permission_targets_message, pending_permission_targets_tool_call,
    tool_permission_detail_text, tool_permission_error_text, tool_permission_state,
    tool_permission_summary, tool_permission_target_summary, tool_permission_title,
};
use serde_json::Map;
use serde_json::json;

// 权限 fixture 默认绑定消息与工具调用；无工具版本用于验证消息级兼容请求。
fn request(
    id: &str,
    message_id: &str,
    call_id: &str,
) -> vw_gateway_client::PendingPermissionRequestDto {
    request_with_tool(id, message_id, Some(call_id))
}

fn request_with_tool(
    id: &str,
    message_id: &str,
    call_id: Option<&str>,
) -> vw_gateway_client::PendingPermissionRequestDto {
    vw_gateway_client::PendingPermissionRequestDto {
        id: id.to_string(),
        session_id: "session-1".to_string(),
        permission: "tool".to_string(),
        patterns: vec![],
        metadata: Map::new(),
        always: vec![],
        tool: call_id.map(|call_id| vw_gateway_client::PendingPermissionToolDto {
            message_id: message_id.to_string(),
            call_id: call_id.to_string(),
        }),
    }
}

fn request_without_tool(id: &str) -> vw_gateway_client::PendingPermissionRequestDto {
    vw_gateway_client::PendingPermissionRequestDto {
        id: id.to_string(),
        session_id: "session-1".to_string(),
        permission: "tool".to_string(),
        patterns: vec![],
        metadata: Map::new(),
        always: vec![],
        tool: None,
    }
}

#[test]
fn permission_badge_labels_handle_empty_request_lists() {
    assert_eq!(pending_permission_badge_label(&[], None), "当前审批");
    assert_eq!(pending_permission_request_badge_label(&[], "missing"), "当前审批");
}

#[test]
fn permission_state_detects_approval_required() {
    let value = json!({"status":"error","error":"approval required before running command"});

    assert_eq!(tool_permission_state("bash", &value), Some(ToolPermissionState::ApprovalRequired));
    assert_eq!(tool_permission_summary("bash", &value), Some("需要权限批准"));
}

#[test]
fn pending_permission_request_matching_prefers_call_id_then_message() {
    let requests = vec![request("req-1", "msg-1", "call-1"), request("req-2", "msg-2", "call-2")];
    let raw = "tool bash\n{\"tool_call_id\":\"call-1\",\"input\":\"ls\"}";

    assert!(pending_permission_targets_message(Some(&requests[0]), Some("msg-1")));
    assert!(pending_permission_targets_tool_call(Some(&requests[0]), Some("msg-1"), raw));
    assert_eq!(
        pending_permission_request_for_tool_call(&requests, Some("msg-1"), raw)
            .map(|item| item.id.as_str()),
        Some("req-1")
    );
    assert_eq!(
        pending_permission_request_for_message(&requests, Some("msg-2"))
            .map(|item| item.id.as_str()),
        Some("req-2")
    );
}

#[test]
fn permission_state_detects_rejection_and_formats_details() {
    let value = json!({
        "status": "denied",
        "error": "permission denied",
        "permission_request": {
            "reason": "Need approval for write",
            "warning": "Modifies tracked files",
            "updated_input": {"path": "src/main.rs"}
        }
    });

    assert_eq!(tool_permission_state("file_write", &value), Some(ToolPermissionState::Rejected));
    assert_eq!(tool_permission_summary("file_write", &value), Some("权限已拒绝"));
    assert_eq!(
        tool_permission_target_summary("file_write", &value).as_deref(),
        Some("src/main.rs")
    );
    assert_eq!(tool_permission_title("写入", ToolPermissionState::Rejected), "写入已拒绝");

    let detail = tool_permission_detail_text("file_write", &value).expect("detail");
    assert!(detail.contains("原因：Need approval for write"));
    assert!(detail.contains("提示：Modifies tracked files"));
    assert!(detail.contains("目标：src/main.rs"));
    assert!(
        tool_permission_error_text("file_write", &value)
            .expect("error text")
            .contains("permission denied")
    );
}

#[test]
fn pending_permission_targets_message_edge_cases() {
    let req_no_tool = request_without_tool("req-1");
    let req_with_tool = request("req-2", "msg-1", "call-1");

    assert!(!pending_permission_targets_message(None, Some("msg-1")));
    assert!(!pending_permission_targets_message(Some(&req_no_tool), Some("msg-1")));
    assert!(!pending_permission_targets_message(Some(&req_with_tool), None));
}

#[test]
fn pending_permission_targets_tool_call_edge_cases() {
    let req_no_tool = request_without_tool("req-1");
    let req_with_tool = request("req-2", "msg-1", "call-1");
    let raw = "tool bash\n{\"tool_call_id\":\"call-1\",\"input\":\"ls\"}";

    assert!(!pending_permission_targets_tool_call(None, Some("msg-1"), raw));
    assert!(!pending_permission_targets_tool_call(Some(&req_no_tool), Some("msg-1"), raw));
    assert!(!pending_permission_targets_tool_call(Some(&req_with_tool), None, raw));

    let req_mismatch = request("req-3", "msg-other", "call-1");
    assert!(!pending_permission_targets_tool_call(Some(&req_mismatch), Some("msg-1"), raw));

    let req_empty_call = request_with_tool("req-4", "msg-1", Some(""));
    assert!(!pending_permission_targets_tool_call(Some(&req_empty_call), Some("msg-1"), raw));
}

#[test]
fn pending_permission_request_for_tool_call_edge_cases() {
    let raw = "tool bash\n{\"tool_call_id\":\"call-1\",\"input\":\"ls\"}";
    let raw_no_call_id = "tool bash\n{}";

    assert!(pending_permission_request_for_tool_call(&[], None, raw).is_none());

    let reqs_match = vec![request("req-1", "msg-1", "call-1")];
    let reqs_no_call_match = vec![request("req-2", "msg-1", "call-other")];
    let reqs_multi =
        vec![request("req-1", "msg-1", "call-other"), request("req-2", "msg-1", "call-another")];
    let reqs_call_match = vec![request("req-3", "msg-1", "call-another")];

    assert_eq!(
        pending_permission_request_for_tool_call(&reqs_match, Some("msg-1"), raw)
            .map(|item| item.id.as_str()),
        Some("req-1")
    );

    assert_eq!(
        pending_permission_request_for_tool_call(
            &reqs_no_call_match,
            Some("msg-1"),
            raw_no_call_id
        )
        .map(|item| item.id.as_str()),
        Some("req-2")
    );

    assert!(
        pending_permission_request_for_tool_call(&reqs_multi, Some("msg-1"), raw_no_call_id)
            .is_none()
    );

    assert!(
        pending_permission_request_for_tool_call(&reqs_call_match, Some("msg-1"), raw).is_none()
    );
}

#[test]
fn pending_permission_request_for_message_edge_cases() {
    assert!(pending_permission_request_for_message(&[], None).is_none());
    assert!(pending_permission_request_for_message(&[], Some("msg-1")).is_none());

    let single = vec![request("req-1", "msg-1", "call-1")];
    assert_eq!(
        pending_permission_request_for_message(&single, Some("msg-1")).map(|item| item.id.as_str()),
        Some("req-1")
    );

    let multi = vec![request("req-1", "msg-1", "call-1"), request("req-2", "msg-1", "call-2")];
    assert!(pending_permission_request_for_message(&multi, Some("msg-1")).is_none());
}

#[test]
fn pending_permission_badge_label_multiple() {
    let reqs = vec![
        request("req-1", "msg-1", "call-1"),
        request("req-2", "msg-2", "call-2"),
        request("req-3", "msg-3", "call-3"),
    ];

    assert_eq!(pending_permission_badge_label(&reqs, Some("req-1")), "当前审批 1/3");
    assert_eq!(pending_permission_badge_label(&reqs, Some("req-2")), "当前审批 2/3");
    assert_eq!(pending_permission_badge_label(&reqs, Some("req-3")), "当前审批 3/3");
    assert_eq!(pending_permission_badge_label(&reqs, None), "当前审批 1/3");
    assert_eq!(pending_permission_badge_label(&reqs, Some("missing")), "当前审批 1/3");
}

#[test]
fn pending_permission_request_badge_label_multiple() {
    let reqs = vec![request("req-1", "msg-1", "call-1"), request("req-2", "msg-2", "call-2")];

    assert_eq!(pending_permission_request_badge_label(&reqs, "req-1"), "待审批 1/2");
    assert_eq!(pending_permission_request_badge_label(&reqs, "req-2"), "待审批 2/2");
    assert_eq!(pending_permission_request_badge_label(&reqs, "missing"), "待审批 1/2");
}

#[test]
fn tool_permission_state_non_matching_status() {
    let value = json!({"status":"completed","error":"some error"});
    assert_eq!(tool_permission_state("bash", &value), None);
}

#[test]
fn tool_permission_state_canonical_denied_fallback() {
    let value = json!({"status":"denied","error":"some other error"});

    assert_eq!(tool_permission_state("file_edit", &value), Some(ToolPermissionState::Rejected));
    assert_eq!(tool_permission_state("notebook_edit", &value), Some(ToolPermissionState::Rejected));
    assert_eq!(tool_permission_state("apply_patch", &value), Some(ToolPermissionState::Rejected));
    assert_eq!(tool_permission_state("skill", &value), Some(ToolPermissionState::Rejected));
    assert_eq!(tool_permission_state("write", &value), Some(ToolPermissionState::Rejected));
}

#[test]
fn tool_permission_state_no_match_fallthrough() {
    let value = json!({"status":"error","error":"some unknown error"});
    assert_eq!(tool_permission_state("bash", &value), None);

    let value_denied = json!({"status":"denied","error":"some unknown error"});
    assert_eq!(tool_permission_state("bash", &value_denied), None);
}

#[test]
fn tool_permission_state_chinese_rejection_keywords() {
    let value1 = json!({"status":"error","error":"权限不足"});
    assert_eq!(tool_permission_state("bash", &value1), Some(ToolPermissionState::Rejected));

    let value2 = json!({"status":"error","error":"用户拒绝授权"});
    assert_eq!(tool_permission_state("bash", &value2), Some(ToolPermissionState::Rejected));
}

#[test]
fn tool_permission_state_approval_keyword_variants() {
    let check =
        |error: &str| tool_permission_state("bash", &json!({"status":"error","error": error}));

    assert_eq!(check("requires user approval"), Some(ToolPermissionState::ApprovalRequired));
    assert_eq!(
        check("requires approval from supervisor"),
        Some(ToolPermissionState::ApprovalRequired)
    );
    assert_eq!(check("interactive approval required"), Some(ToolPermissionState::ApprovalRequired));
}

#[test]
fn tool_permission_state_rejection_keyword_variants() {
    let check =
        |error: &str| tool_permission_state("bash", &json!({"status":"error","error": error}));

    assert_eq!(check("denied by user"), Some(ToolPermissionState::Rejected));
    assert_eq!(check("access denied"), Some(ToolPermissionState::Rejected));
    assert_eq!(check("not allowed"), Some(ToolPermissionState::Rejected));
    assert_eq!(check("forbidden"), Some(ToolPermissionState::Rejected));
}

#[test]
fn tool_permission_summary_none() {
    assert_eq!(tool_permission_summary("bash", &json!({"status":"completed"})), None);
}

#[test]
fn tool_permission_detail_text_missing_permission_request() {
    assert_eq!(tool_permission_detail_text("bash", &json!({"status":"error"})), None);
}

#[test]
fn tool_permission_detail_text_single_fields() {
    let only_reason = json!({
        "permission_request": {"reason": "Need access"}
    });
    let detail = tool_permission_detail_text("bash", &only_reason).expect("detail");
    assert_eq!(detail, "原因：Need access");

    let only_warning = json!({
        "permission_request": {"warning": "This is dangerous"}
    });
    let detail = tool_permission_detail_text("bash", &only_warning).expect("detail");
    assert_eq!(detail, "提示：This is dangerous");

    let only_target = json!({
        "permission_request": {"updated_input": {"path": "/tmp/test.txt"}}
    });
    let detail = tool_permission_detail_text("read", &only_target).expect("detail");
    assert_eq!(detail, "目标：/tmp/test.txt");
}

#[test]
fn tool_permission_error_text_combinations() {
    let both = json!({
        "error": "some error",
        "permission_request": {"reason": "Need approval"}
    });
    let error_text = tool_permission_error_text("bash", &both).expect("error text");
    assert!(error_text.contains("原因：Need approval"));
    assert!(error_text.contains("some error"));

    let detail_only = json!({
        "permission_request": {"reason": "Need approval"}
    });
    assert_eq!(
        tool_permission_error_text("bash", &detail_only).as_deref(),
        Some("原因：Need approval")
    );

    let error_only = json!({
        "error": "some error"
    });
    assert_eq!(tool_permission_error_text("bash", &error_only).as_deref(), Some("some error"));

    let none = json!({});
    assert_eq!(tool_permission_error_text("bash", &none), None);
}

#[test]
fn tool_permission_target_summary_edge_cases() {
    let no_permission = json!({"status":"error"});
    assert_eq!(tool_permission_target_summary("bash", &no_permission), None);

    let no_updated_input = json!({
        "permission_request": {"reason": "Need access"}
    });
    assert_eq!(tool_permission_target_summary("bash", &no_updated_input), None);
}

#[test]
fn tool_permission_title_approval_required() {
    assert_eq!(tool_permission_title("读取", ToolPermissionState::ApprovalRequired), "读取待批准");
}

#[test]
fn tool_permission_detail_text_empty_fields_after_trim() {
    let empty_reason = json!({
        "permission_request": {
            "reason": "  ",
            "warning": "  ",
        }
    });
    let detail = tool_permission_detail_text("bash", &empty_reason);
    assert_eq!(detail, None);
}

#[test]
fn tool_permission_target_summary_input_null() {
    let value = json!({
        "permission_request": {"updated_input": null}
    });
    assert_eq!(tool_permission_target_summary("read", &value), None);
}

#[test]
fn tool_permission_target_summary_string_input() {
    let value = json!({
        "permission_request": {"updated_input": "simple text input"}
    });
    assert_eq!(
        tool_permission_target_summary("bash", &value).as_deref(),
        Some("simple text input")
    );
}

#[test]
fn tool_permission_target_summary_empty_string_input() {
    let value = json!({
        "permission_request": {"updated_input": ""}
    });
    assert_eq!(tool_permission_target_summary("bash", &value), None);
}

#[test]
fn tool_permission_target_summary_whitespace_string_input() {
    let value = json!({
        "permission_request": {"updated_input": "   "}
    });
    assert_eq!(tool_permission_target_summary("bash", &value), None);
}

#[test]
fn tool_permission_target_summary_numeric_input() {
    let value = json!({
        "permission_request": {"updated_input": 42}
    });
    let result = tool_permission_target_summary("bash", &value);
    assert!(result.is_some());
}

#[test]
fn tool_permission_target_summary_array_input() {
    let value = json!({
        "permission_request": {"updated_input": ["a", "b"]}
    });
    let result = tool_permission_target_summary("bash", &value);
    assert!(result.is_some());
}

#[test]
fn tool_permission_target_summary_truncates_long_string_fallbacks() {
    let value = json!({
        "permission_request": {"updated_input": "x".repeat(120)}
    });
    let result = tool_permission_target_summary("unknown_tool", &value).expect("summary");

    assert_eq!(result.chars().count(), 80);
    assert!(result.chars().all(|ch| ch == 'x'));
}

#[test]
fn tool_permission_target_summary_ignores_object_without_inline_summary() {
    let value = json!({
        "permission_request": {"updated_input": {"unexpected": true}}
    });

    assert_eq!(tool_permission_target_summary("unknown_tool", &value), None);
}

#[test]
fn permission_state_reads_request_reason_warning_and_summary_context() {
    let reason = json!({
        "status": "error",
        "permission_request": {"reason": " requires user approval "}
    });
    let warning = json!({
        "status": "error",
        "permission_request": {"warning": " forbidden by workspace policy "}
    });
    let summary = json!({
        "status": "error",
        "summary": "interactive approval required"
    });

    assert_eq!(tool_permission_state("bash", &reason), Some(ToolPermissionState::ApprovalRequired));
    assert_eq!(tool_permission_state("bash", &warning), Some(ToolPermissionState::Rejected));
    assert_eq!(
        tool_permission_state("bash", &summary),
        Some(ToolPermissionState::ApprovalRequired)
    );
}
