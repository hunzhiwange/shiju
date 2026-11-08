// 验证项目路径输入、最近项目加载与基础打开流程。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

// 编译接线：引用父模块符号，确保测试片段仍由真实模块包含。
// 该冒烟用例只承担接线职责，具体行为由同领域测试覆盖。
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

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn file_index_loaded_without_project_does_not_expose_cached_files() {
    let mut app = app();
    app.project_path = None;

    let task = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileIndexLoaded {
            project_path: "/workspace/project".into(),
            result: crate::app::FileIndexLoadResult {
                files: vec!["src/main.rs".to_string()],
                needs_refresh: true,
            },
        },
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
        crate::app::message::project::ProjectMessage::FileIndexLoaded {
            project_path: "/workspace/project".into(),
            result: crate::app::FileIndexLoadResult {
                files: vec!["/workspace/project/src/main.rs".to_string()],
                needs_refresh: false,
            },
        },
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
        crate::app::message::project::ProjectMessage::FileIndexLoaded {
            project_path: "/workspace/project".into(),
            result: crate::app::FileIndexLoadResult {
                files: vec!["/workspace/project/src/main.rs".to_string()],
                needs_refresh: true,
            },
        },
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

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
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

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn unrelated_project_message_is_not_claimed_by_basic_handler() {
    let mut app = app();

    let task =
        super::handle(&mut app, crate::app::message::project::ProjectMessage::RecentOverlayClosed);

    assert!(task.is_none());
}

fn empty_directory_snapshot(
    root: &str,
    empty: &str,
) -> super::super::helpers::FileTreeIndexSnapshot {
    use vw_gateway_client::vw_api_types::file::{FileNodeDto, FileNodeKind};
    super::super::helpers::FileTreeIndexSnapshot::from_dto(
        root,
        &FileNodeDto {
            path: ".".into(),
            name: ".".into(),
            kind: FileNodeKind::Directory,
            size_bytes: None,
            children: Some(vec![
                FileNodeDto {
                    path: empty.into(),
                    name: empty.into(),
                    kind: FileNodeKind::Directory,
                    size_bytes: None,
                    children: Some(vec![]),
                },
                FileNodeDto {
                    path: "readme.txt".into(),
                    name: "readme.txt".into(),
                    kind: FileNodeKind::File,
                    size_bytes: Some(4),
                    children: None,
                },
            ]),
        },
    )
}

#[test]
fn file_tree_empty_directory_snapshot_refresh_updates_tree_and_search_without_attachments() {
    use crate::app::message::project::ProjectMessage;
    let mut app = app();
    app.project_path = Some("/project".into());
    app.files = vec!["/attached.txt".into()];
    app.file_manager_file_tree_refreshing = true;
    let _ = super::handle(
        &mut app,
        ProjectMessage::FileTreeSnapshotReady {
            project_path: "/project".into(),
            snapshot: empty_directory_snapshot("/project", "empty"),
        },
    );
    assert!(!app.file_manager_file_tree_refreshing);
    assert!(app.current_file_tree_model().unwrap().children.contains_key("empty"));
    assert_eq!(app.current_file_index(), ["/project/readme.txt"]);
    assert_eq!(app.search_panel_file_cache_results, ["/project/readme.txt"]);
    assert_eq!(app.files, ["/attached.txt"]);
    // 晚到的本地纯文件缓存不能抹去网关保留的空目录。
    let _ = super::handle(
        &mut app,
        ProjectMessage::FileIndexLoaded {
            project_path: "/project".into(),
            result: crate::app::FileIndexLoadResult { files: vec![], needs_refresh: false },
        },
    );
    assert!(app.current_file_tree_model().unwrap().children.contains_key("empty"));
    let _ = super::handle(
        &mut app,
        ProjectMessage::FileTreeSnapshotReady {
            project_path: "/project".into(),
            snapshot: empty_directory_snapshot("/project", "imported"),
        },
    );
    assert!(!app.current_file_tree_model().unwrap().children.contains_key("empty"));
    assert!(app.current_file_tree_model().unwrap().children.contains_key("imported"));
}

#[test]
fn file_tree_empty_directory_old_project_result_only_updates_its_cache() {
    use crate::app::message::project::ProjectMessage;
    let mut app = app();
    app.project_path = Some("/current".into());
    app.set_file_index("/current", vec!["/current/file.txt".into()]);
    app.file_manager_file_tree_refreshing = true;
    let results = app.search_panel_file_cache_results.clone();
    let _ = super::handle(
        &mut app,
        ProjectMessage::FileTreeSnapshotReady {
            project_path: "/old".into(),
            snapshot: empty_directory_snapshot("/old", "empty"),
        },
    );
    assert_eq!(app.current_file_index(), ["/current/file.txt"]);
    assert_eq!(app.search_panel_file_cache_results, results);
    assert!(app.file_manager_file_tree_refreshing);
    assert!(!app.current_file_tree_model().unwrap().children.contains_key("empty"));
    app.project_path = Some("/old".into());
    assert!(app.current_file_tree_model().unwrap().children.contains_key("empty"));
    assert_eq!(app.current_file_index(), ["/old/readme.txt"]);
}

#[test]
fn file_tree_empty_directory_directory_only_snapshot_is_cached_with_empty_file_index() {
    use crate::app::message::project::ProjectMessage;
    let mut app = app();
    app.project_path = Some("/project".into());
    let mut snapshot = empty_directory_snapshot("/project", "empty");
    snapshot.files.clear();
    snapshot.tree.files.clear();
    let _ = super::handle(
        &mut app,
        ProjectMessage::FileTreeSnapshotReady { project_path: "/project".into(), snapshot },
    );
    assert!(app.current_file_index().is_empty());
    assert!(app.current_file_tree_model().unwrap().children.contains_key("empty"));
    assert!(app.search_panel_file_cache_results.is_empty());
}

#[test]
fn file_tree_empty_directory_old_local_load_does_not_pollute_current_project() {
    let mut app = app();
    app.project_path = Some("/current".into());
    let _ = super::handle(
        &mut app,
        crate::app::message::project::ProjectMessage::FileIndexLoaded {
            project_path: "/old".into(),
            result: crate::app::FileIndexLoadResult {
                files: vec!["/old/file.txt".into()],
                needs_refresh: true,
            },
        },
    );
    assert!(app.current_file_index().is_empty());
    assert!(!app.has_file_index("/current"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn project_file_preferences_update_before_gateway_reply() {
    use crate::app::message::project::ProjectMessage;
    use crate::app::{Message, config};
    use serde_json::json;
    use std::time::Duration;

    const TIMEOUT: Duration = Duration::from_secs(4);

    config::with_editor_settings_gateway(|mut app, requests, responses| {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap();
        let update = |app: &mut crate::app::App, message| app.update(Message::Project(message));
        let finish = |handle: tokio::task::JoinHandle<Vec<Message>>| {
            runtime
                .block_on(async { tokio::time::timeout(TIMEOUT, handle).await.unwrap().unwrap() })
        };
        let capture_patch = |expected| {
            let request = requests.recv_timeout(TIMEOUT).expect("preference PATCH");
            assert!(request.starts_with("PATCH /v1/desktop/preferences "));
            let fields: serde_json::Value =
                serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap();
            assert_eq!(fields, expected);
        };

        app.show_file_manager = false;
        let first = update(&mut app, ProjectMessage::FileManagerShowChanges(true));
        assert!(app.show_file_manager && app.file_manager_show_changes);
        assert!(requests.try_recv().is_err(), "the update already contacted Gateway");

        let first = runtime.spawn(collect_task_messages(first));
        capture_patch(json!({
            "show_file_manager": true, "file_manager_show_changes": true, "file_tree_expanded": []
        }));

        let toggle = || ProjectMessage::ToggleTreeDir("src".into());
        let expanded = update(&mut app, toggle());
        assert!(app.is_file_tree_dir_expanded("src"));
        assert!(iced_runtime::task::into_stream(expanded).is_none());
        let collapsed = update(&mut app, toggle());
        assert!(app.file_tree_expanded.is_empty());
        assert!(iced_runtime::task::into_stream(collapsed).is_none());
        let files = update(&mut app, ProjectMessage::FileManagerShowChanges(false));
        assert!(!app.file_manager_show_changes);
        assert!(iced_runtime::task::into_stream(files).is_none());

        responses.send(Some("{}".into())).unwrap();
        let [saved] = finish(first).try_into().ok().expect("one save completion");
        let second = runtime.spawn(collect_task_messages(app.update(saved)));
        capture_patch(json!({
            "show_file_manager": true, "file_manager_show_changes": false, "file_tree_expanded": []
        }));
        responses.send(Some("{}".into())).unwrap();
        let [saved] = finish(second).try_into().ok().expect("one save completion");
        assert!(iced_runtime::task::into_stream(app.update(saved)).is_none());
        assert!(requests.try_recv().is_err());
    });
}
