// 宿主契约：保留领域加载器与聊天首屏分块器的组合，以及项目重载的 App 重置和 Task 映射；分页局部行为迁至 desktop-session::desktop_split_04_tests。
//! 会话加载测试，验证共享消息转换、冷加载分块和历史预览生成。
//! 预览文本需处理空白、换行、长度边界与重复会话标识，同时保持原消息顺序。

use crate::app::models::{ChatMessage, ChatRole, TokenUsage};

fn message(role: ChatRole, content: &str) -> ChatMessage {
    ChatMessage {
        role,
        content: content.to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }
}

fn session_info(id: &str) -> session_info::Info {
    session_info::Info {
        id: id.to_string(),
        slug: format!("{id}-slug"),
        project_id: "project-1".to_string(),
        directory: "/workspace/project".to_string(),
        parent_id: None,
        summary: None,
        share: None,
        title: "Session".to_string(),
        version: "1".to_string(),
        time: session_info::TimeInfo { created: 10, updated: 20, compacting: None, archived: None },
        permission: None,
        fork_execution_started: None,
        revert: None,
    }
}

#[test]
fn reload_sessions_for_project_clears_current_session_state_before_loading() {
    let (mut app, _startup_task) = crate::app::App::new();
    app.session_previews.insert("session-1".to_string(), "preview".to_string());
    app.archived_session_ids.insert("session-1".to_string());
    app.sessions.push(session_info("session-1"));
    app.active_session_id = Some("session-1".to_string());
    app.chat.push(message(ChatRole::User, "stale message"));
    app.usage =
        TokenUsage { input_tokens: 10, output_tokens: 20, cached_tokens: 3, reasoning_tokens: 4 };
    app.chat_visible_text_cache.push(Some("stale text".to_string()));
    app.chat_message_expanded.insert(0);

    let _reload_task = app.reload_sessions_for_project(None);

    assert!(app.session_previews.is_empty());
    assert!(app.archived_session_ids.is_empty());
    assert!(app.sessions.is_empty());
    assert_eq!(app.active_session_id, None);
    assert!(app.chat.is_empty());
    assert_eq!(app.usage, TokenUsage::default());
    assert!(app.chat_visible_text_cache.is_empty());
    assert!(app.chat_message_expanded.is_empty());
}

#[test]
fn reload_sessions_for_project_with_path_returns_runnable_task() {
    let (mut app, _startup_task) = crate::app::App::new();
    let project_path = "/workspace/project".to_string();

    let reload_task = app.reload_sessions_for_project(Some(project_path.clone()));

    assert!(iced_runtime::task::into_stream(reload_task.collect()).is_some());
    assert!(app.project_sessions_loading.contains(&project_path));
}
