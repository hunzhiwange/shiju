//! 原生菜单动作测试，覆盖稳定动作标识、菜单快照和窗口标签路由。
//! 重点确认项目与非项目屏幕只暴露合法动作，动态标签标识可以无损往返。

use crate::components::top_bar::menus::MenuEntry;
use crate::components::top_bar::menus::Message;
use crate::components::top_bar::types::WorkspaceMode;
use crate::native_menu::{
    NativeMenuSnapshot, WindowTabEntry, help_section, menu_sections, resolve_native_menu_action,
};
use crate::ui_types::Screen;
use desktop_platform::menu_actions::NativeMenuAction;

fn test_app(screen: Screen) -> NativeMenuSnapshot {
    NativeMenuSnapshot { is_project: screen == Screen::Project, ..snapshot_with(Vec::new()) }
}

fn snapshot_with(tabs: Vec<WindowTabEntry>) -> NativeMenuSnapshot {
    NativeMenuSnapshot {
        locale: config_ui::DesktopLocale::SimplifiedChinese,
        is_project: true,
        workspace_mode: WorkspaceMode::Work,
        show_file_manager: true,
        file_manager_show_changes: false,
        open_window_tabs: tabs,
    }
}

#[test]
fn resolve_project_file_actions_match_topbar_messages() {
    let app = test_app(Screen::Project);
    assert!(matches!(
        resolve_native_menu_action(&app, NativeMenuAction::FileNewSession),
        Some(Message::ProjectFileNewSession)
    ));
    assert!(matches!(
        resolve_native_menu_action(&app, NativeMenuAction::FileSave),
        Some(Message::SaveFile)
    ));
}

#[test]
fn resolve_project_edit_clipboard_actions_use_focus_aware_shortcuts() {
    let app = test_app(Screen::Project);
    assert!(matches!(
        resolve_native_menu_action(&app, NativeMenuAction::EditCut),
        Some(Message::CutShortcut)
    ));
    assert!(matches!(
        resolve_native_menu_action(&app, NativeMenuAction::EditCopy),
        Some(Message::CopyShortcut)
    ));
    assert!(matches!(
        resolve_native_menu_action(&app, NativeMenuAction::EditPaste),
        Some(Message::PasteShortcut)
    ));
}

#[test]
fn resolve_help_actions_remain_available() {
    let app = test_app(Screen::Home);
    assert!(matches!(
        resolve_native_menu_action(&app, NativeMenuAction::HelpAbout),
        Some(Message::ToggleAboutModal)
    ));
    assert_eq!(help_section().0, "帮助");
}

#[test]
fn resolve_non_project_edit_actions_are_unavailable() {
    let app = test_app(Screen::Home);
    assert!(resolve_native_menu_action(&app, NativeMenuAction::EditCopy).is_none());
    assert!(matches!(
        resolve_native_menu_action(&app, NativeMenuAction::FileGoHome),
        Some(Message::GoHome)
    ));
}

#[test]
fn menu_sections_exclude_browser_and_follow_workspace_mode() {
    let code = NativeMenuSnapshot {
        locale: config_ui::DesktopLocale::SimplifiedChinese,
        workspace_mode: WorkspaceMode::Code,
        show_file_manager: false,
        file_manager_show_changes: false,
        open_window_tabs: Vec::new(),
        ..snapshot_with(Vec::new())
    };
    let work = NativeMenuSnapshot {
        locale: config_ui::DesktopLocale::SimplifiedChinese,
        file_manager_show_changes: true,
        ..snapshot_with(Vec::new())
    };

    let sections = menu_sections(&code);
    assert_eq!(sections.len(), 3);
    assert_eq!(sections[0].0, "文件");
    assert_eq!(sections[1].0, "编辑");
    assert_eq!(sections[2].0, "视图");
    assert!(!sections.iter().any(|(title, _)| *title == "浏览器"));

    let work_view = &menu_sections(&work)[2].1;
    assert_eq!(work_view.len(), 4);
    assert!(matches!(
        &work_view[1],
        MenuEntry::Item { action: Some(NativeMenuAction::ViewWorkOverview), .. }
    ));
}

#[test]
fn snapshot_includes_open_window_tabs_for_project_switch() {
    use crate::native_menu::{TabInput, window_tab_entries};
    let mut snapshot = test_app(Screen::Project);
    snapshot.open_window_tabs = window_tab_entries(
        snapshot.locale,
        [
            TabInput { id: "home", title: "首页", project_path: None },
            TabInput {
                id: "project_a",
                title: "README.md",
                project_path: Some("/workspace/alpha"),
            },
            TabInput { id: "find:hidden", title: "search", project_path: Some("/workspace/alpha") },
        ],
        Some("project_a"),
    );
    assert_eq!(snapshot.open_window_tabs.len(), 2);
    assert_eq!(snapshot.open_window_tabs[0].id, "home");
    assert!(!snapshot.open_window_tabs[0].active);
    assert_eq!(snapshot.open_window_tabs[1].id, "project_a");
    assert!(snapshot.open_window_tabs[1].active);
    assert_eq!(snapshot.open_window_tabs[1].title, "alpha — README.md");
}

#[tokio::test(flavor = "multi_thread")]
async fn desktop_gettext_area_17_native_snapshot_tracks_language_changes() {
    let mut app = test_app(Screen::Project);
    app.locale = config_ui::DesktopLocale::English;
    let english = app.clone();
    app.locale = config_ui::DesktopLocale::TraditionalChinese;
    let traditional = app.clone();
    assert_ne!(english, traditional);
    assert_eq!(traditional.locale, app.locale);
}
