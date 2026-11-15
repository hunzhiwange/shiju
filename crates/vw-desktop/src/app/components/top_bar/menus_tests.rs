use super::menus::{file_menu, edit_menu, view_menu, help_menu};
use crate::app::{App, Screen};
use crate::app::message::view::MenuType;
fn test_app(screen: Screen) -> App {
 let mut app = App::new().0;
 app.screen = screen;
 app.active_menu = None;
 app
}
#[test]
fn file_menu_renders_screen_specific_items() {
    for screen in [Screen::Project, Screen::Home] {
        let mut app = test_app(screen);
        app.active_menu = Some(MenuType::File);
        let _element = file_menu(&app);
    }
}

#[test]
fn file_menu_renders_when_file_menu_is_closed() {
    let mut app = test_app(Screen::Project);
    app.active_menu = None;

    let _element = file_menu(&app);
}

#[test]
fn edit_menu_renders_project_and_empty_fallback() {
    for screen in [Screen::Project, Screen::Home] {
        let mut app = test_app(screen);
        app.active_menu = Some(MenuType::Edit);
        let _element = edit_menu(&app);
    }
}

#[test]
fn edit_menu_renders_project_when_menu_is_closed() {
    let mut app = test_app(Screen::Project);
    app.active_menu = None;

    let _element = edit_menu(&app);
}

#[test]
fn view_menu_renders_panel_and_mode_toggle_states() {
    let mut app = test_app(Screen::Project);
    app.active_menu = Some(MenuType::View);
    app.file_manager.show_file_manager = false;
    app.file_manager.file_manager_show_changes = false;
    let _element = view_menu(&app);
    drop(_element);

    app.file_manager.show_file_manager = true;
    app.file_manager.file_manager_show_changes = true;
    let _element = view_menu(&app);
}

#[test]
fn help_menu_renders_available_actions() {
    let mut app = test_app(Screen::Home);
    app.active_menu = Some(MenuType::Help);
    let _element = help_menu(&app);
}

#[test]
fn help_menu_renders_closed_state() {
    let app = test_app(Screen::Home);

    let _element = help_menu(&app);
}
