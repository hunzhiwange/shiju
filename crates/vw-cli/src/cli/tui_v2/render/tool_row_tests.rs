use ratatui::text::Line;

use super::TuiTheme;
use super::tool_row::render_tool_row;
use crate::cli::tui_v2::model::{
    UiAssistantMessage, UiMessage, UiMessageBase, UiMessageId, UiTokenUsage, UiToolCall,
    UiToolCallState, UiToolResult, UiTurnTerminal, UiUserMessage,
};
use crate::cli::tui_v2::state::selectors::{
    TuiAssistantTurnEntry, TuiTranscriptItem, select_grouped_transcript, select_tool_row_view,
};
use crate::cli::tui_v2::state::TuiState;

#[test]
fn tui_tool_rows_match_reference_lifecycle_and_expansion() {
    let lifecycle = [
        (UiToolCallState::Queued, "● Read · queued"),
        (UiToolCallState::Running, "◐ Read · running"),
        (UiToolCallState::Complete, "✓ Read · completed"),
        (UiToolCallState::Failed, "✗ Read · failed"),
        (UiToolCallState::Cancelled, "■ Read · interrupted"),
    ];
    for (state, expected) in lifecycle {
        let call = tool_call("read", "call-lifecycle", state, None);
        let group = crate::cli::tui_v2::state::selectors::TuiToolCallGroup {
            call: &call,
            results: Vec::new(),
            progress: Vec::new(),
            notices: Vec::new(),
        };
        let view = select_tool_row_view(&group);
        assert_eq!(
            line_texts(render_tool_row(&view, TuiTheme::default(), false))[0],
            expected
        );
    }

    let mut state = TuiState::default();
    let user_id = UiMessageId::new("user");
    state.messages = vec![
        UiMessage::User(UiUserMessage {
            base: UiMessageBase::new(user_id.clone()),
            text: "run tools".to_string(),
        }),
        UiMessage::ToolCall(tool_call_with_parent(
            "unknown_remote_tool",
            "call-a",
            UiToolCallState::Complete,
            Some("opaque action"),
            &user_id,
            Some(r#"{"token":"must-hide","path":"src/lib.rs"}"#),
        )),
        UiMessage::ToolCall(tool_call_with_parent(
            "read",
            "call-b",
            UiToolCallState::Complete,
            Some("Read files"),
            &user_id,
            Some(r#"{"path":"README.md"}"#),
        )),
        UiMessage::ToolResult(tool_result("read", "call-b", "first", false)),
        UiMessage::ToolResult(tool_result("unknown_remote_tool", "call-a", "ok", false)),
        UiMessage::ToolResult(tool_result("read", "call-b", "second", true)),
        UiMessage::Assistant(UiAssistantMessage {
            base: UiMessageBase::new(UiMessageId::new("assistant"))
                .with_parent_id(user_id.clone()),
            text: "done".to_string(),
            usage: UiTokenUsage::default(),
            step_count: 0,
            terminal: UiTurnTerminal::Done { finish_reason: None },
            model: None,
        }),
    ];
    state.refresh_transcript_projection();

    let transcript = select_grouped_transcript(&state);
    let TuiTranscriptItem::AssistantTurn(turn) = &transcript[1] else {
        panic!("assistant turn");
    };
    let groups = turn
        .preface
        .iter()
        .filter_map(|entry| match entry {
            TuiAssistantTurnEntry::Tool(group) => Some(group),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].results.len(), 1, "out-of-order result pairs by call_id");
    assert_eq!(groups[1].results.len(), 2, "multiple results stay on the same call");

    let unknown = select_tool_row_view(groups[0]);
    let collapsed = line_texts(render_tool_row(&unknown, TuiTheme::default(), false));
    assert_eq!(collapsed, vec!["✓ unknown_remote_tool (opaque action) · completed"]);
    assert!(!collapsed.join("\n").contains("must-hide"));

    let expanded = line_texts(render_tool_row(&unknown, TuiTheme::default(), true));
    assert!(expanded.iter().any(|line| line.contains("\"token\":\"[redacted]\"")));
    assert!(expanded.iter().all(|line| !line.contains("must-hide")));

    let read = select_tool_row_view(groups[1]);
    assert_eq!(
        line_texts(render_tool_row(&read, TuiTheme::default(), false)),
        vec!["✗ Read (Read files) · failed · 2 results"]
    );
}

fn tool_call(
    name: &str,
    call_id: &str,
    state: UiToolCallState,
    summary: Option<&str>,
) -> UiToolCall {
    UiToolCall {
        base: UiMessageBase::new(UiMessageId::new(format!("message-{call_id}"))),
        call_id: Some(call_id.to_string()),
        tool_name: name.to_string(),
        summary: summary.map(str::to_string),
        arguments: None,
        state,
    }
}

fn tool_call_with_parent(
    name: &str,
    call_id: &str,
    state: UiToolCallState,
    summary: Option<&str>,
    parent_id: &UiMessageId,
    arguments: Option<&str>,
) -> UiToolCall {
    let mut call = tool_call(name, call_id, state, summary);
    call.base.parent_id = Some(parent_id.clone());
    call.arguments = arguments.map(str::to_string);
    call
}

fn tool_result(name: &str, call_id: &str, content: &str, is_error: bool) -> UiToolResult {
    UiToolResult {
        base: UiMessageBase::new(UiMessageId::new(format!("result-{call_id}-{content}"))),
        call_id: Some(call_id.to_string()),
        tool_name: name.to_string(),
        content: content.to_string(),
        is_error,
    }
}

fn line_texts(lines: Vec<Line<'static>>) -> Vec<String> {
    lines
        .into_iter()
        .map(|line| {
            line.spans.into_iter().fold(String::new(), |mut out, span| {
                out.push_str(span.content.as_ref());
                out
            })
        })
        .collect()
}
