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

#[test]
fn title_covers_each_screen() {
    let expected = [
        (Screen::Home, "识矩 - 项目"),
        (Screen::Project, "识矩 - 项目"),
        (Screen::Preview, "识矩 - 预览"),
        (Screen::Apps, "识矩 - 应用"),
        (Screen::Usage, "识矩 - 用量"),
        (Screen::Knowledge, "识矩 - 知识库"),
        (Screen::WorkflowTool, "识矩 - 工作流"),
        (Screen::WorkflowManager, "识矩 - 任务编排"),
        (Screen::TaskBoard, "识矩 - 任务看板"),
        (Screen::ShijuApps, "识矩 - 识矩应用"),
        (Screen::ShijuSkills, "识矩 - 识矩技能"),
        (Screen::ExpertHub, "识矩 - 专家·技能·连接器"),
    ];

    for (screen, title) in expected {
        assert_eq!(
            crate::window_lifecycle::title(screen, config_ui::DesktopLocale::SimplifiedChinese),
            title
        );
    }
}

#[test]
fn push_notification_assigns_incrementing_ids() {
    let mut notifications = Vec::new();
    let mut next_notification_id = 7;
    notifications.clear();

    crate::components::notification::push_notification(
        &mut notifications,
        &mut next_notification_id,
        "first".to_string(),
    );
    crate::components::notification::push_notification(
        &mut notifications,
        &mut next_notification_id,
        "second".to_string(),
    );

    assert_eq!(notifications.len(), 2);
    assert_eq!(notifications[0].id, 7);
    assert_eq!(notifications[0].message, "first");
    assert_eq!(notifications[1].id, 8);
    assert_eq!(notifications[1].message, "second");
    assert_eq!(next_notification_id, 9);
}

#[test]
fn desktop_integration_46_late_failure_preserves_newer_copy_feedback() {
    let mut state = crate::clipboard::Feedback::new();
    state.last_copied_code_hash = Some(47);
    state.clear(46);
    assert_eq!(state.last_copied_code_hash, Some(47));
}

#[test]
fn desktop_integration_46_preserves_expected_behavior() {
    let mut state = crate::clipboard::Feedback::new();
    state.last_copied_code_hash = Some(46);
    state.last_copy_time = Some(web_time::SystemTime::now());
    state.clear(46);
    assert_eq!(state.last_copied_code_hash, None);
    assert!(state.last_copy_time.is_none());
}

use crate::components::top_bar::types::ExternalOpenApp;
#[test]
fn can_open_external_defaults_false_and_uses_cached_value() {
    let mut exists = std::collections::HashMap::new();
    exists.clear();

    assert!(!crate::external_apps_update::can_open_external(&exists, ExternalOpenApp::VSCode));

    exists.insert(ExternalOpenApp::VSCode, true);
    exists.insert(ExternalOpenApp::Cursor, false);
    assert!(crate::external_apps_update::can_open_external(&exists, ExternalOpenApp::VSCode));
    assert!(!crate::external_apps_update::can_open_external(&exists, ExternalOpenApp::Cursor));
}

use iced::Theme;
#[test]
fn mini_companion_native_theme_clears_transparently_in_both_modes() {
    use iced::theme::Base;
    for theme in [Theme::Dark, Theme::Light] {
        let mini_theme = crate::window_lifecycle::theme_for_surface(&theme, true);
        assert_eq!(
            crate::window_lifecycle::window_style(&mini_theme).background_color,
            iced::Color::TRANSPARENT
        );
        // Menus and input backgrounds must remain opaque even on the transparent surface.
        assert_eq!(mini_theme.base().background_color, theme.base().background_color);
        assert_eq!(mini_theme.base().text_color, theme.base().text_color);
        assert_eq!(
            crate::window_lifecycle::theme_for_surface(&theme, false).base().background_color,
            theme.base().background_color
        );
        assert_eq!(
            mini_theme.extended_palette().background.strong.color,
            theme.extended_palette().background.strong.color
        );
    }
}
