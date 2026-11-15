use crate::components::top_bar::types::WorkspaceMode;
#[derive(Debug, Clone)]
pub enum Message {
    ToggleWorkspaceModePopover,
    CloseWorkspaceModePopover,
    WorkspaceModeSelected(WorkspaceMode),
}
#[derive(Default)]
pub struct State {
    pub workspace_mode: WorkspaceMode,
    pub work_current_project_menu: Option<String>,
    pub show_workspace_mode_popover: bool,
    pub code_projects_menu_open: bool,
}
fn select_workspace_mode(app: &mut State, mode: WorkspaceMode) {
    app.workspace_mode = mode;
    app.work_current_project_menu = None;
    app.show_workspace_mode_popover = false;
    app.code_projects_menu_open = false;
}

pub fn update(app: &mut State, message: Message) -> Option<WorkspaceMode> {
    match message {
        Message::ToggleWorkspaceModePopover => {
            app.show_workspace_mode_popover = !app.show_workspace_mode_popover;
        }
        Message::CloseWorkspaceModePopover => {
            app.show_workspace_mode_popover = false;
        }
        Message::WorkspaceModeSelected(mode) => {
            select_workspace_mode(app, mode);
            return Some(mode);
        }
    }

    None
}

#[cfg(test)]
#[path = "workspace_mode_tests.rs"]
mod tests;
