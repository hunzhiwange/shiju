// Tests migrated from desktop host state; production types retain their original owners.
mod behavior_0 {
use crate::project_message::edit_meta::*;
#[test]
fn recent_project_defaults_preserve_legacy_metadata_behavior() {
    let meta: RecentProjectMeta = serde_json::from_value(serde_json::json!({
        "path": "/workspace/demo",
        "name": "Demo"
    }))
    .expect("legacy recent project metadata should deserialize");

    assert_eq!(meta.path, "/workspace/demo");
    assert_eq!(meta.name, "Demo");
    assert!(meta.task_board_settings.is_none());
    assert_eq!(meta.session_auto_refresh, default_recent_project_session_auto_refresh());
    assert_eq!(
        meta.session_refresh_interval_seconds,
        default_recent_project_session_refresh_interval_seconds()
    );
    assert!(meta.icon.is_none());
    assert!(meta.icon_color.is_none());
    assert!(meta.worktree_start_command.is_none());
}
}

mod behavior_1 {
use crate::project_message::edit_meta::*;
#[test]
fn recent_project_explicit_session_and_visual_metadata_round_trips() {
    let value = serde_json::json!({
        "path": "/workspace/explicit",
        "name": "Explicit",
        "task_board_settings": null,
        "session_auto_refresh": false,
        "session_refresh_interval_seconds": 0,
        "icon": "folder",
        "icon_color": "#123456",
        "worktree_start_command": "just dev"
    });

    let meta: RecentProjectMeta =
        serde_json::from_value(value.clone()).expect("explicit metadata should deserialize");
    assert!(!meta.session_auto_refresh);
    assert_eq!(meta.session_refresh_interval_seconds, 0);
    assert_eq!(meta.icon.as_deref(), Some("folder"));
    assert_eq!(meta.icon_color.as_deref(), Some("#123456"));
    assert_eq!(meta.worktree_start_command.as_deref(), Some("just dev"));
    assert_eq!(serde_json::to_value(meta).expect("explicit metadata should serialize"), value);
}
}

mod behavior_2 {
use crate::interaction::{FileTreeClipboard, FileTreeClipboardMode};
#[test]
fn workspace_structs_keep_selected_git_and_find_state_1() {
    let clipboard = FileTreeClipboard {
        mode: FileTreeClipboardMode::Copy,
        src_paths: vec!["/tmp/a.txt".to_string()],
    };
    assert!(matches!(clipboard.mode, FileTreeClipboardMode::Copy));
    assert_eq!(clipboard.src_paths, ["/tmp/a.txt"]);


}
}
