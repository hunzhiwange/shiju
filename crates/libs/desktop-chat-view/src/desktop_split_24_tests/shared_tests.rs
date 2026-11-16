use crate::panel::user_question_indices;
use session_ui_types::{ChatMessage, ChatRole};
fn chat_message(role: ChatRole, content: &str) -> ChatMessage {
    ChatMessage {
        role,
        content: content.into(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }
}
fn permission_target_tool_anchor_fraction(
    message: &ChatMessage,
    request: Option<&gateway_interaction::PendingPermissionRequestDto>,
    message_id: Option<&str>,
    show_reasoning_summary: bool,
) -> Option<f32> {
    let (_, visible, _) = crate::message_view::think::split_think(&message.content);
    let cache = crate::message_view::render_cache::build_render_cache_entry(
        &message.content,
        &visible,
        0,
        show_reasoning_summary,
    );
    let last = crate::message_view::assistant_body::deduped_tool_last_indices(&cache.blocks);
    crate::permission_anchor::permission_target_tool_anchor_fraction(
        message,
        request,
        message_id,
        &cache.blocks,
        &last,
    )
}
#[test]
fn permission_target_tool_anchor_fraction_separates_explore_groups_with_text() {
    let message = chat_message(
        ChatRole::Assistant,
        concat!(
            "tool read\n",
            "{\"status\":\"completed\",\"toolCallId\":\"call-1\"}\n",
            "阶段说明\n",
            "tool grep\n",
            "{\"status\":\"running\",\"toolCallId\":\"call-2\"}"
        ),
    );
    let request = gateway_interaction::PendingPermissionRequestDto {
        id: "perm-1".to_string(),
        session_id: "session-1".to_string(),
        permission: "grep".to_string(),
        patterns: Vec::new(),
        metadata: serde_json::Map::new(),
        always: Vec::new(),
        tool: Some(gateway_interaction::PendingPermissionToolDto {
            message_id: "msg-1".to_string(),
            call_id: "call-2".to_string(),
        }),
    };

    assert_eq!(
        permission_target_tool_anchor_fraction(&message, Some(&request), Some("msg-1"), false),
        Some(2.0 / 3.0)
    );
}

#[test]
fn permission_target_tool_anchor_fraction_clamps_first_and_last_tool_anchors() {
    let message = chat_message(
        ChatRole::Assistant,
        concat!(
            "tool bash\n",
            "{\"input\":\"one\",\"toolCallId\":\"call-1\"}\n",
            "tool bash\n",
            "{\"input\":\"two\",\"toolCallId\":\"call-2\"}\n",
            "tool bash\n",
            "{\"input\":\"three\",\"toolCallId\":\"call-3\"}\n",
            "tool bash\n",
            "{\"input\":\"four\",\"toolCallId\":\"call-4\"}"
        ),
    );
    let mut request = gateway_interaction::PendingPermissionRequestDto {
        id: "perm-1".to_string(),
        session_id: "session-1".to_string(),
        permission: "bash".to_string(),
        patterns: Vec::new(),
        metadata: serde_json::Map::new(),
        always: Vec::new(),
        tool: Some(gateway_interaction::PendingPermissionToolDto {
            message_id: "msg-1".to_string(),
            call_id: "call-1".to_string(),
        }),
    };

    assert_eq!(
        permission_target_tool_anchor_fraction(&message, Some(&request), Some("msg-1"), false),
        Some(0.22)
    );

    request.tool.as_mut().expect("tool metadata").call_id = "call-4".to_string();

    assert_eq!(
        permission_target_tool_anchor_fraction(&message, Some(&request), Some("msg-1"), false),
        Some(0.78)
    );
}

#[test]
fn permission_target_tool_anchor_fraction_uses_last_duplicate_tool_card() {
    let message = chat_message(
        ChatRole::Assistant,
        concat!(
            "tool bash\n",
            "{\"input\":\"echo same\",\"toolCallId\":\"call-1\"}\n",
            "tool bash\n",
            "{\"input\":\"echo same\",\"toolCallId\":\"call-2\"}"
        ),
    );
    let request = gateway_interaction::PendingPermissionRequestDto {
        id: "perm-1".to_string(),
        session_id: "session-1".to_string(),
        permission: "bash".to_string(),
        patterns: Vec::new(),
        metadata: serde_json::Map::new(),
        always: Vec::new(),
        tool: Some(gateway_interaction::PendingPermissionToolDto {
            message_id: "msg-1".to_string(),
            call_id: "call-2".to_string(),
        }),
    };

    assert_eq!(
        permission_target_tool_anchor_fraction(&message, Some(&request), Some("msg-1"), false),
        Some(0.50)
    );
}

#[test]
fn permission_target_tool_anchor_fraction_groups_trailing_explore_tools() {
    let message = chat_message(
        ChatRole::Assistant,
        concat!(
            "tool read\n",
            "{\"status\":\"completed\",\"toolCallId\":\"call-1\"}\n",
            "tool grep\n",
            "{\"status\":\"running\",\"toolCallId\":\"call-2\"}"
        ),
    );
    let request = gateway_interaction::PendingPermissionRequestDto {
        id: "perm-1".to_string(),
        session_id: "session-1".to_string(),
        permission: "grep".to_string(),
        patterns: Vec::new(),
        metadata: serde_json::Map::new(),
        always: Vec::new(),
        tool: Some(gateway_interaction::PendingPermissionToolDto {
            message_id: "msg-1".to_string(),
            call_id: "call-2".to_string(),
        }),
    };

    assert_eq!(
        permission_target_tool_anchor_fraction(&message, Some(&request), Some("msg-1"), false),
        Some(0.50)
    );
}

#[test]
fn permission_target_tool_anchor_fraction_returns_none_when_tool_call_is_absent() {
    let message = chat_message(
        ChatRole::Assistant,
        "tool bash\n{\"status\":\"running\",\"toolCallId\":\"call-1\"}",
    );
    let request = gateway_interaction::PendingPermissionRequestDto {
        id: "perm-1".to_string(),
        session_id: "session-1".to_string(),
        permission: "bash".to_string(),
        patterns: Vec::new(),
        metadata: serde_json::Map::new(),
        always: Vec::new(),
        tool: Some(gateway_interaction::PendingPermissionToolDto {
            message_id: "msg-1".to_string(),
            call_id: "missing-call".to_string(),
        }),
    };

    assert_eq!(
        permission_target_tool_anchor_fraction(&message, Some(&request), Some("msg-1"), false),
        None
    );
}

#[test]
fn permission_target_tool_anchor_fraction_centers_matching_message_without_tools() {
    let message = chat_message(ChatRole::Assistant, "没有工具调用");
    let request = gateway_interaction::PendingPermissionRequestDto {
        id: "perm-1".to_string(),
        session_id: "session-1".to_string(),
        permission: "bash".to_string(),
        patterns: Vec::new(),
        metadata: serde_json::Map::new(),
        always: Vec::new(),
        tool: Some(gateway_interaction::PendingPermissionToolDto {
            message_id: "msg-1".to_string(),
            call_id: "call-1".to_string(),
        }),
    };

    assert_eq!(
        permission_target_tool_anchor_fraction(&message, Some(&request), Some("msg-1"), false),
        Some(0.50)
    );
}

#[test]
fn permission_target_tool_anchor_fraction_handles_non_tool_roles_and_empty_tool_list() {
    let user = ChatMessage {
        model: None,
        role: ChatRole::User,
        content: "问题".to_string(),
        think_timing: Vec::new(),
        processing_timing: None,
    };
    let assistant = ChatMessage {
        role: ChatRole::Assistant,
        content: "没有工具调用".to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    };
    let request = gateway_interaction::PendingPermissionRequestDto {
        id: "perm-1".to_string(),
        session_id: "session-1".to_string(),
        permission: "bash".to_string(),
        patterns: Vec::new(),
        metadata: serde_json::Map::new(),
        always: Vec::new(),
        tool: None,
    };

    assert_eq!(
        permission_target_tool_anchor_fraction(&user, Some(&request), Some("msg-1"), false),
        None
    );
    assert_eq!(
        permission_target_tool_anchor_fraction(&assistant, Some(&request), Some("msg-1"), false),
        None
    );
}

#[test]
fn permission_target_tool_anchor_fraction_rejects_non_matching_request() {
    let message = ChatMessage {
        role: ChatRole::Assistant,
        content: "tool bash\n{\"status\":\"running\",\"toolCallId\":\"call-1\"}".to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    };
    let request = gateway_interaction::PendingPermissionRequestDto {
        id: "perm-1".to_string(),
        session_id: "session-1".to_string(),
        permission: "bash".to_string(),
        patterns: Vec::new(),
        metadata: serde_json::Map::new(),
        always: Vec::new(),
        tool: Some(gateway_interaction::PendingPermissionToolDto {
            message_id: "other".to_string(),
            call_id: "call-1".to_string(),
        }),
    };

    assert_eq!(
        permission_target_tool_anchor_fraction(&message, Some(&request), Some("msg-1"), false),
        None
    );
}

#[test]
fn permission_target_tool_anchor_fraction_uses_tool_row_anchor() {
    let message = ChatMessage {
        role: ChatRole::Tool,
        content: "tool bash\n{\"status\":\"running\",\"toolCallId\":\"call-1\"}".to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    };
    let request = gateway_interaction::PendingPermissionRequestDto {
        id: "perm-1".to_string(),
        session_id: "session-1".to_string(),
        permission: "bash".to_string(),
        patterns: Vec::new(),
        metadata: serde_json::Map::new(),
        always: Vec::new(),
        tool: Some(gateway_interaction::PendingPermissionToolDto {
            message_id: "msg-1".to_string(),
            call_id: "call-1".to_string(),
        }),
    };

    assert_eq!(
        permission_target_tool_anchor_fraction(&message, Some(&request), Some("msg-1"), false),
        Some(0.30)
    );
}

#[test]
fn permission_target_tool_anchor_fraction_prefers_later_tool_card() {
    let message = ChatMessage {
        role: ChatRole::Assistant,
        content: concat!(
            "说明\n",
            "tool grep\n",
            "{\"status\":\"completed\",\"toolCallId\":\"call-1\",\"output\":\"a.rs:1\"}\n",
            "tool web_search\n",
            "{\"status\":\"completed\",\"toolCallId\":\"call-2\",\"output\":\"1. result\"}"
        )
        .to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    };
    let request = gateway_interaction::PendingPermissionRequestDto {
        id: "perm-1".to_string(),
        session_id: "session-1".to_string(),
        permission: "web_search".to_string(),
        patterns: Vec::new(),
        metadata: serde_json::Map::new(),
        always: Vec::new(),
        tool: Some(gateway_interaction::PendingPermissionToolDto {
            message_id: "msg-1".to_string(),
            call_id: "call-2".to_string(),
        }),
    };

    let fraction =
        permission_target_tool_anchor_fraction(&message, Some(&request), Some("msg-1"), false);

    assert_eq!(fraction, Some(2.0 / 3.0));
}

#[test]
fn user_question_indices_returns_empty_without_user_messages() {
    let chat = vec![
        ChatMessage {
            role: ChatRole::Assistant,
            content: "只有回答".to_string(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        },
        ChatMessage {
            role: ChatRole::Tool,
            content: "tool read_file".to_string(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        },
    ];

    assert!(user_question_indices(&chat).is_empty());
}

#[test]
fn user_question_indices_collect_all_user_messages() {
    let chat = vec![
        ChatMessage {
            role: ChatRole::User,
            content: "第一个问题".to_string(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        },
        ChatMessage {
            role: ChatRole::Assistant,
            content: "第一个回答".to_string(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        },
        ChatMessage {
            role: ChatRole::User,
            content: "第二个问题".to_string(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        },
    ];

    assert_eq!(user_question_indices(&chat), vec![0, 2]);
}
