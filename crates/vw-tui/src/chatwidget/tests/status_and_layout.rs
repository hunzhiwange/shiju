//! 验证命令与工具生命周期异常时，状态区和历史布局仍提供可诊断结果。
//! 缺失开始、重复终止和缺失工具边界不得被静默当作成功完成。
//! 工具错误、取消和轮次中断都应保留相应终态单元格。
//! 固定终端渲染用于检查诊断文本与终止标记的相对布局。

use super::*;
use crate::vibe::event::V3Event;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

fn rendered(chat: &ChatWidget) -> String {
    let area = Rect::new(0, 0, 80, 24);
    let mut buffer = Buffer::empty(area);
    chat.render(area, &mut buffer);
    buffer
        .content
        .chunks(usize::from(area.width))
        .map(|row| {
            row.iter()
                .map(ratatui::buffer::Cell::symbol)
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn status_and_layout_missing_begin_and_duplicate_terminal_are_diagnostic() {
    // 缺失开始与重复终止都产生诊断，而不是合并成看似成功的单元格。
    let mut chat = ChatWidget::new(ChatWidgetConfig::default());

    chat.handle_v3_event(V3Event::TextDelta {
        turn_id: "turn-missing".into(),
        delta: "Recovered output.".into(),
    });
    assert!(rendered(&chat).contains("• ⠋ Working"));

    chat.handle_v3_event(V3Event::TurnCompleted { turn_id: "turn-missing".into() });
    chat.handle_v3_event(V3Event::TurnCompleted { turn_id: "turn-missing".into() });
    chat.handle_v3_event(V3Event::Unknown {
        type_name: "future.notification".into(),
        detail: "kept for diagnosis".into(),
    });

    let frame = rendered(&chat);
    assert!(frame.contains("missing turn start: event=text delta; turn_id=turn-missing"));
    assert!(frame.contains("• Recovered output."));
    assert!(frame.contains("duplicate turn terminal: event=turn completed"));
    assert!(frame.contains("future.notification: kept for diagnosis"));
    assert!(!frame.contains("Working"));
}

#[test]
fn status_and_layout_tool_error_and_cancel_keep_terminal_cells() {
    // 工具错误和取消保留各自终态，方便用户区分失败原因。
    let mut chat = ChatWidget::new(ChatWidgetConfig::default());
    for event in [
        V3Event::TurnStarted { turn_id: "turn-tools".into() },
        V3Event::ToolStarted {
            turn_id: "turn-tools".into(),
            call_id: "failed".into(),
            tool: "shell".into(),
            command: "false".into(),
        },
        V3Event::ToolFailed {
            turn_id: "turn-tools".into(),
            call_id: "failed".into(),
            message: "command failed".into(),
            duration_ms: 4,
        },
        V3Event::ToolStarted {
            turn_id: "turn-tools".into(),
            call_id: "cancelled".into(),
            tool: "shell".into(),
            command: "sleep 30".into(),
        },
        V3Event::ToolCancelled {
            turn_id: "turn-tools".into(),
            call_id: "cancelled".into(),
            reason: "interrupted".into(),
            duration_ms: 7,
        },
        V3Event::TurnFailed {
            turn_id: "turn-tools".into(),
            message: "turn failed".into(),
        },
    ] {
        chat.handle_v3_event(event);
    }

    let frame = rendered(&chat);
    assert!(frame.contains("› shell false"));
    assert!(frame.contains("command failed"));
    assert!(frame.contains("› shell sleep 30"));
    assert!(frame.contains("interrupted"));
    assert_eq!(frame.matches('✗').count(), 2);
    assert!(frame.contains("error: turn failed"));
    assert!(!frame.contains("Working"));
}

#[test]
fn status_and_layout_cancelled_turn_records_interrupt_terminal() {
    // 轮次取消会记录中断终态，并清理仍在运行的状态指示。
    let mut chat = ChatWidget::new(ChatWidgetConfig::default());
    chat.handle_v3_event(V3Event::TurnStarted { turn_id: "turn-cancel".into() });
    chat.handle_v3_event(V3Event::ThinkingDelta {
        turn_id: "turn-cancel".into(),
        delta: "Still working".into(),
    });
    chat.handle_v3_event(V3Event::TurnCancelled {
        turn_id: "turn-cancel".into(),
        reason: "user interrupt".into(),
    });

    let frame = rendered(&chat);
    assert!(frame.contains("thinking: Still working"));
    assert!(frame.contains("system: Cancelled: user interrupt"));
    assert!(!frame.contains("Working"));
}

#[test]
fn status_and_layout_missing_tool_boundaries_are_not_silently_completed() {
    // 缺少工具边界时保持显式不完整诊断，防止隐藏协议顺序问题。
    let mut chat = ChatWidget::new(ChatWidgetConfig::default());
    chat.handle_v3_event(V3Event::ToolOutputDelta {
        turn_id: "turn-tool-gap".into(),
        call_id: "orphan".into(),
        delta: "partial output".into(),
    });
    chat.handle_v3_event(V3Event::TurnCompleted { turn_id: "turn-tool-gap".into() });

    let frame = rendered(&chat);
    assert!(frame.contains("tool output without start: call_id=orphan"));
    assert!(frame.contains("[missing tool start: orphan]"));
    assert!(frame.contains("tool did not emit a terminal event"));
    assert!(frame.contains("turn completed with running tools: call_ids=orphan"));
    assert!(frame.contains('✗'));
}
