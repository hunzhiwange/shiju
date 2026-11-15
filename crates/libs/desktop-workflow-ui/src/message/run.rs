//! Local workflow state transitions; external effects remain in the desktop adapter.
use super::*;
use desktop_workflow_state::state::WorkflowState;
pub fn handle(
    state: &mut WorkflowState,
    _window_size: (f32, f32),
    message: WorkflowMessage,
) -> bool {
    match message {
        WorkflowMessage::SavedAppRunPromptLoaded { uuid, result } => {
            state.finish_saved_app_run_prompt_load(&uuid, result);
        }
        WorkflowMessage::SavedAppRunQueryChanged(query) => {
            state.set_saved_app_run_query(query);
        }
        WorkflowMessage::SavedAppRunInputChanged(index, value) => {
            state.set_saved_app_run_input_value(index, value);
        }
        WorkflowMessage::SavedAppRunInputToggled(index, value) => {
            state.toggle_saved_app_run_input_value(index, value);
        }
        WorkflowMessage::CancelSavedAppRunPrompt => {
            state.close_saved_app_run_prompt();
        }
        WorkflowMessage::SavedAppRunEvent { uuid, event } => {
            if state.running_saved_app_uuid.as_deref() == Some(uuid.as_str()) {
                state.apply_saved_app_run_event(event);
            }
        }
        WorkflowMessage::CancelSavedAppRun => {
            state.cancel_saved_app_run();
        }
        WorkflowMessage::RetrySavedAppRunPreview => {
            state.retry_saved_app_run_preview();
        }
        WorkflowMessage::CloseSavedAppRunPreview => {
            state.close_saved_app_run_preview();
        }
        WorkflowMessage::OpenSavedAppRunNodeDetail(index) => {
            state.open_saved_app_run_node_detail(index);
        }
        WorkflowMessage::OpenSavedAppRunOutputDetail => {
            state.open_saved_app_run_output_detail();
        }
        WorkflowMessage::SavedAppRunDetailAction(action) => {
            state.saved_app_run_detail_action(action);
        }
        WorkflowMessage::CloseSavedAppRunDetail => {
            state.close_saved_app_run_detail();
        }
        WorkflowMessage::ClearSavedAppRunErrorCopied => {
            state.clear_saved_app_run_error_copied();
        }
        _ => return false,
    }
    true
}

use iced::Task;
pub fn load_saved_app_run_prompt_task(
    uuid: String,
    client: Result<gateway_workflow::GatewayWorkflowClient, String>,
) -> iced::Task<WorkflowMessage> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = client;
        Task::perform(
            async { Err("Web 平台暂不支持读取本地 Workflow 应用".to_string()) },
            move |result| WorkflowMessage::SavedAppRunPromptLoaded { uuid: uuid.clone(), result },
        )
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        Task::perform(
            {
                let uuid = uuid.clone();
                async move {
                    let client = client?;
                    let record = client.workflow_application_get(&uuid).await?;
                    let loaded = load_document_from_text(None, record.workflow_yaml)?;
                    Ok(desktop_workflow_state::state::workflow_run_inputs_from_document(
                        &loaded.document,
                    ))
                }
            },
            move |result| WorkflowMessage::SavedAppRunPromptLoaded { uuid: uuid.clone(), result },
        )
    }
}

pub fn submit_run_prompt(
    state: &mut WorkflowState,
    prompt: desktop_workflow_state::state::WorkflowRunPromptDraft,
) {
    if prompt.loading {
        state.set_saved_app_run_prompt_error("运行参数仍在加载");
        return;
    }

    let (query, inputs) = match super::run_inputs::build_run_request_inputs(&prompt) {
        Ok(request) => request,
        Err(error) => {
            state.set_saved_app_run_prompt_error(error);
            return;
        }
    };

    state.remember_saved_app_run_draft(prompt.clone());
    state.begin_saved_app_run(prompt.uuid, query, inputs);
}
