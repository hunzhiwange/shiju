use super::*;
use crate::app::App;

fn app() -> App {
    App::new().0
}

fn recent_project_meta(path: &str, name: &str) -> RecentProjectMeta {
    RecentProjectMeta {
        path: path.to_string(),
        name: name.to_string(),
        task_board_settings: None,
        session_auto_refresh: false,
        session_refresh_interval_seconds: 120,
        icon: Some("folder".to_string()),
        icon_color: Some("blue".to_string()),
        worktree_start_command: Some("prepare".to_string()),
    }
}

#[test]
fn rename_changed_updates_existing_edit_and_rejects_out_of_bounds_index() {
    let mut app = app();
    app.recent_projects_edits = vec!["Original".to_string()];

    let _task =
        update(&mut app, SettingsMessage::RecentProjectRenameChanged(0, "Renamed".to_string()));
    let _task =
        update(&mut app, SettingsMessage::RecentProjectRenameChanged(1, "Ignored".to_string()));

    assert_eq!(app.recent_projects_edits, vec!["Renamed"]);
}

#[test]
fn rename_save_updates_matching_metadata_without_replacing_other_fields() {
    let mut app = app();
    app.recent_projects = vec!["/workspace/project-a".to_string()];
    app.recent_projects_edits = vec!["Project A".to_string()];
    app.recent_projects_meta = vec![recent_project_meta("/workspace/project-a", "Old name")];

    let _task = update(&mut app, SettingsMessage::RecentProjectRenameSave(0));

    assert_eq!(app.recent_projects_meta.len(), 1);
    let meta = &app.recent_projects_meta[0];
    assert_eq!(meta.name, "Project A");
    assert!(!meta.session_auto_refresh);
    assert_eq!(meta.session_refresh_interval_seconds, 120);
    assert_eq!(meta.icon.as_deref(), Some("folder"));
    assert_eq!(meta.icon_color.as_deref(), Some("blue"));
    assert_eq!(meta.worktree_start_command.as_deref(), Some("prepare"));
}

#[test]
fn rename_save_creates_metadata_with_session_defaults_for_unknown_project() {
    let mut app = app();
    app.recent_projects = vec!["/workspace/project-a".to_string()];
    app.recent_projects_edits = vec!["Project A".to_string()];
    app.recent_projects_meta.clear();

    let _task = update(&mut app, SettingsMessage::RecentProjectRenameSave(0));

    assert_eq!(app.recent_projects_meta.len(), 1);
    let meta = &app.recent_projects_meta[0];
    assert_eq!(meta.path, "/workspace/project-a");
    assert_eq!(meta.name, "Project A");
    assert_eq!(meta.session_auto_refresh, default_recent_project_session_auto_refresh());
    assert_eq!(
        meta.session_refresh_interval_seconds,
        default_recent_project_session_refresh_interval_seconds()
    );
    assert!(meta.task_board_settings.is_none());
    assert!(meta.icon.is_none());
    assert!(meta.icon_color.is_none());
    assert!(meta.worktree_start_command.is_none());
}

#[test]
fn rename_save_rejects_indices_missing_from_either_project_collection() {
    let mut missing_edit = app();
    missing_edit.recent_projects = vec!["/workspace/project-a".to_string()];
    missing_edit.recent_projects_edits.clear();
    missing_edit.recent_projects_meta.clear();

    let _task = update(&mut missing_edit, SettingsMessage::RecentProjectRenameSave(0));

    assert!(missing_edit.recent_projects_meta.is_empty());

    let mut missing_project = app();
    missing_project.recent_projects.clear();
    missing_project.recent_projects_edits = vec!["Project A".to_string()];
    missing_project.recent_projects_meta.clear();

    let _task = update(&mut missing_project, SettingsMessage::RecentProjectRenameSave(0));

    assert!(missing_project.recent_projects_meta.is_empty());
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
fn delete_request_allows_current_project() {
    let mut app = app();
    app.recent_projects = vec!["/workspace/project-a".to_string()];
    app.project_path = Some("/workspace/project-a".to_string());

    let _task = update(&mut app, SettingsMessage::RecentProjectDeleteRequested(0));

    assert_eq!(app.recent_project_delete_confirm_idx, Some(0));
    assert_eq!(app.recent_projects, vec!["/workspace/project-a"]);
    assert!(app.active_toast.is_none());
}

#[test]
fn delete_confirmed_current_project_selects_first_remaining_project() {
    let mut app = app();
    app.recent_projects = vec![
        "/workspace/project-a".to_string(),
        "/workspace/project-b".to_string(),
    ];
    app.recent_projects_edits = vec!["Project A".to_string(), "Project B".to_string()];
    app.project_path = Some("/workspace/project-a".to_string());
    app.recent_project_delete_confirm_idx = Some(0);

    let _task = update(&mut app, SettingsMessage::RecentProjectDeleteConfirmed(0));

    assert_eq!(app.recent_projects, vec!["/workspace/project-b"]);
    assert_eq!(app.recent_projects_edits, vec!["Project B"]);
    assert_eq!(app.project_path.as_deref(), Some("/workspace/project-b"));
    assert_eq!(app.recent_project_delete_confirm_idx, None);
    assert!(app.active_toast.is_none());
}

#[test]
fn delete_confirmed_removes_associated_state_and_rebuilds_display_names() {
    let mut app = app();
    app.recent_projects = vec![
        "/workspace/deleted".to_string(),
        "/workspace/from-meta".to_string(),
        "/workspace/from-path".to_string(),
        "/".to_string(),
    ];
    app.recent_projects_edits = vec![
        "Deleted".to_string(),
        "Stale meta".to_string(),
        "Stale path".to_string(),
        "Stale root".to_string(),
    ];
    app.recent_projects_meta = vec![
        recent_project_meta("/workspace/deleted", "Deleted"),
        recent_project_meta("/workspace/from-meta", "Metadata name"),
    ];
    app.project_worktree_enabled.insert("/workspace/deleted".to_string(), true);
    app.project_worktree_enabled.insert("/workspace/from-meta".to_string(), true);
    app.recent_project_delete_confirm_idx = Some(0);

    let _task = update(&mut app, SettingsMessage::RecentProjectDeleteConfirmed(0));

    assert_eq!(app.recent_projects, vec!["/workspace/from-meta", "/workspace/from-path", "/"]);
    assert_eq!(app.recent_projects_edits, vec!["Metadata name", "from-path", "/"]);
    assert_eq!(app.recent_projects_meta.len(), 1);
    assert_eq!(app.recent_projects_meta[0].path, "/workspace/from-meta");
    assert!(!app.project_worktree_enabled.contains_key("/workspace/deleted"));
    assert_eq!(app.project_worktree_enabled.get("/workspace/from-meta"), Some(&true));
    assert_eq!(app.recent_project_delete_confirm_idx, None);
}

#[test]
fn delete_confirmed_handles_missing_edit_for_existing_project() {
    let mut app = app();
    app.recent_projects = vec!["/workspace/deleted".to_string()];
    app.recent_projects_edits.clear();
    app.recent_projects_meta = vec![recent_project_meta("/workspace/deleted", "Deleted")];

    let _task = update(&mut app, SettingsMessage::RecentProjectDeleteConfirmed(0));

    assert!(app.recent_projects.is_empty());
    assert!(app.recent_projects_edits.is_empty());
    assert!(app.recent_projects_meta.is_empty());
    assert_eq!(app.recent_project_delete_confirm_idx, None);
}

#[test]
fn delete_confirmed_rejects_out_of_bounds_index_but_closes_confirmation() {
    let mut app = app();
    app.recent_projects = vec!["/workspace/project-a".to_string()];
    app.recent_projects_edits = vec!["Project A".to_string()];
    app.recent_projects_meta = vec![recent_project_meta("/workspace/project-a", "Project A")];
    app.project_worktree_enabled.insert("/workspace/project-a".to_string(), true);
    app.recent_project_delete_confirm_idx = Some(4);

    let _task = update(&mut app, SettingsMessage::RecentProjectDeleteConfirmed(4));

    assert_eq!(app.recent_projects, vec!["/workspace/project-a"]);
    assert_eq!(app.recent_projects_edits, vec!["Project A"]);
    assert_eq!(app.recent_projects_meta.len(), 1);
    assert_eq!(app.recent_projects_meta[0].path, "/workspace/project-a");
    assert_eq!(app.project_worktree_enabled.get("/workspace/project-a"), Some(&true));
    assert_eq!(app.recent_project_delete_confirm_idx, None);
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

#[test]
fn unrelated_settings_message_leaves_project_state_unchanged() {
    let mut app = app();
    app.recent_projects = vec!["/workspace/project-a".to_string()];
    app.recent_projects_edits = vec!["Project A".to_string()];
    app.recent_projects_meta = vec![recent_project_meta("/workspace/project-a", "Project A")];
    app.project_worktree_enabled.insert("/workspace/project-a".to_string(), true);
    app.recent_project_delete_confirm_idx = Some(0);

    let _task = update(&mut app, SettingsMessage::ToggleSettingsSidebar);

    assert_eq!(app.recent_projects, vec!["/workspace/project-a"]);
    assert_eq!(app.recent_projects_edits, vec!["Project A"]);
    assert_eq!(app.recent_projects_meta.len(), 1);
    assert_eq!(app.recent_projects_meta[0].path, "/workspace/project-a");
    assert_eq!(app.project_worktree_enabled.get("/workspace/project-a"), Some(&true));
    assert_eq!(app.recent_project_delete_confirm_idx, Some(0));
}
