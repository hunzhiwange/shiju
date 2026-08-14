use super::actions::{
    NativeMenuSnapshot, WindowTabEntry, help_section, menu_sections, parse_window_tab_menu_id,
    project_display_label_for_test, resolve_native_menu_action, window_tab_menu_id,
    window_tab_title_for_test,
};
use crate::app::components::top_bar::menu_actions::NativeMenuAction;
use crate::app::components::top_bar::menus::MenuEntry;
use crate::app::state::WorkspaceMode;
use crate::app::{App, Message, Screen, message};

fn test_app(screen: Screen) -> App {
    let (mut app, _task) = App::new();
    app.screen = screen;
    app
}

fn snapshot_with(tabs: Vec<WindowTabEntry>) -> NativeMenuSnapshot {
    NativeMenuSnapshot {
        is_project: true,
        workspace_mode: WorkspaceMode::Work,
        show_file_manager: true,
        file_manager_show_changes: false,
        open_window_tabs: tabs,
    }
}

#[test]
fn native_menu_action_ids_round_trip() {
    let action = NativeMenuAction::ViewWorkBrowser;
    assert_eq!(NativeMenuAction::from_id(action.id()), Some(action));
    assert!(NativeMenuAction::from_id("native.unknown").is_none());
}

#[test]
fn resolve_project_file_actions_match_topbar_messages() {
    let app = test_app(Screen::Project);
    assert!(matches!(
        resolve_native_menu_action(&app, NativeMenuAction::FileNewSession),
        Some(Message::View(message::ViewMessage::ProjectFileNewSession))
    ));
    assert!(matches!(
        resolve_native_menu_action(&app, NativeMenuAction::FileSave),
        Some(Message::Preview(message::PreviewMessage::SaveFile))
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
        Some(Message::View(message::ViewMessage::ToggleAboutModal))
    ));
    assert_eq!(help_section().0, "帮助");
}

#[test]
fn resolve_non_project_edit_actions_are_unavailable() {
    let app = test_app(Screen::Home);
    assert!(resolve_native_menu_action(&app, NativeMenuAction::EditCopy).is_none());
    assert!(matches!(
        resolve_native_menu_action(&app, NativeMenuAction::FileGoHome),
        Some(Message::View(message::ViewMessage::GoHome))
    ));
}

#[test]
fn menu_sections_exclude_browser_and_follow_workspace_mode() {
    let code = NativeMenuSnapshot {
        workspace_mode: WorkspaceMode::Code,
        show_file_manager: false,
        file_manager_show_changes: false,
        open_window_tabs: Vec::new(),
        ..snapshot_with(Vec::new())
    };
    let work = NativeMenuSnapshot { file_manager_show_changes: true, ..snapshot_with(Vec::new()) };

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
fn project_display_label_uses_basename_or_fallback() {
    assert_eq!(project_display_label_for_test(None), "未打开项目");
    assert_eq!(project_display_label_for_test(Some("")), "未打开项目");
    assert_eq!(project_display_label_for_test(Some("/Users/me/work/vibe-window")), "vibe-window");
}

#[test]
fn window_tab_menu_ids_round_trip() {
    let id = window_tab_menu_id("project_/repo/alpha");
    assert_eq!(parse_window_tab_menu_id(&id), Some("project_/repo/alpha"));
    assert!(parse_window_tab_menu_id("native.window.other").is_none());
    assert!(parse_window_tab_menu_id("native.window.tab.").is_none());
}

#[test]
fn window_tab_title_prefers_project_and_title() {
    assert_eq!(
        window_tab_title_for_test("README.md", Some("/Users/me/work/vibe-window")),
        "vibe-window — README.md"
    );
    assert_eq!(
        window_tab_title_for_test("vibe-window", Some("/Users/me/work/vibe-window")),
        "vibe-window"
    );
    assert_eq!(window_tab_title_for_test("首页", None), "首页");
}

#[test]
fn snapshot_includes_open_window_tabs_for_project_switch() {
    let mut app = test_app(Screen::Project);
    app.open_tabs = vec![
        crate::app::AppTab {
            id: "home".into(),
            title: "首页".into(),
            screen: Screen::Home,
            project_path: None,
        },
        crate::app::AppTab {
            id: "project_a".into(),
            title: "README.md".into(),
            screen: Screen::Project,
            project_path: Some("/workspace/alpha".into()),
        },
        crate::app::AppTab {
            id: "find:hidden".into(),
            title: "search".into(),
            screen: Screen::Project,
            project_path: Some("/workspace/alpha".into()),
        },
    ];
    app.active_tab_id = Some("project_a".into());

    let snapshot = NativeMenuSnapshot::from_app(&app);
    assert_eq!(snapshot.open_window_tabs.len(), 2);
    assert_eq!(snapshot.open_window_tabs[0].id, "home");
    assert!(!snapshot.open_window_tabs[0].active);
    assert_eq!(snapshot.open_window_tabs[1].id, "project_a");
    assert!(snapshot.open_window_tabs[1].active);
    assert_eq!(snapshot.open_window_tabs[1].title, "alpha — README.md");
}

#[test]
fn native_menu_action_ids_are_unique() {
    let ids = [
        NativeMenuAction::FileNewSession.id(),
        NativeMenuAction::FileSave.id(),
        NativeMenuAction::EditCopy.id(),
        NativeMenuAction::ViewWorkBrowser.id(),
        NativeMenuAction::HelpAbout.id(),
    ];
    let mut sorted = ids.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), ids.len());
}
