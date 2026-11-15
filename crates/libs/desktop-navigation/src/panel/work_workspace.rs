use std::collections::HashSet;
use crate::presentation::WorkResourceMode;
#[derive(Debug, Clone)]
pub enum Message {
    WorkResourceModeSelected(WorkResourceMode),
    WorkResourcesToggled,
    WorkResourceFullscreenToggled,
    WorkResourceHalfFullscreenToggled,
    WorkSpaceToggled(String),
    WorkSplitRatioChanged(f32),
}
#[derive(Default)]
pub struct State {
    pub work_resource_mode: WorkResourceMode,
    pub work_resources_visible: bool,
    pub dragging_work_split: bool,
    pub work_split_drag_anchor_x: Option<f32>,
    pub work_resource_fullscreen: bool,
    pub work_resource_half_fullscreen: bool,
    pub work_split_ratio: f32,
    pub work_chat_fullscreen: bool,
    pub work_chat_half_fullscreen: bool,
    pub work_expanded_spaces: HashSet<String>,
}
const MIN_WORK_SPLIT_RATIO: f32 = 0.25;
const MAX_WORK_SPLIT_RATIO: f32 = 0.75;

pub fn update(app: &mut State, message: Message) {
    match message {
        Message::WorkResourceModeSelected(mode) => {
            app.work_resource_mode = mode;
            app.work_resources_visible = true;
        }
        Message::WorkResourcesToggled => {
            app.work_resources_visible = !app.work_resources_visible;
            if !app.work_resources_visible {
                app.dragging_work_split = false;
                app.work_split_drag_anchor_x = None;
                app.work_resource_fullscreen = false;
                app.work_resource_half_fullscreen = false;
            }
        }
        Message::WorkSplitRatioChanged(ratio) if ratio.is_finite() => {
            app.work_split_ratio = ratio.clamp(MIN_WORK_SPLIT_RATIO, MAX_WORK_SPLIT_RATIO);
        }
        Message::WorkResourceFullscreenToggled => {
            app.work_resource_fullscreen = !app.work_resource_fullscreen;
            app.work_resource_half_fullscreen = false;
            if app.work_resource_fullscreen {
                app.work_chat_fullscreen = false;
                app.work_chat_half_fullscreen = false;
            }
        }
        Message::WorkResourceHalfFullscreenToggled => {
            app.work_resource_half_fullscreen = !app.work_resource_half_fullscreen;
            app.work_resource_fullscreen = false;
            if app.work_resource_half_fullscreen {
                app.work_chat_fullscreen = false;
                app.work_chat_half_fullscreen = false;
            }
        }
        Message::WorkSpaceToggled(project_path) => {
            if !app.work_expanded_spaces.insert(project_path.clone()) {
                app.work_expanded_spaces.remove(&project_path);
            }
        }
        _ => {}
    }

}

#[cfg(test)]
#[path = "work_workspace_tests.rs"]
mod tests;
