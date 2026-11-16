use crate::panel::settings as domain;
#[derive(Default)]
struct GitSnapshot {
    show_git_diff_summary: bool,
}
#[derive(Default)]
struct FileSnapshot {
    show_file_manager: bool,
}
#[derive(Default)]
struct TerminalSnapshot {
    is_visible: bool,
}
#[derive(Default)]
struct PanelFixture {
    navigation_panels: domain::State,
    navigation_mode: crate::panel::workspace_mode::State,
    popover_visibility: desktop_chat_input::popovers::Visibility,
    layout_drag: crate::view_layout::drag::State,
    git: GitSnapshot,
    file_manager: FileSnapshot,
    terminal: TerminalSnapshot,
}
fn update(app: &mut PanelFixture, message: domain::Message) -> Option<domain::Effect> {
    domain::update(
        domain::Context {
            state: &mut app.navigation_panels,
            popovers: &mut app.popover_visibility,
            drag: &mut app.layout_drag,
            git_diff_summary: &mut app.git.show_git_diff_summary,
            file_manager_visible: &mut app.file_manager.show_file_manager,
            plugin_catalog_empty: true,
            plugin_loading: false,
            mcp_loading: false,
            mcp_saving: false,
        },
        app.navigation_mode.workspace_mode,
        None,
        message,
    )
}
#[test]
fn system_settings_toggle_clears_help_and_flips_visibility() {
    use crate::panel::settings::Message as ViewMessage;
    use desktop_ui_controls::settings_navigation::SystemTab;

    let mut app = PanelFixture::default();
    app.navigation_panels.show_system_settings = true;
    app.navigation_panels.system_settings_help_tab = Some(SystemTab::General);

    let _ = update(&mut app, ViewMessage::ToggleSystemSettings);
    assert!(!app.navigation_panels.show_system_settings);
    assert_eq!(app.navigation_panels.system_settings_help_tab, None);

    let _ = update(&mut app, ViewMessage::ToggleSystemSettings);
    assert!(app.navigation_panels.show_system_settings);
}

#[test]
fn opening_system_settings_tab_closes_transient_popovers() {
    use crate::components::top_bar::menus::MenuType;
    use crate::panel::settings::Message as ViewMessage;
    use desktop_ui_controls::settings_navigation::SystemTab;

    let mut app = PanelFixture::default();
    app.navigation_panels.show_system_settings = false;
    app.navigation_panels.system_settings_help_tab = Some(SystemTab::Memory);
    app.navigation_panels.active_menu = Some(MenuType::Help);
    app.popover_visibility.model = true;
    app.popover_visibility.mode = true;
    app.popover_visibility.file = true;
    app.popover_visibility.usage = true;
    app.popover_visibility.session_tool_selector = true;

    let _ = update(&mut app, ViewMessage::OpenSystemSettingsTab(SystemTab::General));

    assert!(app.navigation_panels.show_system_settings);
    assert_eq!(app.navigation_panels.system_settings_help_tab, None);
    assert_eq!(app.navigation_panels.active_menu, None);
    assert!(!app.popover_visibility.model);
    assert!(!app.popover_visibility.mode);
    assert!(!app.popover_visibility.file);
    assert!(!app.popover_visibility.usage);
    assert!(!app.popover_visibility.session_tool_selector);
}

#[test]
fn panel_visibility_and_menu_actions_update_only_local_state() {
    use crate::components::top_bar::menus::MenuType;
    use crate::panel::settings::Message as ViewMessage;

    let mut app = PanelFixture::default();
    app.navigation_mode.workspace_mode = crate::components::top_bar::types::WorkspaceMode::Code;

    let initial_diff = app.navigation_panels.show_diff;
    let _ = update(&mut app, ViewMessage::ToggleDiffPanel);
    assert_eq!(app.navigation_panels.show_diff, !initial_diff);

    let initial_summary = app.git.show_git_diff_summary;
    let _ = update(&mut app, ViewMessage::ToggleGitDiffSummary);
    assert_eq!(app.git.show_git_diff_summary, !initial_summary);

    app.layout_drag.dragging_file_manager = true;
    app.layout_drag.file_manager_drag_anchor_x = Some(42.0);
    let _ = update(&mut app, ViewMessage::FileManagerPanelVisible(false));
    assert!(!app.file_manager.show_file_manager);
    assert!(!app.layout_drag.dragging_file_manager);
    assert_eq!(app.layout_drag.file_manager_drag_anchor_x, None);

    let _ = update(&mut app, ViewMessage::AutoMaxToggled(true));
    assert!(app.navigation_panels.auto_max_mode);

    let _ = update(&mut app, ViewMessage::ToggleMenu(Some(MenuType::File)));
    assert_eq!(app.navigation_panels.active_menu, Some(MenuType::File));
    let _ = update(&mut app, ViewMessage::ToggleMenu(Some(MenuType::File)));
    assert_eq!(app.navigation_panels.active_menu, None);
    let _ = update(&mut app, ViewMessage::ToggleMenu(Some(MenuType::Help)));
    assert_eq!(app.navigation_panels.active_menu, Some(MenuType::Help));
    let _ = update(&mut app, ViewMessage::ToggleMenu(None));
    assert_eq!(app.navigation_panels.active_menu, None);

    app.navigation_panels.active_menu = Some(MenuType::View);
    let _ = update(&mut app, ViewMessage::MenuAction);
    assert_eq!(app.navigation_panels.active_menu, None);
}

#[test]
fn work_mode_ignores_code_only_review_toggle() {
    use crate::components::top_bar::types::WorkspaceMode;
    use crate::panel::settings::Message as ViewMessage;

    let mut app = PanelFixture::default();
    app.navigation_mode.workspace_mode = WorkspaceMode::Work;
    app.navigation_panels.show_diff = false;
    app.terminal.is_visible = false;

    let _ = update(&mut app, ViewMessage::ToggleDiffPanel);

    assert!(!app.navigation_panels.show_diff);
    assert!(!app.terminal.is_visible);
}

#[test]
fn about_modal_toggle_round_trips_visibility() {
    use crate::panel::settings::Message as ViewMessage;

    let mut app = PanelFixture::default();
    let initial = app.navigation_panels.show_about_modal;

    let _ = update(&mut app, ViewMessage::ToggleAboutModal);
    assert_eq!(app.navigation_panels.show_about_modal, !initial);

    let _ = update(&mut app, ViewMessage::ToggleAboutModal);
    assert_eq!(app.navigation_panels.show_about_modal, initial);
}

#[test]
fn menu_hover_only_switches_an_already_open_menu() {
    use crate::components::top_bar::menus::MenuType;
    use crate::panel::settings::Message as ViewMessage;

    let mut app = PanelFixture::default();
    let _ = update(&mut app, ViewMessage::MenuHovered(MenuType::Help));
    assert_eq!(app.navigation_panels.active_menu, None);

    app.navigation_panels.active_menu = Some(MenuType::File);
    let _ = update(&mut app, ViewMessage::MenuHovered(MenuType::View));
    assert_eq!(app.navigation_panels.active_menu, Some(MenuType::View));
}
