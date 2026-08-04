use super::super::{ViewMessage, update as update_panel};
use crate::app::{App, state::WorkResourceMode};

#[derive(Debug, PartialEq)]
struct CodeLayoutSnapshot {
    show_diff: bool,
    show_file_manager: bool,
    split_ratio: f32,
    chat_panel_fullscreen: bool,
    chat_panel_half_fullscreen: bool,
    git_diff_fullscreen: bool,
    show_preview_fullscreen_overlay: bool,
    terminal_visible: bool,
    terminal_height: f32,
    settings_panel_width: f32,
    file_manager_width: f32,
}

fn code_layout_snapshot(app: &App) -> CodeLayoutSnapshot {
    CodeLayoutSnapshot {
        show_diff: app.show_diff,
        show_file_manager: app.show_file_manager,
        split_ratio: app.split_ratio,
        chat_panel_fullscreen: app.chat_panel_fullscreen,
        chat_panel_half_fullscreen: app.chat_panel_half_fullscreen,
        git_diff_fullscreen: app.git_diff_fullscreen,
        show_preview_fullscreen_overlay: app.show_preview_fullscreen_overlay,
        terminal_visible: app.terminal.is_visible,
        terminal_height: app.terminal.height,
        settings_panel_width: app.settings_panel_width,
        file_manager_width: app.file_manager_width,
    }
}

#[test]
fn work_workspace_messages_preserve_code_layout_preferences() {
    let mut app = App::new().0;
    app.show_diff = false;
    app.show_file_manager = false;
    app.split_ratio = 0.37;
    app.chat_panel_fullscreen = true;
    app.chat_panel_half_fullscreen = true;
    app.git_diff_fullscreen = true;
    app.show_preview_fullscreen_overlay = true;
    app.terminal.is_visible = true;
    app.terminal.height = 333.0;
    app.settings_panel_width = 451.0;
    app.file_manager_width = 287.0;
    let code_layout = code_layout_snapshot(&app);

    let _task = update_panel(
        &mut app,
        ViewMessage::WorkResourceModeSelected(WorkResourceMode::WorkspaceFiles),
    );
    assert_eq!(app.work_resource_mode, WorkResourceMode::WorkspaceFiles);
    assert!(app.work_resources_visible);
    let _task =
        update_panel(&mut app, ViewMessage::WorkResourceModeSelected(WorkResourceMode::Browser));
    assert_eq!(app.work_resource_mode, WorkResourceMode::Browser);

    let _task = update_panel(&mut app, ViewMessage::WorkSplitRatioChanged(0.1));
    assert_eq!(app.work_split_ratio, 0.25);
    let _task = update_panel(&mut app, ViewMessage::WorkSplitRatioChanged(0.9));
    assert_eq!(app.work_split_ratio, 0.75);
    let _task = update_panel(&mut app, ViewMessage::WorkSplitRatioChanged(0.55));
    assert_eq!(app.work_split_ratio, 0.55);

    app.work_chat_fullscreen = true;
    app.work_chat_half_fullscreen = true;
    let _task = update_panel(&mut app, ViewMessage::WorkResourceHalfFullscreenToggled);
    assert!(app.work_resource_half_fullscreen);
    assert!(!app.work_resource_fullscreen);
    assert!(!app.work_chat_fullscreen);
    assert!(!app.work_chat_half_fullscreen);

    let _task = update_panel(&mut app, ViewMessage::WorkResourceFullscreenToggled);
    assert!(app.work_resource_fullscreen);
    assert!(!app.work_resource_half_fullscreen);
    assert!(!app.work_chat_fullscreen);
    let _task = update_panel(&mut app, ViewMessage::WorkResourceFullscreenToggled);
    assert!(!app.work_resource_fullscreen);
    let _task =
        update_panel(&mut app, ViewMessage::WorkResourceModeSelected(WorkResourceMode::Overview));
    assert_eq!(app.work_resource_mode, WorkResourceMode::Overview);

    assert_eq!(code_layout_snapshot(&app), code_layout);
}

#[test]
fn work_resources_toggle_hides_whole_panel_and_mode_selection_restores_it() {
    let mut app = App::new().0;
    assert!(!app.work_resources_visible);

    let _task = update_panel(&mut app, ViewMessage::WorkResourcesToggled);
    assert!(app.work_resources_visible);

    app.dragging_work_split = true;
    app.work_split_drag_anchor_x = Some(640.0);
    app.work_resource_fullscreen = true;
    app.work_resource_half_fullscreen = true;

    let _task = update_panel(&mut app, ViewMessage::WorkResourcesToggled);

    assert!(!app.work_resources_visible);
    assert!(!app.dragging_work_split);
    assert!(app.work_split_drag_anchor_x.is_none());
    assert!(!app.work_resource_fullscreen);
    assert!(!app.work_resource_half_fullscreen);

    let _task = update_panel(
        &mut app,
        ViewMessage::WorkResourceModeSelected(WorkResourceMode::Browser),
    );
    assert!(app.work_resources_visible);
    assert_eq!(app.work_resource_mode, WorkResourceMode::Browser);
}

#[test]
fn work_workspace_space_toggle_is_path_scoped() {
    let mut app = App::new().0;
    let first_path = "/workspace/project".to_string();
    let second_path = "/workspace/project-copy".to_string();

    let _task = update_panel(&mut app, ViewMessage::WorkSpaceToggled(first_path.clone()));
    assert!(app.work_expanded_spaces.contains(&first_path));
    assert!(!app.work_expanded_spaces.contains(&second_path));

    let _task = update_panel(&mut app, ViewMessage::WorkSpaceToggled(second_path.clone()));
    assert!(app.work_expanded_spaces.contains(&first_path));
    assert!(app.work_expanded_spaces.contains(&second_path));

    let _task = update_panel(&mut app, ViewMessage::WorkSpaceToggled(first_path.clone()));
    assert!(!app.work_expanded_spaces.contains(&first_path));
    assert!(app.work_expanded_spaces.contains(&second_path));
}
