// 宿主契约：窗口聊天工作集移动及项目重载的跨域生命周期；纯会话状态测试归 desktop-session::host_module_1_tests。
//! 活跃会话状态测试，覆盖步骤索引、消息元数据和问题索引的增量维护。
//! 会话清理与步骤替换必须同步释放原始载荷，避免派生缓存与主状态失配。

use super::*;

fn message(role: ChatRole, content: &str) -> ChatMessage {
    ChatMessage {
        role,
        content: content.to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }
}

#[test]
fn switch_stashes_previous_chat_without_cloning_payload() {
    let (mut app, _task) = App::new();
    let content = "large message payload ".repeat(4_096);
    let expected_pointer = content.as_ptr();
    app.session_state.active_session_id = Some("previous".to_string());
    app.chat_view.chat = vec![ChatMessage {
        role: ChatRole::Assistant,
        content,
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }];

    app.stash_active_session_view();

    let snapshot = app.session_state.session_view_cache.get("previous").expect("previous view should be cached");
    assert_eq!(snapshot.chat()[0].content.as_ptr(), expected_pointer);
    assert!(app.chat_view.chat.is_empty());
}

#[test]
fn repeated_stash_and_project_reload_preserve_running_session_history() {
    let (mut app, _) = App::new();
    app.session_state.active_session_id = Some("running".to_string());
    app.chat_view.chat = vec![message(ChatRole::User, "question"), message(ChatRole::Assistant, "partial")];
    app.chat_view.chat_message_ids = vec![Some("user-id".to_string()), None];
    app.get_session_runtime_mut("running").request_lifecycle.is_requesting = true;

    app.stash_active_session_view();
    assert!(app.session_state.active_session_id.is_none());
    app.stash_active_session_view();
    let _ = app.reload_sessions_for_project(Some("/workspace/other".to_string()));

    let snapshot = app.session_state.session_view_cache.get("running").expect("stashed history");
    assert_eq!(snapshot.chat().len(), 2);
    app.session_state.active_session_id = Some("running".to_string());
    app.restore_chat_for_session("running");
    assert_eq!(app.chat_view.chat.len(), 2);
    assert_eq!(app.chat_view.chat[0].content, "question");
    assert_eq!(app.chat_view.chat[1].content, "partial");
    assert_eq!(app.chat_view.chat_message_ids, vec![Some("user-id".to_string()), None]);
}
