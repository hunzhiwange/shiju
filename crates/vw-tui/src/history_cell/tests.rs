use super::*;
use crate::vibe::event::{V3Attachment, V3DisplayEvent, V3StepStatus};

#[test]
fn v3_display_event_mapping_is_exhaustive() {
    let events = vec![
        V3DisplayEvent::User { text: "hello".into(), attachments: vec![] },
        V3DisplayEvent::Assistant { markdown: "answer".into() },
        V3DisplayEvent::ToolCall {
            call_id: "call-1".into(),
            tool: "shell".into(),
            command: "printf hi".into(),
        },
        V3DisplayEvent::ToolResult {
            call_id: "call-1".into(),
            output: "hi".into(),
            exit_code: 0,
            duration_ms: 12,
        },
        V3DisplayEvent::Thinking { text: "thinking".into() },
        V3DisplayEvent::Step { title: "step".into(), status: V3StepStatus::InProgress },
        V3DisplayEvent::System { text: "system".into() },
        V3DisplayEvent::Error { message: "error".into() },
        V3DisplayEvent::Attachment {
            attachment: V3Attachment { label: "file".into(), path: Some("/tmp/file".into()) },
        },
        V3DisplayEvent::Progress { label: "progress".into(), detail: Some("detail".into()) },
        V3DisplayEvent::CompactBoundary { summary: "summary".into() },
        V3DisplayEvent::Unknown { type_name: "future".into(), detail: "payload".into() },
    ];

    for event in &events {
        let cell = history_cell_for(event);
        assert!(!cell.display_lines(80).is_empty(), "event had no cell: {event:?}");
    }
    assert_eq!(events.len(), 12);
}

#[test]
fn v3_display_events_use_copied_codex_history_cells() {
    let user = V3DisplayEvent::User { text: "a user message".into(), attachments: vec![] };
    let tool = V3DisplayEvent::ToolCall {
        call_id: "call-1".into(),
        tool: "shell".into(),
        command: "echo hi".into(),
    };

    assert!(history_cell_for(&user).as_any().is::<UserHistoryCell>());
    assert!(history_cell_for(&tool).as_any().is::<ExecHistoryCell>());
}

#[test]
fn history_cells_keep_bullets_indentation_and_tool_status() {
    let assistant =
        render_display_event(&V3DisplayEvent::Assistant { markdown: "first\nsecond".into() }, 40);
    assert_eq!(assistant[0].to_string(), "• first");
    assert_eq!(assistant[1].to_string(), "  second");

    let result = render_display_event(
        &V3DisplayEvent::ToolResult {
            call_id: "call-1".into(),
            output: "done".into(),
            exit_code: 0,
            duration_ms: 5,
        },
        40,
    );
    assert!(result.iter().any(|line| line.to_string().contains('✓')));
}

#[test]
fn v3_display_event_history_snapshot() {
    let events = [
        V3DisplayEvent::User { text: "inspect the change".into(), attachments: vec![] },
        V3DisplayEvent::Assistant { markdown: "I found a **small** change.".into() },
        V3DisplayEvent::ToolResult {
            call_id: "call-1".into(),
            output: "ok".into(),
            exit_code: 0,
            duration_ms: 8,
        },
        V3DisplayEvent::Step {
            title: "finish the migration".into(),
            status: V3StepStatus::Completed,
        },
    ];
    let rendered = events
        .iter()
        .flat_map(|event| render_display_event(event, 48))
        .map(|line| line.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    insta::assert_snapshot!(rendered);
}
