use super::{FileTreeAction, LoadedProjectInfo, ProjectMessage};

#[test]
fn loaded_project_info_keeps_project_path_info_and_branch() {
    let info = project_types::Info {
        id: "project-id".to_string(),
        worktree: "/tmp/project".to_string(),
        vcs: Some(project_types::Vcs::Git),
        name: Some("Project".to_string()),
        icon: None,
        commands: None,
        time: project_types::TimeInfo { created: 1, updated: 2, initialized: None },
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

// 行为契约：使用代表性场景固定该分支的可观察结果。
// 断言聚焦输入、状态与返回任务，不复述内部控制流程。
#[test]
fn project_message_variants_are_cloneable_and_debuggable() {
    let message = ProjectMessage::<(), (), ()>::ProjectLoadMoreSessions("/tmp/project".to_string());
    let cloned = message.clone();

    assert!(format!("{cloned:?}").contains("ProjectLoadMoreSessions"));
}

// 行为契约：使用代表性场景固定该分支的可观察结果。
// 断言聚焦输入、状态与返回任务，不复述内部控制流程。
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
        FileTreeAction::Duplicate,
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
            "Duplicate",
            "Paste",
            "CopyPath",
            "CopyRelativePath",
            "Rename",
            "Delete",
        ]
    );
}
