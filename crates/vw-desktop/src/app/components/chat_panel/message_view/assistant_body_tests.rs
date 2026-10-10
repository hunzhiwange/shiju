//! 验证助手消息中正文、思考块与工具块的分组、去重和渲染顺序。
//! 块序列 fixture 专门覆盖隐藏推理、尾随探索工具和可见边界触发的分组刷新。

use super::assistant_body::{
    count_visible_non_explore_tool_cards, deduped_tool_last_indices,
    deduped_tool_last_indices_with_metadata, render_assistant_body,
    should_highlight_pending_permission_tool,
};
use super::markdown::parse_markdown_blocks;
use crate::app::models::{ParsedChatBlock, ThinkTiming, ToolBlockRenderMetadata};
use crate::app::{App, Message};
use iced::Element;
use iced::advanced::widget::Tree;
use iced::widget::text_editor;
use serde_json::Map;

fn command_block(name: &str, call_id: &str, command: &str) -> ParsedChatBlock {
    ParsedChatBlock::Tool {
        raw: format!(
            "tool {name}\n{{\"tool_call_id\":\"{call_id}\",\"input\":\"{command}\",\"status\":\"completed\"}}"
        ),
    }
}

// 顶层节点数用于验证分组边界，不绑定各卡片内部容易变化的控件结构。
fn rendered_top_level_count(
    app: &App,
    blocks: Vec<ParsedChatBlock>,
    special_text_blocks: Vec<String>,
    think_timing: &[ThinkTiming],
) -> usize {
    let body = render_assistant_body(
        app,
        0,
        think_timing,
        special_text_blocks,
        String::new(),
        blocks,
        Vec::new(),
        true,
        false,
        "",
    );
    let element: Element<'_, Message> = body.into();
    Tree::new(element.as_widget()).children.len()
}

#[test]
fn pending_permission_highlight_matches_active_request_or_direct_target() {
    assert!(should_highlight_pending_permission_tool(None, None, true));
    assert!(should_highlight_pending_permission_tool(Some("req-1"), Some("req-1"), false));
    assert!(!should_highlight_pending_permission_tool(Some("req-1"), Some("req-2"), false));
    assert!(!should_highlight_pending_permission_tool(Some("req-1"), None, false));
    assert!(!should_highlight_pending_permission_tool(None, None, false));
}

#[test]
fn deduped_tool_last_indices_ignores_invalid_and_explore_blocks() {
    let blocks = vec![
        ParsedChatBlock::Text { content: "before".to_string() },
        ParsedChatBlock::Tool {
            raw: "tool read\n{\"input\":\"{\\\"filePath\\\":\\\"/tmp/a.rs\\\"}\",\"status\":\"completed\"}"
                .to_string(),
        },
        ParsedChatBlock::Tool { raw: "not a tool".to_string() },
        ParsedChatBlock::Tool {
            raw: "tool bash\n{\"input\":\"echo first\",\"status\":\"completed\"}".to_string(),
        },
        ParsedChatBlock::Tool {
            raw: "tool shell\n{\"input\":\"echo first\",\"status\":\"completed\"}".to_string(),
        },
    ];

    let last = deduped_tool_last_indices(&blocks);

    assert_eq!(last.len(), 1);
    assert_eq!(last.get("bash:echo first"), Some(&4));
}

#[test]
fn visible_non_explore_tool_count_skips_hidden_duplicates_and_post_explore_file_lists() {
    let blocks = vec![
        ParsedChatBlock::Tool {
            raw: "tool read\n{\"input\":\"{\\\"filePath\\\":\\\"src/main.rs\\\"}\",\"status\":\"completed\"}"
                .to_string(),
        },
        ParsedChatBlock::Text { content: "\n".to_string() },
        ParsedChatBlock::Tool {
            raw: "tool bash\n{\"input\":\"ls\",\"status\":\"completed\",\"output\":\"- src/main.rs\\n- src/lib.rs\"}"
                .to_string(),
        },
        ParsedChatBlock::Tool {
            raw: "tool bash\n{\"input\":\"pwd\",\"status\":\"completed\"}".to_string(),
        },
        ParsedChatBlock::Tool {
            raw: "tool shell\n{\"input\":\"pwd\",\"status\":\"completed\"}".to_string(),
        },
        ParsedChatBlock::Tool { raw: "not a tool".to_string() },
        ParsedChatBlock::Tool { raw: "tool custom\nnot-json".to_string() },
    ];
    let last = deduped_tool_last_indices(&blocks);

    assert_eq!(count_visible_non_explore_tool_cards(&blocks, &[], &last), 2);
}

#[test]
fn visible_text_closes_pending_explore_group_before_file_list_tool() {
    let blocks = vec![
        ParsedChatBlock::Tool {
            raw: "tool grep\n{\"input\":\"{\\\"pattern\\\":\\\"App\\\"}\",\"status\":\"completed\"}"
                .to_string(),
        },
        ParsedChatBlock::Text { content: "summary".to_string() },
        ParsedChatBlock::Tool {
            raw: "tool bash\n{\"input\":\"ls\",\"status\":\"completed\",\"output\":\"- src/main.rs\\n- src/lib.rs\"}"
                .to_string(),
        },
    ];
    let last = deduped_tool_last_indices(&blocks);

    assert_eq!(count_visible_non_explore_tool_cards(&blocks, &[], &last), 1);
}

#[test]
fn think_block_closes_pending_explore_group_before_file_list_tool_count() {
    let blocks = vec![
        ParsedChatBlock::Tool {
            raw: "tool grep\n{\"input\":\"{\\\"pattern\\\":\\\"App\\\"}\",\"status\":\"completed\"}"
                .to_string(),
        },
        ParsedChatBlock::Think { content: "reason".to_string(), open: false },
        ParsedChatBlock::Tool {
            raw: "tool bash\n{\"input\":\"ls\",\"status\":\"completed\",\"output\":\"- src/main.rs\\n- src/lib.rs\"}"
                .to_string(),
        },
    ];
    let last = deduped_tool_last_indices(&blocks);

    assert_eq!(count_visible_non_explore_tool_cards(&blocks, &[], &last), 1);
}

#[test]
fn cached_tool_metadata_drives_dedup_and_count_without_reparsing_raw_blocks() {
    let blocks = vec![
        ParsedChatBlock::Tool { raw: "invalid first raw".to_string() },
        ParsedChatBlock::Tool { raw: "invalid second raw".to_string() },
    ];
    let metadata = vec![
        Some(ToolBlockRenderMetadata {
            name: Some("bash".to_string()),
            identity: Some("bash:pwd".to_string()),
        }),
        Some(ToolBlockRenderMetadata {
            name: Some("bash".to_string()),
            identity: Some("bash:pwd".to_string()),
        }),
    ];

    let last = deduped_tool_last_indices_with_metadata(&blocks, &metadata);

    assert_eq!(last.get("bash:pwd"), Some(&1));
    assert_eq!(count_visible_non_explore_tool_cards(&blocks, &metadata, &last), 1);
}

#[test]
fn render_assistant_body_builds_plain_text_body() {
    let app = App::new().0;

    let body = render_assistant_body(
        &app,
        0,
        &[],
        Vec::new(),
        "hello".to_string(),
        Vec::new(),
        Vec::new(),
        false,
        false,
        "hello",
    );

    std::hint::black_box(body);
}

#[test]
fn special_markdown_text_uses_single_editor_for_cross_block_selection() {
    let mut app = App::new().0;
    let text = "# 标题\n\n第一段\n\n- 列表项";
    app.chat_special_text_editors.insert(0, text_editor::Content::with_text(text));
    app.chat_markdown_blocks.insert(0, parse_markdown_blocks(text));
    assert!(app.chat_markdown_blocks[&0].len() > 1);

    let body = render_assistant_body(
        &app,
        0,
        &[],
        vec![text.to_string()],
        String::new(),
        vec![ParsedChatBlock::Text { content: text.to_string() }],
        Vec::new(),
        true,
        false,
        text,
    );
    let element: Element<'_, Message> = body.into();
    let tree = Tree::new(element.as_widget());

    assert_eq!(tree.children.len(), 1);
    assert_eq!(tree.children[0].children.len(), 1);
}

#[test]
fn render_assistant_body_builds_api_error_card_for_special_text_block() {
    let app = App::new().0;
    let error = r#"{"name":"APIError","message":"bad gateway","is_retryable":true}"#;
    let blocks = vec![ParsedChatBlock::Text { content: error.to_string() }];

    let body = render_assistant_body(
        &app,
        0,
        &[],
        vec![error.to_string()],
        String::new(),
        blocks,
        Vec::new(),
        true,
        false,
        error,
    );

    std::hint::black_box(body);
}

#[test]
fn render_assistant_body_defers_text_before_trailing_explore_tools() {
    let app = App::new().0;
    let blocks = vec![
        ParsedChatBlock::Text { content: "summary".to_string() },
        ParsedChatBlock::Tool {
            raw:
                "tool grep\n{\"status\":\"completed\",\"input\":\"{\\\"pattern\\\":\\\"App\\\"}\"}"
                    .to_string(),
        },
        ParsedChatBlock::Text { content: "\n".to_string() },
    ];

    let body = render_assistant_body(
        &app,
        0,
        &[],
        vec!["summary".to_string()],
        String::new(),
        blocks,
        Vec::new(),
        true,
        false,
        "summary",
    );

    std::hint::black_box(body);
}

#[test]
fn render_assistant_body_handles_special_text_think_and_tool_blocks() {
    let app = App::new().0;
    let timing = vec![ThinkTiming { start_ms: 1, end_ms: Some(2), last_update_ms: 2 }];
    let blocks = vec![
        ParsedChatBlock::Think { content: "reason".to_string(), open: true },
        ParsedChatBlock::Tool {
            raw: "tool read\n{\"input\":\"{\\\"filePath\\\":\\\"src/main.rs\\\"}\",\"status\":\"completed\"}"
                .to_string(),
        },
        ParsedChatBlock::Text { content: "\n".to_string() },
        ParsedChatBlock::Text { content: "answer".to_string() },
        ParsedChatBlock::Tool {
            raw: "tool custom\n{\"status\":\"completed\",\"output\":\"done\"}".to_string(),
        },
    ];

    let body = render_assistant_body(
        &app,
        0,
        &timing,
        vec!["answer".to_string()],
        String::new(),
        blocks,
        Vec::new(),
        true,
        false,
        "answer",
    );

    std::hint::black_box(body);
}

#[test]
fn render_assistant_body_groups_explore_tools_before_visible_text() {
    let app = App::new().0;
    let blocks = vec![
        ParsedChatBlock::Tool {
            raw: "tool grep\n{\"status\":\"running\",\"input\":\"{\\\"pattern\\\":\\\"App\\\"}\"}"
                .to_string(),
        },
        ParsedChatBlock::Text { content: "summary".to_string() },
    ];

    let body = render_assistant_body(
        &app,
        0,
        &[],
        vec!["summary".to_string()],
        String::new(),
        blocks,
        Vec::new(),
        true,
        true,
        "summary",
    );

    std::hint::black_box(body);
}

#[test]
fn render_assistant_body_groups_adjacent_commands_into_one_top_level_summary() {
    let app = App::new().0;
    let blocks = vec![
        command_block("bash", "call-1", "printf first"),
        command_block("shell", "call-2", "printf 中文"),
    ];

    assert_eq!(rendered_top_level_count(&app, blocks, Vec::new(), &[]), 1);
}

#[test]
fn render_assistant_body_keeps_commands_grouped_across_hidden_reasoning() {
    let mut app = App::new().0;
    app.dialogue_flow_show_reasoning_summary = false;
    let blocks = vec![
        command_block("bash", "call-1", "printf first"),
        ParsedChatBlock::Think { content: "hidden reasoning".to_string(), open: false },
        command_block("shell", "call-2", "printf second"),
    ];

    assert_eq!(rendered_top_level_count(&app, blocks, Vec::new(), &[]), 1);
}

#[test]
fn render_assistant_body_flushes_command_groups_at_visible_boundaries() {
    let app = App::new().0;

    let text_blocks = vec![
        command_block("bash", "call-1", "first"),
        ParsedChatBlock::Text { content: "正文边界".to_string() },
        command_block("terminal", "call-2", "second"),
    ];
    assert_eq!(rendered_top_level_count(&app, text_blocks, vec!["正文边界".to_string()], &[]), 3);

    let other_tool_blocks = vec![
        command_block("bash", "call-1", "first"),
        ParsedChatBlock::Tool {
            raw: "tool custom_tool\n{\"input\":\"data\",\"status\":\"completed\"}".to_string(),
        },
        command_block("execute", "call-2", "second"),
    ];
    assert_eq!(rendered_top_level_count(&app, other_tool_blocks, Vec::new(), &[]), 3);

    let explore_blocks = vec![
        command_block("bash", "call-1", "first"),
        ParsedChatBlock::Tool {
            raw:
                "tool grep\n{\"input\":\"{\\\"pattern\\\":\\\"App\\\"}\",\"status\":\"completed\"}"
                    .to_string(),
        },
        command_block("bash", "call-2", "second"),
    ];
    assert_eq!(rendered_top_level_count(&app, explore_blocks, Vec::new(), &[]), 3);

    let think_blocks = vec![
        command_block("bash", "call-1", "first"),
        ParsedChatBlock::Think { content: "思考边界".to_string(), open: false },
        command_block("bash", "call-2", "second"),
    ];
    assert!(rendered_top_level_count(&app, think_blocks, Vec::new(), &[]) >= 2);
}

#[test]
fn render_assistant_body_keeps_pending_command_outside_summary_group() {
    let mut app = App::new().0;
    app.chat_message_ids = vec![Some("msg-1".to_string())];
    let request = vw_gateway_client::PendingPermissionRequestDto {
        id: "req-1".to_string(),
        session_id: "session-1".to_string(),
        permission: "tool".to_string(),
        patterns: vec![],
        metadata: Map::new(),
        always: vec![],
        tool: Some(vw_gateway_client::PendingPermissionToolDto {
            message_id: "msg-1".to_string(),
            call_id: "call-2".to_string(),
        }),
    };
    app.permission_modal_request_id = Some(request.id.clone());
    app.permission_modal_request = Some(request.clone());
    app.permission_modal_requests = vec![request];
    let blocks = vec![
        command_block("bash", "call-1", "safe"),
        command_block("bash", "call-2", "requires approval"),
    ];

    assert_eq!(rendered_top_level_count(&app, blocks, Vec::new(), &[]), 2);
}

#[test]
fn render_assistant_body_builds_api_error_card_without_special_blocks() {
    let app = App::new().0;
    let error = r#"{"name":"APIError","message":"request to https://api.example.test failed","is_retryable":false}"#;

    let body = render_assistant_body(
        &app,
        0,
        &[],
        Vec::new(),
        error.to_string(),
        Vec::new(),
        Vec::new(),
        false,
        false,
        error,
    );

    std::hint::black_box(body);
}

#[test]
fn render_assistant_body_falls_back_for_unknown_tool_block() {
    let app = App::new().0;
    let blocks = vec![ParsedChatBlock::Tool { raw: "not a tool".to_string() }];

    let body = render_assistant_body(
        &app,
        0,
        &[],
        Vec::new(),
        String::new(),
        blocks,
        Vec::new(),
        true,
        false,
        "not a tool",
    );

    std::hint::black_box(body);
}
