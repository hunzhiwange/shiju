//! # Workflow 入口模块
//!
//! 该模块汇总 workflow 子模块，并提供初始化、顶层标签同步、视图入口与消息分发入口。

pub mod canvas;
mod message;
pub mod model;
pub mod state;
pub mod view;

#[cfg(test)]
mod dify_node_editor_round_trip_tests;
#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
#[cfg(test)]
mod tests;

pub use message::WorkflowMessage;

use crate::app::{App, AppTab, Message, Screen};
use iced::Task;

pub(crate) const WORKFLOW_TOOL_TAB_ID: &str = "workflow_tool";
pub(crate) const NODE_EDITOR_DSL_EDITOR_ID: &str = "workflow-node-advanced-dsl-editor";

pub(crate) fn sync_top_tab(app: &mut App) {
    let title = app.workflow_state.title().to_string();

    if let Some(pos) = app.open_tabs.iter().position(|t| t.id == "apps") {
        app.open_tabs.remove(pos);
    }

    if let Some(tab) = app.open_tabs.iter_mut().find(|t| t.id == WORKFLOW_TOOL_TAB_ID) {
        tab.title = title;
        tab.screen = Screen::WorkflowTool;
        tab.project_path = None;
    } else {
        app.open_tabs.push(AppTab {
            id: WORKFLOW_TOOL_TAB_ID.to_string(),
            title,
            screen: Screen::WorkflowTool,
            project_path: None,
        });
    }

    app.active_tab_id = Some(WORKFLOW_TOOL_TAB_ID.to_string());
    app.screen = Screen::WorkflowTool;
}

pub fn ensure_initialized(app: &mut App) -> Task<Message> {
    sync_top_tab(app);

    if !app.workflow_state.saved_apps_loaded && !app.workflow_state.saved_apps_loading {
        app.workflow_state.begin_saved_apps_load();
        return message::load_saved_apps_task();
    }

    Task::none()
}

pub fn view(app: &App) -> iced::Element<'_, Message> {
    view::view(&app.workflow_state)
}

pub fn update(app: &mut App, message: WorkflowMessage) -> Task<Message> {
    message::update(app, message)
}
