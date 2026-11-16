use crate::projects::{self, Event, State};
#[derive(Default)]
struct Projects {
    recent_project_query: String,
    project_tools_menu_path: Option<String>,
    recent_project_delete_confirm_idx: Option<usize>,
    recent_projects: Vec<String>,
    recent_projects_edits: Vec<String>,
    project_worktree_enabled: std::collections::HashMap<String, bool>,
}
fn app() -> Projects {
    Projects::default()
}
enum SettingsMessage {
    RecentProjectQueryChanged(String),
    RecentProjectDeleteRequested(usize),
    RecentProjectDeleteCanceled,
    ProjectEnableWorktreeToggled(String, bool),
}
fn update(app: &mut Projects, message: SettingsMessage) {
    let event = match message {
        SettingsMessage::RecentProjectQueryChanged(value) => Event::QueryChanged(value),
        SettingsMessage::RecentProjectDeleteRequested(index) => Event::DeleteRequested(index),
        SettingsMessage::RecentProjectDeleteCanceled => Event::DeleteCanceled,
        SettingsMessage::ProjectEnableWorktreeToggled(path, enabled) => {
            projects::set_worktree_enabled(&mut app.project_worktree_enabled, &path, enabled);
            return;
        }
    };
    projects::apply(
        State {
            query: &mut app.recent_project_query,
            tools_menu_path: &mut app.project_tools_menu_path,
            delete_confirm_idx: &mut app.recent_project_delete_confirm_idx,
            paths: &mut app.recent_projects,
            edits: &mut app.recent_projects_edits,
        },
        event,
        None,
    );
}
#[test]
fn search_updates_query_and_dismisses_previous_row_actions() {
    let mut app = app();
    app.project_tools_menu_path = Some("/workspace/project-a".into());
    app.recent_project_delete_confirm_idx = Some(0);

    let _task = update(&mut app, SettingsMessage::RecentProjectQueryChanged("project-b".into()));

    assert_eq!(app.recent_project_query, "project-b");
    assert!(app.project_tools_menu_path.is_none());
    assert!(app.recent_project_delete_confirm_idx.is_none());
}
#[test]
fn delete_request_and_cancel_only_change_confirmation_state() {
    let mut app = app();
    app.recent_projects = vec!["/workspace/project-a".to_string()];

    let _task = update(&mut app, SettingsMessage::RecentProjectDeleteRequested(7));

    assert_eq!(app.recent_project_delete_confirm_idx, Some(7));
    assert_eq!(app.recent_projects, vec!["/workspace/project-a"]);

    let _task = update(&mut app, SettingsMessage::RecentProjectDeleteCanceled);

    assert_eq!(app.recent_project_delete_confirm_idx, None);
    assert_eq!(app.recent_projects, vec!["/workspace/project-a"]);
}
#[test]
fn worktree_toggle_inserts_enabled_entry_and_removes_disabled_entry() {
    let mut app = app();
    let path = "/workspace/project-a";

    let _task =
        update(&mut app, SettingsMessage::ProjectEnableWorktreeToggled(path.to_string(), true));

    assert_eq!(app.project_worktree_enabled.get(path), Some(&true));

    let _task =
        update(&mut app, SettingsMessage::ProjectEnableWorktreeToggled(path.to_string(), false));

    assert!(!app.project_worktree_enabled.contains_key(path));
}
