use super::{FileTreeAction, LoadedProjectInfo, ProjectMessage, update};
use crate::app::App;

#[test]
fn test_module_is_wired() {
    let module = module_path!();

    assert!(module.ends_with("mod_tests"));
}

#[test]
fn update_returns_no_task_for_unhandled_start_deferred_tasks_message() {
    let (mut app, _task) = App::new();
    let before_project_path = app.project_path.clone();

    let _task = update(
        &mut app,
        ProjectMessage::StartDeferredTasks { project_path: "/tmp/project".to_string() },
    );

    assert_eq!(app.project_path, before_project_path);
}

#[test]
fn loaded_project_info_keeps_project_path_info_and_branch() {
    let info = vw_shared::project::Info {
        id: "project-id".to_string(),
        worktree: "/tmp/project".to_string(),
        vcs: Some(vw_shared::project::Vcs::Git),
        name: Some("Project".to_string()),
        icon: None,
        commands: None,
        time: vw_shared::project::TimeInfo { created: 1, updated: 2, initialized: None },
        sandboxes: vec!["/tmp/project".to_string()],
    };

    let loaded = LoadedProjectInfo {
        project_path: "/tmp/project".to_string(),
        info,
        current_branch: Some("main".to_string()),
    };
    let cloned = loaded.clone();

    assert_eq!(cloned.project_path, "/tmp/project");
    assert_eq!(cloned.info.name.as_deref(), Some("Project"));
    assert_eq!(cloned.current_branch.as_deref(), Some("main"));
    assert!(format!("{loaded:?}").contains("LoadedProjectInfo"));
}

#[test]
fn project_message_variants_are_cloneable_and_debuggable() {
    let message = ProjectMessage::ProjectLoadMoreSessions("/tmp/project".to_string());
    let cloned = message.clone();

    assert!(format!("{cloned:?}").contains("ProjectLoadMoreSessions"));
}

#[test]
fn file_tree_action_variants_are_cloneable_and_debuggable() {
    let actions = [
        FileTreeAction::Open,
        FileTreeAction::RevealInFinder,
        FileTreeAction::OpenInTerminal,
        FileTreeAction::AddToChat,
        FileTreeAction::FindInFolder,
        FileTreeAction::Cut,
        FileTreeAction::Copy,
        FileTreeAction::Paste,
        FileTreeAction::CopyPath,
        FileTreeAction::CopyRelativePath,
        FileTreeAction::Rename,
        FileTreeAction::Delete,
    ];

    let cloned_names = actions.map(|action| format!("{:?}", action.clone()));

    assert_eq!(
        cloned_names,
        [
            "Open",
            "RevealInFinder",
            "OpenInTerminal",
            "AddToChat",
            "FindInFolder",
            "Cut",
            "Copy",
            "Paste",
            "CopyPath",
            "CopyRelativePath",
            "Rename",
            "Delete",
        ]
    );
}
