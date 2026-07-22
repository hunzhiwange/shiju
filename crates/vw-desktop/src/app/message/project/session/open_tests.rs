#[test]
fn test_module_is_wired() {
    let module = module_path!();

    assert!(module.ends_with("open_tests"));
}

fn app() -> crate::app::App {
    crate::app::App::new().0
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
        info: vw_shared::project::Info {
            id: "project-id".to_string(),
            worktree: project_path.to_string(),
            vcs: Some(vw_shared::project::Vcs::Git),
            name: Some("Project".to_string()),
            icon: None,
            commands: None,
            time: vw_shared::project::TimeInfo { created: 1, updated, initialized: None },
            sandboxes: Vec::new(),
        },
        current_branch: current_branch.map(str::to_string),
    }
}

fn session_info(id: &str, additions: i64, deletions: i64) -> vw_shared::session::info::Info {
    vw_shared::session::info::Info {
        id: id.to_string(),
        slug: id.to_string(),
        project_id: "project".to_string(),
        directory: "/tmp/project".to_string(),
        parent_id: None,
        summary: Some(vw_shared::session::info::Summary {
            additions,
            deletions,
            files: 1,
            diffs: None,
        }),
        share: None,
        title: id.to_string(),
        version: "1".to_string(),
        time: vw_shared::session::info::TimeInfo {
            created: 1,
            updated: 1,
            compacting: None,
            archived: None,
        },
        permission: None,
        revert: None,
    }
}

fn chat_message(content: &str) -> crate::app::models::ChatMessage {
    crate::app::models::ChatMessage {
        role: crate::app::models::ChatRole::User,
        content: content.to_string(),
        think_timing: Vec::new(),
    }
}

#[test]
fn merge_project_sessions_returns_incoming_when_no_existing_list() {
    let incoming = vec![session_info("incoming", 1, 2)];

    let merged = super::merge_project_sessions(None, incoming.clone());

    assert_eq!(merged.len(), 1);
    assert_eq!(merged[0].id, incoming[0].id);
}

#[test]
fn merge_project_sessions_replaces_existing_summary() {
    let existing = vec![session_info("task-board-1", 0, 0), session_info("local-only", 2, 1)];
    let incoming = vec![session_info("task-board-1", 7, 3)];

    let merged = super::merge_project_sessions(Some(existing), incoming);

    assert_eq!(merged.len(), 2);
    let summary = merged[0].summary.as_ref().expect("incoming summary should be kept");
    assert_eq!(merged[0].id, "task-board-1");
    assert_eq!(summary.additions, 7);
    assert_eq!(summary.deletions, 3);
    assert_eq!(merged[1].id, "local-only");
}

#[test]
fn sessions_loaded_updates_list_or_pushes_notification() {
    let mut app = app();
    let sessions = vec![session_info("s1", 1, 0)];

    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::SessionsLoaded(Ok(sessions.clone())),
    );

    assert!(task.is_some());
    assert_eq!(app.sessions.len(), 1);
    assert_eq!(app.sessions[0].id, "s1");

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::SessionsLoaded(Err("boom".to_string())),
    );
    assert!(app.notifications.iter().any(|item| item.message.contains("boom")));
}

#[test]
fn session_bootstrap_loaded_sets_previews_archived_and_sessions() {
    let mut app = app();
    let sessions = vec![session_info("s1", 1, 1)];
    let mut previews = std::collections::HashMap::new();
    previews.insert("s1".to_string(), "preview text".to_string());
    let archived = std::collections::HashSet::from(["old".to_string()]);

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::SessionBootstrapLoaded {
            result: Ok(sessions),
            previews: previews.clone(),
            archived_session_ids: archived.clone(),
        },
    );

    assert_eq!(app.session_previews, previews);
    assert_eq!(app.archived_session_ids, archived);
    assert_eq!(app.sessions[0].id, "s1");
}

#[test]
fn session_bootstrap_loaded_keeps_metadata_and_reports_session_error() {
    let mut app = app();
    let previews = std::collections::HashMap::from([(
        "failed-session".to_string(),
        "cached preview".to_string(),
    )]);
    let archived = std::collections::HashSet::from(["archived-session".to_string()]);

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::SessionBootstrapLoaded {
            result: Err("gateway unavailable".to_string()),
            previews: previews.clone(),
            archived_session_ids: archived.clone(),
        },
    );

    assert_eq!(app.session_previews, previews);
    assert_eq!(app.archived_session_ids, archived);
    assert!(app.sessions.is_empty());
    assert!(app.notifications.iter().any(|item| item.message.contains("gateway unavailable")));
}

#[tokio::test(flavor = "multi_thread")]
async fn recent_hover_uses_cached_sessions_without_scheduling_gateway_load() {
    let mut app = app();
    let path = "/tmp/cached-project".to_string();
    app.project_sessions.insert(path.clone(), Vec::new());

    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::RecentHovered(Some(path.clone())),
    )
    .expect("recent hover should return a task");
    let messages = collect_task_messages(task).await;

    assert_eq!(app.hovered_recent_project, Some(path.clone()));
    assert!(!app.project_sessions_loading.contains(&path));
    assert!(matches!(
        messages.as_slice(),
        [crate::app::Message::Chat(crate::app::message::ChatMessage::LoadInputPanelTodos)]
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn recent_hover_none_clears_hover_and_schedules_todo_refresh() {
    let mut app = app();
    app.hovered_recent_project = Some("/tmp/project".to_string());

    let task =
        super::handle(&mut app, crate::app::message::project::ProjectMessage::RecentHovered(None))
            .expect("recent hover should return a task");
    let messages = collect_task_messages(task).await;

    assert!(app.hovered_recent_project.is_none());
    assert!(matches!(
        messages.as_slice(),
        [crate::app::Message::Chat(crate::app::message::ChatMessage::LoadInputPanelTodos)]
    ));
}

#[test]
fn open_project_session_uses_known_directory_and_restores_cached_state() {
    let mut app = app();
    let mut target = session_info("target", 1, 1);
    target.directory = "/tmp/known-project".to_string();
    app.sessions = vec![target];
    app.project_path = Some("/tmp/known-project".to_string());
    app.active_session_id = Some("previous".to_string());
    app.chat = vec![chat_message("previous chat")];
    app.chat_message_ids = vec![Some("previous-message".to_string())];
    app.usage.input_tokens = 99;
    app.active_session_view_state.updated_ms = 42;
    app.active_session_view_state.base_ready = true;
    app.get_session_runtime_mut("target").is_requesting = true;
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

    assert!(task.is_some());
    assert_eq!(app.project_path.as_deref(), Some("/tmp/known-project"));
    assert_eq!(app.active_session_id.as_deref(), Some("target"));
    assert_eq!(app.chat.len(), 1);
    assert_eq!(app.chat[0].content, "cached target chat");
    assert_eq!(app.chat_message_ids, vec![Some("target-message".to_string())]);
    assert_eq!(app.usage, crate::app::models::TokenUsage::default());
    assert_eq!(app.active_session_view_state.updated_ms, 0);
    assert!(app.active_session_view_state.ui_preparing);
    assert!(!app.active_session_view_state.base_ready);
}

#[test]
fn open_recent_pressed_clears_hover_menus_and_new_session_picker_state() {
    let mut app = app();
    app.hovered_recent_project = Some("/tmp/project".to_string());
    app.session_menu_id = Some("session-1".to_string());
    app.session_menu_anchor = Some(iced::Point::new(8.0, 12.0));
    app.new_session_picker_project = Some("/tmp/project".to_string());
    app.new_session_picker_options = vec![("/tmp/worktree".to_string(), "Worktree".to_string())];
    app.new_session_worktree_name = "feature".to_string();
    app.new_session_delete_error = Some("stale delete".to_string());

    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::OpenRecentPressed("/tmp/project".to_string()),
    );

    assert!(task.is_some());
    assert!(app.hovered_recent_project.is_none());
    assert!(app.session_menu_id.is_none());
    assert!(app.session_menu_anchor.is_none());
    assert!(app.new_session_picker_project.is_none());
    assert!(app.new_session_picker_options.is_empty());
    assert!(app.new_session_worktree_name.is_empty());
    assert!(app.new_session_delete_error.is_none());
}

#[test]
fn recent_hover_marks_uncached_project_loading_once() {
    let mut app = app();
    let path = "/tmp/uncached-project".to_string();

    let first_task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::RecentHovered(Some(path.clone())),
    );

    assert!(first_task.is_some());
    assert!(app.project_sessions_loading.contains(&path));
    let first_refresh = app
        .project_sessions_last_refresh_at
        .get(&path)
        .copied()
        .expect("first hover should record refresh time");

    let second_task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::RecentHovered(Some(path.clone())),
    );

    assert!(second_task.is_some());
    assert_eq!(app.project_sessions_loading.len(), 1);
    assert_eq!(app.project_sessions_last_refresh_at.get(&path), Some(&first_refresh));
}

#[test]
fn project_sessions_loaded_merges_and_clears_loading_state() {
    let mut app = app();
    let path = "/tmp/project".to_string();
    app.project_sessions_loading.insert(path.clone());
    app.project_sessions.insert(path.clone(), vec![session_info("local", 0, 0)]);

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectSessionsLoaded(
            path.clone(),
            Ok(vec![session_info("remote", 3, 1)]),
        ),
    );

    assert!(!app.project_sessions_loading.contains(&path));
    let sessions = app.project_sessions.get(&path).expect("sessions should be stored");
    assert_eq!(sessions.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(), vec!["remote", "local"]);
    assert_eq!(app.project_session_load_counts.get(&path), Some(&10));
}

#[test]
fn project_sessions_loaded_records_error_message() {
    let mut app = app();
    let path = "/tmp/project".to_string();
    app.project_sessions_loading.insert(path.clone());

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectSessionsLoaded(
            path.clone(),
            Err("offline".to_string()),
        ),
    );

    assert!(!app.project_sessions_loading.contains(&path));
    assert_eq!(app.error_message.as_deref(), Some("Failed to load project sessions: offline"));
}

#[test]
fn project_sessions_loaded_prefers_incoming_duplicate_and_keeps_unique_existing_sessions() {
    let mut app = app();
    let path = "/tmp/project".to_string();
    app.project_sessions.insert(
        path.clone(),
        vec![session_info("duplicate", 1, 1), session_info("existing", 2, 2)],
    );

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectSessionsLoaded(
            path.clone(),
            Ok(vec![session_info("duplicate", 9, 8), session_info("incoming", 3, 4)]),
        ),
    );

    let sessions = app.project_sessions.get(&path).expect("sessions should be stored");
    assert_eq!(
        sessions.iter().map(|session| session.id.as_str()).collect::<Vec<_>>(),
        vec!["duplicate", "incoming", "existing"]
    );
    let summary = sessions[0].summary.as_ref().expect("duplicate should retain incoming data");
    assert_eq!((summary.additions, summary.deletions), (9, 8));
}

#[test]
fn project_session_scroll_and_load_more_update_per_project_state() {
    let mut app = app();
    let path = "/tmp/project".to_string();

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectSessionListScrollChanged {
            project_path: path.clone(),
            has_vertical_scrollbar: true,
        },
    );
    assert_eq!(app.project_session_has_vertical_scrollbar.get(&path), Some(&true));

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectLoadMoreSessions(path.clone()),
    );
    assert_eq!(app.project_session_load_counts.get(&path), Some(&15));

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectLoadMoreSessions(path.clone()),
    );
    assert_eq!(app.project_session_load_counts.get(&path), Some(&20));
}

#[test]
fn recent_overlay_closed_preserves_open_picker_but_otherwise_resets_overlay_state() {
    let mut app = app();
    app.new_session_picker_project = Some("/tmp/project".to_string());
    app.hovered_recent_project = Some("/tmp/project".to_string());

    let _ =
        super::handle(&mut app, crate::app::message::project::ProjectMessage::RecentOverlayClosed);
    assert!(app.new_session_picker_project.is_some());
    assert!(app.hovered_recent_project.is_some());

    app.new_session_picker_project = None;
    app.session_menu_id = Some("s1".to_string());
    app.project_tools_menu_path = Some("/tmp/project".to_string());

    let _ =
        super::handle(&mut app, crate::app::message::project::ProjectMessage::RecentOverlayClosed);

    assert!(app.hovered_recent_project.is_none());
    assert!(app.session_menu_id.is_none());
    assert!(app.project_tools_menu_path.is_none());
}

#[test]
fn recent_overlay_closed_clears_anchor_and_all_picker_state() {
    let mut app = app();
    app.hovered_recent_project = Some("/tmp/project".to_string());
    app.session_menu_id = Some("session-1".to_string());
    app.session_menu_anchor = Some(iced::Point::new(12.0, 24.0));
    app.project_tools_menu_path = Some("/tmp/project".to_string());
    app.new_session_picker_options = vec![("/tmp/worktree".to_string(), "Worktree".to_string())];
    app.new_session_worktree_name = "feature".to_string();
    app.new_session_confirm_delete_directory = Some("/tmp/worktree".to_string());
    app.new_session_delete_error = Some("old error".to_string());

    let _ =
        super::handle(&mut app, crate::app::message::project::ProjectMessage::RecentOverlayClosed);

    assert!(app.hovered_recent_project.is_none());
    assert!(app.session_menu_id.is_none());
    assert!(app.session_menu_anchor.is_none());
    assert!(app.project_tools_menu_path.is_none());
    assert!(app.new_session_picker_options.is_empty());
    assert!(app.new_session_worktree_name.is_empty());
    assert!(app.new_session_confirm_delete_directory.is_none());
    assert!(app.new_session_delete_error.is_none());
}

#[test]
fn project_create_session_opens_picker_when_worktree_enabled() {
    let mut app = app();
    let path = "/tmp/project".to_string();
    app.project_worktree_enabled.insert(path.clone(), true);
    app.new_session_picker_options.push(("old".to_string(), "旧".to_string()));
    app.new_session_worktree_name = "old-name".to_string();
    app.new_session_delete_error = Some("old error".to_string());

    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectCreateSession(path.clone()),
    );

    assert!(task.is_some());
    assert_eq!(app.new_session_picker_project, Some(path));
    assert!(app.new_session_picker_options.is_empty());
    assert!(app.new_session_worktree_name.is_empty());
    assert!(app.new_session_delete_error.is_none());
}

#[test]
fn project_create_session_without_worktree_resets_picker_before_scheduling_creation() {
    let mut app = app();
    let path = "/tmp/project".to_string();
    app.project_worktree_enabled.insert(path.clone(), false);
    app.new_session_picker_project = Some(path.clone());
    app.new_session_picker_options = vec![(path.clone(), "Main".to_string())];
    app.new_session_worktree_name = "stale-name".to_string();
    app.new_session_reset_error = Some("stale error".to_string());

    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectCreateSession(path),
    );

    assert!(task.is_some());
    assert!(app.new_session_picker_project.is_none());
    assert!(app.new_session_picker_options.is_empty());
    assert!(app.new_session_worktree_name.is_empty());
    assert!(app.new_session_reset_error.is_none());
}

#[test]
fn project_info_loaded_ignores_error_and_stale_project_result() {
    let mut app = app();
    app.project_path = Some("/tmp/current".to_string());
    app.project_id = Some("original-id".to_string());
    app.selected_branch = Some("original-branch".to_string());
    app.project_updated_at_ms = Some(7);

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

    assert_eq!(app.project_id.as_deref(), Some("original-id"));
    assert_eq!(app.selected_branch.as_deref(), Some("original-branch"));
    assert_eq!(app.project_updated_at_ms, Some(7));
}

#[test]
fn project_info_loaded_applies_current_project_identity_timestamp_and_branch() {
    let mut app = app();
    app.project_path = Some("/tmp/current".to_string());

    let _ =
        super::handle(
            &mut app,
            crate::app::message::project::ProjectMessage::ProjectInfoLoaded(Ok(
                loaded_project_info("/tmp/current", 42, Some("main")),
            )),
        );

    assert_eq!(app.project_id.as_deref(), Some("project-id"));
    assert_eq!(app.project_updated_at_ms, Some(42));
    assert_eq!(app.selected_branch.as_deref(), Some("main"));
}

#[test]
fn project_info_loaded_maps_zero_timestamp_to_none_and_keeps_branch_when_missing() {
    let mut app = app();
    app.project_path = Some("/tmp/current".to_string());
    app.selected_branch = Some("existing".to_string());
    app.project_updated_at_ms = Some(42);

    let _ =
        super::handle(
            &mut app,
            crate::app::message::project::ProjectMessage::ProjectInfoLoaded(Ok(
                loaded_project_info("/tmp/current", 0, None),
            )),
        );

    assert_eq!(app.project_updated_at_ms, None);
    assert_eq!(app.selected_branch.as_deref(), Some("existing"));
}

#[test]
fn project_info_loaded_updates_recent_project_icon_color_and_start_command() {
    let mut app = app();
    app.project_path = Some("/tmp/current".to_string());
    app.recent_projects_meta = vec![recent_project_meta("/tmp/current", true, 60)];
    let mut loaded = loaded_project_info("/tmp/current", 11, Some("main"));
    loaded.info.icon = Some(vw_shared::project::Icon {
        url: None,
        override_icon: Some("rocket".to_string()),
        color: Some("#112233".to_string()),
    });
    loaded.info.commands =
        Some(vw_shared::project::Commands { start: Some("cargo run".to_string()) });

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectInfoLoaded(Ok(loaded)),
    );

    let meta = &app.recent_projects_meta[0];
    assert_eq!(meta.icon.as_deref(), Some("rocket"));
    assert_eq!(meta.icon_color.as_deref(), Some("#112233"));
    assert_eq!(meta.worktree_start_command.as_deref(), Some("cargo run"));
    assert_eq!(app.project_id.as_deref(), Some("project-id"));
}

#[test]
fn project_branches_loaded_ignores_stale_project_and_applies_current_project() {
    let mut app = app();
    app.project_path = Some("/tmp/current".to_string());
    app.selected_branch = Some("main".to_string());

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectBranchesLoaded {
            project_path: "/tmp/other".to_string(),
            selected_branch: Some("feature".to_string()),
            branches: vec!["feature".to_string()],
        },
    );
    assert_eq!(app.selected_branch.as_deref(), Some("main"));

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectBranchesLoaded {
            project_path: "/tmp/current".to_string(),
            selected_branch: Some("develop".to_string()),
            branches: vec!["main".to_string(), "develop".to_string()],
        },
    );

    assert_eq!(app.selected_branch.as_deref(), Some("develop"));
    assert_eq!(app.branches, vec!["main".to_string(), "develop".to_string()]);
}

#[test]
fn project_load_sessions_skips_duplicate_loads_and_marks_new_load() {
    let mut app = app();
    let path = "/tmp/project".to_string();
    app.project_sessions_loading.insert(path.clone());

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectLoadSessions(path.clone()),
    );
    assert_eq!(app.project_sessions_loading.len(), 1);

    app.project_sessions_loading.clear();
    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectLoadSessions(path.clone()),
    );

    assert!(task.is_some());
    assert!(app.project_sessions_loading.contains(&path));
    assert!(app.project_sessions_last_refresh_at.contains_key(&path));
}

#[tokio::test(flavor = "multi_thread")]
async fn project_sessions_refresh_tick_skips_missing_disabled_and_not_due_projects() {
    let mut app = app();

    let no_project_task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectSessionsRefreshTick,
    )
    .expect("refresh tick should be handled");
    assert!(collect_task_messages(no_project_task).await.is_empty());

    let path = "/tmp/project".to_string();
    app.hovered_recent_project = Some(path.clone());
    app.recent_projects_meta = vec![recent_project_meta(&path, false, 60)];
    let disabled_task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectSessionsRefreshTick,
    )
    .expect("disabled refresh tick should be handled");
    assert!(collect_task_messages(disabled_task).await.is_empty());

    app.recent_projects_meta = vec![recent_project_meta(&path, true, 3600)];
    app.project_sessions_last_refresh_at.insert(path, web_time::Instant::now());
    let not_due_task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectSessionsRefreshTick,
    )
    .expect("not-due refresh tick should be handled");
    assert!(collect_task_messages(not_due_task).await.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn project_sessions_refresh_tick_schedules_selected_project_when_due() {
    let mut app = app();
    let hovered_path = "/tmp/hovered".to_string();
    let settings_path = "/tmp/settings".to_string();
    app.hovered_recent_project = Some(hovered_path);
    app.project_path = Some(settings_path.clone());
    app.show_settings = true;
    app.recent_projects_meta = vec![recent_project_meta(&settings_path, true, 1)];

    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectSessionsRefreshTick,
    )
    .expect("due refresh tick should be handled");
    let messages = collect_task_messages(task).await;

    assert!(matches!(
        messages.as_slice(),
        [crate::app::Message::Project(
            crate::app::message::project::ProjectMessage::ProjectLoadSessions(path)
        )] if path == &settings_path
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn project_sessions_refresh_tick_uses_hovered_project_outside_settings() {
    let mut app = app();
    let hovered_path = "/tmp/hovered".to_string();
    app.hovered_recent_project = Some(hovered_path.clone());
    app.project_path = Some("/tmp/open-project".to_string());
    app.show_settings = false;
    app.recent_projects_meta = vec![recent_project_meta(&hovered_path, true, 60)];

    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectSessionsRefreshTick,
    )
    .expect("due refresh tick should be handled");
    let messages = collect_task_messages(task).await;

    assert!(matches!(
        messages.as_slice(),
        [crate::app::Message::Project(
            crate::app::message::project::ProjectMessage::ProjectLoadSessions(path)
        )] if path == &hovered_path
    ));
}

#[test]
fn handle_returns_none_for_message_owned_by_another_project_submodule() {
    let mut app = app();

    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectToolsMenuClosed,
    );

    assert!(task.is_none());
}
