// 验证外部打开目标的可用性约束、偏好与任务调度。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use crate::app::message::view::MenuType;
use crate::app::state::ExternalOpenApp;

fn new_app() -> crate::app::App {
    crate::app::App::new().0
}

fn has_scheduled_action(task: iced::Task<crate::app::Message>) -> bool {
    task.units() > 0
}

// 编译接线：引用父模块符号，确保测试片段仍由真实模块包含。
// 该冒烟用例只承担接线职责，具体行为由同领域测试覆盖。
#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("external_tests"));
}

#[test]
fn can_open_external_requires_cached_true_value() {
    let mut app = new_app();
    app.open_external_exists.clear();

    assert!(!super::can_open_external(&app, ExternalOpenApp::VSCode));

    app.open_external_exists.insert(ExternalOpenApp::VSCode, false);
    assert!(!super::can_open_external(&app, ExternalOpenApp::VSCode));

    app.open_external_exists.insert(ExternalOpenApp::VSCode, true);
    assert!(super::can_open_external(&app, ExternalOpenApp::VSCode));
}

#[test]
fn preferred_open_without_project_closes_menu_without_scheduling_action() {
    let mut app = new_app();
    app.active_menu = Some(MenuType::OpenExternal);
    app.project_path = None;
    app.open_external_exists.insert(app.open_external_app, true);

    let task = super::update(&mut app, super::ViewMessage::OpenProjectInExternalPreferred);

    assert_eq!(app.active_menu, None);
    assert!(!has_scheduled_action(task));
}

#[test]
fn preferred_open_rejects_unavailable_target() {
    let mut app = new_app();
    app.active_menu = Some(MenuType::OpenExternal);
    app.project_path = Some("/workspace/project".to_string());
    app.open_external_exists.insert(app.open_external_app, false);

    let task = super::update(&mut app, super::ViewMessage::OpenProjectInExternalPreferred);

    assert_eq!(app.active_menu, None);
    assert!(!has_scheduled_action(task));
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn preferred_open_schedules_available_target_without_polling_gateway() {
    let mut app = new_app();
    app.active_menu = Some(MenuType::OpenExternal);
    app.project_path = Some("/workspace/project".to_string());
    app.open_external_app = ExternalOpenApp::Cursor;
    app.open_external_exists.insert(ExternalOpenApp::Cursor, true);

    let task = super::update(&mut app, super::ViewMessage::OpenProjectInExternalPreferred);

    assert_eq!(app.active_menu, None);
    assert!(has_scheduled_action(task));
}

#[test]
fn explicit_open_updates_preference_and_closes_menu_without_project() {
    let mut app = new_app();
    app.active_menu = Some(MenuType::OpenExternal);
    app.project_path = None;
    app.open_external_app = ExternalOpenApp::VSCode;

    let task = super::update(
        &mut app,
        super::ViewMessage::OpenProjectInExternalWith(ExternalOpenApp::Cursor),
    );

    assert_eq!(app.active_menu, None);
    assert_eq!(app.open_external_app, ExternalOpenApp::Cursor);
    assert!(!has_scheduled_action(task));
}

#[test]
fn explicit_open_rejects_unavailable_target_after_saving_preference() {
    let mut app = new_app();
    app.active_menu = Some(MenuType::OpenExternal);
    app.project_path = Some("/workspace/project".to_string());
    app.open_external_exists.insert(ExternalOpenApp::Cursor, false);

    let task = super::update(
        &mut app,
        super::ViewMessage::OpenProjectInExternalWith(ExternalOpenApp::Cursor),
    );

    assert_eq!(app.active_menu, None);
    assert_eq!(app.open_external_app, ExternalOpenApp::Cursor);
    assert!(!has_scheduled_action(task));
}

#[test]
fn explicit_open_schedules_available_target() {
    let mut app = new_app();
    app.active_menu = Some(MenuType::OpenExternal);
    app.project_path = Some("/workspace/project".to_string());
    app.open_external_exists.insert(ExternalOpenApp::VSCode, true);

    let task = super::update(
        &mut app,
        super::ViewMessage::OpenProjectInExternalWith(ExternalOpenApp::VSCode),
    );

    assert_eq!(app.active_menu, None);
    assert_eq!(app.open_external_app, ExternalOpenApp::VSCode);
    assert!(has_scheduled_action(task));
}

#[test]
fn finder_open_schedules_reveal_without_polling_gateway() {
    let mut app = new_app();

    let task = super::update(
        &mut app,
        super::ViewMessage::OpenPathInFinder("/workspace/project".to_string()),
    );

    assert!(has_scheduled_action(task));
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn finder_open_preserves_current_menu_while_scheduling_reveal() {
    let mut app = new_app();
    app.active_menu = Some(MenuType::Help);

    let task = super::update(
        &mut app,
        super::ViewMessage::OpenPathInFinder("/workspace/other".to_string()),
    );

    assert_eq!(app.active_menu, Some(MenuType::Help));
    assert!(has_scheduled_action(task));
}

#[test]
fn copy_without_project_closes_menu_without_scheduling_action() {
    let mut app = new_app();
    app.active_menu = Some(MenuType::OpenExternal);
    app.project_path = None;

    let task = super::update(&mut app, super::ViewMessage::CopyProjectPath);

    assert_eq!(app.active_menu, None);
    assert!(!has_scheduled_action(task));
}

#[test]
fn copy_existing_project_schedules_clipboard_action_without_polling_clipboard() {
    let mut app = new_app();
    app.project_path = Some(
        std::env::current_dir()
            .expect("current directory should exist")
            .to_string_lossy()
            .into_owned(),
    );

    let task = super::update(&mut app, super::ViewMessage::CopyProjectPath);

    assert!(has_scheduled_action(task));
}

#[test]
fn copy_missing_project_path_falls_back_and_schedules_clipboard_action() {
    let mut app = new_app();
    app.project_path = Some("/path/that/does/not/exist/vibewindow".to_string());

    let task = super::update(&mut app, super::ViewMessage::CopyProjectPath);

    assert!(has_scheduled_action(task));
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn unrelated_message_is_rejected_without_mutating_menu() {
    let mut app = new_app();
    app.active_menu = Some(MenuType::Help);

    let task = super::update(&mut app, super::ViewMessage::ToggleUsagePopover);

    assert_eq!(app.active_menu, Some(MenuType::Help));
    assert!(!has_scheduled_action(task));
}
