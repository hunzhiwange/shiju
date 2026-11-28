//! Desktop effects for the workflow domain.
use crate::app::{App, Message};
use desktop_workflow_ui::controller::{Context, Effect, Output};
pub use desktop_workflow_ui::message::WorkflowMessage;
use iced::Task;
pub fn update(app: &mut App, message: WorkflowMessage) -> Task<Message> {
    if let WorkflowMessage::HostEffect(effect) = message {
        return match *effect {
            Effect::None => Task::none(),
            Effect::TextContext(effect) => Task::done(desktop_ui_controls::selectable_text::markdown_selection::SelectionMessage::context(effect)),
            Effect::TextSelection(session, index, text_index, text) => Task::done(desktop_ui_controls::selectable_text::markdown_selection::SelectionMessage::selection(session, index, text_index, text)),
            Effect::Error(error) => app.show_message_toast(crate::app::state::ToastKind::Error, error.message()),
            Effect::After(duration, message) => {
                crate::app::message::after(duration, Message::WorkflowTool(*message))
            }
            Effect::SaveError(error) => {
                let task = app.show_error_toast(format!("保存失败: {error}"));
                if let Some(toast) = &mut app.toast_state.active_toast {
                    toast.localized_message = Some(crate::app::i18n::format::Message::lines(vec![
                        crate::app::i18n::format::Message::new("保存失败", vec![]),
                        error.message(),
                    ]));
                }
                task
            }
        };
    }
    let mut context = Context {
        workflow_state: &mut app.workflow_state,
        window_size: app.window_size,
        tenant: crate::app::session_gateway::active_tenant_scope(),
        gateway: workflow_gateway_client,
        sync_top_tab: false,
    };
    let task = desktop_workflow_ui::controller::update(&mut context, message);
    let sync = context.sync_top_tab;
    if sync {
        super::sync_top_tab(app);
    }
    task.map(|output| {
        Message::WorkflowTool(match output {
            Output::Workflow(message) => message,
            Output::Effect(effect) => WorkflowMessage::HostEffect(Box::new(effect)),
        })
    })
}
pub(crate) fn sync_workflow_tenant(app: &mut App) {
    app.workflow_state.select_tenant(crate::app::session_gateway::active_tenant_scope());
}
pub(crate) fn load_saved_apps_task() -> Task<Message> {
    desktop_workflow_ui::message::effects::load_saved_apps_task(
        workflow_gateway_client,
        crate::app::session_gateway::active_tenant_scope(),
    )
    .map(Message::WorkflowTool)
}
fn workflow_gateway_client() -> Result<gateway_workflow::GatewayWorkflowClient, String> {
    crate::app::config::gateway_client()
        .map(|client| gateway_workflow::GatewayWorkflowClient::new(client.transport().clone()))
}
