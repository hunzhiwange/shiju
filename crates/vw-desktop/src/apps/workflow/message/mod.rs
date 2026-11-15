//! # Workflow 消息模块
//!
//! 该模块定义 workflow 的消息类型，并将 UI、文件、导出和编辑操作分发到状态更新逻辑。

use super::model::{
    LoadedWorkflow, load_document_from_path, load_document_from_value,
};
#[cfg(test)]
use super::model::{WorkflowAppMeta, WorkflowConnectionEndpoint, create_blank_workflow};
use super::model::load_document_from_text;

#[cfg(test)]
use super::state::*;
use crate::app::{App, Message};
#[cfg(test)]
use iced::Point;
use iced::Task;
#[cfg(test)]
use iced::widget::text_editor;

#[cfg(test)]
mod video_edit_tests;

mod app;
mod canvas;
mod document;
mod node;
mod run;
mod video_edit;

#[cfg(test)]
#[path = "app_tests.rs"]
mod app_tests;
#[cfg(test)]
#[path = "canvas_tests.rs"]
mod canvas_tests;
#[cfg(test)]
#[path = "document_tests.rs"]
mod document_tests;
#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
#[cfg(test)]
#[path = "node_tests.rs"]
mod node_tests;
#[cfg(test)]
mod tests;

pub use desktop_workflow_ui::message::{WorkflowMessage, WorkflowSaveTarget};

pub fn update(app: &mut App, message: WorkflowMessage) -> Task<Message> {
    sync_video_tenant(app);
    video_edit::handle(app, message.clone())
        .or_else(|| app::handle(app, message.clone()))
        .or_else(|| run::handle(app, message.clone()))
        .or_else(|| node::handle(app, message.clone()))
        .or_else(|| document::handle(app, message.clone()))
        .or_else(|| canvas::handle(app, message))
        .unwrap_or_else(Task::none)
}

pub(crate) fn sync_video_tenant(app: &mut App) {
    app.workflow_state.select_tenant(crate::app::session_gateway::active_tenant_scope());
}

pub(super) fn apply_loaded(app: &mut App, loaded: LoadedWorkflow) {
    app.workflow_state.apply_loaded(loaded, app.window_size);
    crate::apps::workflow::sync_top_tab(app);
}

pub(crate) fn load_saved_apps_task() -> Task<Message> {
    desktop_workflow_ui::message::effects::load_saved_apps_task(workflow_gateway_client, crate::app::session_gateway::active_tenant_scope()).map(Message::WorkflowTool)
}

pub(super) fn open_saved_app_task(uuid: String) -> Task<Message> {
    desktop_workflow_ui::message::effects::open_saved_app_task(workflow_gateway_client, uuid).map(Message::WorkflowTool)
}

pub(super) fn delete_saved_app_task(uuid: String) -> Task<Message> {
    desktop_workflow_ui::message::effects::delete_saved_app_task(workflow_gateway_client, uuid).map(Message::WorkflowTool)
}

pub(super) fn save_active_app(app: &mut App, force_picker: bool) -> Task<Message> {
    let Some(entry) = app.workflow_state.active_entry_snapshot() else {
        return Task::none();
    };

    if !force_picker && entry.local_uuid.as_deref() == Some(video_edit::TEMPLATE_ID) {
        return video_edit::save_graph(app, entry);
    }
    save_entry(entry, force_picker)
}

#[cfg(test)]
pub(super) fn suggested_new_node_position(app: &App) -> Point {
    let zoom = app.workflow_state.zoom.max(0.0001);
    let center_screen = Point::new(app.window_size.0 * 0.52, app.window_size.1 * 0.5);
    let offset = (app.workflow_state.document.nodes.len() % 6) as f32 * 28.0;
    Point::new(
        (center_screen.x - app.workflow_state.pan.x) / zoom + offset,
        (center_screen.y - app.workflow_state.pan.y) / zoom + offset,
    )
}

pub(super) fn save_entry(
    entry: super::state::WorkflowAppEntry,
    force_picker: bool,
) -> Task<Message> {
    desktop_workflow_ui::message::effects::save_entry(workflow_gateway_client, entry, force_picker).map(Message::WorkflowTool)
}

pub(super) fn open_file() -> Task<Message> {
    desktop_workflow_ui::message::effects::open_file().map(Message::WorkflowTool)
}

#[cfg(test)]
use desktop_workflow_ui::message::effects::suggested_export_file_name;

pub(super) fn export_svg(app: &mut App) -> Task<Message> {
    desktop_workflow_ui::message::effects::export_svg(&mut app.workflow_state).map(Message::WorkflowTool)
}

pub(super) fn export_png(app: &mut App) -> Task<Message> {
    desktop_workflow_ui::message::effects::export_png(&mut app.workflow_state).map(Message::WorkflowTool)
}

pub(super) fn export_jpeg(app: &mut App) -> Task<Message> {
    desktop_workflow_ui::message::effects::export_jpeg(&mut app.workflow_state).map(Message::WorkflowTool)
}

pub(super) fn export_finished(
    app: &mut App,
    result: Result<(), crate::app::i18n::workflow::Feedback>,
) {
    desktop_workflow_ui::message::effects::export_finished(&mut app.workflow_state, result)
}

fn workflow_gateway_client() -> Result<gateway_workflow::GatewayWorkflowClient, String> {
    crate::app::config::gateway_client().map(|client| gateway_workflow::GatewayWorkflowClient::new(client.transport().clone()))
}
