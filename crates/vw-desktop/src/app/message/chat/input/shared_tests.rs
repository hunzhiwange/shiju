//! 处理聊天输入区的局部消息。
//! 本模块将编辑器操作、文件检索和工具细节限制在输入面板边界内。

use super::shared::{
    close_input_context_menu, locate_chat_message, locate_chat_message_index,
    permission_target_tool_anchor_fraction, preferred_chat_message_index_by_id,
    sync_global_input_editor_if_needed,
};
use crate::app::App;
use crate::app::components::chat_panel::height_index::CHAT_MESSAGE_GAP;
use crate::app::components::chat_panel::user_question_indices;
use crate::app::components::chat_panel::virtualizer::ChatVirtualizer;
use crate::app::models::{ChatMessage, ChatRole};
use iced::widget::text_editor;

fn test_app() -> App {
    App::new().0
}

fn chat_message(role: ChatRole, content: &str) -> ChatMessage {
    ChatMessage {
        role,
        content: content.to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }
}

#[test]
fn close_input_context_menu_clears_open_state_and_position() {
    let mut app = test_app();
    app.input_context_menu_open = true;
    app.input_context_menu_pos = Some((12.0, 24.0));

    close_input_context_menu(&mut app);

    assert!(!app.input_context_menu_open);
    assert_eq!(app.input_context_menu_pos, None);
}

#[test]
fn sync_global_input_editor_copies_only_without_active_session() {
    let mut app = test_app();
    app.input_editor = text_editor::Content::with_text("stale");
    app.current_session_runtime_mut().input_editor = text_editor::Content::with_text("draft");

    sync_global_input_editor_if_needed(&mut app);

    assert_eq!(app.input_editor.text(), "draft");

    app.active_session_id = Some("session-1".to_string());
    app.input_editor = text_editor::Content::with_text("visible");
    app.current_session_runtime_mut().input_editor = text_editor::Content::with_text("hidden");

    sync_global_input_editor_if_needed(&mut app);

    assert_eq!(app.input_editor.text(), "visible");
}

#[test]
fn locate_chat_message_returns_none_for_missing_message_id() {
    let mut app = test_app();
    app.chat = vec![chat_message(ChatRole::Assistant, "answer")];
    app.chat_message_ids = vec![Some("msg-1".to_string())];
    app.chat_auto_scroll = true;

    let _task = locate_chat_message(&mut app, "missing");

    assert!(app.chat_auto_scroll);
}

#[test]
fn locate_chat_message_index_disables_auto_scroll_for_valid_index() {
    let mut app = test_app();
    app.chat =
        vec![chat_message(ChatRole::User, "question"), chat_message(ChatRole::Assistant, "answer")];
    app.chat_message_ids = vec![Some("msg-1".to_string()), Some("msg-2".to_string())];
    app.chat_auto_scroll = true;
    app.chat_virtualizer.set_viewport_height(200.0);
    app.sync_chat_virtualizer_compat_fields();

    let _task = locate_chat_message_index(&mut app, 1);

    assert!(!app.chat_auto_scroll);
    assert_eq!(app.chat_message_estimated_heights.len(), app.chat.len());
}

#[test]
fn locate_chat_message_index_preserves_scroll_state_for_out_of_bounds_index() {
    let mut app = test_app();
    app.chat = vec![chat_message(ChatRole::Assistant, "answer")];
    app.chat_auto_scroll = true;

    let _task = locate_chat_message_index(&mut app, 1);

    assert!(app.chat_auto_scroll);
    assert!(app.chat_message_estimated_heights.is_empty());
}

#[test]
fn locate_chat_message_index_schedules_missing_chunk_for_active_session() {
    let mut app = test_app();
    app.active_session_id = Some("session-1".to_string());
    app.chat = vec![chat_message(ChatRole::Assistant, "answer")];

    let _task = locate_chat_message_index(&mut app, 0);

    assert!(app.active_session_view_state.preparing_chat_ui_chunks.contains(&0));
}

#[test]
fn locate_chat_message_index_does_not_reschedule_preparing_chunk() {
    let mut app = test_app();
    app.active_session_id = Some("session-1".to_string());
    app.chat = vec![chat_message(ChatRole::Assistant, "answer")];
    app.mark_chat_ui_chunks_preparing(&[0]);

    let _task = locate_chat_message_index(&mut app, 0);

    assert!(app.active_session_view_state.preparing_chat_ui_chunks.contains(&0));
    assert!(!app.active_session_view_state.dirty_preparing_chat_ui_chunks.contains(&0));
}

#[test]
fn chat_scroll_semantics_locate_message_uses_virtualizer_anchor() {
    let mut app = test_app();
    app.chat =
        (0..8).map(|idx| chat_message(ChatRole::Assistant, &format!("answer {idx}"))).collect();
    app.chat_message_ids = (0..8).map(|idx| Some(format!("msg-{idx}"))).collect();
    app.chat_auto_scroll = true;
    app.chat_virtualizer = ChatVirtualizer::from_estimated_heights(&[100.0; 8]);
    app.chat_virtualizer.set_viewport_height(220.0);
    app.chat_virtualizer.focus_turn_on_user(6);
    app.sync_chat_virtualizer_compat_fields();
    for idx in 0..app.chat.len() {
        app.chat_message_measured_heights.insert(idx, 100.0);
    }

    let _task = locate_chat_message_index(&mut app, 4);

    let expected_scroll_top = 4.0 * (100.0 + CHAT_MESSAGE_GAP) + 100.0 * 0.50 - 220.0 * 0.18;
    assert!(!app.chat_auto_scroll);
    assert_eq!(app.chat_virtualizer.turn_focus_user_index(), None);
    assert!((app.chat_virtualizer.scroll_top_px() - expected_scroll_top).abs() < 0.01);
    assert_eq!(app.chat_scroll_offset_y, app.chat_virtualizer.relative_y());
}

#[test]
fn preferred_chat_message_index_prefers_tool_row_for_duplicate_message_ids() {
    let chat = vec![
        ChatMessage {
            role: ChatRole::Assistant,
            content: "分析中".to_string(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        },
        ChatMessage {
            role: ChatRole::Tool,
            content: "tool grep\n{\"status\":\"completed\",\"output\":\"2 matches\"}\n".to_string(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        },
    ];
    let message_ids = vec![Some("msg-1".to_string()), Some("msg-1".to_string())];

    assert_eq!(preferred_chat_message_index_by_id(&chat, &message_ids, "msg-1"), Some(1));
}

#[test]
fn preferred_chat_message_index_falls_back_to_first_match_without_tool_row() {
    let chat = vec![
        ChatMessage {
            role: ChatRole::Assistant,
            content: "第一段".to_string(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        },
        ChatMessage {
            role: ChatRole::Assistant,
            content: "第二段".to_string(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        },
    ];
    let message_ids = vec![Some("msg-2".to_string()), Some("msg-2".to_string())];

    assert_eq!(preferred_chat_message_index_by_id(&chat, &message_ids, "msg-2"), Some(0));
}

#[test]
fn preferred_chat_message_index_returns_none_for_missing_id() {
    let chat = vec![ChatMessage {
        role: ChatRole::Assistant,
        content: "回答".to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }];
    let message_ids = vec![Some("msg-1".to_string())];

    assert_eq!(preferred_chat_message_index_by_id(&chat, &message_ids, "missing"), None);
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
    let request = vw_gateway_client::PendingPermissionRequestDto {
        id: "perm-1".to_string(),
        session_id: "session-1".to_string(),
        permission: "web_search".to_string(),
        patterns: Vec::new(),
        metadata: serde_json::Map::new(),
        always: Vec::new(),
        tool: Some(vw_gateway_client::PendingPermissionToolDto {
            message_id: "msg-1".to_string(),
            call_id: "call-2".to_string(),
        }),
    };

    let fraction =
        permission_target_tool_anchor_fraction(&message, Some(&request), Some("msg-1"), false);

    assert_eq!(fraction, Some(2.0 / 3.0));
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
    let request = vw_gateway_client::PendingPermissionRequestDto {
        id: "perm-1".to_string(),
        session_id: "session-1".to_string(),
        permission: "bash".to_string(),
        patterns: Vec::new(),
        metadata: serde_json::Map::new(),
        always: Vec::new(),
        tool: Some(vw_gateway_client::PendingPermissionToolDto {
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
fn permission_target_tool_anchor_fraction_rejects_non_matching_request() {
    let message = ChatMessage {
        role: ChatRole::Assistant,
        content: "tool bash\n{\"status\":\"running\",\"toolCallId\":\"call-1\"}".to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    };
    let request = vw_gateway_client::PendingPermissionRequestDto {
        id: "perm-1".to_string(),
        session_id: "session-1".to_string(),
        permission: "bash".to_string(),
        patterns: Vec::new(),
        metadata: serde_json::Map::new(),
        always: Vec::new(),
        tool: Some(vw_gateway_client::PendingPermissionToolDto {
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
    let request = vw_gateway_client::PendingPermissionRequestDto {
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
fn permission_target_tool_anchor_fraction_centers_matching_message_without_tools() {
    let message = chat_message(ChatRole::Assistant, "没有工具调用");
    let request = vw_gateway_client::PendingPermissionRequestDto {
        id: "perm-1".to_string(),
        session_id: "session-1".to_string(),
        permission: "bash".to_string(),
        patterns: Vec::new(),
        metadata: serde_json::Map::new(),
        always: Vec::new(),
        tool: Some(vw_gateway_client::PendingPermissionToolDto {
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
fn permission_target_tool_anchor_fraction_returns_none_when_tool_call_is_absent() {
    let message = chat_message(
        ChatRole::Assistant,
        "tool bash\n{\"status\":\"running\",\"toolCallId\":\"call-1\"}",
    );
    let request = vw_gateway_client::PendingPermissionRequestDto {
        id: "perm-1".to_string(),
        session_id: "session-1".to_string(),
        permission: "bash".to_string(),
        patterns: Vec::new(),
        metadata: serde_json::Map::new(),
        always: Vec::new(),
        tool: Some(vw_gateway_client::PendingPermissionToolDto {
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
    let request = vw_gateway_client::PendingPermissionRequestDto {
        id: "perm-1".to_string(),
        session_id: "session-1".to_string(),
        permission: "grep".to_string(),
        patterns: Vec::new(),
        metadata: serde_json::Map::new(),
        always: Vec::new(),
        tool: Some(vw_gateway_client::PendingPermissionToolDto {
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
    let request = vw_gateway_client::PendingPermissionRequestDto {
        id: "perm-1".to_string(),
        session_id: "session-1".to_string(),
        permission: "bash".to_string(),
        patterns: Vec::new(),
        metadata: serde_json::Map::new(),
        always: Vec::new(),
        tool: Some(vw_gateway_client::PendingPermissionToolDto {
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
    let mut request = vw_gateway_client::PendingPermissionRequestDto {
        id: "perm-1".to_string(),
        session_id: "session-1".to_string(),
        permission: "bash".to_string(),
        patterns: Vec::new(),
        metadata: serde_json::Map::new(),
        always: Vec::new(),
        tool: Some(vw_gateway_client::PendingPermissionToolDto {
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
    let request = vw_gateway_client::PendingPermissionRequestDto {
        id: "perm-1".to_string(),
        session_id: "session-1".to_string(),
        permission: "grep".to_string(),
        patterns: Vec::new(),
        metadata: serde_json::Map::new(),
        always: Vec::new(),
        tool: Some(vw_gateway_client::PendingPermissionToolDto {
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
