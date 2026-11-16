//! 会话历史转换测试。
//!
//! 本模块验证通道侧聊天消息转换为会话 UI 历史时的角色映射与内容保真。
//! 这些断言用于保护工具消息等非普通用户文本在历史记录中不被误归类，
//! 从而避免后续渲染、摘要或恢复会话时丢失上下文语义。

use super::*;

#[test]
fn to_session_history_preserves_tool_role() {
    // 工具消息通常包含命令、输出和结构化片段，角色必须保持为 Tool，
    // 否则 UI 与会话恢复逻辑会把它误当成普通助手文本。
    let turns = vec![ChatMessage {
        role: "tool".to_string(),
        content: "tool shell\npwd\n/Users/demo\n".to_string(),
    }];

    let history = channel_support::session_events::to_session_history(&turns);

    assert_eq!(history.len(), 1);
    assert_eq!(history[0].role, session_port::ui_types::ChatRole::Tool);
    assert_eq!(history[0].content, turns[0].content);
}
