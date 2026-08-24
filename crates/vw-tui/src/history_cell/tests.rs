//! 验证 v3 展示事件到复制版 Codex 历史单元的完整映射与渲染外观。
//! 类型断言守住复用边界，快照和结构断言覆盖用户可见的层级与状态。

use super::*;
use crate::vibe::event::{
    V3Attachment, V3DisplayEvent, V3FileChange, V3FileDiff, V3FileDiffEntry, V3StepStatus,
};

#[test]
// 每一种事件变体都必须生成非空历史单元，新增变体也应显式纳入清单。
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
            tool: "shell".into(),
            command: "printf hi".into(),
            output: "hi".into(),
            exit_code: 0,
            duration_ms: 12,
        },
        V3DisplayEvent::FileDiff {
            diff: V3FileDiff {
                files: vec![V3FileDiffEntry {
                    path: "src/lib.rs".into(),
                    change: V3FileChange::Update {
                        unified_diff: "@@ -1 +1 @@\n-old\n+new".into(),
                    },
                }],
            },
        },
        V3DisplayEvent::Thinking { text: "thinking".into() },
        V3DisplayEvent::Step { title: "step".into(), status: V3StepStatus::InProgress },
        V3DisplayEvent::System { text: "system".into() },
        V3DisplayEvent::Error { message: "error".into() },
        V3DisplayEvent::Attachment {
            attachment: V3Attachment {
                kind: crate::V3AttachmentKind::File,
                label: "file".into(),
                path: Some("/tmp/file".into()),
                media_type: None,
                text: None,
            },
        },
        V3DisplayEvent::Progress { label: "progress".into(), detail: Some("detail".into()) },
        V3DisplayEvent::CompactBoundary { summary: "summary".into() },
        V3DisplayEvent::Unknown { type_name: "future".into(), detail: "payload".into() },
    ];

    for event in &events {
        let cell = history_cell_for(event);
        assert!(!cell.display_lines(80).is_empty(), "event had no cell: {event:?}");
    }
    assert_eq!(events.len(), 13);
}

#[test]
fn v3_file_diff_history_renders_patch() {
    let lines = render_display_event(
        &V3DisplayEvent::FileDiff {
            diff: V3FileDiff {
                files: vec![V3FileDiffEntry {
                    path: "src/lib.rs".into(),
                    change: V3FileChange::Update {
                        unified_diff: "@@ -1,2 +1,2 @@\n-old value\n+new value\n context".into(),
                    },
                }],
            },
        },
        80,
    );
    let text = lines.iter().map(Line::to_string).collect::<Vec<_>>().join("\n");

    assert!(text.contains("src/lib.rs"));
    assert!(text.contains("+1 -1"));
    assert!(text.contains("@@ -1,2 +1,2 @@"));
    assert!(text.contains("-old value"));
    assert!(text.contains("+new value"));

    let removed = lines.iter().find(|line| line.to_string() == "-old value").unwrap();
    let added = lines.iter().find(|line| line.to_string() == "+new value").unwrap();
    assert!(removed.spans.iter().any(|span| span.style.fg == Some(Color::Red)));
    assert!(added.spans.iter().any(|span| span.style.fg == Some(Color::Green)));
}

#[test]
// 用户消息和工具调用应继续复用既有历史单元类型，而非引入平行实现。
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
// 多行助手文本的项目符号层级和成功工具结果的状态标记都应保持稳定。
fn history_cells_keep_bullets_indentation_and_tool_status() {
    let assistant =
        render_display_event(&V3DisplayEvent::Assistant { markdown: "first\nsecond".into() }, 40);
    assert_eq!(assistant[0].to_string(), "• first");
    assert_eq!(assistant[1].to_string(), "  second");

    let result = render_display_event(
        &V3DisplayEvent::ToolResult {
            call_id: "call-1".into(),
            tool: "shell".into(),
            command: "printf done".into(),
            output: "done".into(),
            exit_code: 0,
            duration_ms: 5,
        },
        40,
    );
    assert!(result.iter().any(|line| line.to_string().contains('✓')));
}

#[test]
// 代表性事件序列的组合快照用于捕获跨单元间距与样式回归。
fn v3_display_event_history_snapshot() {
    let events = [
        V3DisplayEvent::User { text: "inspect the change".into(), attachments: vec![] },
        V3DisplayEvent::Assistant { markdown: "I found a **small** change.".into() },
        V3DisplayEvent::ToolResult {
            call_id: "call-1".into(),
            tool: "shell".into(),
            command: "printf ok".into(),
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
