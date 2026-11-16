//! Desktop effects for the workflow domain.
use crate::app::{App, Message};
use desktop_workflow_ui::controller::{Context, Effect, Output};
pub use desktop_workflow_ui::message::{WorkflowMessage, WorkflowSaveTarget};
use iced::{Task, widget::text_editor};
pub fn update(app: &mut App, message: WorkflowMessage) -> Task<Message> {
    if let WorkflowMessage::HostEffect(effect) = message {
        return match *effect {
            Effect::Error(error) => app.show_error_toast(error),
            Effect::Warning(error) => app.show_warning_toast(error),
            Effect::Success(message) => app.show_success_toast(message),
            Effect::After(duration, message) => {
                crate::app::message::after(duration, Message::WorkflowTool(*message))
            }
            Effect::SaveError(error) => {
                let task = app.show_error_toast(format!("保存失败: {error}"));
                if let Some(toast) = &mut app.active_toast {
                    toast.localized_message = Some(crate::app::i18n::format::Message::lines(vec![
                        crate::app::i18n::format::Message::new("保存失败", vec![]),
                        error.message(),
                    ]));
                }
                task
            }
            Effect::StartNativeVideoEdit => start_native_video_edit(app),
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
pub(crate) fn sync_video_tenant(app: &mut App) {
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
fn start_native_video_edit(app: &mut App) -> Task<Message> {
    let command = match app.workflow_state.video_edit.native_command() {
        Ok(command) => command,
        Err(e) => return app.show_error_toast(e),
    };
    if app.project_path.is_none() {
        return app.show_error_toast("请先打开原生项目，再从模板发起剪辑");
    }
    if app.acp_agent.is_some() || app.current_session_runtime().acp_agent.is_some() {
        return app.show_error_toast("请先切换原生会话，剪辑不支持 ACP");
    }
    let runtime = app.current_session_runtime();
    if runtime.task_mode_enabled || runtime.workflow_mode_enabled || !app.files.is_empty() {
        return app.show_error_toast("请使用普通原生聊天模式，并先处理已有附件后发起剪辑");
    }
    if !app.current_session_runtime().input_editor.text().trim().is_empty() {
        return app.show_error_toast("会话中已有未发送草稿，请先处理后再启动剪辑");
    }
    app.workflow_state.video_edit.open = false;
    app.screen = crate::app::Screen::Project;
    app.active_tab_id = app.active_session_id.clone();
    app.current_session_runtime_mut().input_editor = text_editor::Content::with_text(&command);
    app.input_editor = text_editor::Content::with_text(&command);
    crate::app::message::chat::update(app, crate::app::message::ChatMessage::SendPressed)
}

#[cfg(test)]
mod video_edit_tests;
