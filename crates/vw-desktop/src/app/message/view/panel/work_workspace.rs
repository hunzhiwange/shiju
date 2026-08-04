//! 处理工作模式资源区的独立呈现状态。

use super::ViewMessage;
use crate::app::{App, Message};
use iced::Task;

const MIN_WORK_SPLIT_RATIO: f32 = 0.25;
const MAX_WORK_SPLIT_RATIO: f32 = 0.75;

pub(super) fn update(app: &mut App, message: ViewMessage) -> Task<Message> {
    match message {
        ViewMessage::WorkResourceModeSelected(mode) => {
            app.work_resource_mode = mode;
            app.work_resources_visible = true;
        }
        ViewMessage::WorkResourcesToggled => {
            app.work_resources_visible = !app.work_resources_visible;
            if !app.work_resources_visible {
                app.dragging_work_split = false;
                app.work_split_drag_anchor_x = None;
                app.work_resource_fullscreen = false;
                app.work_resource_half_fullscreen = false;
            }
        }
        ViewMessage::WorkSplitRatioChanged(ratio) if ratio.is_finite() => {
            app.work_split_ratio = ratio.clamp(MIN_WORK_SPLIT_RATIO, MAX_WORK_SPLIT_RATIO);
        }
        ViewMessage::WorkResourceFullscreenToggled => {
            app.work_resource_fullscreen = !app.work_resource_fullscreen;
            app.work_resource_half_fullscreen = false;
            if app.work_resource_fullscreen {
                app.work_chat_fullscreen = false;
                app.work_chat_half_fullscreen = false;
            }
        }
        ViewMessage::WorkResourceHalfFullscreenToggled => {
            app.work_resource_half_fullscreen = !app.work_resource_half_fullscreen;
            app.work_resource_fullscreen = false;
            if app.work_resource_half_fullscreen {
                app.work_chat_fullscreen = false;
                app.work_chat_half_fullscreen = false;
            }
        }
        ViewMessage::WorkSpaceToggled(project_path) => {
            if !app.work_expanded_spaces.insert(project_path.clone()) {
                app.work_expanded_spaces.remove(&project_path);
            }
        }
        _ => {}
    }

    Task::none()
}

#[cfg(test)]
#[path = "work_workspace_tests.rs"]
mod work_workspace_tests;
