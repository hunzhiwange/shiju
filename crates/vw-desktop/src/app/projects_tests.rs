#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("projects_tests"));
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

#[tokio::test]
async fn project_settings_refresh_task_reloads_agents_and_skills_for_project() {
    let messages =
        collect_task_messages(super::project_settings_refresh_task("/repo/alpha".to_string()))
            .await;

    assert!(messages.iter().any(|message| matches!(
        message,
        crate::app::Message::Settings(crate::app::message::settings::SettingsMessage::Agents(
            crate::app::message::settings::AgentsMessage::ProjectChanged(project_path)
        )) if project_path == "/repo/alpha"
    )));
    assert!(messages.iter().any(|message| matches!(
        message,
        crate::app::Message::Settings(
            crate::app::message::settings::SettingsMessage::SkillsRefresh
        )
    )));
}

#[tokio::test]
async fn open_first_recent_project_or_home_goes_home_when_no_projects_remain() {
    let (mut app, _) = crate::app::App::new();
    app.recent_projects.clear();

    let messages = collect_task_messages(app.open_first_recent_project_or_home()).await;

    assert!(messages.iter().any(|message| matches!(
        message,
        crate::app::Message::View(crate::app::message::ViewMessage::GoHome)
    )));
}

#[test]
fn open_project_immediately_moves_agent_settings_to_new_project_scope() {
    let (mut app, _) = crate::app::App::new();
    app.agents_settings.project_path = Some("/repo/old".to_string());
    app.agents_settings.loading = false;

    let _ = app.open_project("/repo/new".to_string());

    assert_eq!(app.project_path.as_deref(), Some("/repo/new"));
    assert_eq!(app.agents_settings.project_path.as_deref(), Some("/repo/new"));
    assert!(app.agents_settings.loading);
}

fn recent_meta(path: &str) -> super::RecentProjectMeta {
    super::RecentProjectMeta {
        path: path.to_string(),
        name: "Project".to_string(),
        task_board_settings: None,
        session_auto_refresh: crate::app::state::default_recent_project_session_auto_refresh(),
        session_refresh_interval_seconds:
            crate::app::state::default_recent_project_session_refresh_interval_seconds(),
        icon: None,
        icon_color: None,
        worktree_start_command: None,
    }
}

#[test]
fn recent_project_dir_name_trims_whitespace_and_trailing_separators() {
    assert_eq!(super::recent_project_dir_name("  /repo/alpha///  "), Some("alpha"));
    assert_eq!(super::recent_project_dir_name("C:\\repo\\beta\\"), Some("beta"));
}

#[test]
fn recent_project_dir_name_rejects_empty_and_separator_only_paths() {
    assert_eq!(super::recent_project_dir_name("   "), None);
    assert_eq!(super::recent_project_dir_name("///"), None);
    assert_eq!(super::recent_project_dir_name("\\\\"), None);
}

#[test]
fn is_visible_recent_project_path_rejects_hidden_last_segment() {
    assert!(!super::is_visible_recent_project_path(
        "/var/folders/dt/hvzzy7ds3756n3cpg14j17200000gn/T/.tmp0fBHwT"
    ));
}

#[test]
fn is_visible_recent_project_path_rejects_windows_hidden_last_segment() {
    assert!(!super::is_visible_recent_project_path("C:\\Users\\me\\.tmp0fBHwT\\"));
}

#[test]
fn is_visible_recent_project_path_accepts_visible_project_under_hidden_parent() {
    assert!(super::is_visible_recent_project_path("/Users/me/.local/vibe-window"));
}

#[test]
fn is_visible_recent_project_path_rejects_empty_and_dot_names() {
    assert!(!super::is_visible_recent_project_path(""));
    assert!(!super::is_visible_recent_project_path("/repo/."));
    assert!(!super::is_visible_recent_project_path("/repo/.."));
}

#[test]
fn parse_recent_projects_filters_hidden_projects() {
    let content = r#"[
        "/Users/me/vibe-window",
        "/var/folders/dt/hvzzy7ds3756n3cpg14j17200000gn/T/.tmp0fBHwT",
        "/Users/me/vibe-window"
    ]"#;

    assert_eq!(
        super::parse_recent_projects(content),
        Some(vec!["/Users/me/vibe-window".to_string()])
    );
}

#[test]
fn parse_recent_projects_accepts_legacy_metadata_array() {
    let content = serde_json::to_string(&vec![
        recent_meta(" /repo/alpha "),
        recent_meta("/repo/beta"),
        recent_meta("/repo/alpha"),
    ])
    .expect("recent project metadata should serialize");

    assert_eq!(
        super::parse_recent_projects(&content),
        Some(vec!["/repo/alpha".to_string(), "/repo/beta".to_string()])
    );
}

#[test]
fn parse_recent_projects_accepts_mixed_array_entries() {
    let content = r#"[
        "/repo/alpha",
        {"path": " /repo/beta "},
        {"path": 7},
        false,
        "/repo/.hidden",
        "/repo/alpha"
    ]"#;

    assert_eq!(
        super::parse_recent_projects(content),
        Some(vec!["/repo/alpha".to_string(), "/repo/beta".to_string()])
    );
}

#[test]
fn parse_recent_projects_accepts_nested_recent_projects_entries() {
    let content = r#"{
        "recent_projects": [
            {"path": "/repo/alpha"},
            "/repo/beta",
            null,
            {"name": "missing path"}
        ]
    }"#;

    assert_eq!(
        super::parse_recent_projects(content),
        Some(vec!["/repo/alpha".to_string(), "/repo/beta".to_string()])
    );
}

#[test]
fn parse_recent_projects_rejects_malformed_or_unrelated_objects() {
    assert_eq!(super::parse_recent_projects("not json"), None);
    assert_eq!(super::parse_recent_projects(r#"{"projects": []}"#), None);
    assert_eq!(super::parse_recent_projects(r#"{"recent_projects": {}}"#), None);
}

#[test]
fn parse_recent_projects_returns_empty_list_for_valid_scalar_json() {
    assert_eq!(super::parse_recent_projects("null"), Some(vec![]));
    assert_eq!(super::parse_recent_projects("42"), Some(vec![]));
}

#[test]
fn normalize_recent_projects_trims_filters_and_preserves_first_seen_order() {
    let normalized = super::normalize_recent_projects(vec![
        " /repo/beta ".to_string(),
        "".to_string(),
        "/repo/.hidden".to_string(),
        "/repo/alpha".to_string(),
        "/repo/beta".to_string(),
    ]);

    assert_eq!(normalized, vec!["/repo/beta".to_string(), "/repo/alpha".to_string()]);
}

#[test]
fn normalize_recent_projects_preserves_more_than_recent_list_storage_limit() {
    let recent = (0..12).map(|index| format!("/repo/project-{index}")).collect::<Vec<_>>();

    let normalized = super::normalize_recent_projects(recent.clone());

    assert_eq!(normalized, recent);
}

#[test]
fn normalize_recent_projects_meta_filters_hidden_projects() {
    let metas = vec![
        recent_meta("/Users/me/vibe-window"),
        recent_meta("/var/folders/dt/hvzzy7ds3756n3cpg14j17200000gn/T/.tmp0fBHwT"),
        recent_meta(" /Users/me/vibe-window "),
    ];

    let normalized = super::normalize_recent_projects_meta(metas);

    assert_eq!(normalized.len(), 1);
    assert_eq!(normalized[0].path, "/Users/me/vibe-window");
}

#[test]
fn normalize_recent_projects_meta_keeps_first_metadata_for_duplicate_path() {
    let mut first = recent_meta(" /repo/alpha ");
    first.name = "First".to_string();
    let mut duplicate = recent_meta("/repo/alpha");
    duplicate.name = "Second".to_string();

    let normalized = super::normalize_recent_projects_meta(vec![first, duplicate]);

    assert_eq!(normalized.len(), 1);
    assert_eq!(normalized[0].path, "/repo/alpha");
    assert_eq!(normalized[0].name, "First");
}

#[test]
fn ensure_recent_project_keeps_existing_project_order() {
    let mut recent =
        vec!["/repo/alpha".to_string(), "/repo/beta".to_string(), "/repo/gamma".to_string()];

    super::ensure_recent_project(&mut recent, "/repo/beta".to_string());

    assert_eq!(
        recent,
        vec!["/repo/alpha".to_string(), "/repo/beta".to_string(), "/repo/gamma".to_string()]
    );
}

#[test]
fn ensure_recent_project_adds_new_project_to_front() {
    let mut recent = vec!["/repo/alpha".to_string(), "/repo/beta".to_string()];

    super::ensure_recent_project(&mut recent, "/repo/gamma".to_string());

    assert_eq!(
        recent,
        vec!["/repo/gamma".to_string(), "/repo/alpha".to_string(), "/repo/beta".to_string()]
    );
}

#[test]
fn push_recent_project_moves_existing_to_front_and_truncates_to_storage_limit() {
    let mut recent = (0..11).map(|index| format!("/repo/project-{index}")).collect::<Vec<_>>();

    super::push_recent_project(&mut recent, "/repo/project-5".to_string());

    assert_eq!(recent.len(), 10);
    assert_eq!(recent.first().map(String::as_str), Some("/repo/project-5"));
    assert_eq!(recent.iter().filter(|path| *path == "/repo/project-5").count(), 1);
    assert!(!recent.contains(&"/repo/project-10".to_string()));
}

#[test]
fn open_project_resets_git_capability_until_project_info_loads() {
    let (mut app, _task) = crate::app::App::new();
    app.project_path = Some("/repo/old".to_string());
    app.project_is_git_repo = true;

    let _ = app.open_project("/repo/new".to_string());

    assert_eq!(app.project_path.as_deref(), Some("/repo/new"));
    assert!(!app.project_is_git_repo);
}

#[test]
fn reset_task_board_project_state_clears_project_scoped_board_runtime() {
    let (mut app, _task) = crate::app::App::new();
    let mut task = crate::app::task::Task::new(1);
    task.id = "T-old-project".to_string();
    task.status = crate::app::task::TaskStatus::Running;
    app.task_board_tasks = vec![task.clone()];
    app.task_board_executor.start_task(&task.id);
    app.task_board_executor_running = true;
    app.task_board_selected_tasks.insert(task.id.clone());
    app.task_board_viewing_logs = Some(task);
    app.task_board_worktree_snapshot_loading = true;
    app.project_is_git_repo = true;

    app.reset_task_board_project_state();

    assert!(!app.project_is_git_repo);
    assert!(app.task_board_tasks.is_empty());
    assert!(app.task_board_executor.running_tasks.is_empty());
    assert!(!app.task_board_executor_running);
    assert!(app.task_board_selected_tasks.is_empty());
    assert!(app.task_board_viewing_logs.is_none());
    assert!(!app.task_board_worktree_snapshot_loading);
}

#[test]
fn refresh_branches_clears_project_branch_snapshot() {
    let (mut app, _task) = crate::app::App::new();
    app.project_path = Some("/repo/alpha".to_string());
    app.selected_branch = Some("feature/static-tests".to_string());
    app.branches = vec!["main".to_string(), "feature/static-tests".to_string()];
    app.project_updated_at_ms = Some(123);

    app.refresh_branches();

    assert_eq!(app.selected_branch, None);
    assert!(app.branches.is_empty());
    assert_eq!(app.project_updated_at_ms, None);
}

#[test]
fn refresh_branches_leaves_snapshot_unchanged_without_project() {
    let (mut app, _task) = crate::app::App::new();
    app.project_path = None;
    app.selected_branch = Some("main".to_string());
    app.branches = vec!["main".to_string()];
    app.project_updated_at_ms = Some(456);

    app.refresh_branches();

    assert_eq!(app.selected_branch.as_deref(), Some("main"));
    assert_eq!(app.branches, vec!["main".to_string()]);
    assert_eq!(app.project_updated_at_ms, Some(456));
}

#[test]
fn switch_project_terminal_keeps_hidden_project_without_pty() {
    let (mut app, _task) = crate::app::App::new();
    assert!(app.terminal.tabs.is_empty());
    app.terminal.is_visible = false;

    assert!(app.switch_project_terminal(None, Some("/repo/lazy".to_string())));

    assert!(!app.terminal.is_visible);
    assert!(app.terminal.tabs.is_empty());
    assert_eq!(app.terminal.active_id, None);
}

#[test]
fn switch_project_terminal_is_noop_for_matching_paths() {
    let (mut app, _task) = crate::app::App::new();

    let created = app
        .switch_project_terminal(Some("/repo/alpha".to_string()), Some("/repo/alpha".to_string()));

    assert!(!created);
    assert!(app.terminals_by_project.is_empty());
}

#[test]
fn switch_project_terminal_saves_previous_and_creates_blank_without_new_project() {
    let (mut app, _task) = crate::app::App::new();
    app.terminal.is_visible = true;
    app.terminal.height = 240.0;

    let created = app.switch_project_terminal(Some("/repo/alpha".to_string()), None);

    assert!(!created);
    assert!(app.terminals_by_project.contains_key("/repo/alpha"));
    assert!(!app.terminal.is_visible);
    assert_eq!(app.terminal.height, 240.0);
}

#[test]
fn switch_project_terminal_creates_new_terminal_then_restores_cached_state() {
    let (mut app, _task) = crate::app::App::new();
    app.terminal.is_visible = true;

    assert!(app.switch_project_terminal(None, Some("/repo/alpha".to_string())));
    app.terminal.is_visible = false;
    assert!(
        app.switch_project_terminal(
            Some("/repo/alpha".to_string()),
            Some("/repo/beta".to_string()),
        )
    );

    app.terminal.is_visible = true;
    let restored = app
        .switch_project_terminal(Some("/repo/beta".to_string()), Some("/repo/alpha".to_string()));

    assert!(!restored);
    assert!(!app.terminal.is_visible);
    assert!(app.terminals_by_project.contains_key("/repo/beta"));
}

#[test]
fn open_project_removes_apps_tab_creates_project_tab_and_recent_edit_name() {
    let (mut app, _task) = crate::app::App::new();
    app.open_tabs.clear();
    app.project_path = None;
    app.active_tab_id = None;
    app.recent_projects.clear();
    app.recent_projects_edits.clear();
    app.open_tabs.push(crate::app::AppTab {
        id: "apps".to_string(),
        title: "Apps".to_string(),
        screen: crate::app::Screen::Apps,
        project_path: None,
    });
    let mut meta = recent_meta("/repo/alpha");
    meta.name = "Alpha Project".to_string();
    app.recent_projects_meta = vec![meta];

    let _ = app.open_project("/repo/alpha".to_string());

    assert!(app.open_tabs.iter().all(|tab| tab.id != "apps"));
    assert_eq!(app.project_path.as_deref(), Some("/repo/alpha"));
    assert_eq!(app.screen, crate::app::Screen::Project);
    assert_eq!(app.active_tab_id.as_deref(), Some("project_/repo/alpha"));
    assert!(app.open_tabs.iter().any(|tab| {
        tab.id == "project_/repo/alpha"
            && tab.title == "alpha"
            && tab.screen == crate::app::Screen::Project
            && tab.project_path.as_deref() == Some("/repo/alpha")
    }));
    assert_eq!(app.recent_projects.first().map(String::as_str), Some("/repo/alpha"));
    assert_eq!(app.recent_projects_edits.first().map(String::as_str), Some("Alpha Project"));
    assert!(app.selected_branch.is_none());
    assert!(app.branches.is_empty());
}
