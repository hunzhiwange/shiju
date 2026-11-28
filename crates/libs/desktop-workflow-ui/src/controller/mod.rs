//! Complete workflow lifecycle; desktop supplies only gateway, tenant and host effects.
use crate::message::{WorkflowMessage, WorkflowSaveTarget};
use desktop_workflow_model::*;
use desktop_workflow_state::state::*;
use iced::Task;
pub(crate) mod app;
pub(crate) mod canvas;
pub(crate) mod document;
pub(crate) mod node;
pub(crate) mod run;
#[derive(Debug, Clone)]
pub enum Effect {
    TextContext(desktop_ui_controls::text_editor_context_menu::TextContextEffect),
    TextSelection(String, usize, usize, Option<String>),
    None,
    Error(desktop_i18n::workflow::Feedback),
    SaveError(desktop_i18n::workflow::Feedback),
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
    fn show_error_toast(&self, error: impl Into<desktop_i18n::workflow::Feedback>) -> Task<Output> {
        Task::done(Output::Effect(Effect::Error(error.into())))
    }
}
pub(crate) fn sync_top_tab(app: &mut Context<'_>) {
    app.sync_top_tab = true;
}
pub fn update(app: &mut Context<'_>, message: WorkflowMessage) -> Task<Output> {
    app.workflow_state.select_tenant(app.tenant.clone());

    app::handle(app, message.clone())
        .or_else(|| run::handle(app, message.clone()))
        .or_else(|| node::handle(app, message.clone()))
        .or_else(|| document::handle(app, message.clone()))
        .or_else(|| canvas::handle(app, message))
        .unwrap_or_else(Task::none)
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

    save_entry(app, entry, force_picker)
}

#[cfg(test)]
fn suggested_new_node_position(app: &Context<'_>) -> Point {
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

pub(crate) fn export_svg(app: &mut Context<'_>) -> Task<Output> {
    crate::message::effects::export_svg(app.workflow_state).map(Output::Workflow)
}

pub(crate) fn export_png(app: &mut Context<'_>) -> Task<Output> {
    crate::message::effects::export_png(app.workflow_state).map(Output::Workflow)
}

pub(crate) fn export_jpeg(app: &mut Context<'_>) -> Task<Output> {
    crate::message::effects::export_jpeg(app.workflow_state).map(Output::Workflow)
}

pub(crate) fn export_finished(
    app: &mut Context<'_>,
    result: Result<(), desktop_i18n::workflow::Feedback>,
) {
    crate::message::effects::export_finished(app.workflow_state, result)
}


#[cfg(test)]
pub(crate) use crate::message::effects::suggested_export_file_name;

#[cfg(test)]
#[path = "../host_module_12_tests.rs"]
mod host_module_12_tests;

#[cfg(test)]
use iced::Point;
