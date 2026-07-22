use super::external::{
    ExternalAppLogo, OPEN_EXTERNAL_TARGETS, external_app_label, external_app_logo,
    file_manager_label, open_external_module, supported_external_apps,
};
use crate::app::assets::Icon;
use crate::app::message::view::MenuType;
use crate::app::state::{ExternalOpenApp, RuntimePlatform};
use crate::app::{App, Screen};

fn test_app() -> App {
    App::new().0
}

#[test]
fn file_manager_label_falls_back_and_uses_platform_label() {
    assert_eq!(file_manager_label(None), "File Manager");
    assert_eq!(file_manager_label(Some(RuntimePlatform::MacOs)), "Finder");
    assert_eq!(file_manager_label(Some(RuntimePlatform::Windows)), "File Explorer");
    assert_eq!(file_manager_label(Some(RuntimePlatform::Linux)), "File Manager");
}

#[test]
fn external_app_label_uses_platform_only_for_finder() {
    assert_eq!(
        external_app_label(ExternalOpenApp::Finder, Some(RuntimePlatform::Windows)),
        "File Explorer"
    );
    assert_eq!(
        external_app_label(ExternalOpenApp::VSCode, Some(RuntimePlatform::Windows)),
        "VS Code"
    );
    assert_eq!(external_app_label(ExternalOpenApp::Cursor, None), "Cursor");
}

#[test]
fn external_app_logo_covers_every_supported_target() {
    let expected = [
        (ExternalOpenApp::Finder, ExternalAppLogo::Svg(Icon::AppFileExplorer)),
        (ExternalOpenApp::Trae, ExternalAppLogo::Svg(Icon::AppTrae)),
        (ExternalOpenApp::Windsurf, ExternalAppLogo::Image(Icon::AppWindsurf)),
        (ExternalOpenApp::Kiro, ExternalAppLogo::Svg(Icon::AppKiro)),
        (ExternalOpenApp::Cursor, ExternalAppLogo::Svg(Icon::AppCursor)),
        (ExternalOpenApp::VSCode, ExternalAppLogo::Svg(Icon::AppVSCode)),
        (ExternalOpenApp::Zed, ExternalAppLogo::Svg(Icon::AppZed)),
        (ExternalOpenApp::TextMate, ExternalAppLogo::Image(Icon::AppTextMate)),
        (ExternalOpenApp::Antigravity, ExternalAppLogo::Svg(Icon::AppAntigravity)),
        (ExternalOpenApp::Terminal, ExternalAppLogo::Image(Icon::AppTerminal)),
        (ExternalOpenApp::ITerm2, ExternalAppLogo::Svg(Icon::AppITerm2)),
        (ExternalOpenApp::Ghostty, ExternalAppLogo::Svg(Icon::AppGhostty)),
        (ExternalOpenApp::Xcode, ExternalAppLogo::Image(Icon::AppXcode)),
        (ExternalOpenApp::AndroidStudio, ExternalAppLogo::Svg(Icon::AppAndroidStudio)),
        (ExternalOpenApp::PowerShell, ExternalAppLogo::Svg(Icon::AppPowerShell)),
        (ExternalOpenApp::SublimeText, ExternalAppLogo::Svg(Icon::AppSublimeText)),
    ];

    assert_eq!(OPEN_EXTERNAL_TARGETS.len(), expected.len());
    for (target, logo) in expected {
        assert_eq!(external_app_logo(target, None), logo);
    }
    assert_eq!(
        external_app_logo(ExternalOpenApp::Finder, Some(RuntimePlatform::MacOs)),
        ExternalAppLogo::Image(Icon::AppFinder)
    );
}

#[test]
fn finder_logo_uses_file_explorer_svg_on_non_macos_platforms() {
    for platform in [RuntimePlatform::Windows, RuntimePlatform::Linux] {
        assert_eq!(
            external_app_logo(ExternalOpenApp::Finder, Some(platform)),
            ExternalAppLogo::Svg(Icon::AppFileExplorer)
        );
    }
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
