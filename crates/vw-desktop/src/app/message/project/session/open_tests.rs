// 宿主保留依据：打开会话关联历史准备、热缓存、模型权限、工作区和异步世代，保留宿主装配契约。
// 验证会话打开、工作区切换及聊天草稿迁移流程。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

// 编译接线：引用父模块符号，确保测试片段仍由真实模块包含。
// 该冒烟用例只承担接线职责，具体行为由同领域测试覆盖。
#[test]
fn test_module_is_wired() {
    let module = module_path!();

    assert!(module.ends_with("open_tests"));
}

fn app() -> crate::app::App {
    crate::app::App::new().0
}

#[test]
fn work_space_switch_carries_chat_and_input_draft_into_a_new_session() {
    let mut app = app();
    app.navigation_tabs.project_path = Some("/workspace/alpha".to_string());
    app.session_state.active_session_id = Some("session-1".to_string());
    app.chat_view.chat.push(crate::app::models::ChatMessage {
        role: crate::app::models::ChatRole::User,
        content: "保留这段聊天".to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    });
    app.get_session_runtime_mut("session-1").input_editor =
        iced::widget::text_editor::Content::with_text("未发送草稿");

    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::SwitchWorkSpacePressed(
            "/workspace/beta".to_string(),
        ),
    );

    assert!(task.is_some());
    assert_eq!(app.navigation_tabs.project_path.as_deref(), Some("/workspace/beta"));
    assert!(app.session_state.active_session_id.is_none());
    assert_eq!(app.chat_view.chat.len(), 1);
    assert_eq!(app.chat_view.chat[0].content, "保留这段聊天");
    assert_eq!(app.current_session_runtime().input_editor.text(), "未发送草稿");
}

async fn collect_task_messages(task: iced::Task<crate::app::Message>) -> Vec<crate::app::Message> {
    use iced_runtime::futures::futures::StreamExt;

    let Some(mut stream) = iced_runtime::task::into_stream(task.collect()) else {
        return Vec::new();
    };
    while let Some(action) = stream.next().await {
        if let iced_runtime::Action::Output(messages) = action {
            return messages;
        }
    }
    Vec::new()
}

fn recent_project_meta(
    path: &str,
    session_auto_refresh: bool,
    session_refresh_interval_seconds: u64,
) -> crate::app::state::RecentProjectMeta {
    crate::app::state::RecentProjectMeta {
        path: path.to_string(),
        name: "Project".to_string(),
        task_board_settings: None,
        session_auto_refresh,
        session_refresh_interval_seconds,
        icon: None,
        icon_color: None,
        worktree_start_command: None,
    }
}

fn loaded_project_info(
    project_path: &str,
    updated: u64,
    current_branch: Option<&str>,
) -> crate::app::message::project::LoadedProjectInfo {
    crate::app::message::project::LoadedProjectInfo {
        project_path: project_path.to_string(),
        info: project_types::Info {
            id: "project-id".to_string(),
            worktree: project_path.to_string(),
            vcs: Some(project_types::Vcs::Git),
            name: Some("Project".to_string()),
            icon: None,
            commands: None,
            time: project_types::TimeInfo { created: 1, updated, initialized: None },
            sandboxes: Vec::new(),
        },
        current_branch: current_branch.map(str::to_string),
    }
}

fn session_info(id: &str, additions: i64, deletions: i64) -> session_info::Info {
    session_info::Info {
        id: id.to_string(),
        slug: id.to_string(),
        project_id: "project".to_string(),
        directory: "/tmp/project".to_string(),
        parent_id: None,
        summary: Some(session_info::Summary {
            additions,
            deletions,
            files: 1,
            diffs: None,
        }),
        share: None,
        title: id.to_string(),
        version: "1".to_string(),
        time: session_info::TimeInfo {
            created: 1,
            updated: 1,
            compacting: None,
            archived: None,
        },
        permission: None,
        fork_execution_started: None,
        revert: None,
    }
}

fn chat_message(content: &str) -> crate::app::models::ChatMessage {
    crate::app::models::ChatMessage {
        role: crate::app::models::ChatRole::User,
        content: content.to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }
}

fn ui_store_session(id: &str, message_ids: Vec<Option<String>>) -> crate::app::models::ChatSession {
    crate::app::models::ChatSession {
        id: id.to_string(),
        title: "History".to_string(),
        messages: vec![chat_message("loaded history")],
        message_ids,
        calls: Vec::new(),
        steps: Vec::new(),
        created_ms: 1,
        updated_ms: 2,
    }
}







#[tokio::test(flavor = "multi_thread")]
async fn recent_hover_uses_cached_sessions_without_scheduling_gateway_load() {
    let mut app = app();
    let path = "/tmp/cached-project".to_string();
    app.workspace_state.project_sessions.insert(path.clone(), Vec::new());

    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::RecentHovered(Some(path.clone())),
    )
    .expect("recent hover should return a task");
    let messages = collect_task_messages(task).await;

    assert_eq!(app.workspace_state.hovered_recent_project, Some(path.clone()));
    assert!(!app.workspace_state.project_sessions_loading.contains(&path));
    assert!(messages.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn repeated_recent_hover_does_not_reload_active_session_todos() {
    let mut app = app();
    let path = "/tmp/cached-project".to_string();
    app.workspace_state.project_sessions.insert(path.clone(), Vec::new());
    app.workspace_state.hovered_recent_project = Some(path.clone());

    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::RecentHovered(Some(path)),
    )
    .expect("repeated recent hover should return a task");
    let messages = collect_task_messages(task).await;

    assert!(messages.is_empty(), "repeated hover emitted {messages:?}");
}

#[tokio::test(flavor = "multi_thread")]
async fn recent_hover_none_clears_hover_without_scheduling_todo_refresh() {
    let mut app = app();
    app.workspace_state.hovered_recent_project = Some("/tmp/project".to_string());

    let task =
        super::handle(&mut app, crate::app::message::project::ProjectMessage::RecentHovered(None))
            .expect("recent hover should return a task");
    let messages = collect_task_messages(task).await;

    assert!(app.workspace_state.hovered_recent_project.is_none());
    assert!(messages.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn open_requesting_session_prepares_restored_cached_state() {
    let mut app = app();
    let previous_scroll_id = app.chat_view.chat_scroll_id.clone();
    let mut target = session_info("target", 1, 1);
    target.directory = "/tmp/known-project".to_string();
    app.session_state.sessions = vec![target];
    app.navigation_tabs.project_path = Some("/tmp/known-project".to_string());
    app.session_state.active_session_id = Some("previous".to_string());
    app.chat_view.chat = vec![chat_message("previous chat")];
    app.chat_view.chat_message_ids = vec![Some("previous-message".to_string())];
    app.chat_view.usage.input_tokens = 99;
    app.chat_view.active_session_view_state.updated_ms = 42;
    app.chat_view.active_session_view_state.base_ready = true;
    app.get_session_runtime_mut("target").request_lifecycle.is_requesting = true;
    app.store_session_chat_snapshot(
        "target".to_string(),
        crate::app::session::shared_chat_messages(vec![chat_message("cached target chat")]),
        vec![Some("target-message".to_string())],
    );

    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::OpenProjectSessionPressed(
            "/tmp/stale-project".to_string(),
            "target".to_string(),
        ),
    );

    let messages = collect_task_messages(task.expect("session open should return a task")).await;
    assert_eq!(app.navigation_tabs.project_path.as_deref(), Some("/tmp/known-project"));
    assert_eq!(app.session_state.active_session_id.as_deref(), Some("target"));
    assert_eq!(app.chat_view.chat.len(), 1);
    assert_eq!(app.chat_view.chat[0].content, "cached target chat");
    assert_eq!(app.chat_view.chat_message_ids, vec![Some("target-message".to_string())]);
    assert_eq!(app.chat_view.usage, crate::app::models::TokenUsage::default());
    assert_eq!(app.chat_view.active_session_view_state.updated_ms, 0);
    assert!(app.chat_view.active_session_view_state.ui_preparing);
    assert!(!app.chat_view.active_session_view_state.base_ready);
    assert_ne!(app.chat_view.chat_scroll_id, previous_scroll_id);

    let prepared = messages
        .into_iter()
        .find(|message| {
            matches!(
                message,
                crate::app::Message::Project(
                    crate::app::message::project::ProjectMessage::SessionUiPrepared { .. }
                )
            )
        })
        .expect("requesting cache should schedule local UI preparation");
    let crate::app::Message::Project(prepared) = prepared else {
        unreachable!("matched project message above");
    };
    super::super::lifecycle::handle(&mut app, prepared);

    assert!(!app.chat_view.active_session_view_state.ui_preparing);
    assert!(app.chat_view.active_session_view_state.base_ready);
}

#[test]
fn open_project_session_restores_hot_view_without_prewarm() {
    let mut app = app();
    app.navigation_tabs.project_path = Some("/tmp/project".to_string());
    app.session_state.sessions = vec![session_info("a", 1, 1), session_info("b", 1, 1)];
    app.session_state.active_session_id = Some("a".to_string());
    app.chat_view.chat = vec![chat_message("session a")];
    app.chat_view.chat_message_ids = vec![Some("a-message".to_string())];
    app.chat_view.active_session_view_state.base_ready = true;
    app.chat_view.active_session_view_state.prepared_chat_ui_chunks.insert(0);
    app.chat_view.active_session_view_state.pinned_chat_ui_chunk_start = Some(0);
    app.chat_view.chat_render_cache.insert(0, crate::app::models::ChatRenderCacheEntry::default());
    app.chat_view.chat_virtualizer.sync_len(app.chat_view.chat.len(), 100.0);
    app.chat_view.chat_virtualizer.set_viewport_height(100.0);
    app.chat_view.chat_virtualizer.scroll_to_item_anchor(0, 0.0, 0.0);
    app.sync_chat_virtualizer_compat_fields();
    let a_scroll_id = app.chat_view.chat_scroll_id.clone();
    let a_window = app.chat_view.chat_virtualizer.window();

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::OpenProjectSessionPressed(
            "/tmp/project".to_string(),
            "b".to_string(),
        ),
    );
    app.chat_view.chat = vec![chat_message("session b")];
    app.chat_view.active_session_view_state.base_ready = true;

    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::OpenProjectSessionPressed(
            "/tmp/project".to_string(),
            "a".to_string(),
        ),
    );

    assert!(task.is_some());
    assert_eq!(app.session_state.active_session_id.as_deref(), Some("a"));
    assert_eq!(app.chat_view.chat[0].content, "session a");
    assert_eq!(app.chat_view.chat_scroll_id, a_scroll_id);
    assert_eq!(app.chat_view.chat_virtualizer.window().visible_start_idx, a_window.visible_start_idx);
    assert_eq!(app.chat_view.chat_virtualizer.window().visible_end_idx, a_window.visible_end_idx);
    assert!(app.chat_view.active_session_view_state.base_ready);
    assert!(!app.chat_view.active_session_view_state.ui_preparing);
    assert!(app.chat_view.active_session_view_state.prepared_chat_ui_chunks.contains(&0));
    assert!(!app.chat_view.active_session_view_state.preparing_chat_ui_chunks.contains(&0));
    assert!(app.chat_view.chat_render_cache.contains_key(&0));
}

#[test]
fn hot_restore_preserves_virtual_window_and_scroll_anchor() {
    let mut app = app();
    app.navigation_tabs.project_path = Some("/tmp/project".to_string());
    app.session_state.sessions = vec![session_info("a", 1, 1), session_info("b", 1, 1)];
    app.session_state.active_session_id = Some("a".to_string());
    app.chat_view.chat = (0..80).map(|idx| chat_message(&format!("message {idx}"))).collect();
    app.chat_view.active_session_view_state.base_ready = true;
    app.chat_view.chat_virtualizer.sync_len(app.chat_view.chat.len(), 100.0);
    app.chat_view.chat_virtualizer.set_viewport_height(300.0);
    app.chat_view.chat_virtualizer.scroll_to_item_anchor(40, 0.0, 0.0);
    app.sync_chat_virtualizer_compat_fields();
    let expected_window = app.chat_view.chat_virtualizer.window();
    let expected_scroll_id = app.chat_view.chat_scroll_id.clone();
    let expected_offset = app.chat_view.chat_scroll_offset_y;

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::OpenProjectSessionPressed(
            "/tmp/project".to_string(),
            "b".to_string(),
        ),
    );
    app.chat_view.chat = vec![chat_message("session b")];
    app.chat_view.active_session_view_state.base_ready = true;
    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::OpenProjectSessionPressed(
            "/tmp/project".to_string(),
            "a".to_string(),
        ),
    );

    let restored_window = app.chat_view.chat_virtualizer.window();
    assert_eq!(app.chat_view.chat_scroll_id, expected_scroll_id);
    assert_eq!(app.chat_view.chat_scroll_offset_y, expected_offset);
    assert_eq!(restored_window.visible_start_idx, expected_window.visible_start_idx);
    assert_eq!(restored_window.visible_end_idx, expected_window.visible_end_idx);
    assert_eq!(restored_window.render_start_idx, expected_window.render_start_idx);
    assert_eq!(restored_window.render_end_idx, expected_window.render_end_idx);
}

// 状态转换：覆盖交互前后的字段同步、选择关系与清理规则。
// 连续操作仍需保持模块不变量，并清除已经失效的瞬态反馈。
#[test]
fn cold_session_open_applies_cached_history_and_prepares_base_ui() {
    let mut app = app();
    app.session_state.active_session_id = Some("target".to_string());
    app.chat_view.chat_render_cache.insert(99, crate::app::models::ChatRenderCacheEntry::default());
    let prepared = crate::app::session::prepare_session_open_from_ui_store(
        ui_store_session("target", vec![Some("message-1".to_string())]),
        false,
    );

    let _ = super::super::lifecycle::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::SessionOpenPrepared {
            target_session_id: "target".to_string(),
            project_path: "/tmp/project".to_string(),
            open_generation: 0,
            source: crate::app::message::project::PreparedSessionOpenSource::UiStore,
            result: Ok(prepared),
        },
    )
    .expect("prepared session should be handled");

    assert_eq!(app.chat_view.chat.len(), 1);
    assert_eq!(app.chat_view.chat[0].content, "loaded history");
    assert_eq!(app.chat_view.chat_message_ids, vec![Some("message-1".to_string())]);
    assert!(app.chat_view.active_session_view_state.base_ready);
    assert!(!app.chat_view.active_session_view_state.ui_preparing);
    assert!(app.chat_view.active_session_view_state.prepared_chat_ui_chunks.contains(&0));
    assert!(!app.chat_view.chat_render_cache.contains_key(&99));
}

#[test]
fn prepared_session_open_moves_messages_into_app() {
    let mut app = app();
    app.session_state.active_session_id = Some("target".to_string());
    let prepared = crate::app::session::prepare_session_open_from_ui_store(
        ui_store_session("target", vec![Some("message-1".to_string())]),
        false,
    );
    let messages_ptr = prepared.messages.as_ptr();

    let _ = super::super::lifecycle::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::SessionOpenPrepared {
            target_session_id: "target".to_string(),
            project_path: "/tmp/project".to_string(),
            open_generation: 0,
            source: crate::app::message::project::PreparedSessionOpenSource::UiStore,
            result: Ok(prepared),
        },
    )
    .expect("prepared session should be handled");

    assert_eq!(app.chat_view.chat.as_ptr(), messages_ptr);
}

#[test]
fn equal_remote_refresh_keeps_hot_render_cache_and_scroll_id() {
    let mut app = app();
    app.session_state.active_session_id = Some("target".to_string());
    let ui_store = ui_store_session("target", vec![Some("message-1".to_string())]);
    let prepared = crate::app::session::prepare_session_open_from_ui_store(ui_store.clone(), false);
    let _ = super::super::lifecycle::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::SessionOpenPrepared {
            target_session_id: "target".to_string(),
            project_path: "/tmp/project".to_string(),
            open_generation: 0,
            source: crate::app::message::project::PreparedSessionOpenSource::UiStore,
            result: Ok(prepared),
        },
    );
    let scroll_id = app.chat_view.chat_scroll_id.clone();
    let render_hash = app.chat_view.chat_render_cache.get(&0).map(|entry| entry.content_hash);
    let remote = crate::app::session::prepare_session_open_from_ui_store(ui_store, false);

    let task = super::super::lifecycle::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::SessionOpenPrepared {
            target_session_id: "target".to_string(),
            project_path: "/tmp/project".to_string(),
            open_generation: 0,
            source: crate::app::message::project::PreparedSessionOpenSource::RemoteHistory,
            result: Ok(remote),
        },
    )
    .expect("equal remote result should be handled");

    assert_eq!(app.chat_view.chat_scroll_id, scroll_id);
    assert_eq!(app.chat_view.chat_render_cache.get(&0).map(|entry| entry.content_hash), render_hash);
    assert_eq!(task.units(), 0);
}

#[test]
fn empty_remote_refresh_finishes_session_loading() {
    let mut app = app();
    app.session_state.active_session_id = Some("target".to_string());
    app.chat_view.active_session_view_state.ui_preparing = true;
    let prepared = crate::app::session::prepare_session_open(
        "target".to_string(),
        Vec::new(),
        Vec::new(),
        crate::app::models::TokenUsage::default(),
        Vec::new(),
        Vec::new(),
        0,
        None,
        false,
    );

    let _ = super::super::lifecycle::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::SessionOpenPrepared {
            target_session_id: "target".to_string(),
            project_path: "/tmp/project".to_string(),
            open_generation: 0,
            source: crate::app::message::project::PreparedSessionOpenSource::RemoteHistory,
            result: Ok(prepared),
        },
    );

    assert!(!app.chat_view.active_session_view_state.ui_preparing);
}

#[test]
fn short_remote_refresh_preserves_prepared_history() {
    let mut app = app();
    app.session_state.active_session_id = Some("target".to_string());
    let mut local = ui_store_session(
        "target",
        vec![
            Some("message-1".to_string()),
            Some("message-2".to_string()),
            Some("message-3".to_string()),
        ],
    );
    local.messages = vec![chat_message("first"), chat_message("second"), chat_message("third")];
    let _ = super::super::lifecycle::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::SessionOpenPrepared {
            target_session_id: "target".to_string(),
            project_path: "/tmp/project".to_string(),
            open_generation: 0,
            source: crate::app::message::project::PreparedSessionOpenSource::UiStore,
            result: Ok(crate::app::session::prepare_session_open_from_ui_store(local, false)),
        },
    );

    let mut tail = ui_store_session("target", vec![Some("message-3".to_string())]);
    tail.messages = vec![chat_message("third")];
    let _ = super::super::lifecycle::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::SessionOpenPrepared {
            target_session_id: "target".to_string(),
            project_path: "/tmp/project".to_string(),
            open_generation: 0,
            source: crate::app::message::project::PreparedSessionOpenSource::RemoteHistory,
            result: Ok(crate::app::session::prepare_session_open_from_ui_store(tail, false)),
        },
    );

    assert_eq!(
        app.chat_view.chat_message_ids,
        vec![
            Some("message-1".to_string()),
            Some("message-2".to_string()),
            Some("message-3".to_string()),
        ]
    );
    assert_eq!(
        app.chat_view.chat.iter().map(|message| message.content.as_str()).collect::<Vec<_>>(),
        vec!["first", "second", "third"]
    );

    let empty = crate::app::session::prepare_session_open(
        "target".to_string(),
        Vec::new(),
        Vec::new(),
        crate::app::models::TokenUsage::default(),
        Vec::new(),
        Vec::new(),
        0,
        None,
        false,
    );
    let _ = super::super::lifecycle::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::SessionOpenPrepared {
            target_session_id: "target".to_string(),
            project_path: "/tmp/project".to_string(),
            open_generation: 0,
            source: crate::app::message::project::PreparedSessionOpenSource::RemoteHistory,
            result: Ok(empty),
        },
    );

    assert_eq!(
        app.chat_view.chat_message_ids,
        vec![
            Some("message-1".to_string()),
            Some("message-2".to_string()),
            Some("message-3".to_string()),
        ]
    );
}

#[test]
fn remote_refresh_does_not_restore_messages_after_explicit_reset() {
    let mut app = app();
    app.session_state.active_session_id = Some("target".to_string());
    app.chat_view.chat = vec![chat_message("retained")];
    app.chat_view.chat_message_ids = vec![Some("message-1".to_string())];
    app.mark_session_history_reset_boundary("target".to_string(), app.chat_view.chat_message_ids.clone());

    let mut stale = ui_store_session(
        "target",
        vec![Some("message-1".to_string()), Some("message-2".to_string())],
    );
    stale.messages = vec![chat_message("retained"), chat_message("removed")];
    let _ = super::super::lifecycle::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::SessionOpenPrepared {
            target_session_id: "target".to_string(),
            project_path: "/tmp/project".to_string(),
            open_generation: 0,
            source: crate::app::message::project::PreparedSessionOpenSource::RemoteHistory,
            result: Ok(crate::app::session::prepare_session_open_from_ui_store(stale, false)),
        },
    );

    assert_eq!(app.chat_view.chat_message_ids, vec![Some("message-1".to_string())]);
    assert_eq!(
        app.chat_view.chat.iter().map(|message| message.content.as_str()).collect::<Vec<_>>(),
        vec!["retained"]
    );
}

#[test]
fn remote_history_error_finishes_session_loading() {
    let mut app = app();
    app.session_state.active_session_id = Some("target".to_string());
    app.chat_view.active_session_view_state.ui_preparing = true;

    let _ = super::super::lifecycle::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::SessionOpenPrepared {
            target_session_id: "target".to_string(),
            project_path: "/tmp/project".to_string(),
            open_generation: 0,
            source: crate::app::message::project::PreparedSessionOpenSource::RemoteHistory,
            result: Err("gateway unavailable".to_string()),
        },
    );

    assert!(!app.chat_view.active_session_view_state.ui_preparing);
    assert_eq!(
        app.input_feedback.error_message.as_deref(),
        Some("Failed to load session messages: gateway unavailable")
    );
}

#[test]
fn changed_remote_refresh_replaces_snapshot_once() {
    let mut app = app();
    app.session_state.active_session_id = Some("target".to_string());
    let initial = ui_store_session("target", vec![Some("message-1".to_string())]);
    let _ = super::super::lifecycle::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::SessionOpenPrepared {
            target_session_id: "target".to_string(),
            project_path: "/tmp/project".to_string(),
            open_generation: 0,
            source: crate::app::message::project::PreparedSessionOpenSource::UiStore,
            result: Ok(crate::app::session::prepare_session_open_from_ui_store(initial, false)),
        },
    );
    let initial_scroll_id = app.chat_view.chat_scroll_id.clone();

    let mut changed = ui_store_session("target", vec![Some("message-2".to_string())]);
    changed.messages[0].content = "remote replacement".to_string();
    let prepared = crate::app::session::prepare_session_open_from_ui_store(changed, false);
    let _ = super::super::lifecycle::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::SessionOpenPrepared {
            target_session_id: "target".to_string(),
            project_path: "/tmp/project".to_string(),
            open_generation: 0,
            source: crate::app::message::project::PreparedSessionOpenSource::RemoteHistory,
            result: Ok(prepared.clone()),
        },
    );
    let replacement_scroll_id = app.chat_view.chat_scroll_id.clone();

    assert_ne!(replacement_scroll_id, initial_scroll_id);
    assert_eq!(app.chat_view.chat[0].content, "remote replacement");

    let _ = super::super::lifecycle::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::SessionOpenPrepared {
            target_session_id: "target".to_string(),
            project_path: "/tmp/project".to_string(),
            open_generation: 0,
            source: crate::app::message::project::PreparedSessionOpenSource::RemoteHistory,
            result: Ok(prepared),
        },
    );

    assert_eq!(app.chat_view.chat_scroll_id, replacement_scroll_id);
    assert_eq!(app.chat_view.chat[0].content, "remote replacement");
}

#[test]
fn stale_async_result_cannot_overwrite_new_active_session() {
    let mut app = app();
    app.session_state.active_session_id = Some("a".to_string());
    let stale_open_generation = app.session_state.session_view_cache.begin_open("a");
    app.session_state.active_session_id = Some("b".to_string());
    let current_open_generation = app.session_state.session_view_cache.begin_open("b");
    app.chat_view.chat = vec![chat_message("session b")];

    let stale = crate::app::session::prepare_session_open_from_ui_store(
        ui_store_session("a", vec![Some("a-message".to_string())]),
        false,
    );
    let _ = super::super::lifecycle::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::SessionOpenPrepared {
            target_session_id: "a".to_string(),
            project_path: "/tmp/project".to_string(),
            open_generation: stale_open_generation,
            source: crate::app::message::project::PreparedSessionOpenSource::RemoteHistory,
            result: Ok(stale),
        },
    );

    assert_eq!(app.session_state.active_session_id.as_deref(), Some("b"));
    assert_eq!(app.chat_view.chat[0].content, "session b");
    assert!(app.session_state.session_view_cache.is_current_open("b", current_open_generation));
}

#[test]
fn open_project_session_uses_global_model_until_its_history_is_restored() {
    let mut app = app();
    let mut target = session_info("target", 1, 1);
    target.directory = "/tmp/known-project".to_string();
    app.session_state.sessions = vec![target];
    app.navigation_tabs.project_path = Some("/tmp/known-project".to_string());
    app.session_state.active_session_id = Some("previous".to_string());
    app.session_state.model = "provider/previous-model".to_string();
    app.session_state.auto_model = false;
    app.get_session_runtime_mut("previous").model = "provider/previous-model".to_string();

    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::OpenProjectSessionPressed(
            "/tmp/known-project".to_string(),
            "target".to_string(),
        ),
    );

    assert!(task.is_some());
    assert_eq!(app.session_state.active_session_id.as_deref(), Some("target"));
    assert_eq!(app.session_state.model, "provider/previous-model");
    assert!(!app.session_state.auto_model);
    let runtime = app.session_state.session_runtime_states.get("target").expect("target runtime");
    assert_eq!(runtime.model, "provider/previous-model");
    assert!(!runtime.auto_model);
    assert!(runtime.restore_model_from_history);
}

#[test]
fn child_session_navigation_project_route_checks_persisted_source() {
    use crate::app::message::project::ProjectMessage;

    for (source, opens) in [
        ("agent-tool:run", false),
        ("plan-run:run", false),
        ("ordinary-fork", true),
        ("visible-task:run", true),
    ] {
        let mut app = app();
        let path = "/tmp/known-project";
        app.navigation_tabs.project_path = Some(path.into());
        app.session_state.input_popovers.recommended_prompt = true;
        app.session_state.active_session_id = Some("parent".into());
        let mut target = session_info("target", 0, 0);
        target.parent_id = Some("nested-parent".into());
        target.directory = path.into();
        target.slug = source.into();
        app.session_state.sessions = vec![target];
        app.get_session_runtime_mut("target").workflow_mode_enabled = true;

        let _ = super::handle(
            &mut app,
            ProjectMessage::OpenProjectSessionPressed(path.into(), "target".into()),
        );
        assert_eq!(
            app.session_state.active_session_id.as_deref(),
            Some(if opens { "target" } else { "parent" }),
            "{source}"
        );
        assert_eq!(app.session_state.input_popovers.recommended_prompt, !opens);
        assert!(app.get_session_runtime("target").workflow_mode_enabled);
    }
}

#[test]
fn child_session_navigation_missing_metadata_keeps_the_current_session() {
    use crate::app::message::project::ProjectMessage;

    let mut app = app();
    app.navigation_tabs.project_path = Some("/tmp/known-project".into());
    app.session_state.active_session_id = Some("parent".into());
    app.chat_view.chat = vec![chat_message("父会话内容")];

    let _ = super::handle(
        &mut app,
        ProjectMessage::OpenProjectSessionPressed(
            "/tmp/known-project".into(),
            "uncached-child".into(),
        ),
    );
    assert_eq!(app.session_state.active_session_id.as_deref(), Some("parent"));
    assert_eq!(app.chat_view.chat[0].content, "父会话内容");
}

#[test]
fn child_session_navigation_resolution_rejects_internal_and_stale_results() {
    use crate::app::message::project::ProjectMessage;

    let mut app = app();
    let path = "/tmp/known-project";
    app.navigation_tabs.project_path = Some(path.into());
    app.session_state.active_session_id = Some("parent".into());
    let _ = super::handle(
        &mut app,
        ProjectMessage::OpenProjectSessionPressed(path.into(), "uncached".into()),
    );
    let generation = app.session_state.session_view_cache.navigation_generation;
    let mut info = session_info("uncached", 0, 0);
    info.directory = path.into();
    info.mark_agent_tool();
    let reply = |info, generation| ProjectMessage::SessionNavigationResolved {
        generation,
        from_project: Some(path.into()),
        from_session: Some("parent".into()),
        destination: Some((path.into(), Box::new(info))),
    };

    let _ = super::handle(&mut app, reply(info.clone(), generation));
    assert_eq!(app.session_state.active_session_id.as_deref(), Some("parent"));

    info.slug = "visible-task:run".into();
    let _ = super::handle(&mut app, reply(info.clone(), generation - 1));
    assert_eq!(app.session_state.active_session_id.as_deref(), Some("parent"));

    app.navigation_tabs.project_path = Some("/tmp/other-project".into());
    let _ = super::handle(&mut app, reply(info.clone(), generation));
    assert_eq!(app.session_state.active_session_id.as_deref(), Some("parent"));

    app.navigation_tabs.project_path = Some(path.into());
    let _ = super::handle(&mut app, reply(info, generation));
    assert_eq!(app.session_state.active_session_id.as_deref(), Some("uncached"));
}

#[test]
fn open_project_session_keeps_workflow_mode_for_root_session() {
    let mut app = app();
    let mut root = session_info("root", 0, 0);
    root.directory = "/tmp/known-project".to_string();
    app.session_state.sessions = vec![root];
    app.navigation_tabs.project_path = Some("/tmp/known-project".to_string());
    app.get_session_runtime_mut("root").workflow_mode_enabled = true;
    app.get_session_runtime_mut("root").goal_mode_enabled = true;

    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::OpenProjectSessionPressed(
            "/tmp/known-project".to_string(),
            "root".to_string(),
        ),
    );

    assert!(task.is_some());
    assert!(app.get_session_runtime("root").workflow_mode_enabled);
    assert!(app.get_session_runtime("root").goal_mode_enabled);
}










// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。




#[test]
fn project_create_session_opens_picker_when_worktree_enabled() {
    let mut app = app();
    let path = "/tmp/project".to_string();
    app.navigation_tabs.project_path = Some("/tmp/other-project".to_string());
    app.workspace_state.project_worktree_enabled.insert(path.clone(), true);
    app.workspace_state.new_session_picker_options.push(("old".to_string(), "旧".to_string()));
    app.workspace_state.new_session_worktree_name = "old-name".to_string();
    app.workspace_state.new_session_delete_error = Some("old error".to_string());

    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectCreateSession(path.clone()),
    );

    assert!(task.is_some());
    assert_eq!(app.navigation_tabs.project_path.as_deref(), Some(path.as_str()));
    assert_eq!(app.workspace_state.new_session_picker_project, Some(path));
    assert!(app.workspace_state.new_session_picker_options.is_empty());
    assert!(app.workspace_state.new_session_worktree_name.is_empty());
    assert!(app.workspace_state.new_session_delete_error.is_none());
}

#[test]
fn desktop_first_response_project_create_session_opens_a_blank_draft() {
    let mut app = app();
    let path = "/tmp/project".to_string();
    app.navigation_tabs.project_path = Some("/tmp/other-project".to_string());
    app.workspace_state.project_worktree_enabled.insert(path.clone(), false);
    app.workspace_state.new_session_picker_project = Some(path.clone());
    app.workspace_state.new_session_picker_options = vec![(path.clone(), "Main".to_string())];
    app.workspace_state.new_session_worktree_name = "stale-name".to_string();
    app.workspace_state.new_session_reset_error = Some("stale error".to_string());

    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectCreateSession(path.clone()),
    );

    assert!(task.is_some());
    assert_eq!(app.navigation_tabs.project_path.as_deref(), Some(path.as_str()));
    assert!(app.workspace_state.new_session_picker_project.is_none());
    assert!(app.workspace_state.new_session_picker_options.is_empty());
    assert!(app.workspace_state.new_session_worktree_name.is_empty());
    assert!(app.workspace_state.new_session_reset_error.is_none());
    assert!(!app.workspace_state.new_session_creating);
    assert!(app.session_state.active_session_id.is_none());
    assert!(app.session_state.sessions.is_empty());
    assert!(app.chat_view.chat.is_empty());
}

#[test]
fn project_create_session_ignores_duplicate_request_while_creation_is_running() {
    let mut app = app();
    app.navigation_tabs.project_path = Some("/tmp/current".to_string());
    app.workspace_state.new_session_creating = true;

    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectCreateSession(
            "/tmp/duplicate".to_string(),
        ),
    );

    assert!(task.is_some());
    assert_eq!(app.navigation_tabs.project_path.as_deref(), Some("/tmp/current"));
    assert!(app.workspace_state.new_session_creating);
}

#[test]
fn session_create_failure_clears_progress_and_shows_error() {
    let mut app = app();
    app.workspace_state.new_session_creating = true;

    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::SessionCreateFailed(
            "gateway unavailable".to_string(),
        ),
    );

    assert!(task.is_some());
    assert!(!app.workspace_state.new_session_creating);
    assert!(
        app.toast_state.active_toast.as_ref().is_some_and(|toast| toast.message.contains("创建新会话失败"))
    );
}

#[test]
fn project_info_loaded_ignores_error_and_stale_project_result() {
    let mut app = app();
    app.navigation_tabs.project_path = Some("/tmp/current".to_string());
    app.workspace_state.project_id = Some("original-id".to_string());
    app.git.selected_branch = Some("original-branch".to_string());
    app.workspace_state.project_updated_at_ms = Some(7);

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectInfoLoaded(Err(
            "not available".to_string()
        )),
    );
    let _ =
        super::handle(
            &mut app,
            crate::app::message::project::ProjectMessage::ProjectInfoLoaded(Ok(
                loaded_project_info("/tmp/stale", 99, Some("stale-branch")),
            )),
        );

    assert_eq!(app.workspace_state.project_id.as_deref(), Some("original-id"));
    assert_eq!(app.git.selected_branch.as_deref(), Some("original-branch"));
    assert_eq!(app.workspace_state.project_updated_at_ms, Some(7));
}

#[test]
fn project_info_loaded_applies_current_project_identity_timestamp_and_branch() {
    let mut app = app();
    app.navigation_tabs.project_path = Some("/tmp/current".to_string());

    let _ =
        super::handle(
            &mut app,
            crate::app::message::project::ProjectMessage::ProjectInfoLoaded(Ok(
                loaded_project_info("/tmp/current", 42, Some("main")),
            )),
        );

    assert_eq!(app.workspace_state.project_id.as_deref(), Some("project-id"));
    assert_eq!(app.workspace_state.project_updated_at_ms, Some(42));
    assert_eq!(app.git.selected_branch.as_deref(), Some("main"));
    assert!(app.workspace_state.project_is_git_repo);
}

#[test]
fn project_info_loaded_sets_non_git_capability_when_vcs_absent() {
    let mut app = app();
    app.navigation_tabs.project_path = Some("/tmp/current".to_string());
    app.workspace_state.project_is_git_repo = true;
    app.task_board.task_board_worktree_snapshot_loading = true;
    app.task_board.task_board_last_worktree_snapshot_at_ms = 99;
    app.task_board.task_board_worktree_snapshot = Some(crate::app::task::WorktreePoolSnapshot {
        repo_root: "/tmp/stale".to_string(),
        pool_root: "/tmp/stale/.worktrees".to_string(),
        base_branch: "main".to_string(),
        idle_count: 1,
        busy_count: 0,
        tainted_count: 0,
        recycling_count: 0,
        dead_count: 0,
        merge_target_locks: Vec::new(),
        slots: Vec::new(),
    });
    let mut loaded = loaded_project_info("/tmp/current", 42, Some("main"));
    loaded.info.vcs = None;

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectInfoLoaded(Ok(loaded)),
    );

    assert!(!app.workspace_state.project_is_git_repo);
    assert!(app.task_board.task_board_worktree_snapshot.is_none());
    assert!(!app.task_board.task_board_worktree_snapshot_loading);
    assert_eq!(app.task_board.task_board_last_worktree_snapshot_at_ms, 0);
}

#[test]
fn project_info_loaded_maps_zero_timestamp_to_none_and_keeps_branch_when_missing() {
    let mut app = app();
    app.navigation_tabs.project_path = Some("/tmp/current".to_string());
    app.git.selected_branch = Some("existing".to_string());
    app.workspace_state.project_updated_at_ms = Some(42);

    let _ =
        super::handle(
            &mut app,
            crate::app::message::project::ProjectMessage::ProjectInfoLoaded(Ok(
                loaded_project_info("/tmp/current", 0, None),
            )),
        );

    assert_eq!(app.workspace_state.project_updated_at_ms, None);
    assert_eq!(app.git.selected_branch.as_deref(), Some("existing"));
}

#[test]
fn project_info_loaded_updates_recent_project_icon_color_and_start_command() {
    let mut app = app();
    app.navigation_tabs.project_path = Some("/tmp/current".to_string());
    app.workspace_state.recent_projects_meta = vec![recent_project_meta("/tmp/current", true, 60)];
    let mut loaded = loaded_project_info("/tmp/current", 11, Some("main"));
    loaded.info.icon = Some(project_types::Icon {
        url: None,
        override_icon: Some("rocket".to_string()),
        color: Some("#112233".to_string()),
    });
    loaded.info.commands =
        Some(project_types::Commands { start: Some("cargo run".to_string()) });

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectInfoLoaded(Ok(loaded)),
    );

    let meta = &app.workspace_state.recent_projects_meta[0];
    assert_eq!(meta.icon.as_deref(), Some("rocket"));
    assert_eq!(meta.icon_color.as_deref(), Some("#112233"));
    assert_eq!(meta.worktree_start_command.as_deref(), Some("cargo run"));
    assert_eq!(app.workspace_state.project_id.as_deref(), Some("project-id"));
}

#[test]
fn project_branches_loaded_ignores_stale_project_and_applies_current_project() {
    let mut app = app();
    app.navigation_tabs.project_path = Some("/tmp/current".to_string());
    app.git.selected_branch = Some("main".to_string());

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectBranchesLoaded {
            project_path: "/tmp/other".to_string(),
            selected_branch: Some("feature".to_string()),
            branches: vec!["feature".to_string()],
        },
    );
    assert_eq!(app.git.selected_branch.as_deref(), Some("main"));

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectBranchesLoaded {
            project_path: "/tmp/current".to_string(),
            selected_branch: Some("develop".to_string()),
            branches: vec!["main".to_string(), "develop".to_string()],
        },
    );

    assert_eq!(app.git.selected_branch.as_deref(), Some("develop"));
    assert_eq!(app.git.branches, vec!["main".to_string(), "develop".to_string()]);
}



// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn handle_returns_none_for_message_owned_by_another_project_submodule() {
    let mut app = app();

    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectToolsMenuClosed,
    );

    assert!(task.is_none());
}

#[test]
fn bounded_session_open_and_prepend_keep_save_sequence_base() {
    use crate::app::message::project::{PreparedSessionOpenSource, ProjectMessage};
    use session_ui_types::SessionMessageRecord;
    let mut app = app();
    app.session_state.active_session_id = Some("paged".into());
    let generation = 0;
    let record = |seq_no| SessionMessageRecord {
        seq_no,
        message_id: Some(format!("message-{seq_no}")),
        message: chat_message("loaded history"),
    };
    let prepared = crate::app::session::prepare_bounded_session_open(
        vw_api_session::GatewaySessionUiSummary {
            agent_binding: None,
            full_access: None,
            id: "paged".into(),
            title: "Session".into(),
            updated_ms: 42,
            message_count: 102,
            call_count: 0,
            last_content: None,
        },
        vw_api_session::GatewaySessionMessagePage {
            items: vec![record(100), record(101)],
            next_cursor: Some("older".into()),
            previous_cursor: None,
            total_count: 102,
        },
        false,
    );
    let _ = super::super::lifecycle::handle(
        &mut app,
        ProjectMessage::SessionOpenPrepared {
            target_session_id: "paged".into(),
            project_path: "/repo".into(),
            open_generation: generation,
            source: PreparedSessionOpenSource::RemoteHistory,
            result: Ok(prepared),
        },
    );
    assert_eq!(app.session_state.session_runtime_states["paged"].stream_save.seq_base, 100);
    assert!(app.session_state.paged_chat_timeline.as_mut().unwrap().begin_prepend("older"));
    let page = crate::app::session::prepare_timeline_page(
        "paged",
        vw_api_session::GatewaySessionMessagePage {
            items: vec![record(98), record(99)],
            next_cursor: Some("older-again".into()),
            previous_cursor: None,
            total_count: 102,
        },
    );
    let _ = super::super::lifecycle::handle(
        &mut app,
        ProjectMessage::SessionHistoryPageLoaded {
            session_id: "paged".into(),
            open_generation: generation,
            requested_cursor: "older".into(),
            result: Ok(page),
        },
    );
    assert_eq!(app.chat_view.chat.len(), 4);
    assert_eq!(app.session_state.session_runtime_states["paged"].stream_save.seq_base, 98);
}

#[test]
fn fork_selection_reopen_projects_authoritative_state_for_both_directories() {
    for directory in ["/repo", "/repo-worktree"] {
        for started in [false, true] {
            let mut app = app();
            app.navigation_tabs.project_path = Some("/repo".into());
            let mut info = session_info("fork", 0, 0);
            info.directory = directory.into();
            info.fork_execution_started = Some(started);
            let _ = super::open_project_session(
                &mut app,
                "/repo".into(),
                "fork".into(),
                Some(info),
                None,
            );
            assert_eq!(app.current_session_has_started(), started);
            assert_eq!(app.known_session_directory("fork").as_deref(), Some(directory));
        }
    }
}

#[test]
fn fork_selection_reopen_late_inherited_binding_keeps_explicit_native_selection() {
    let mut app = app();
    app.session_state.last_llm_full_access = Some(false);
    app.session_state.active_session_id = Some("fork".into());
    app.current_session_runtime_mut().fork_execution_started = Some(false);
    let _ = crate::app::message::chat::update(
        &mut app,
        crate::app::message::ChatMessage::SessionAgentSelected(None),
    );
    let mut prepared = crate::app::session::prepare_session_open_from_ui_store(
        ui_store_session("fork", vec![Some("inherited".into())]),
        false,
    );
    prepared.full_access = Some(true);
    prepared.agent_binding = Some(vw_api_chat_identity::SessionAgentIdentity {
        agent: "acp-claude".into(),
        acp_agent: Some("claude".into()),
        label: Some("Inherited Claude".into()),
    });
    let _ = super::super::lifecycle::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::SessionOpenPrepared {
            target_session_id: "fork".into(),
            project_path: "/repo".into(),
            open_generation: 0,
            source: crate::app::message::project::PreparedSessionOpenSource::UiStore,
            result: Ok(prepared),
        },
    );
    assert_eq!(app.current_session_runtime().acp_agent, None);
    assert_eq!(app.current_session_runtime().agent, None);
    assert!(!app.current_session_full_access());
}

#[test]
fn fork_selection_reopen_new_drafts_do_not_inherit_fork_authority() {
    let mut app = app();
    app.navigation_tabs.project_path = Some("/repo".into());
    app.session_state.active_session_id = Some("fork".into());
    app.current_session_runtime_mut().fork_execution_started = Some(false);
    app.current_session_runtime_mut().fork_request_submitted = true;
    app.current_session_runtime_mut().fork_agent_selected = true;
    let _ = app.switch_work_space_preserving_chat("/other".into());
    assert_eq!(app.current_session_runtime().fork_execution_started, None);
    assert!(!app.current_session_runtime().fork_request_submitted);
    assert!(!app.current_session_runtime().fork_agent_selected);
    let _ = super::open_session_draft(&mut app, "/repo".into(), "/repo".into());
    assert_eq!(app.current_session_runtime().fork_execution_started, None);
    assert!(!app.current_session_has_started());
}

#[test]
fn fork_selection_reopen_hot_cache_uses_fresh_authority() {
    let mut app = app();
    app.navigation_tabs.project_path = Some("/repo".into());
    let mut info = session_info("fork", 0, 0);
    info.directory = "/repo".into();
    info.fork_execution_started = Some(false);
    let _ = super::open_project_session(
        &mut app,
        "/repo".into(),
        "fork".into(),
        Some(info.clone()),
        None,
    );
    app.chat_view.chat = vec![chat_message("继承历史")];
    app.chat_view.chat_message_ids = vec![Some("inherited".into())];
    app.chat_view.active_session_view_state.ui_preparing = false;
    app.chat_view.active_session_view_state.base_ready = true;
    // A rejected local submission must not defeat a fresh unstarted server state.
    app.current_session_runtime_mut().fork_request_submitted = true;
    let _ = super::open_project_session(
        &mut app,
        "/repo".into(),
        "fork".into(),
        Some(info.clone()),
        None,
    );
    assert_eq!(app.chat_view.chat[0].content, "继承历史");
    assert!(!app.current_session_has_started());
    // A cached unstarted view must not unlock a fork started by another client.
    info.fork_execution_started = Some(true);
    let _ = super::open_project_session(&mut app, "/repo".into(), "fork".into(), Some(info), None);
    assert_eq!(app.chat_view.chat[0].content, "继承历史");
    assert!(app.current_session_has_started());
}
