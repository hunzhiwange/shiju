//! 处理代码/工作界面模式的弹层与选择状态。

use super::ViewMessage;
use crate::app::{App, Message, set_config_field};
use iced::Task;

fn select_workspace_mode(app: &mut App, mode: crate::app::state::WorkspaceMode) {
    app.workspace_mode = mode;
    app.show_workspace_mode_popover = false;
}

pub(super) fn update(app: &mut App, message: ViewMessage) -> Task<Message> {
    match message {
        ViewMessage::ToggleWorkspaceModePopover => {
            app.show_workspace_mode_popover = !app.show_workspace_mode_popover;
            if app.show_workspace_mode_popover {
                app.active_menu = None;
            }
        }
        ViewMessage::CloseWorkspaceModePopover => {
            app.show_workspace_mode_popover = false;
        }
        ViewMessage::WorkspaceModeSelected(mode) => {
            select_workspace_mode(app, mode);
            set_config_field(
                "workspace_mode",
                serde_json::Value::String(mode.as_str().to_string()),
            );
        }
        _ => {}
    }

    Task::none()
}

#[cfg(test)]
#[path = "workspace_mode_tests.rs"]
mod workspace_mode_tests;
