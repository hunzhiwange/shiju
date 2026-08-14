#[test]
fn test_module_is_wired() {
    let module = module_path!();

    assert!(module.ends_with("basic_tests"));
}

fn app() -> crate::app::App {
    crate::app::App::new().0
}

async fn collect_task_messages(task: iced::Task<crate::app::Message>) -> Vec<crate::app::Message> {
    use iced_runtime::Action;
    use iced_runtime::futures::futures::StreamExt;

    let Some(mut stream) = iced_runtime::task::into_stream(task.collect()) else {
        return Vec::new();
    };

    while let Some(action) = stream.next().await {
        if let Action::Output(messages) = action {
            return messages;
        }
    }

    Vec::new()
}

fn recent_project(path: &str, name: &str) -> crate::app::RecentProjectMeta {
    crate::app::RecentProjectMeta {
        path: path.to_string(),
        name: name.to_string(),
        task_board_settings: None,
        session_auto_refresh: true,
        session_refresh_interval_seconds: 60,
        icon: None,
        icon_color: None,
        worktree_start_command: None,
    }
}

#[test]
fn project_path_changed_updates_input() {
    let mut app = app();

    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ProjectPathChanged(
            "/tmp/project".to_string(),
        ),
    );

    assert!(task.is_some());
    assert_eq!(app.project_path_input, "/tmp/project");
}

#[test]
fn open_project_pressed_uses_non_empty_input_path() {
    let mut app = app();
    app.project_path_input = "/workspace/from-input".to_string();

    let task =
        super::handle(&mut app, crate::app::message::project::ProjectMessage::OpenProjectPressed);

    assert!(task.is_some());
    assert_eq!(app.project_path_input, "/workspace/from-input");
}

#[test]
fn recent_projects_loaded_success_replaces_all_recent_project_views() {
    let mut app = app();
    app.recent_projects = vec!["stale".to_string()];
    app.recent_projects_edits = vec!["Stale".to_string()];
    app.recent_projects_meta = vec![recent_project("stale", "Stale")];
    let projects =
        vec![recent_project("/workspace/one", "One"), recent_project("/workspace/two", "Two")];

    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::RecentProjectsLoaded(Ok(projects)),
    );

    assert!(task.is_some());
    assert_eq!(app.recent_projects, ["/workspace/one", "/workspace/two"]);
    assert_eq!(app.recent_projects_edits, ["One", "Two"]);
    assert_eq!(app.recent_projects_meta.len(), 2);
    assert_eq!(app.recent_projects_meta[1].name, "Two");
}

#[test]
fn recent_projects_loaded_error_preserves_existing_projects() {
    let mut app = app();
    app.recent_projects = vec!["/workspace/keep".to_string()];
    app.recent_projects_edits = vec!["Keep".to_string()];
    app.recent_projects_meta = vec![recent_project("/workspace/keep", "Keep")];

    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::RecentProjectsLoaded(Err(
            "gateway unavailable".to_string(),
        )),
    );

    assert!(task.is_some());
    assert_eq!(app.recent_projects, ["/workspace/keep"]);
    assert_eq!(app.recent_projects_edits, ["Keep"]);
    assert_eq!(app.recent_projects_meta[0].path, "/workspace/keep");
}

#[test]
fn file_url_changed_and_add_file_pressed_append_file_once() {
    let mut app = app();

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileUrlChanged(
            "https://example.test/a.rs".to_string(),
        ),
    );
    let _ = super::handle(&mut app, crate::app::message::project::ProjectMessage::AddFilePressed);

    assert_eq!(app.files, vec!["https://example.test/a.rs".to_string()]);
    assert!(app.file_url_input.is_empty());
}

#[test]
fn add_file_pressed_ignores_empty_input() {
    let mut app = app();
    app.file_url_input.clear();

    let _ = super::handle(&mut app, crate::app::message::project::ProjectMessage::AddFilePressed);

    assert!(app.files.is_empty());
}

#[test]
fn attachment_files_picked_appends_paths() {
    let mut app = app();
    let first = tempfile::NamedTempFile::new().expect("temp attachment should be created");
    let second = tempfile::NamedTempFile::new().expect("temp attachment should be created");
    let first_path = first.path().canonicalize().unwrap().to_string_lossy().to_string();
    let second_path = second.path().canonicalize().unwrap().to_string_lossy().to_string();

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::AttachmentFilesPicked(Some(vec![
            first.path().to_string_lossy().to_string(),
            second.path().to_string_lossy().to_string(),
        ])),
    );

    assert!(app.files.contains(&first_path));
    assert!(app.files.contains(&second_path));
}

#[test]
fn attachment_files_picked_none_preserves_existing_files() {
    let mut app = app();
    app.files = vec!["existing.txt".to_string()];

    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::AttachmentFilesPicked(None),
    );

    assert!(task.is_some());
    assert_eq!(app.files, ["existing.txt"]);
}

#[test]
fn remove_attached_file_removes_matching_path() {
    let mut app = app();
    app.files = vec!["/tmp/a.txt".to_string(), "/tmp/b.txt".to_string()];

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::RemoveAttachedFile("/tmp/a.txt".to_string()),
    );

    assert_eq!(app.files, vec!["/tmp/b.txt".to_string()]);
}

#[test]
fn remove_attached_file_removes_every_duplicate_and_preserves_other_paths() {
    let mut app = app();
    app.files = vec!["a.txt".to_string(), "b.txt".to_string(), "a.txt".to_string()];

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::RemoveAttachedFile("a.txt".to_string()),
    );

    assert_eq!(app.files, ["b.txt"]);
}

#[test]
fn file_index_loaded_without_project_does_not_expose_cached_files() {
    let mut app = app();
    app.project_path = None;

    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileIndexLoaded(
            crate::app::FileIndexLoadResult {
                files: vec!["src/main.rs".to_string()],
                needs_refresh: true,
            },
        ),
    );

    assert!(task.is_some());
    assert!(app.current_file_index().is_empty());
}

#[test]
fn file_index_loaded_for_project_sets_cached_files_without_changing_project() {
    let mut app = app();
    app.project_path = Some("/workspace/project".to_string());

    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileIndexLoaded(
            crate::app::FileIndexLoadResult {
                files: vec!["/workspace/project/src/main.rs".to_string()],
                needs_refresh: false,
            },
        ),
    );

    assert!(task.is_some());
    assert_eq!(app.project_path.as_deref(), Some("/workspace/project"));
    assert_eq!(app.current_file_index(), ["/workspace/project/src/main.rs"]);
}

#[test]
fn file_index_loaded_requests_refresh_when_project_exists_and_index_is_stale() {
    let mut app = app();
    app.project_path = Some("/workspace/project".to_string());

    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileIndexLoaded(
            crate::app::FileIndexLoadResult {
                files: vec!["/workspace/project/src/main.rs".to_string()],
                needs_refresh: true,
            },
        ),
    );

    assert!(task.is_some());
    assert_eq!(app.current_file_index(), ["/workspace/project/src/main.rs"]);
}

#[test]
fn file_index_ready_sets_index_and_stops_refreshing() {
    let mut app = app();
    app.project_path = Some("/tmp/project".to_string());
    app.file_manager_file_tree_refreshing = true;

    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileIndexReady(vec![
            "/tmp/project/src/main.rs".to_string(),
        ]),
    );

    assert!(task.is_some());
    assert_eq!(app.current_file_index(), &["/tmp/project/src/main.rs".to_string()]);
    assert!(!app.file_manager_file_tree_refreshing);
}

#[test]
fn file_index_ready_without_active_refresh_only_sets_index() {
    let mut app = app();
    app.project_path = Some("/tmp/project".to_string());
    app.file_manager_file_tree_refreshing = false;

    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileIndexReady(vec![
            "/tmp/project/src/lib.rs".to_string(),
        ]),
    );

    assert!(task.is_some());
    assert_eq!(app.current_file_index(), &["/tmp/project/src/lib.rs".to_string()]);
    assert!(!app.file_manager_file_tree_refreshing);
}

#[test]
fn toggle_tree_dir_toggles_expanded_state() {
    let mut app = app();

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ToggleTreeDir("src".to_string()),
    );
    assert!(app.file_tree_expanded.contains(&"src".to_string()));

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::ToggleTreeDir("src".to_string()),
    );
    assert!(!app.file_tree_expanded.contains(&"src".to_string()));
}

#[test]
fn file_manager_show_changes_true_clears_active_preview_and_expands_changed_dirs() {
    let mut app = app();
    app.active_preview_path = Some("/tmp/project/src/main.rs".to_string());
    app.show_file_manager = false;
    app.git_changed_files =
        vec!["src/main.rs".to_string(), "src/nested/lib.rs".to_string(), "README.md".to_string()];

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileManagerShowChanges(true),
    );

    assert!(app.file_manager_show_changes);
    assert!(app.active_preview_path.is_none());
    assert!(app.show_file_manager);
    assert!(app.file_tree_expanded.contains(&"chg:src".to_string()));
    assert!(app.file_tree_expanded.contains(&"chg:src/nested".to_string()));
    assert!(!app.file_tree_expanded.contains(&"src".to_string()));
}

#[tokio::test(flavor = "multi_thread")]
async fn file_manager_show_changes_true_in_work_mode_reveals_workspace_files() {
    let mut app = app();
    app.screen = crate::app::Screen::Project;
    app.workspace_mode = crate::app::state::WorkspaceMode::Work;
    app.work_resources_visible = false;
    app.work_resource_mode = crate::app::state::WorkResourceMode::Browser;
    app.work_chat_fullscreen = true;
    app.git_changed_files = vec!["src/main.rs".to_string()];

    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileManagerShowChanges(true),
    )
    .expect("task");

    assert!(app.file_manager_show_changes);
    assert!(app.work_resources_visible);
    assert_eq!(app.work_resource_mode, crate::app::state::WorkResourceMode::WorkspaceFiles);
    assert!(!app.work_chat_fullscreen);
    assert!(collect_task_messages(task).await.iter().any(|message| {
        matches!(
            message,
            crate::app::Message::View(crate::app::message::ViewMessage::WorkResourceModeSelected(
                crate::app::state::WorkResourceMode::WorkspaceFiles
            ))
        )
    }));
}

#[test]
fn file_manager_show_changes_false_preserves_preview_and_resets_find_selection() {
    let mut app = app();
    app.active_preview_path = Some("/tmp/project/src/main.rs".to_string());
    app.active_find_results_tab_id = Some("find-results".to_string());
    app.show_file_manager = false;

    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileManagerShowChanges(false),
    );

    assert!(task.is_some());
    assert!(!app.file_manager_show_changes);
    assert_eq!(app.active_preview_path.as_deref(), Some("/tmp/project/src/main.rs"));
    assert!(app.active_find_results_tab_id.is_none());
    assert!(app.show_file_manager);
}

#[test]
fn file_manager_refresh_changes_ignores_refresh_while_git_files_are_loading() {
    let mut app = app();
    app.git_changed_files_loading = true;
    app.file_manager_changes_refreshing = false;

    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileManagerRefreshChanges,
    );

    assert!(task.is_some());
    assert!(!app.file_manager_changes_refreshing);
}

#[test]
fn file_manager_refresh_changes_ignores_duplicate_refresh() {
    let mut app = app();
    app.git_changed_files_loading = false;
    app.file_manager_changes_refreshing = true;

    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileManagerRefreshChanges,
    );

    assert!(task.is_some());
    assert!(app.file_manager_changes_refreshing);
    assert_eq!(app.file_manager_refresh_frame, 0);
}

#[test]
fn file_manager_refresh_changes_marks_refreshing_when_idle() {
    let mut app = app();
    app.git_changed_files_loading = false;
    app.file_manager_changes_refreshing = false;

    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileManagerRefreshChanges,
    );

    assert!(task.is_some());
    assert!(app.file_manager_changes_refreshing);
}

#[test]
fn file_manager_refresh_file_tree_ignores_duplicate_refresh() {
    let mut app = app();
    app.file_manager_file_tree_refreshing = true;

    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileManagerRefreshFileTree,
    );

    assert!(task.is_some());
    assert!(app.file_manager_file_tree_refreshing);
}

#[test]
fn file_manager_refresh_file_tree_marks_refreshing_when_idle() {
    let mut app = app();
    app.project_path = None;
    app.file_manager_file_tree_refreshing = false;

    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileManagerRefreshFileTree,
    );

    assert!(task.is_some());
    assert!(app.file_manager_file_tree_refreshing);
}

#[test]
fn file_manager_refresh_animation_tick_stops_when_all_work_is_idle() {
    let mut app = app();
    app.file_manager_refresh_frame = 7;
    app.file_manager_changes_refreshing = false;
    app.file_manager_file_tree_refreshing = false;
    app.git_commit_in_progress = false;

    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileManagerRefreshAnimationTick,
    );

    assert!(task.is_some());
    assert_eq!(app.file_manager_refresh_frame, 7);
}

#[test]
fn file_manager_refresh_animation_tick_wraps_frame_while_work_is_active() {
    let mut app = app();
    app.file_manager_refresh_frame = usize::MAX;
    app.file_manager_changes_refreshing = true;

    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileManagerRefreshAnimationTick,
    );

    assert!(task.is_some());
    assert_eq!(app.file_manager_refresh_frame, 0);
}

#[test]
fn open_changed_file_without_project_returns_none_task_only() {
    let mut app = app();
    app.project_path = None;

    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::OpenChangedFile("src/main.rs".to_string()),
    );

    assert!(task.is_some());
    assert!(app.project_path.is_none());
    assert!(app.file_manager_show_changes);
}

#[tokio::test(flavor = "multi_thread")]
async fn open_changed_file_in_work_mode_reveals_workspace_changes_and_focuses_file() {
    let mut app = app();
    app.screen = crate::app::Screen::Project;
    app.workspace_mode = crate::app::state::WorkspaceMode::Work;
    app.project_path = Some("/workspace/project".to_string());
    app.work_resources_visible = false;
    app.work_resource_mode = crate::app::state::WorkResourceMode::Overview;
    app.work_chat_fullscreen = true;
    app.file_manager_show_changes = false;
    app.show_file_manager = false;
    app.git_changed_files = vec!["src/nested/lib.rs".to_string(), "README.md".to_string()];

    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::OpenChangedFile(
            "src/nested/lib.rs".to_string(),
        ),
    )
    .expect("task");

    assert!(app.work_resources_visible);
    assert_eq!(app.work_resource_mode, crate::app::state::WorkResourceMode::WorkspaceFiles);
    assert!(!app.work_chat_fullscreen);
    assert!(app.file_manager_show_changes);
    assert!(app.show_file_manager);
    assert!(app.file_tree_expanded.contains(&"chg:src".to_string()));
    assert!(app.file_tree_expanded.contains(&"chg:src/nested".to_string()));

    let messages = collect_task_messages(task).await;
    assert!(messages.iter().any(|message| {
        matches!(
            message,
            crate::app::Message::View(crate::app::message::ViewMessage::WorkResourceModeSelected(
                crate::app::state::WorkResourceMode::WorkspaceFiles
            ))
        )
    }));
    assert!(messages.iter().any(|message| {
        matches!(
            message,
            crate::app::Message::Git(crate::app::message::GitMessage::FocusFile(path))
                if path == "src/nested/lib.rs"
        )
    }));
}

#[test]
fn open_changed_file_expands_collapsed_change_dirs_when_already_in_changes_view() {
    let mut app = app();
    app.screen = crate::app::Screen::Project;
    app.workspace_mode = crate::app::state::WorkspaceMode::Work;
    app.project_path = Some("/workspace/project".to_string());
    app.work_resources_visible = true;
    app.work_resource_mode = crate::app::state::WorkResourceMode::WorkspaceFiles;
    app.file_manager_show_changes = true;
    app.show_file_manager = true;
    app.git_changed_files = vec!["src/nested/lib.rs".to_string()];
    app.file_tree_expanded.clear();
    app.file_tree_expanded_set.clear();

    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::OpenChangedFile(
            "src/nested/lib.rs".to_string(),
        ),
    );

    assert!(app.file_manager_show_changes);
    assert!(app.file_tree_expanded.contains(&"chg:src".to_string()));
    assert!(app.file_tree_expanded.contains(&"chg:src/nested".to_string()));
}

#[test]
fn file_tree_add_to_chat_rejects_path_outside_project_without_mutating_search() {
    let mut app = app();
    app.project_path = Some("/workspace/project".to_string());
    app.show_file_search = true;
    app.file_search_query = "main".to_string();

    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeAddToChat {
            path: "/workspace/other/main.rs".to_string(),
            line: 3,
            column: 5,
        },
    );

    assert!(task.is_some());
    assert!(app.show_file_search);
    assert_eq!(app.file_search_query, "main");
    assert!(app.current_session_runtime().input_editor.text().is_empty());
}

#[test]
fn file_tree_add_to_chat_without_project_preserves_editor_and_search_state() {
    let mut app = app();
    app.project_path = None;
    app.show_file_search = true;
    app.file_search_query = "main".to_string();
    app.file_search_selected_index = 2;

    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeAddToChat {
            path: "/workspace/project/src/main.rs".to_string(),
            line: 3,
            column: 5,
        },
    );

    assert!(task.is_some());
    assert!(app.show_file_search);
    assert_eq!(app.file_search_query, "main");
    assert_eq!(app.file_search_selected_index, 2);
    assert!(app.current_session_runtime().input_editor.text().is_empty());
}

#[test]
fn file_tree_add_to_chat_inserts_relative_mention_and_resets_search_state() {
    let mut app = app();
    app.project_path = Some("/workspace/project".to_string());
    app.show_file_search = true;
    app.file_search_query = "main".to_string();
    app.file_search_selected_index = 4;

    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileTreeAddToChat {
            path: "/workspace/project/src/main.rs".to_string(),
            line: 3,
            column: 5,
        },
    );

    assert!(task.is_some());
    assert!(!app.show_file_search);
    assert!(app.file_search_query.is_empty());
    assert_eq!(app.file_search_selected_index, 0);
    assert_eq!(app.current_session_runtime().input_editor.text(), "@src/main.rs:3:5 ");
    assert_eq!(app.focus_area, crate::app::FocusArea::None);
}

#[test]
fn start_deferred_tasks_rejects_stale_project_path() {
    let mut app = app();
    app.project_path = Some("/workspace/current".to_string());

    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::StartDeferredTasks {
            project_path: "/workspace/stale".to_string(),
        },
    );

    assert!(task.is_some());
    assert_eq!(app.project_path.as_deref(), Some("/workspace/current"));
}

#[test]
fn unrelated_project_message_is_not_claimed_by_basic_handler() {
    let mut app = app();

    let task =
        super::handle(&mut app, crate::app::message::project::ProjectMessage::RecentOverlayClosed);

    assert!(task.is_none());
}
