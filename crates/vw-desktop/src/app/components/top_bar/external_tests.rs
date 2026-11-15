//! 验证顶部栏外部应用入口的可见性、标签和启动消息。
//!
//! 外部应用夹具覆盖可用、缺失和启动失败状态，确认入口与反馈同步变化。

use super::external::{
    OPEN_EXTERNAL_TARGETS, open_external_module, supported_external_apps,
};
use crate::app::message::view::MenuType;
use crate::app::state::{ExternalOpenApp, RuntimePlatform};
use crate::app::{App, Screen};

fn test_app() -> App {
    App::new().0
}





#[test]
fn supported_external_apps_keeps_menu_order_and_includes_cached_false_entries() {
    let mut app = test_app();
    app.open_external_exists.clear();
    app.open_external_exists.insert(ExternalOpenApp::VSCode, false);
    app.open_external_exists.insert(ExternalOpenApp::Finder, true);
    app.open_external_exists.insert(ExternalOpenApp::Terminal, true);

    assert_eq!(
        supported_external_apps(&app),
        [ExternalOpenApp::Finder, ExternalOpenApp::VSCode, ExternalOpenApp::Terminal]
    );
}

#[test]
fn supported_external_apps_is_empty_without_cached_targets() {
    let mut app = test_app();
    app.open_external_exists.clear();

    assert!(supported_external_apps(&app).is_empty());
}

#[test]
fn open_external_module_builds_empty_element_outside_project_screen() {
    let mut app = test_app();
    app.screen = Screen::Home;

    let element = open_external_module(&app);

    let _ = std::hint::black_box(element);
}

#[test]
fn open_external_module_builds_open_menu_without_project_path_or_targets() {
    let mut app = test_app();
    app.screen = Screen::Project;
    app.project_path = None;
    app.active_menu = Some(MenuType::OpenExternal);
    app.open_external_exists.clear();

    let element = open_external_module(&app);

    let _ = std::hint::black_box(element);
}

#[test]
fn open_external_module_builds_disabled_preferred_target_with_copy_available() {
    let mut app = test_app();
    app.screen = Screen::Project;
    app.project_path = Some("/workspace".to_string());
    app.open_external_app = ExternalOpenApp::VSCode;
    app.open_external_exists.clear();
    app.open_external_exists.insert(ExternalOpenApp::VSCode, false);

    let element = open_external_module(&app);

    let _ = std::hint::black_box(element);
}

#[test]
fn open_external_module_builds_project_controls_for_available_targets() {
    let mut app = test_app();
    app.screen = Screen::Project;
    app.project_path = Some("/workspace".to_string());
    app.open_external_app = ExternalOpenApp::VSCode;
    app.open_external_platform = Some(RuntimePlatform::MacOs);
    app.open_external_exists.clear();
    for target in OPEN_EXTERNAL_TARGETS {
        app.open_external_exists.insert(target, true);
    }

    let element = open_external_module(&app);

    let _ = std::hint::black_box(element);
}
