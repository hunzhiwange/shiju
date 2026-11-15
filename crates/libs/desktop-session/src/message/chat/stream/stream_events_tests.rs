use session_ui_types::{ChatMessage, ChatRole, ThinkTiming};
use super::tail::StreamTailState;
use super::stream_events::*;
fn message(role: ChatRole, content: &str) -> ChatMessage { ChatMessage { role, content: content.into(), model: None, think_timing: Vec::new(), processing_timing: None } }
#[test]
fn update_think_timing_ignores_empty_delta() {
    let mut message = message(ChatRole::Assistant, "<think>open");
    let state = StreamTailState::default();
    message.think_timing.push(ThinkTiming { start_ms: 1, end_ms: None, last_update_ms: 1 });

    update_think_timing_from_delta(&mut message, &state, 99);

    assert_eq!(message.think_timing[0].last_update_ms, 1);
    assert_eq!(message.think_timing[0].end_ms, None);
}

#[test]
fn update_think_timing_ignores_plain_delta_without_think_blocks() {
    let mut message = message(ChatRole::Assistant, "plain");
    let mut state = StreamTailState::default();
    state.apply_delta(" text");

    update_think_timing_from_delta(&mut message, &state, 42);

    assert!(message.think_timing.is_empty());
}

#[test]
fn update_think_timing_opens_and_closes_think_block() {
    let mut message = message(ChatRole::Assistant, "");
    let mut state = StreamTailState::default();

    state.apply_delta("<think>working");
    update_think_timing_from_delta(&mut message, &state, 10);
    message.content.push_str("<think>working");
    state.apply_delta("</think>done");
    update_think_timing_from_delta(&mut message, &state, 25);

    assert_eq!(message.think_timing.len(), 1);
    assert_eq!(message.think_timing[0].start_ms, 10);
    assert_eq!(message.think_timing[0].end_ms, Some(25));
    assert_eq!(message.think_timing[0].last_update_ms, 25);
}

#[test]
fn update_think_timing_closes_previous_block_and_keeps_latest_open() {
    let mut message = message(ChatRole::Assistant, "");
    let mut state = StreamTailState::default();

    state.apply_delta("<think>first</think><think>second");
    update_think_timing_from_delta(&mut message, &state, 40);

    assert_eq!(message.think_timing.len(), 2);
    assert_eq!(message.think_timing[0].end_ms, Some(40));
    assert_eq!(message.think_timing[0].last_update_ms, 40);
    assert_eq!(message.think_timing[1].end_ms, None);
    assert_eq!(message.think_timing[1].last_update_ms, 40);
}

#[test]
fn stream_done_message_ids_resize_and_assign_last_assistant_and_parent_user() {
    let chat = vec![
        message(ChatRole::User, "first"),
        message(ChatRole::Assistant, "old"),
        message(ChatRole::User, "latest"),
        message(ChatRole::Assistant, "answer"),
    ];
    let mut ids = vec![Some("first-id".to_string())];

    apply_stream_done_message_ids(
        &chat,
        &mut ids,
        Some("assistant-id".to_string()),
        Some("parent-id".to_string()),
    );

    assert_eq!(ids.len(), 4);
    assert_eq!(ids[0].as_deref(), Some("first-id"));
    assert_eq!(ids[2].as_deref(), Some("parent-id"));
    assert_eq!(ids[3].as_deref(), Some("assistant-id"));
}

#[test]
fn stream_done_message_ids_ignore_assistant_id_without_assistant_message() {
    let chat = vec![
        message(ChatRole::System, "context"),
        message(ChatRole::User, "question"),
        message(ChatRole::Tool, "result"),
    ];
    let mut ids = Vec::new();

    apply_stream_done_message_ids(
        &chat,
        &mut ids,
        Some("unused-assistant-id".to_string()),
        Some("parent-id".to_string()),
    );

    assert_eq!(ids, vec![None, Some("parent-id".to_string()), None]);
}

#[test]
fn message_ids_tail_returns_at_most_four_entries() {
    let ids = vec![Some("1".to_string()), None, Some("3".to_string()), None, Some("5".to_string())];

    assert_eq!(
        message_ids_tail(&ids),
        vec![None, Some("3".to_string()), None, Some("5".to_string())]
    );
}

#[test]
fn message_ids_tail_preserves_short_and_empty_inputs() {
    let ids = vec![Some("1".to_string()), None];

    assert_eq!(message_ids_tail(&ids), ids);
    assert!(message_ids_tail(&[]).is_empty());
}

#[test]
// 工具块解析要求行边界和完整 JSON，避免普通聊天文本被误识别为工具调用。
fn parse_tool_block_len_requires_tool_prefix_and_complete_json() {
    let block = "tool shell\n{\"tool_call_id\":\"call-1\",\"status\":\"ok\"}\ntrailing";

    assert_eq!(parse_tool_block_len(block), Some(block.find("trailing").unwrap()));
    assert_eq!(parse_tool_block_len("prefix tool shell\n{}"), None);
    assert_eq!(parse_tool_block_len("tool shell\n{"), None);
}

#[test]
fn parse_tool_block_len_accepts_multiline_json_without_trailing_newline() {
    let block = "tool shell\n{\n  \"tool_call_id\": \"call-1\",\n  \"status\": \"ok\"\n}";

    assert_eq!(parse_tool_block_len(block), Some(block.len()));
}

#[test]
fn tool_call_id_from_raw_block_rejects_missing_or_non_string_ids() {
    assert_eq!(
        tool_call_id_from_raw_block("tool shell\n{\"tool_call_id\":\"call-1\"}"),
        Some("call-1".to_string())
    );
    assert_eq!(tool_call_id_from_raw_block("tool shell"), None);
    assert_eq!(tool_call_id_from_raw_block("tool shell\nnot-json"), None);
    assert_eq!(tool_call_id_from_raw_block("tool shell\n{\"tool_call_id\":7}"), None);
}

#[test]
fn tool_block_start_allowed_accepts_line_start_and_colon_prefix_only() {
    assert!(tool_block_start_allowed("tool shell\n{}", 0));
    let ascii_colon = "工具:\ntool shell\n{}";
    let full_width_colon = "工具：\ntool shell\n{}";
    assert!(tool_block_start_allowed(ascii_colon, ascii_colon.find("tool").unwrap()));
    assert!(tool_block_start_allowed(
        full_width_colon,
        full_width_colon.find("tool").unwrap()
    ));
    assert!(!tool_block_start_allowed("inline tool shell\n{}", 7));
}

#[test]
fn tool_block_start_allowed_accepts_same_line_colon_after_whitespace() {
    let content = "工具： \ttool shell\n{}";
    let start = content.find("tool").unwrap();

    assert!(tool_block_start_allowed(content, start));
    assert!(!tool_block_start_allowed("label  tool shell\n{}", 7));
}

#[test]
fn workflow_tool_block_upsert_replaces_matching_call_id() {
    let running = r#"tool workflow_node
{"tool_call_id":"workflow-node-2-llm","status":"running","summary":"Run Task · llm"}
"#;
    let finished = r#"tool workflow_node
{"tool_call_id":"workflow-node-2-llm","status":"completed","summary":"Run Task · llm","output":"ok"}
"#;
    let mut content = String::new();

    upsert_tool_block_by_call_id(&mut content, running);
    upsert_tool_block_by_call_id(&mut content, finished);

    assert_eq!(content.matches("tool workflow_node").count(), 1);
    assert!(content.contains("\"status\":\"completed\""));
    assert!(!content.contains("\"status\":\"running\""));
}

#[test]
fn workflow_tool_block_upsert_appends_when_call_id_is_missing_or_different() {
    let mut content = "prefix".to_string();

    upsert_tool_block_by_call_id(&mut content, "tool shell\n{\"status\":\"ok\"}\n");
    upsert_tool_block_by_call_id(
        &mut content,
        "tool shell\n{\"tool_call_id\":\"call-2\",\"status\":\"ok\"}\n",
    );

    assert_eq!(content.matches("tool shell").count(), 2);
    assert!(content.ends_with('\n'));
}

#[test]
fn workflow_tool_block_upsert_ignores_inline_text_that_looks_like_a_tool_block() {
    let mut content =
        "documentation tool shell\n{\"tool_call_id\":\"call-1\",\"status\":\"old\"}\n".to_string();

    upsert_tool_block_by_call_id(
        &mut content,
        "tool shell\n{\"tool_call_id\":\"call-1\",\"status\":\"new\"}\n",
    );

    assert_eq!(content.matches("tool shell").count(), 2);
    assert!(content.contains("\"status\":\"old\""));
    assert!(content.contains("\"status\":\"new\""));
}

#[test]
fn append_tool_block_normalizes_separators_and_trailing_newlines() {
    let mut content = "prefix".to_string();

    append_tool_block(&mut content, "tool shell\n{}\n\n");

    assert_eq!(content, "prefix\ntool shell\n{}\n");
}

#[test]
fn append_stream_delta_to_tail_appends_or_creates_assistant_message() {
    let mut assistant_chat = vec![message(ChatRole::Assistant, "hello")];
    let mut assistant_ids = vec![Some("assistant-id".to_string())];

    let changed_idx = append_stream_delta_to_tail(
        &mut assistant_chat,
        &mut assistant_ids,
        " world".to_string(),
        10,
        &mut StreamTailState::default(),
    );

    assert_eq!(changed_idx, 0);
    assert_eq!(assistant_chat[0].content, "hello world");
    assert_eq!(assistant_ids, vec![Some("assistant-id".to_string())]);

    let mut user_chat = vec![message(ChatRole::User, "question")];
    let mut user_ids = Vec::new();
    let changed_idx = append_stream_delta_to_tail(
        &mut user_chat,
        &mut user_ids,
        "answer".to_string(),
        20,
        &mut StreamTailState::default(),
    );

    assert_eq!(changed_idx, 1);
    assert_eq!(user_chat[1].role, ChatRole::Assistant);
    assert_eq!(user_chat[1].content, "answer");
    assert!(user_chat[1].think_timing.is_empty());
    assert_eq!(user_ids, vec![None, None]);
}

#[test]
fn upsert_workflow_tool_to_tail_reuses_assistant_or_creates_one() {
    let running = "tool workflow_node\n{\"tool_call_id\":\"node-1\",\"status\":\"running\"}\n";
    let finished = "tool workflow_node\n{\"tool_call_id\":\"node-1\",\"status\":\"completed\"}\n";
    let mut assistant_chat = vec![message(ChatRole::Assistant, running)];
    let mut assistant_ids = vec![Some("assistant-id".to_string())];

    let changed_idx = upsert_workflow_tool_to_tail(
        &mut assistant_chat,
        &mut assistant_ids,
        finished.to_string(),
    );

    assert_eq!(changed_idx, 0);
    assert!(assistant_chat[0].content.contains("\"status\":\"completed\""));
    assert!(!assistant_chat[0].content.contains("\"status\":\"running\""));
    assert_eq!(assistant_ids, vec![Some("assistant-id".to_string())]);

    let mut user_chat = vec![message(ChatRole::User, "question")];
    let mut user_ids = Vec::new();
    let changed_idx =
        upsert_workflow_tool_to_tail(&mut user_chat, &mut user_ids, running.to_string());

    assert_eq!(changed_idx, 1);
    assert_eq!(user_chat[1].role, ChatRole::Assistant);
    assert!(user_chat[1].content.contains("\"status\":\"running\""));
    assert_eq!(user_ids, vec![None, None]);
}

#[test]
fn continuation_label_from_history_uses_last_user_message() {
    let history = vec![
        message(ChatRole::User, "first"),
        message(ChatRole::Assistant, "answer"),
        message(ChatRole::User, "second"),
    ];

    assert_eq!(continuation_label_from_history(&history), "second");
    assert_eq!(continuation_label_from_history(&[]), "");
}

#[test]
fn stream_tail_height_uses_delta_without_full_message_scan() {
    let mut state = StreamTailState::default();

    state.apply_delta("short");
    let initial_height = state.estimated_height();
    state.apply_delta("\nsecond line\nthird line");

    assert_eq!(state.processed_bytes(), "short\nsecond line\nthird line".len());
    assert!(state.estimated_height() > initial_height);
}
