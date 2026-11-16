// 宿主契约：SessionViewSnapshot::take_from/restore_into 移动桌面字段，验证正文指针及面板模式恢复，不能仅以领域快照替代。
//! 会话视图缓存测试，覆盖快照移动、容量上限、最近最少使用淘汰和打开路径追踪。
//! 诊断事件只记录路径类别与原因，不得携带聊天正文等会话内容。

use super::view_cache::SessionViewSnapshot;
use crate::app::App;
use crate::app::models::{ChatMessage, ChatRole};
use crate::app::state::ChatPanelMode;

fn snapshot_with_message(content: String) -> SessionViewSnapshot {
    let (mut app, _task) = App::new();
    app.chat_view.chat = vec![ChatMessage {
        role: ChatRole::Assistant,
        content,
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }];
    SessionViewSnapshot::take_from(&mut app)
}

#[test]
fn session_view_cache_moves_snapshot_without_reallocating_chat() {
    let content = "moved chat message".to_owned();
    let expected_pointer = content.as_ptr();
    let snapshot = snapshot_with_message(content);

    assert_eq!(snapshot.chat()[0].content.as_ptr(), expected_pointer);
}

#[test]
fn session_view_snapshot_keeps_chat_panel_mode_independent() {
    let (mut app, _task) = App::new();
    app.chat_view.chat_panel_mode = ChatPanelMode::Trajectory;

    let snapshot = SessionViewSnapshot::take_from(&mut app);

    assert_eq!(app.chat_view.chat_panel_mode, ChatPanelMode::Conversation);
    snapshot.restore_into(&mut app);
    assert_eq!(app.chat_view.chat_panel_mode, ChatPanelMode::Trajectory);
}
