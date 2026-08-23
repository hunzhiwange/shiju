//! 验证 v3 聊天组件复制后的命令生命周期仍与既有界面一致。
//! 测试从事件序列驱动执行开始、输出和结束，再检查终端渲染结果。
//! 辅助渲染器固定视口与后端，避免快照受真实终端能力影响。
//! 诊断断言用于暴露遗漏边界或重复终止事件，而非静默修复状态。

use super::*;
use crate::vibe::event::V3Event;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

fn rendered(chat: &ChatWidget, width: u16, height: u16) -> String {
    let area = Rect::new(0, 0, width, height);
    let mut buffer = Buffer::empty(area);
    chat.render(area, &mut buffer);
    (0..height)
        .map(|y| {
            let mut line = (0..width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>();
            while line.ends_with(' ') {
                line.pop();
            }
            line
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn v3_event_sequence_preserves_copied_chatwidget_lifecycle() {
    let mut chat = ChatWidget::new(ChatWidgetConfig::default());

    for event in [
        V3Event::TurnStarted { turn_id: "turn-12".into() },
        V3Event::ThinkingDelta {
            turn_id: "turn-12".into(),
            delta: "Inspecting the workspace.".into(),
        },
        V3Event::ToolStarted {
            turn_id: "turn-12".into(),
            call_id: "call-1".into(),
            tool: "shell".into(),
            command: "cargo check -p vw-tui".into(),
        },
        V3Event::ToolOutputDelta {
            turn_id: "turn-12".into(),
            call_id: "call-1".into(),
            delta: "Checking vw-tui\n".into(),
        },
    ] {
        chat.handle_v3_event(event);
    }

    insta::assert_snapshot!(&rendered(&chat, 72, 18), @r###"
thinking: Inspecting the workspace.

› shell cargo check -p vw-tui
Checking vw-tui
  •
• Working (0s • esc to interrupt)

› Compose a VibeWindow task
  esc interrupt










"###);

    for event in [
        V3Event::ToolCompleted {
            turn_id: "turn-12".into(),
            call_id: "call-1".into(),
            output: "Finished dev profile\n".into(),
            exit_code: 0,
            duration_ms: 12,
        },
        V3Event::TextDelta {
            turn_id: "turn-12".into(),
            delta: "The check passed.".into(),
        },
        V3Event::TurnCompleted { turn_id: "turn-12".into() },
    ] {
        chat.handle_v3_event(event);
    }

    insta::assert_snapshot!(&rendered(&chat, 72, 18), @r###"
thinking: Inspecting the workspace.

› shell cargo check -p vw-tui
Finished dev profile
  ✓ 12ms

• The check passed.

› Compose a VibeWindow task
  ? shortcuts

"###);
}
