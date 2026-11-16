use crate::components::top_bar::menus::{
    Inputs, MenuType, edit_menu, file_menu, help_menu, view_menu,
};
fn test_app(is_project: bool) -> Inputs<'static> {
    Inputs {
        i18n: desktop_i18n::I18n::new(Default::default()),
        is_project,
        active_menu: None,
        workspace_mode: crate::components::top_bar::types::WorkspaceMode::Code,
        show_file_manager: false,
        file_manager_show_changes: false,
        update_version: None,
    }
}
#[test]
fn file_menu_renders_screen_specific_items() {
    for screen in [true, false] {
        let mut app = test_app(screen);
        app.active_menu = Some(MenuType::File);
        let _element = file_menu(&app);
    }
}

#[test]
fn file_menu_renders_when_file_menu_is_closed() {
    let mut app = test_app(true);
    app.active_menu = None;

    let _element = file_menu(&app);
}

#[test]
fn edit_menu_renders_project_and_empty_fallback() {
    for screen in [true, false] {
        let mut app = test_app(screen);
        app.active_menu = Some(MenuType::Edit);
        let _element = edit_menu(&app);
    }
}

#[test]
fn edit_menu_renders_project_when_menu_is_closed() {
    let mut app = test_app(true);
    app.active_menu = None;

    let _element = edit_menu(&app);
}

#[test]
fn view_menu_renders_panel_and_mode_toggle_states() {
    let mut app = test_app(true);
    app.active_menu = Some(MenuType::View);
    app.show_file_manager = false;
    app.file_manager_show_changes = false;
    let _element = view_menu(&app);
    drop(_element);

    app.show_file_manager = true;
    app.file_manager_show_changes = true;
    let _element = view_menu(&app);
}

#[test]
fn help_menu_renders_available_actions() {
    let mut app = test_app(false);
    app.active_menu = Some(MenuType::Help);
    let _element = help_menu(&app);
}

#[test]
fn help_menu_renders_closed_state() {
    let app = test_app(false);

    let _element = help_menu(&app);
}
