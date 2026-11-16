//! 纯响应预览的凭据清理、字符边界与 ACP 判定。

use super::{is_acp_request, response_preview, should_execute_structured_tool_calls_locally};
use serde_json::json;

#[test]
// 预览先清理凭据再截断，避免秘密在截断边界附近残留。
fn response_preview_trims_scrubs_credentials_and_truncates() {
    let secret = "abcdefghijklmnopqrstuvwxyz";
    let preview = response_preview(&format!("  token={secret}  "));

    assert_eq!(preview, "token=abcd*[REDACTED]");
    assert!(!preview.contains(secret));

    let long_preview = response_preview(&"x".repeat(300));
    assert!(long_preview.len() < 300);
    assert!(long_preview.ends_with("..."));
}

#[test]
fn response_preview_handles_empty_input_and_exact_character_limit() {
    assert_eq!(response_preview(" \n\t "), "");

    let at_limit = "你".repeat(240);
    assert_eq!(response_preview(&at_limit), at_limit);

    let over_limit = "你".repeat(241);
    assert_eq!(response_preview(&over_limit), format!("{}...", "你".repeat(240)));
}

#[test]
fn acp_detection_accepts_test_flag_or_non_empty_agent() {
    assert!(is_acp_request(&json!({"acp_test": true})));
    assert!(is_acp_request(&json!({"acp_agent": "copilot"})));
    assert!(!is_acp_request(&json!({"acp_test": false, "acp_agent": "  "})));
    assert!(!is_acp_request(&json!({})));
}

#[test]
fn acp_detection_ignores_wrong_types_and_trims_agent_names() {
    assert!(!is_acp_request(&json!({"acp_test": "true"})));
    assert!(!is_acp_request(&json!({"acp_agent": 7})));
    assert!(is_acp_request(&json!({"acp_test": false, "acp_agent": "  copilot  "})));
    assert!(is_acp_request(&json!({"acp_test": true, "acp_agent": "  "})));
}

#[test]
fn structured_tool_calls_run_locally_only_outside_acp() {
    assert!(!should_execute_structured_tool_calls_locally(&json!({"acp_test": true})));
    assert!(!should_execute_structured_tool_calls_locally(&json!({"acp_agent": "copilot"})));
    assert!(should_execute_structured_tool_calls_locally(&json!({"acp_test": false})));
    assert!(should_execute_structured_tool_calls_locally(&json!({"acp_agent": "  "})));
}
