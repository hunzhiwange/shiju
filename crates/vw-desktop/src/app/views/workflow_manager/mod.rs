//! Desktop lifecycle manager wiring.
use crate::app::{App, Message};
use desktop_workflow_ui::workflow_manager as domain;
pub use domain::{WorkflowManagerMessage, WorkflowManagerState, WorkflowManagerTab, WorkflowDraftStatus, WorkflowManagerFeedback, LifecycleWorkflowListResult, update};
use iced::{Element, Task};
fn client() -> domain::ClientResult {
    Ok(gateway_task_lifecycle::GatewayTaskLifecycleClient::new(crate::app::config::gateway_client()?.transport().clone()))
}
fn map_message(message: domain::Message) -> Message {
    match message {
        domain::Message::WorkflowManager(message) => Message::WorkflowManager(message),
        domain::Message::CopyCode(text) => Message::CopyCode(text),
        domain::Message::AppendText(text) => crate::app::components::text_editor_context_menu::text_context_message(desktop_ui_controls::text_editor_context_menu::TextContextEffect::Append(text)),
        domain::Message::OpenUrl(url) => crate::app::components::text_editor_context_menu::text_context_message(desktop_ui_controls::text_editor_context_menu::TextContextEffect::OpenUrl(url)),
        domain::Message::None => Message::None,
    }
}
pub(crate) fn load_lifecycle_workflows_task(notify_error: bool) -> Task<Message> {
    domain::load_lifecycle_workflows_task(notify_error, client()).map(map_message)
}
pub fn view(app: &App) -> Element<'_, Message> {
    domain::view(&app.workflow_manager, desktop_i18n::I18n::new(app.desktop_locale), &app.app_theme).map(map_message)
}
/// 更新应用中的工作流状态、刷新运行时缓存并派发网关副作用。
pub fn update_app(app: &mut App, message: WorkflowManagerMessage) -> Task<Message> {
    let mut registry = domain::registry_from_runtime();
    let _changed = update(&mut app.workflow_manager, &mut registry, message);
    if let Some(effect) = app.workflow_manager.take_pending_effect() {
        return domain::workflow_manager_effect_task(effect, client()).map(map_message);
    }
    let feedback_task = if let Some(feedback) = app.workflow_manager.feedback().cloned() {
        if let Some(message) = feedback.localized.clone() {
            let kind = if feedback.is_error() {
                crate::app::state::ToastKind::Error
            } else {
                crate::app::state::ToastKind::Success
            };
            app.show_localized_toast(
                kind,
                &message.source,
                message.arguments.into_iter().map(Into::into).collect(),
            )
        } else if feedback.is_error() {
            app.show_error_toast(feedback.message().to_string())
        } else if feedback.is_success() {
            app.show_success_toast(feedback.message().to_string())
        } else {
            Task::none()
        }
    } else {
        Task::none()
    };
    feedback_task
}
