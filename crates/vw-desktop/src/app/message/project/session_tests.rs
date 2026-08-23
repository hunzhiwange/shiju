// 验证项目会话消息向打开、编辑和工作树模块分派。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::handle;
use crate::app::App;
use crate::app::message::project::ProjectMessage;

// 编译接线：引用父模块符号，确保测试片段仍由真实模块包含。
// 该冒烟用例只承担接线职责，具体行为由同领域测试覆盖。
#[test]
fn test_module_is_wired() {
    let module = module_path!();

    assert!(module.ends_with("session_tests"));
}

#[test]
fn handle_returns_none_for_non_session_message() {
    let (mut app, _task) = App::new();

    let task = handle(&mut app, ProjectMessage::ProjectPathChanged("/tmp/project".to_string()));

    assert!(task.is_none());
}

// 消息与任务边界：确认输入消息产生预期状态或返回任务。
// 场景外状态必须保持不变，避免无关消息触发隐藏副作用。
#[test]
fn handle_dispatches_recent_overlay_close_to_open_module() {
    let (mut app, _task) = App::new();
    app.hovered_recent_project = Some("/tmp/project".to_string());
    app.session_menu_id = Some("session-1".to_string());
    app.project_tools_menu_path = Some("/tmp/project".to_string());

    let task = handle(&mut app, ProjectMessage::RecentOverlayClosed);

    assert!(task.is_some());
    assert!(app.hovered_recent_project.is_none());
    assert!(app.session_menu_id.is_none());
    assert!(app.project_tools_menu_path.is_none());
}

#[test]
fn handle_dispatches_project_tools_toggle_to_edit_module() {
    let (mut app, _task) = App::new();
    let path = "/tmp/project".to_string();

    let task = handle(&mut app, ProjectMessage::ProjectToolsMenuToggled(path.clone()));

    assert!(task.is_some());
    assert_eq!(app.project_tools_menu_path, Some(path));
}

// 消息与任务边界：确认输入消息产生预期状态或返回任务。
// 场景外状态必须保持不变，避免无关消息触发隐藏副作用。
#[test]
fn handle_dispatches_project_create_session_picker_close_to_worktree_module() {
    let (mut app, _task) = App::new();
    app.new_session_picker_project = Some("/tmp/project".to_string());

    let _task = handle(&mut app, ProjectMessage::ProjectCreateSessionPickerClose)
        .expect("close picker should be handled");

    assert!(app.new_session_picker_project.is_none());
}

// 消息与任务边界：确认输入消息产生预期状态或返回任务。
// 场景外状态必须保持不变，避免无关消息触发隐藏副作用。
#[test]
fn handle_dispatches_session_right_click_to_lifecycle_module() {
    let (mut app, _task) = App::new();

    let task =
        handle(&mut app, ProjectMessage::SessionRightClicked("session-1".to_string(), 12.5, 24.0));

    assert!(task.is_some());
    assert_eq!(app.session_menu_id.as_deref(), Some("session-1"));
    assert_eq!(app.session_menu_anchor, Some(iced::Point::new(12.5, 24.0)));
}
