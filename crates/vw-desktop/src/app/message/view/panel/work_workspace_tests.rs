//! 工作模式资源面板测试，覆盖资源类型、分栏比例和按路径保存的空间状态。
//! 这些消息只应修改工作模式布局，不得污染代码模式下已有的面板偏好。

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
        show_diff: app.navigation_panels.show_diff,
        show_file_manager: app.file_manager.show_file_manager,
        split_ratio: app.layout_drag.split_ratio,
        chat_panel_fullscreen: app.chat_interaction.chat_panel_fullscreen,
        chat_panel_half_fullscreen: app.chat_interaction.chat_panel_half_fullscreen,
        git_diff_fullscreen: app.git.git_diff_fullscreen,
        show_preview_fullscreen_overlay: app.preview_state.show_preview_fullscreen_overlay,
        terminal_visible: app.terminal.is_visible,
        terminal_height: app.terminal.height,
        settings_panel_width: app.layout_drag.settings_panel_width,
        file_manager_width: app.layout_drag.file_manager_width,
    }
}

#[test]
fn work_workspace_messages_preserve_code_layout_preferences() {
    let mut app = App::new().0;
    app.navigation_panels.show_diff = false;
    app.file_manager.show_file_manager = false;
    app.layout_drag.split_ratio = 0.37;
    app.chat_interaction.chat_panel_fullscreen = true;
    app.chat_interaction.chat_panel_half_fullscreen = true;
    app.git.git_diff_fullscreen = true;
    app.preview_state.show_preview_fullscreen_overlay = true;
    app.terminal.is_visible = true;
    app.terminal.height = 333.0;
    app.layout_drag.settings_panel_width = 451.0;
    app.layout_drag.file_manager_width = 287.0;
    let code_layout = code_layout_snapshot(&app);

    let _task = update_panel(
        &mut app,
        ViewMessage::WorkResourceModeSelected(WorkResourceMode::WorkspaceFiles),
    );
    assert_eq!(app.navigation_workspace.work_resource_mode, WorkResourceMode::WorkspaceFiles);
    assert!(app.navigation_workspace.work_resources_visible);
    let _task =
        update_panel(&mut app, ViewMessage::WorkResourceModeSelected(WorkResourceMode::Browser));
    assert_eq!(app.navigation_workspace.work_resource_mode, WorkResourceMode::Browser);

    let _task = update_panel(&mut app, ViewMessage::WorkSplitRatioChanged(0.1));
    assert_eq!(app.navigation_workspace.work_split_ratio, 0.25);
    let _task = update_panel(&mut app, ViewMessage::WorkSplitRatioChanged(0.9));
    assert_eq!(app.navigation_workspace.work_split_ratio, 0.75);
    let _task = update_panel(&mut app, ViewMessage::WorkSplitRatioChanged(0.55));
    assert_eq!(app.navigation_workspace.work_split_ratio, 0.55);

    app.navigation_workspace.work_chat_fullscreen = true;
    app.navigation_workspace.work_chat_half_fullscreen = true;
    let _task = update_panel(&mut app, ViewMessage::WorkResourceHalfFullscreenToggled);
    assert!(app.navigation_workspace.work_resource_half_fullscreen);
    assert!(!app.navigation_workspace.work_resource_fullscreen);
    assert!(!app.navigation_workspace.work_chat_fullscreen);
    assert!(!app.navigation_workspace.work_chat_half_fullscreen);

    let _task = update_panel(&mut app, ViewMessage::WorkResourceFullscreenToggled);
    assert!(app.navigation_workspace.work_resource_fullscreen);
    assert!(!app.navigation_workspace.work_resource_half_fullscreen);
    assert!(!app.navigation_workspace.work_chat_fullscreen);
    let _task = update_panel(&mut app, ViewMessage::WorkResourceFullscreenToggled);
    assert!(!app.navigation_workspace.work_resource_fullscreen);
    let _task =
        update_panel(&mut app, ViewMessage::WorkResourceModeSelected(WorkResourceMode::Overview));
    assert_eq!(app.navigation_workspace.work_resource_mode, WorkResourceMode::Overview);

    assert_eq!(code_layout_snapshot(&app), code_layout);
}
