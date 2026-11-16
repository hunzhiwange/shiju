//! Complete workflow lifecycle; desktop supplies only gateway, tenant and host effects.
use crate::message::{WorkflowMessage, WorkflowSaveTarget};
use desktop_workflow_model::*;
use desktop_workflow_state::state::*;
use iced::{Point, Task, widget::text_editor};
pub(crate) mod app;
pub(crate) mod canvas;
pub(crate) mod document;
pub(crate) mod node;
pub(crate) mod run;
pub(crate) mod video_edit;
#[derive(Debug, Clone)]
pub enum Effect {
    Error(String),
    Warning(String),
    Success(String),
    SaveError(desktop_i18n::workflow::Feedback),
    StartNativeVideoEdit,
    After(std::time::Duration, Box<WorkflowMessage>),
}
#[derive(Debug, Clone)]
pub enum Output {
    Workflow(WorkflowMessage),
    Effect(Effect),
}
pub struct Context<'a> {
    pub workflow_state: &'a mut WorkflowState,
    pub window_size: (f32, f32),
    pub tenant: vw_api_transport::TenantScope,
    pub sync_top_tab: bool,
    pub gateway: fn() -> Result<gateway_workflow::GatewayWorkflowClient, String>,
}
impl Context<'_> {
    fn show_error_toast(&self, error: impl ToString) -> Task<Output> {
        Task::done(Output::Effect(Effect::Error(error.to_string())))
    }
    fn show_warning_toast(&self, error: impl ToString) -> Task<Output> {
        Task::done(Output::Effect(Effect::Warning(error.to_string())))
    }
    fn show_success_toast(&self, error: impl ToString) -> Task<Output> {
        Task::done(Output::Effect(Effect::Success(error.to_string())))
    }
}
pub(crate) fn sync_top_tab(app: &mut Context<'_>) {
    app.sync_top_tab = true;
}
pub fn update(app: &mut Context<'_>, message: WorkflowMessage) -> Task<Output> {
    app.workflow_state.select_tenant(app.tenant.clone());
    let task = video_edit::handle(app, message.clone())
        .or_else(|| app::handle(app, message.clone()))
        .or_else(|| run::handle(app, message.clone()))
        .or_else(|| node::handle(app, message.clone()))
        .or_else(|| document::handle(app, message.clone()))
        .or_else(|| canvas::handle(app, message))
        .unwrap_or_else(Task::none);
    task
}
fn after(duration: std::time::Duration, message: Output) -> Task<Output> {
    let Output::Workflow(message) = message else { unreachable!() };
    Task::done(Output::Effect(Effect::After(duration, Box::new(message))))
}
pub(crate) fn apply_loaded(app: &mut Context<'_>, loaded: LoadedWorkflow) {
    app.workflow_state.apply_loaded(loaded, app.window_size);
    sync_top_tab(app);
}

pub(crate) fn load_saved_apps_task(app: &Context<'_>) -> Task<Output> {
    crate::message::effects::load_saved_apps_task(app.gateway, app.tenant.clone())
        .map(Output::Workflow)
}

pub(crate) fn open_saved_app_task(app: &Context<'_>, uuid: String) -> Task<Output> {
    crate::message::effects::open_saved_app_task(app.gateway, uuid).map(Output::Workflow)
}

pub(crate) fn delete_saved_app_task(app: &Context<'_>, uuid: String) -> Task<Output> {
    crate::message::effects::delete_saved_app_task(app.gateway, uuid).map(Output::Workflow)
}

pub(crate) fn save_active_app(app: &mut Context<'_>, force_picker: bool) -> Task<Output> {
    let Some(entry) = app.workflow_state.active_entry_snapshot() else {
        return Task::none();
    };

    if !force_picker && entry.local_uuid.as_deref() == Some(video_edit::TEMPLATE_ID) {
        return video_edit::save_graph(app, entry);
    }
    save_entry(app, entry, force_picker)
}

#[cfg(test)]
pub(crate) fn suggested_new_node_position(app: &Context<'_>) -> Point {
    let zoom = app.workflow_state.zoom.max(0.0001);
    let center_screen = Point::new(app.window_size.0 * 0.52, app.window_size.1 * 0.5);
    let offset = (app.workflow_state.document.nodes.len() % 6) as f32 * 28.0;
    Point::new(
        (center_screen.x - app.workflow_state.pan.x) / zoom + offset,
        (center_screen.y - app.workflow_state.pan.y) / zoom + offset,
    )
}

pub(crate) fn save_entry(
    app: &Context<'_>,
    entry: desktop_workflow_state::state::WorkflowAppEntry,
    force_picker: bool,
) -> Task<Output> {
    crate::message::effects::save_entry(app.gateway, entry, force_picker).map(Output::Workflow)
}

pub(crate) fn open_file() -> Task<Output> {
    crate::message::effects::open_file().map(Output::Workflow)
}

#[cfg(test)]
use crate::message::effects::suggested_export_file_name;

pub(crate) fn export_svg(app: &mut Context<'_>) -> Task<Output> {
    crate::message::effects::export_svg(&mut app.workflow_state).map(Output::Workflow)
}

pub(crate) fn export_png(app: &mut Context<'_>) -> Task<Output> {
    crate::message::effects::export_png(&mut app.workflow_state).map(Output::Workflow)
}

pub(crate) fn export_jpeg(app: &mut Context<'_>) -> Task<Output> {
    crate::message::effects::export_jpeg(&mut app.workflow_state).map(Output::Workflow)
}

pub(crate) fn export_finished(
    app: &mut Context<'_>,
    result: Result<(), desktop_i18n::workflow::Feedback>,
) {
    crate::message::effects::export_finished(&mut app.workflow_state, result)
}

#[cfg(test)]
pub(crate) use crate::message::effects::suggested_export_file_name;
