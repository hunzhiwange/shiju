//! 验证问题视图渲染。
//! 测试覆盖用户提问的文本、状态和交互入口。

use super::tools::question_request_targets_message;

fn request_with_message_id(message_id: &str) -> vw_shared::question::Request {
    vw_shared::question::Request {
        mcp_form: None,
        id: "q-1".to_string(),
        session_id: "session-1".to_string(),
        questions: vec![],
        tool: Some(vw_shared::question::ToolMeta {
            message_id: message_id.to_string(),
            call_id: "call-1".to_string(),
        }),
    }
}

#[test]
fn question_request_targets_matching_message_id() {
    let request = request_with_message_id("msg-1");

    assert!(question_request_targets_message(Some(&request), Some("msg-1")));
    assert!(!question_request_targets_message(Some(&request), Some("msg-2")));
}

#[test]
fn question_request_without_tool_meta_matches_current_message() {
    let request = vw_shared::question::Request {
        mcp_form: None,
        id: "q-1".to_string(),
        session_id: "session-1".to_string(),
        questions: vec![],
        tool: None,
    };

    assert!(question_request_targets_message(Some(&request), Some("msg-1")));
    assert!(question_request_targets_message(Some(&request), None));
}

#[test]
fn empty_request_never_matches_message() {
    assert!(!question_request_targets_message(None, Some("msg-1")));
}
