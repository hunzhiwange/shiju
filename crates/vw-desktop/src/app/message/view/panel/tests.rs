// 验证主面板消息向设置、标签、弹层和外部模块分派。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::{ViewMessage, update};
use crate::app::{App, Screen, message::view::MenuType};

fn test_app() -> App {
    App::new().0
}

// 消息与任务边界：确认输入消息产生预期状态或返回任务。
// 场景外状态必须保持不变，避免无关消息触发隐藏副作用。
#[test]
fn settings_message_routes_to_settings_update() {
    let mut app = test_app();
    app.show_about_modal = false;

    let _task = update(&mut app, ViewMessage::ToggleAboutModal);

    assert!(app.show_about_modal);
}

#[test]
fn popover_message_routes_to_popovers_update() {
    let mut app = test_app();
    app.show_mode_popover = false;

    let _task = update(&mut app, ViewMessage::ToggleModePopover);

    assert!(app.show_mode_popover);
}

#[test]
fn tab_message_routes_to_tabs_update() {
    let mut app = test_app();

    let _task = update(&mut app, ViewMessage::TabHovered(Some("usage".to_string())));

    assert_eq!(app.hovered_tab_id.as_deref(), Some("usage"));
}

// 消息与任务边界：确认输入消息产生预期状态或返回任务。
// 场景外状态必须保持不变，避免无关消息触发隐藏副作用。
#[test]
fn app_message_routes_to_apps_update() {
    let mut app = test_app();

    let _task = update(&mut app, ViewMessage::OpenApps);

    assert_eq!(app.screen, Screen::Apps);
    assert_eq!(app.active_tab_id.as_deref(), Some("apps"));
    assert!(app.open_tabs.iter().any(|tab| tab.id == "apps"));
}

#[test]
fn usage_message_routes_to_usage_update() {
    let mut app = test_app();
    app.active_session_id = None;

    let _task = update(&mut app, ViewMessage::UsageStepToggled(7));

    assert!(app.usage_step_expanded.contains(&7));
    assert!(app.usage_step_raw_loading.is_empty());
}

#[test]
fn web_message_routes_to_web_update() {
    let mut app = test_app();

    let _task =
        update(&mut app, ViewMessage::WebBookmarkTitleChanged("Rust reference".to_string()));

    assert_eq!(app.web_bookmark_title_input, "Rust reference");
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn external_message_routes_to_external_update_without_project() {
    let mut app = test_app();
    app.active_menu = Some(MenuType::OpenExternal);
    app.project_path = None;

    let _task = update(&mut app, ViewMessage::OpenProjectInExternalPreferred);

    assert_eq!(app.active_menu, None);
    assert!(app.project_path.is_none());
}

#[test]
fn replay_snapshot_message_is_ignored_by_panel_router() {
    let mut app = test_app();
    app.show_about_modal = true;

    let _task =
        update(&mut app, ViewMessage::ReplaySessionFromSnapshot("snapshot.json".to_string()));

    assert!(app.show_about_modal);
}

#[test]
fn close_request_routes_without_mutating_window_registration() {
    let mut app = test_app();
    let main_window_id = iced::window::Id::unique();
    let task_pet_window_id = iced::window::Id::unique();
    app.main_window_id = Some(main_window_id);
    app.task_pet_window_id = Some(task_pet_window_id);

    let task = update(&mut app, ViewMessage::CloseRequested(main_window_id));
    let _ = std::hint::black_box(task);

    assert_eq!(app.main_window_id, Some(main_window_id));
    assert_eq!(app.task_pet_window_id, Some(task_pet_window_id));
    assert!(app.shutdown_requested);
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn unrelated_message_is_ignored_by_panel_router() {
    let mut app = test_app();
    app.show_about_modal = true;

    let _task = update(&mut app, ViewMessage::ActivityAnimationTick);

    assert!(app.show_about_modal);
}
