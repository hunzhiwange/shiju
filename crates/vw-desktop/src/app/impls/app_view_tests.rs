use super::*;
use serde_json::Map;

fn test_app() -> App {
    App::new().0
}

#[test]
fn view_window_routes_task_pet_window_to_pet_view() {
    let mut app = test_app();
    let pet_window = iced::window::Id::unique();
    app.task_pet_window_id = Some(pet_window);

    let _pet: iced::Element<'_, Message> = app.view_window(pet_window);
    let _main: iced::Element<'_, Message> = app.view_window(iced::window::Id::unique());
}

#[test]
fn view_builds_for_primary_screens_without_modal_state() {
    let mut app = test_app();

    for screen in [
        Screen::Home,
        Screen::Project,
        Screen::Apps,
        Screen::Usage,
        Screen::Knowledge,
        Screen::WorkflowManager,
        Screen::TaskBoard,
    ] {
        app.screen = screen;
        let _element: iced::Element<'_, Message> = app.view();
    }
}

#[test]
fn view_builds_preview_screen() {
    let mut app = test_app();
    app.screen = Screen::Preview;

    std::hint::black_box(app.view());
}

#[test]
fn view_builds_workflow_tool_screen() {
    let mut app = test_app();
    app.screen = Screen::WorkflowTool;

    std::hint::black_box(app.view());
}

#[test]
fn view_builds_with_settings_notifications_error_and_toast_overlays() {
    let mut app = test_app();
    app.show_system_settings = true;
    app.notifications_expanded = true;
    app.error_message = Some("failure".to_string());
    let _ = app.show_success_toast("saved");

    let _element: iced::Element<'_, Message> = app.view();
}

#[test]
fn view_builds_with_about_cli_search_and_rename_overlays() {
    let mut app = test_app();
    app.show_about_modal = true;
    app.show_cli_install_modal = true;
    app.cli_install_modal_show_update_action = true;
    app.cli_install_modal_title = "CLI".to_string();
    app.cli_install_modal_message = "Install".to_string();
    app.cli_install_modal_current_version = "1.0.0".to_string();
    app.cli_install_modal_server_version = "1.1.0".to_string();
    app.cli_install_modal_show_install_action = true;
    app.show_search_overlay = true;
    app.file_tree_rename_path = Some("src/lib.rs".to_string());
    app.file_tree_rename_value = "main.rs".to_string();
    app.session_rename_id = Some("session-1".to_string());
    app.session_rename_value = "Session".to_string();

    let _element: iced::Element<'_, Message> = app.view();
}

#[test]
fn view_builds_with_plain_cli_install_modal() {
    let mut app = test_app();
    app.show_cli_install_modal = true;
    app.cli_install_modal_show_update_action = false;
    app.cli_install_modal_title = "Install CLI".to_string();
    app.cli_install_modal_message = "Install the CLI to continue".to_string();

    std::hint::black_box(app.view());
}

#[test]
fn view_builds_with_project_edit_and_permission_modals_together() {
    let mut app = test_app();
    app.project_edit_path = Some("/workspace/project".to_string());
    app.project_edit_name = "Project".to_string();
    app.permission_modal_request = Some(vw_gateway_client::PendingPermissionRequestDto {
        id: "permission-1".to_string(),
        session_id: "session-1".to_string(),
        permission: "shell".to_string(),
        patterns: Vec::new(),
        metadata: Map::new(),
        always: Vec::new(),
        tool: None,
    });

    let element: iced::Element<'_, Message> = app.view();

    std::hint::black_box(element);
}
