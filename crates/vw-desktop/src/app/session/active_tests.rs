// 宿主契约：步骤、原始载荷、轨迹投影与消息索引同步；会话缓存、运行态和项目目录共同参与。局部步骤算法已有 desktop-session::session::active_tests 覆盖。
//! 活跃会话状态测试，覆盖步骤索引、消息元数据和问题索引的增量维护。
//! 会话清理与步骤替换必须同步释放原始载荷，避免派生缓存与主状态失配。

use super::*;
use session_info::{Info, TimeInfo};

fn message(role: ChatRole, content: &str) -> ChatMessage {
    ChatMessage {
        role,
        content: content.to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }
}

fn session_info(id: &str, directory: &str, updated_ms: u64) -> Info {
    Info {
        id: id.to_string(),
        slug: id.to_string(),
        project_id: "project".to_string(),
        directory: directory.to_string(),
        parent_id: None,
        summary: None,
        share: None,
        title: id.to_string(),
        version: "0.0.0".to_string(),
        time: TimeInfo { created: 1, updated: updated_ms, compacting: None, archived: None },
        permission: None,
        fork_execution_started: None,
        revert: None,
    }
}

#[test]
fn sync_active_session_from_chat_only_advances_session_timestamp() {
    let (mut app, _task) = App::new();
    app.active_session_id = Some("active".to_string());
    app.sessions = vec![session_info("active", "/active", 2_000)];
    app.active_session_view_state.updated_ms = 1_000;
    app.chat = vec![message(ChatRole::User, "question")];

    app.sync_active_session_from_chat();
    assert_eq!(app.active_session_view_state.updated_ms, 2_000);
    assert_eq!(app.active_session_view_state.message_meta_texts, vec![None]);

    app.sessions[0].time.updated = 500;
    app.sync_active_session_from_chat();
    assert_eq!(app.active_session_view_state.updated_ms, 2_000);
}

#[test]
fn cache_pruning_retains_requesting_sessions_and_removes_inactive_sessions() {
    let (mut app, _task) = App::new();
    app.active_session_id = Some("requesting".to_string());
    app.chat = vec![message(ChatRole::User, "kept")];
    app.chat_message_ids = vec![Some("message-1".to_string())];
    app.get_session_runtime_mut("requesting").request_lifecycle.is_requesting = true;
    app.store_session_chat_snapshot(
        "inactive".to_string(),
        shared_chat_messages(vec![message(ChatRole::User, "removed")]),
        vec![Some("message-2".to_string())],
    );

    app.cache_active_session_chat();

    assert!(app.cached_chat_messages("requesting").is_some());
    assert_eq!(
        app.cached_chat_message_ids("requesting"),
        Some(vec![Some("message-1".to_string())])
    );
    assert!(app.cached_chat_messages("inactive").is_none());
    assert!(app.cached_chat_message_ids("inactive").is_none());

    app.get_session_runtime_mut("requesting").request_lifecycle.is_requesting = false;
    app.prune_inactive_session_chat_cache();
    assert!(app.cached_chat_messages("requesting").is_none());
    assert!(app.cached_chat_message_ids("requesting").is_none());
}

#[test]
fn known_session_directory_prefers_loaded_sessions_then_searches_projects() {
    let (mut app, _task) = App::new();
    app.sessions = vec![session_info("duplicate", "/loaded", 1)];
    app.project_sessions.insert(
        "/project".to_string(),
        vec![
            session_info("duplicate", "/project-duplicate", 1),
            session_info("project-only", "/project-only", 1),
        ],
    );

    assert_eq!(app.known_session_directory("duplicate").as_deref(), Some("/loaded"));
    assert_eq!(app.known_session_directory("project-only").as_deref(), Some("/project-only"));
    assert_eq!(app.known_session_directory("missing"), None);
}

#[test]
fn history_reset_boundary_replaces_previous_value_and_is_session_scoped() {
    let (mut app, _task) = App::new();
    app.mark_session_history_reset_boundary("session-a".to_string(), vec![Some("old".to_string())]);
    app.mark_session_history_reset_boundary(
        "session-a".to_string(),
        vec![Some("retained".to_string()), None],
    );

    assert_eq!(
        app.session_history_reset_boundary("session-a"),
        Some([Some("retained".to_string()), None].as_slice())
    );
    assert_eq!(app.session_history_reset_boundary("session-b"), None);
}

#[test]
fn switch_stashes_previous_chat_without_cloning_payload() {
    let (mut app, _task) = App::new();
    let content = "large message payload ".repeat(4_096);
    let expected_pointer = content.as_ptr();
    app.active_session_id = Some("previous".to_string());
    app.chat = vec![ChatMessage {
        role: ChatRole::Assistant,
        content,
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }];

    app.stash_active_session_view();

    let snapshot = app.session_view_cache.get("previous").expect("previous view should be cached");
    assert_eq!(snapshot.chat()[0].content.as_ptr(), expected_pointer);
    assert!(app.chat.is_empty());
}

#[test]
fn repeated_stash_and_project_reload_preserve_running_session_history() {
    let (mut app, _) = App::new();
    app.active_session_id = Some("running".to_string());
    app.chat = vec![message(ChatRole::User, "question"), message(ChatRole::Assistant, "partial")];
    app.chat_message_ids = vec![Some("user-id".to_string()), None];
    app.get_session_runtime_mut("running").request_lifecycle.is_requesting = true;

    app.stash_active_session_view();
    assert!(app.active_session_id.is_none());
    app.stash_active_session_view();
    let _ = app.reload_sessions_for_project(Some("/workspace/other".to_string()));

    let snapshot = app.session_view_cache.get("running").expect("stashed history");
    assert_eq!(snapshot.chat().len(), 2);
    app.active_session_id = Some("running".to_string());
    app.restore_chat_for_session("running");
    assert_eq!(app.chat.len(), 2);
    assert_eq!(app.chat[0].content, "question");
    assert_eq!(app.chat[1].content, "partial");
    assert_eq!(app.chat_message_ids, vec![Some("user-id".to_string()), None]);
}
