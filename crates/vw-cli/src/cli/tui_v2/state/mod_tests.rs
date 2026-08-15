use serde_json::json;
use vw_shared::session::ui_types::{
    ChatMessage, ChatRole, ChatSession, ChatSessionMeta, ChatSessionStep, TokenUsage,
};

use super::{
    TuiModelCatalogEntry, TuiScrollState, TuiSessionPreview, TuiState, is_persistable_chat_message,
    persisted_slot_index_for_message_index, raw_message_id_from_ui_message,
};
use crate::cli::tui_v2::model::{
    UiMessage, UiMessageBase, UiMessageId, UiSystemMessage, UiSystemMessageLevel, UiToolResult,
    UiTurnTerminal,
};

fn session_with_step(finish_reason: Option<&str>, finished_ms: Option<u64>) -> ChatSession {
    ChatSession {
        id: "s1".to_string(),
        title: "Session".to_string(),
        messages: vec![ChatMessage {
            role: ChatRole::Assistant,
            content: "answer".to_string(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        }],
        message_ids: vec![Some("assistant-1".to_string())],
        calls: Vec::new(),
        steps: vec![ChatSessionStep {
            index: 1,
            started_ms: 10,
            finished_ms,
            start_snapshot_path: Some("start.json".to_string()),
            finish_snapshot_path: Some("finish.json".to_string()),
            usage: TokenUsage::default(),
            cost_usd: None,
            finish_reason: finish_reason.map(ToOwned::to_owned),
            model: Some("model-a".to_string()),
            tool_timings: Vec::new(),
        }],
        created_ms: 1,
        updated_ms: 2,
    }
}

#[test]
fn scroll_clamps_and_sticky_state_follow_anchor_rules() {
    let mut scroll = TuiScrollState {
        top_message: 99,
        viewport_messages: 10,
        viewport_height: 10,
        viewport_width: 80,
        overscan: 2,
        follow_tail: false,
        sticky_message: None,
        last_seen_message: None,
    };

    scroll.clamp(3);
    assert_eq!(scroll.top_message, 2);
    assert_eq!(scroll.sticky_message, Some(1));

    scroll.snap_to_tail(0);
    assert_eq!(scroll.top_message, 0);
    assert_eq!(scroll.sticky_message, None);

    scroll.sync_viewport(24, 100);
    assert_eq!(scroll.viewport_messages, 24);
    assert_eq!(scroll.viewport_height, 24);
    assert_eq!(scroll.viewport_width, 100);

    scroll.top_message = 4;
    scroll.clamp_to_anchors(&[0, 2, 5]);
    assert_eq!(scroll.top_message, 2);
    assert_eq!(scroll.sticky_message, Some(0));

    scroll.snap_to_tail_anchors(&[0, 2, 5]);
    assert_eq!(scroll.top_message, 5);
    assert_eq!(scroll.sticky_message, Some(2));

    scroll.clamp_to_anchors(&[]);
    assert_eq!(scroll.top_message, 0);
    assert_eq!(scroll.sticky_message, None);
}

#[test]
fn session_preview_conversions_and_refresh_cover_empty_session_id() {
    let meta = ChatSessionMeta {
        id: "s1".to_string(),
        title: "Meta".to_string(),
        updated_ms: 9,
        message_count: 2,
        call_count: 3,
        last_content: Some("last".to_string()),
    };

    assert_eq!(TuiSessionPreview::from(&meta), TuiSessionPreview::from(meta));

    let mut state = TuiState::default();
    state.session.title = "No id".to_string();
    state.refresh_session_preview();
    assert!(state.session.preview.is_none());
    assert_eq!(state.status.session_title, "No id");
}

#[test]
fn tui_state_defaults_to_visible_execution_details() {
    assert!(TuiState::default().status.verbose);
}

#[test]
fn session_preview_from_chat_session_uses_last_message_and_snapshot_counts() {
    let session = ChatSession {
        id: "snapshot".to_string(),
        title: "Snapshot".to_string(),
        messages: vec![ChatMessage {
            role: ChatRole::Assistant,
            content: "latest answer".to_string(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        }],
        message_ids: vec![None],
        calls: vec![json!({"call": 1}), json!({"call": 2})],
        steps: Vec::new(),
        created_ms: 1,
        updated_ms: 9,
    };

    let preview = TuiSessionPreview::from_chat_session(&session);

    assert_eq!(preview.id, "snapshot");
    assert_eq!(preview.message_count, 1);
    assert_eq!(preview.call_count, 2);
    assert_eq!(preview.last_content.as_deref(), Some("latest answer"));
}

#[test]
fn session_preview_hides_reasoning_marker_and_content() {
    let session = ChatSession {
        id: "snapshot".to_string(),
        title: "Snapshot".to_string(),
        messages: vec![ChatMessage {
            role: ChatRole::Assistant,
            content: "latest answer\n\n[reasoning]\nprivate chain".to_string(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        }],
        message_ids: vec![None],
        calls: Vec::new(),
        steps: Vec::new(),
        created_ms: 1,
        updated_ms: 9,
    };

    let preview = TuiSessionPreview::from_chat_session(&session);

    assert_eq!(preview.last_content.as_deref(), Some("latest answer"));
}

#[test]
fn tool_snapshot_parsing_handles_json_error_and_plain_payloads() {
    let snapshot = ChatSession {
        id: "tools".to_string(),
        title: "Tools".to_string(),
        messages: vec![
            ChatMessage {
                role: ChatRole::Tool,
                content: "tool write\n{\"status\":\"error\",\"error\":\"denied\"}".to_string(),
                model: None,
                think_timing: Vec::new(),
                processing_timing: None,
            },
            ChatMessage {
                role: ChatRole::Tool,
                content: "plain output".to_string(),
                model: None,
                think_timing: Vec::new(),
                processing_timing: None,
            },
        ],
        message_ids: vec![None, None],
        calls: vec![json!({"call": 1})],
        steps: Vec::new(),
        created_ms: 1,
        updated_ms: 2,
    };

    let state = TuiState::from_chat_session(&snapshot);

    assert!(
        matches!(&state.messages[0], UiMessage::ToolResult(tool) if tool.tool_name == "write" && tool.content == "denied" && tool.is_error)
    );
    assert!(
        matches!(&state.messages[1], UiMessage::ToolResult(tool) if tool.tool_name == "tool" && tool.content == "plain output" && !tool.is_error)
    );
    assert_eq!(state.session.persisted_calls, snapshot.calls);
}

#[test]
fn snapshot_terminal_classifies_running_timeout_cancel_error_and_pending() {
    assert_eq!(
        TuiState::from_chat_session(&session_with_step(None, None)).status.turn_terminal,
        UiTurnTerminal::Streaming
    );
    assert_eq!(
        TuiState::from_chat_session(&session_with_step(Some("deadline exceeded"), Some(20)))
            .status
            .turn_terminal,
        UiTurnTerminal::TimedOut { message: "deadline exceeded".to_string() }
    );
    assert_eq!(
        TuiState::from_chat_session(&session_with_step(Some("cancelled"), Some(20)))
            .status
            .turn_terminal,
        UiTurnTerminal::Cancelled { reason: Some("cancelled".to_string()) }
    );
    assert_eq!(
        TuiState::from_chat_session(&session_with_step(Some("failed"), Some(20)))
            .status
            .turn_terminal,
        UiTurnTerminal::Error { message: "failed".to_string() }
    );
}

#[test]
fn persistable_message_helpers_track_slots_and_raw_gateway_ids() {
    let system = UiMessage::System(UiSystemMessage {
        base: UiMessageBase::new(UiMessageId::gateway("system-1")),
        text: "persist me".to_string(),
        level: UiSystemMessageLevel::Info,
    });
    let local_system = UiMessage::System(UiSystemMessage {
        base: UiMessageBase::new(UiMessageId::local("notice")),
        text: "local only".to_string(),
        level: UiSystemMessageLevel::Warning,
    });
    let tool = UiMessage::ToolResult(UiToolResult {
        base: UiMessageBase::new(UiMessageId::gateway("tool-1")),
        call_id: None,
        tool_name: "grep".to_string(),
        content: "ok".to_string(),
        is_error: false,
    });
    let messages = vec![system.clone(), local_system.clone(), tool.clone()];

    assert!(is_persistable_chat_message(&system));
    assert!(!is_persistable_chat_message(&local_system));
    assert_eq!(persisted_slot_index_for_message_index(&messages, 0), Some(0));
    assert_eq!(persisted_slot_index_for_message_index(&messages, 1), None);
    assert_eq!(persisted_slot_index_for_message_index(&messages, 2), Some(1));
    assert_eq!(raw_message_id_from_ui_message(&tool).as_deref(), Some("tool-1"));
}

#[test]
fn clear_messages_preserves_session_context_and_resets_runtime_surfaces() {
    let mut state = TuiState::from_chat_session(&ChatSession {
        id: "s1".to_string(),
        title: "Title".to_string(),
        messages: vec![ChatMessage {
            role: ChatRole::Assistant,
            content: "answer".to_string(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        }],
        message_ids: vec![None],
        calls: vec![json!({"call": 1})],
        steps: Vec::new(),
        created_ms: 1,
        updated_ms: 2,
    });
    state.tasks.sync_error = Some("err".to_string());
    state.runtime.thinking_open = true;

    state.clear_messages();

    assert!(state.messages.is_empty());
    assert_eq!(state.session.session_id.as_deref(), Some("s1"));
    assert!(state.session.persisted_messages.is_empty());
    assert!(state.session.persisted_calls.is_empty());
    assert_eq!(state.status.turn_terminal, UiTurnTerminal::Pending);
    assert!(state.tasks.sync_error.is_none());
    assert!(!state.runtime.thinking_open);
}

#[test]
fn model_catalog_entry_formats_detail_and_matches_queries() {
    let entry = TuiModelCatalogEntry {
        provider_id: "anthropic".to_string(),
        provider_name: "Anthropic".to_string(),
        model_id: "claude".to_string(),
        model_name: "Claude Sonnet".to_string(),
    };

    assert_eq!(entry.qualified_id(), "anthropic/claude");
    assert_eq!(entry.suggestion_detail(), "Anthropic · Claude Sonnet");
    assert!(entry.matches_query("sonnet"));
    assert!(entry.matches_query("ANTHROPIC/CLAUDE"));
    assert!(entry.matches_query(" "));
    assert!(!entry.matches_query("gemini"));

    let unnamed = TuiModelCatalogEntry { model_name: "  ".to_string(), ..entry.clone() };
    let matching = TuiModelCatalogEntry { model_name: "claude".to_string(), ..entry };
    assert_eq!(unnamed.suggestion_detail(), "供应商: Anthropic");
    assert_eq!(matching.suggestion_detail(), "供应商: Anthropic");
}
