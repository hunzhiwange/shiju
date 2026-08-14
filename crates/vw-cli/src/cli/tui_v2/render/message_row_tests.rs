use ratatui::text::Line;

use super::TuiTheme;
use super::message_row::{render_transcript_item_lines, render_transcript_item_lines_with_options};
use crate::cli::tui_v2::model::{
    UiAssistantMessage, UiErrorMessage, UiMessage, UiMessageBase, UiMessageId, UiStep, UiStepState,
    UiSystemMessage, UiSystemMessageLevel, UiThinkingBlock, UiThinkingPhase, UiTokenUsage,
    UiToolCall, UiToolCallState, UiToolResult, UiTurnTerminal, UiUserMessage,
};
use crate::cli::tui_v2::state::selectors::{
    TuiAssistantTurnGroup, TuiCollapsedExploreResults, TuiCollapsedToolCount, TuiExploreToolKind,
    TuiToolCallGroup,
};
use crate::cli::tui_v2::state::{TuiAssistantTurnEntry, TuiTranscriptItem};

#[test]
fn render_standalone_user_system_tool_result_and_error_messages() {
    let theme = TuiTheme::default();
    assert_eq!(
        line_texts(render_transcript_item_lines(
            &TuiTranscriptItem::Standalone(&UiMessage::User(UiUserMessage {
                base: base("user"),
                text: "hello\nworld".to_string(),
            })),
            theme,
        )),
        vec!["❯ hello", "  world"]
    );
    assert_eq!(
        line_texts(render_transcript_item_lines(
            &TuiTranscriptItem::Standalone(&UiMessage::System(UiSystemMessage {
                base: base("system"),
                text: "warn".to_string(),
                level: UiSystemMessageLevel::Warning,
            })),
            theme,
        )),
        vec!["● warn"]
    );
    assert_eq!(
        line_texts(render_transcript_item_lines(
            &TuiTranscriptItem::Standalone(&UiMessage::ToolResult(UiToolResult {
                base: base("result"),
                call_id: Some("call".to_string()),
                tool_name: "read".to_string(),
                content: "ok".to_string(),
                is_error: false,
            })),
            theme,
        )),
        vec!["结果 ok"]
    );
    assert_eq!(
        line_texts(render_transcript_item_lines(
            &TuiTranscriptItem::Standalone(&UiMessage::Error(UiErrorMessage {
                base: base("error"),
                message: "boom".to_string(),
                recoverable: false,
            })),
            theme,
        )),
        vec!["● boom"]
    );
}

#[test]
fn render_standalone_assistant_tool_call_step_and_thinking_messages() {
    let theme = TuiTheme::default();
    assert_eq!(
        line_texts(render_transcript_item_lines(
            &TuiTranscriptItem::Standalone(&UiMessage::Assistant(UiAssistantMessage {
                base: base("assistant"),
                text: "answer".to_string(),
                usage: UiTokenUsage::default(),
                step_count: 2,
                terminal: UiTurnTerminal::Streaming,
                model: None,
            })),
            theme,
        )),
        vec!["● answer"]
    );
    assert_eq!(
        line_texts(render_transcript_item_lines(
            &TuiTranscriptItem::Standalone(&UiMessage::ToolCall(UiToolCall {
                base: base("call"),
                call_id: Some("call".to_string()),
                tool_name: "search".to_string(),
                summary: Some("found matches".to_string()),
                arguments: None,
                state: UiToolCallState::Complete,
            })),
            theme,
        )),
        vec!["✓ search (found matches) · completed"]
    );
    assert_eq!(
        line_texts(render_transcript_item_lines(
            &TuiTranscriptItem::Standalone(&UiMessage::Step(UiStep {
                base: base("step"),
                step_index: 3,
                started_ms: 1,
                finished_ms: Some(2),
                usage: UiTokenUsage::default(),
                finish_reason: Some("done".to_string()),
                model: None,
                state: UiStepState::Complete,
            })),
            theme,
        )),
        vec!["步骤 #3 Complete · done"]
    );
    assert_eq!(
        line_texts(render_transcript_item_lines(
            &TuiTranscriptItem::Standalone(&UiMessage::Thinking(UiThinkingBlock {
                base: base("thinking"),
                summary: None,
                content: "internal".to_string(),
                timing: Vec::new(),
                collapsed: true,
                phase: UiThinkingPhase::Closed,
            })),
            theme,
        )),
        vec!["∴ Thinking (Ctrl+O to expand)"]
    );
}

#[test]
fn render_assistant_turn_includes_preface_and_children_with_blank_separators() {
    let assistant = UiAssistantMessage {
        base: base("assistant"),
        text: "final".to_string(),
        usage: UiTokenUsage::default(),
        step_count: 1,
        terminal: UiTurnTerminal::Done { finish_reason: None },
        model: None,
    };
    let thinking = UiThinkingBlock {
        base: base("thinking"),
        summary: Some("summary".to_string()),
        content: "hidden".to_string(),
        timing: Vec::new(),
        collapsed: false,
        phase: UiThinkingPhase::Closed,
    };
    let step = UiStep {
        base: base("step"),
        step_index: 1,
        started_ms: 1,
        finished_ms: None,
        usage: UiTokenUsage::default(),
        finish_reason: None,
        model: None,
        state: UiStepState::Running,
    };
    let result = UiToolResult {
        base: base("result"),
        call_id: None,
        tool_name: "bash".to_string(),
        content: "stderr".to_string(),
        is_error: true,
    };
    let item = TuiTranscriptItem::AssistantTurn(TuiAssistantTurnGroup {
        assistant: &assistant,
        preface: vec![TuiAssistantTurnEntry::Thinking(&thinking)],
        children: vec![
            TuiAssistantTurnEntry::Step(&step),
            TuiAssistantTurnEntry::ToolResult(&result),
        ],
    });

    assert_eq!(
        line_texts(render_transcript_item_lines(&item, TuiTheme::default())),
        vec![
            "∴ Thinking (Ctrl+O to expand)",
            "",
            "子项步骤 #1 Running · 进行中",
            "",
            "子项结果 stderr",
            "",
            "● final",
        ]
    );
}

#[test]
fn render_standalone_messages_cover_empty_text_levels_and_filtered_tool_summary() {
    let theme = TuiTheme::default();

    assert_eq!(
        line_texts(render_transcript_item_lines(
            &TuiTranscriptItem::Standalone(&UiMessage::User(UiUserMessage {
                base: base("empty_user"),
                text: String::new(),
            })),
            theme,
        )),
        Vec::<String>::new()
    );
    assert_eq!(
        line_texts(render_transcript_item_lines(
            &TuiTranscriptItem::Standalone(&UiMessage::System(UiSystemMessage {
                base: base("info_system"),
                text: "info".to_string(),
                level: UiSystemMessageLevel::Info,
            })),
            theme,
        )),
        Vec::<String>::new()
    );
    assert_eq!(
        line_texts(render_transcript_item_lines(
            &TuiTranscriptItem::Standalone(&UiMessage::System(UiSystemMessage {
                base: base("success_system"),
                text: "ok".to_string(),
                level: UiSystemMessageLevel::Success,
            })),
            theme,
        )),
        vec!["✓ ok"]
    );
    assert_eq!(
        line_texts(render_transcript_item_lines(
            &TuiTranscriptItem::Standalone(&UiMessage::ToolCall(UiToolCall {
                base: base("blank_call"),
                call_id: None,
                tool_name: "grep".to_string(),
                summary: Some("   ".to_string()),
                arguments: None,
                state: UiToolCallState::Running,
            })),
            theme,
        )),
        vec!["◐ Search · running"]
    );
    assert_eq!(
        line_texts(render_transcript_item_lines(
            &TuiTranscriptItem::Standalone(&UiMessage::Thinking(UiThinkingBlock {
                base: base("thinking_summary"),
                summary: Some("visible".to_string()),
                content: "hidden".to_string(),
                timing: Vec::new(),
                collapsed: false,
                phase: UiThinkingPhase::Closed,
            })),
            theme,
        )),
        vec!["∴ Thinking (Ctrl+O to expand)"]
    );
}

#[test]
fn render_assistant_terminal_labels_only_appear_in_verbose_mode() {
    let cases = [
        (UiTurnTerminal::Pending, "待命"),
        (UiTurnTerminal::Cancelled { reason: Some("user".to_string()) }, "已取消"),
        (UiTurnTerminal::TimedOut { message: "slow".to_string() }, "超时"),
        (UiTurnTerminal::Error { message: "failed".to_string() }, "错误"),
    ];

    for (terminal, label) in cases {
        assert_eq!(
            line_texts(render_transcript_item_lines_with_options(
                &TuiTranscriptItem::Standalone(&UiMessage::Assistant(UiAssistantMessage {
                    base: base(label),
                    text: "answer".to_string(),
                    usage: UiTokenUsage::default(),
                    step_count: 0,
                    terminal,
                    model: None,
                })),
                TuiTheme::default(),
                80,
                true,
            )),
            vec!["● answer".to_string(), format!("  {label}")]
        );
    }
}

#[test]
fn render_assistant_turn_tool_and_collapsed_entries() {
    let assistant = UiAssistantMessage {
        base: base("assistant_tool_turn"),
        text: "done".to_string(),
        usage: UiTokenUsage::default(),
        step_count: 0,
        terminal: UiTurnTerminal::Pending,
        model: None,
    };
    let call = UiToolCall {
        base: base("call"),
        call_id: Some("call".to_string()),
        tool_name: "read".to_string(),
        summary: None,
        arguments: None,
        state: UiToolCallState::Complete,
    };
    let result = UiToolResult {
        base: base("tool_result"),
        call_id: Some("call".to_string()),
        tool_name: "read".to_string(),
        content: "file".to_string(),
        is_error: false,
    };
    let tool_group = TuiToolCallGroup {
        call: &call,
        results: vec![&result],
        progress: Vec::new(),
        notices: Vec::new(),
    };
    let collapsed = TuiCollapsedExploreResults {
        summary: "explore results: read x1, grep x1".to_string(),
        tool_counts: vec![
            TuiCollapsedToolCount { kind: TuiExploreToolKind::Read, count: 1 },
            TuiCollapsedToolCount { kind: TuiExploreToolKind::Grep, count: 1 },
        ],
        calls: Vec::new(),
        total_results: 2,
        total_progress: 0,
        notice_count: 0,
    };
    let item = TuiTranscriptItem::AssistantTurn(TuiAssistantTurnGroup {
        assistant: &assistant,
        preface: Vec::new(),
        children: vec![
            TuiAssistantTurnEntry::Tool(tool_group),
            TuiAssistantTurnEntry::CollapsedTools(collapsed),
        ],
    });

    assert_eq!(
        line_texts(render_transcript_item_lines(&item, TuiTheme::default())),
        vec![
            "✓ Read · completed",
            "",
            "子项工具组 explore results: read x1, grep x1 · 2 条结果",
            "",
            "● done",
        ]
    );
}

#[test]
fn render_assistant_turn_without_entries_has_no_leading_separator() {
    let assistant = UiAssistantMessage {
        base: base("assistant_only"),
        text: "solo".to_string(),
        usage: UiTokenUsage::default(),
        step_count: 0,
        terminal: UiTurnTerminal::Done { finish_reason: Some("stop".to_string()) },
        model: None,
    };
    let item = TuiTranscriptItem::AssistantTurn(TuiAssistantTurnGroup {
        assistant: &assistant,
        preface: Vec::new(),
        children: Vec::new(),
    });

    assert_eq!(
        line_texts(render_transcript_item_lines(&item, TuiTheme::default())),
        vec!["● solo"]
    );
}

fn base(id: &str) -> UiMessageBase {
    UiMessageBase::new(UiMessageId::new(id))
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
