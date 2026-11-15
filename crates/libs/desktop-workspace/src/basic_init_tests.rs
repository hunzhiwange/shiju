use super::*;
use crate::project_message::edit_meta::RecentProjectMeta;
fn recent_project_meta(path: &str, name: &str) -> RecentProjectMeta {
    RecentProjectMeta {
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
fn display_name_for_path_uses_recent_project_meta_name() {
    let meta = vec![recent_project_meta("/workspace/app", "App Workspace")];

    assert_eq!(display_name_for_path(&meta, "/workspace/app"), "App Workspace");
}

#[test]
fn display_name_for_path_uses_first_matching_meta_name() {
    let meta = vec![
        recent_project_meta("/workspace/app", "First Name"),
        recent_project_meta("/workspace/app", "Second Name"),
    ];

    assert_eq!(display_name_for_path(&meta, "/workspace/app"), "First Name");
}

#[test]
fn display_name_for_path_falls_back_to_last_path_segment() {
    let meta = vec![recent_project_meta("/workspace/other", "Other")];

    assert_eq!(display_name_for_path(&meta, "/workspace/app"), "app");
}

#[test]
fn display_name_for_path_returns_original_path_when_segment_is_unavailable() {
    let meta = Vec::new();

    assert_eq!(display_name_for_path(&meta, "/"), "/");
}

#[test]
fn display_name_for_path_handles_relative_and_non_utf8_like_fallbacks() {
    let meta = Vec::new();

    assert_eq!(display_name_for_path(&meta, "workspace/app"), "app");
    assert_eq!(display_name_for_path(&meta, ""), "");
}
