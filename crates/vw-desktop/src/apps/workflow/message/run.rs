//! 处理已保存工作流应用的参数加载、草稿编辑与运行提交消息。

use super::*;

/// 消费已保存应用运行消息；不属于本模块的消息返回 `None`。
pub(super) fn handle(app: &mut App, message: WorkflowMessage) -> Option<Task<Message>> {
    if desktop_workflow_ui::message::run::handle(
        &mut app.workflow_state,
        app.window_size,
        message.clone(),
    ) {
        return Some(Task::none());
    }
    Some(match message {
        WorkflowMessage::OpenSavedAppRunPrompt(uuid) => {
            if uuid == super::video_edit::TEMPLATE_ID {
                return Some(
                    super::video_edit::handle(app, WorkflowMessage::OpenVideoTemplate)
                        .unwrap_or_else(Task::none),
                );
            }
            app.workflow_state.open_saved_app_run_prompt(uuid.clone());
            load_saved_app_run_prompt_task(uuid)
        }
        WorkflowMessage::SubmitSavedAppRunPrompt => {
            let Some(prompt) = app.workflow_state.saved_app_run_prompt.clone() else {
                return Some(Task::none());
            };
            if prompt.uuid == super::video_edit::TEMPLATE_ID {
                return Some(
                    app.show_error_toast("内置剪辑模板必须从原生会话执行，请使用模板设置"),
                );
            }
            desktop_workflow_ui::message::run::submit_run_prompt(&mut app.workflow_state, prompt);
            Task::none()
        }
        WorkflowMessage::SavedAppRunFinished { uuid, result } => {
            if app.workflow_state.running_saved_app_uuid.as_deref() != Some(uuid.as_str()) {
                return Some(Task::none());
            }

            match result {
                Ok(()) => app.workflow_state.finish_saved_app_run(),
                Err(error) => app.workflow_state.fail_saved_app_run(
                    crate::apps::workflow::state::WorkflowRunErrorPreview {
                        status: 0,
                        code: "transport_error".to_string(),
                        message: error,
                    },
                ),
            }
            Task::none()
        }
        WorkflowMessage::CopySavedAppRunError => {
            let Some(report) = app.workflow_state.copy_saved_app_run_error() else {
                return Some(Task::none());
            };
            Task::batch(vec![
                iced::clipboard::write(report),
                crate::app::message::after(
                    std::time::Duration::from_secs(2),
                    Message::WorkflowTool(WorkflowMessage::ClearSavedAppRunErrorCopied),
                ),
            ])
        }
        _ => return None,
    })
}

fn load_saved_app_run_prompt_task(uuid: String) -> Task<Message> {
    desktop_workflow_ui::message::run::load_saved_app_run_prompt_task(
        uuid,
        crate::app::config::gateway_client()
            .map(|client| gateway_workflow::GatewayWorkflowClient::new(client.transport().clone())),
    )
    .map(Message::WorkflowTool)
}

#[cfg(test)]
#[path = "run_tests.rs"]
mod run_tests;
