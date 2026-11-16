//! Panel drag geometry, independent of native windows and cross-domain drag owners.
use super::{
    MAX_WORK_SIDEBAR_WIDTH, MAX_WORK_SPLIT_RATIO, MIN_WORK_SIDEBAR_WIDTH, MIN_WORK_SPLIT_RATIO,
};
#[derive(Clone, Copy)]
pub enum Message {
    SplitDragStarted,
    WorkSidebarDragStarted,
    WorkSplitDragStarted,
    SettingsDragStarted,
    FileManagerDragStarted,
    PointerMoved(f32, f32),
}
#[derive(Default)]
pub struct State {
    pub dragging_file_manager: bool,
    pub dragging_settings: bool,
    pub dragging_split: bool,
    pub dragging_work_sidebar: bool,
    pub file_manager_drag_anchor_x: Option<f32>,
    pub file_manager_start_width: f32,
    pub file_manager_width: f32,
    pub settings_drag_anchor_x: Option<f32>,
    pub settings_drag_start_width: f32,
    pub settings_panel_width: f32,
    pub split_drag_anchor_x: Option<f32>,
    pub split_drag_start_ratio: f32,
    pub split_ratio: f32,
    pub work_sidebar_drag_anchor_x: Option<f32>,
    pub work_sidebar_drag_start_width: f32,
    pub work_sidebar_width: f32,
    pub work_split_drag_start_ratio: f32,
}
pub fn update(
    app: &mut State,
    workspace: &mut crate::panel::work_workspace::State,
    sidebar_collapsed: bool,
    window_width: f32,
    message: Message,
) -> bool {
    match message {
        Message::SplitDragStarted => {
            app.dragging_split = true;
            app.split_drag_anchor_x = None;
            app.split_drag_start_ratio = app.split_ratio;
            false
        }
        Message::WorkSidebarDragStarted => {
            app.dragging_work_sidebar = true;
            app.work_sidebar_drag_anchor_x = None;
            app.work_sidebar_drag_start_width = app.work_sidebar_width;
            false
        }
        Message::WorkSplitDragStarted => {
            workspace.dragging_work_split = true;
            workspace.work_split_drag_anchor_x = None;
            app.work_split_drag_start_ratio = workspace.work_split_ratio;
            false
        }
        Message::SettingsDragStarted => {
            app.dragging_settings = true;
            app.settings_drag_anchor_x = None;
            app.settings_drag_start_width = app.settings_panel_width;
            false
        }
        Message::FileManagerDragStarted => {
            app.dragging_file_manager = true;
            app.file_manager_drag_anchor_x = None;
            app.file_manager_start_width = app.file_manager_width;
            false
        }
        Message::PointerMoved(x, y) => {
            let _ = y;
            if app.dragging_settings {
                let Some(value) = super::drag_value(
                    &mut app.settings_drag_anchor_x,
                    x,
                    app.settings_drag_start_width,
                    1.0,
                    150.0,
                    800.0,
                ) else {
                    return false;
                };
                app.settings_panel_width = value;
            }
            if app.dragging_file_manager {
                let Some(value) = super::drag_value(
                    &mut app.file_manager_drag_anchor_x,
                    x,
                    app.file_manager_start_width,
                    -1.0,
                    180.0,
                    600.0,
                ) else {
                    return false;
                };
                app.file_manager_width = value;
            }
            if app.dragging_split {
                let Some(value) = super::drag_value(
                    &mut app.split_drag_anchor_x,
                    x,
                    app.split_drag_start_ratio,
                    window_width.max(1.0),
                    0.2,
                    0.8,
                ) else {
                    return false;
                };
                app.split_ratio = value;
            }
            if app.dragging_work_sidebar {
                let Some(value) = super::drag_value(
                    &mut app.work_sidebar_drag_anchor_x,
                    x,
                    app.work_sidebar_drag_start_width,
                    1.0,
                    MIN_WORK_SIDEBAR_WIDTH,
                    MAX_WORK_SIDEBAR_WIDTH,
                ) else {
                    return false;
                };
                app.work_sidebar_width = value;
            }
            if workspace.dragging_work_split {
                let rail_width = if sidebar_collapsed {
                    crate::presentation::WORK_SIDEBAR_RAIL_WIDTH
                } else {
                    0.0
                };
                let available_width = (window_width - app.work_sidebar_width - rail_width).max(1.0);
                let Some(value) = super::drag_value(
                    &mut workspace.work_split_drag_anchor_x,
                    x,
                    app.work_split_drag_start_ratio,
                    available_width,
                    MIN_WORK_SPLIT_RATIO,
                    MAX_WORK_SPLIT_RATIO,
                ) else {
                    return false;
                };
                workspace.work_split_ratio = value;
            }
            true
        }
    }
}

/// Finish local drags and return the file-pane preference only when it was dragged.
pub fn finish(app: &mut State, workspace: &mut crate::panel::work_workspace::State) -> Option<f32> {
    if app.dragging_settings {
        app.dragging_settings = false;
        app.settings_drag_anchor_x = None;
    }
    let save = if app.dragging_file_manager {
        app.dragging_file_manager = false;
        app.file_manager_drag_anchor_x = None;
        Some(app.file_manager_width)
    } else {
        None
    };
    app.dragging_split = false;
    if app.dragging_work_sidebar {
        app.dragging_work_sidebar = false;
        app.work_sidebar_drag_anchor_x = None;
    }
    if workspace.dragging_work_split {
        workspace.dragging_work_split = false;
        workspace.work_split_drag_anchor_x = None;
    }
    save
}
