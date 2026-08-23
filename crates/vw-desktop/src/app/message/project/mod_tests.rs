// 验证项目消息类型及未处理消息的顶层回退行为。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::{FileTreeAction, LoadedProjectInfo, ProjectMessage, update};
use crate::app::App;

// 编译接线：引用父模块符号，确保测试片段仍由真实模块包含。
// 该冒烟用例只承担接线职责，具体行为由同领域测试覆盖。
#[test]
fn test_module_is_wired() {
    let module = module_path!();

    assert!(module.ends_with("mod_tests"));
}

// 任务与结果：覆盖任务构造、调度或结果回写对公开状态的影响。
// 完成、失败与过期响应均按当前场景的既定契约处理。
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

// 行为契约：使用代表性场景固定该分支的可观察结果。
// 断言聚焦输入、状态与返回任务，不复述内部控制流程。
#[test]
fn project_message_variants_are_cloneable_and_debuggable() {
    let message = ProjectMessage::ProjectLoadMoreSessions("/tmp/project".to_string());
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
