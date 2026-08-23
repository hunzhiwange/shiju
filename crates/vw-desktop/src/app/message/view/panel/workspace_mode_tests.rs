//! 工作区模式切换测试，确认模式弹层状态与代码面板偏好彼此隔离。
//! 往返切换后仍需保留文件管理器、终端和设置面板的用户尺寸。

use super::*;
use crate::app::state::WorkspaceMode;

#[test]
fn workspace_mode_message_preserves_code_panel_preferences() {
    let mut app = App::new().0;
    app.workspace_mode = WorkspaceMode::Code;
    app.show_workspace_mode_popover = true;
    app.show_diff = true;
    app.show_file_manager = false;
    app.terminal.is_visible = true;
    let file_manager_width = app.file_manager_width;
    let terminal_height = app.terminal.height;
    let settings_panel_width = app.settings_panel_width;

    select_workspace_mode(&mut app, WorkspaceMode::Work);

    assert_eq!(app.workspace_mode, WorkspaceMode::Work);
    assert!(!app.show_workspace_mode_popover);
    assert!(app.show_diff);
    assert!(!app.show_file_manager);
    assert!(app.terminal.is_visible);
    assert_eq!(app.file_manager_width, file_manager_width);

    select_workspace_mode(&mut app, WorkspaceMode::Code);

    assert_eq!(app.workspace_mode, WorkspaceMode::Code);
    assert!(app.show_diff);
    assert!(!app.show_file_manager);
    assert!(app.terminal.is_visible);
    assert_eq!(app.file_manager_width, file_manager_width);
    assert_eq!(app.terminal.height, terminal_height);
    assert_eq!(app.settings_panel_width, settings_panel_width);
}

#[test]
fn workspace_mode_popover_toggle_and_close_are_local() {
    let mut app = App::new().0;
    assert!(!app.show_workspace_mode_popover);

    let _ = update(&mut app, ViewMessage::ToggleWorkspaceModePopover);
    assert!(app.show_workspace_mode_popover);

    let _ = update(&mut app, ViewMessage::CloseWorkspaceModePopover);
    assert!(!app.show_workspace_mode_popover);
}
