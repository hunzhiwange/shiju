//! Local workflow state transitions; external effects remain in the desktop adapter.
use super::*;
use desktop_workflow_state::state::WorkflowState;
pub fn handle(
    state: &mut WorkflowState,
    _window_size: (f32, f32),
    message: WorkflowMessage,
) -> bool {
    match message {
        WorkflowMessage::CloseVideoTemplate => {
            state.video_edit.open = false;
            state.video_edit.restore_confirm = false;
        }
        WorkflowMessage::VideoTemplateYamlAction(action) => {
            if !state.video_edit.busy
                && let Some(editor) = &mut state.video_edit.editor
            {
                editor.perform(action);
            }
        }
        WorkflowMessage::VideoTemplatePathChanged(path) => {
            state.video_edit.path = path;
        }
        WorkflowMessage::RequestVideoTemplateRestore => {
            state.video_edit.restore_confirm = true;
        }
        WorkflowMessage::CancelVideoTemplateRestore => {
            state.video_edit.restore_confirm = false;
        }
        _ => return false,
    }
    true
}

use desktop_workflow_state::state::video_edit::TEMPLATE_ID;
use iced::Task;
use vw_api_transport::TenantScope;
use vw_api_workflow::WorkflowRecord;
pub fn load_template(
    tenant: TenantScope,
    client: Result<gateway_workflow::GatewayWorkflowClient, String>,
    graph: bool,
) -> Task<WorkflowMessage> {
    Task::perform(
        async move {
            let result = async { client?.workflow_application_get(TEMPLATE_ID).await }.await;
            (tenant, result)
        },
        move |(tenant, result)| WorkflowMessage::VideoTemplateLoaded { tenant, graph, result },
    )
}

pub fn persist(
    client: Result<gateway_workflow::GatewayWorkflowClient, String>,
    tenant: TenantScope,
    record: WorkflowRecord,
    yaml: String,
    graph: bool,
    restore: bool,
) -> Task<WorkflowMessage> {
    Task::perform(
        async move {
            let result = async {
                let client = client?;
                if restore {
                    client.workflow_template_restore(TEMPLATE_ID, record.updated_at_ms, true).await
                } else {
                    client
                        .workflow_template_update(
                            TEMPLATE_ID,
                            &vw_api_workflow::WorkflowRecordUpsertBody {
                                uuid: Some(TEMPLATE_ID.into()),
                                name: record.name,
                                description: record.description,
                                workflow_yaml: yaml,
                            },
                            record.updated_at_ms,
                        )
                        .await
                }
            }
            .await;
            (tenant, result)
        },
        move |(tenant, result)| WorkflowMessage::VideoTemplatePersisted {
            tenant,
            graph,
            restore,
            result,
        },
    )
}
