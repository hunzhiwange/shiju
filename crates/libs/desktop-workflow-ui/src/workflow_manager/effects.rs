//! Lifecycle network effects with a host-provided gateway client.
use super::*;

/// 创建从网关刷新生命周期列表的异步任务。
pub fn load_lifecycle_workflows_task(notify_error: bool, client: ClientResult) -> Task<Message> {
    Task::perform(load_lifecycle_workflows_via_gateway(client), move |result| {
        Message::WorkflowManager(WorkflowManagerMessage::LifecycleWorkflowsLoaded {
            result,
            notify_error,
        })
    })
}

pub fn workflow_manager_effect_task(effect: WorkflowManagerEffect, client: ClientResult) -> Task<Message> {
    match effect {
        WorkflowManagerEffect::LoadLifecycles { notify_error } => {
            load_lifecycle_workflows_task(notify_error, client)
        }
        WorkflowManagerEffect::SaveLifecycle { workflow_id, original_id, body } => {
            let message_workflow_id = workflow_id.clone();
            Task::perform(save_lifecycle_workflow_via_gateway(client, original_id, body), move |result| {
                Message::WorkflowManager(WorkflowManagerMessage::LifecycleWorkflowSaved {
                    workflow_id: message_workflow_id.clone(),
                    result,
                })
            })
        }
        WorkflowManagerEffect::DeleteLifecycle { workflow_id } => {
            let message_workflow_id = workflow_id.clone();
            Task::perform(delete_lifecycle_workflow_via_gateway(client, workflow_id), move |result| {
                Message::WorkflowManager(WorkflowManagerMessage::LifecycleWorkflowDeleted {
                    workflow_id: message_workflow_id.clone(),
                    result,
                })
            })
        }
        WorkflowManagerEffect::ResetLifecycle { workflow_id } => {
            let message_workflow_id = workflow_id.clone();
            Task::perform(reset_lifecycle_workflow_via_gateway(client, workflow_id), move |result| {
                Message::WorkflowManager(WorkflowManagerMessage::LifecycleWorkflowReset {
                    workflow_id: message_workflow_id.clone(),
                    result,
                })
            })
        }
    }
}

async fn load_lifecycle_workflows_via_gateway(client: ClientResult) -> LifecycleWorkflowListResult {
    let client = client?;
    client.task_lifecycle_workflows_list().await
}

async fn save_lifecycle_workflow_via_gateway(
    client: ClientResult,
    original_id: Option<String>,
    body: TaskLifecycleWorkflowUpsertBody,
) -> LifecycleWorkflowListResult {
    let client = client?;
    save_lifecycle_workflow_with_client(&client, original_id.as_deref(), &body).await?;
    client.task_lifecycle_workflows_list().await
}

async fn save_lifecycle_workflow_with_client(
    client: &gateway_task_lifecycle::GatewayTaskLifecycleClient,
    original_id: Option<&str>,
    body: &TaskLifecycleWorkflowUpsertBody,
) -> std::result::Result<(), String> {
    // 网关不支持直接重命名：先创建新 ID，成功后再删除旧记录，避免先删造成丢失。
    let workflow_id = body.workflow.id.as_str();
    match original_id.map(str::trim).filter(|id| !id.is_empty()) {
        Some(original_id) if original_id == workflow_id => {
            client.task_lifecycle_workflows_update(original_id, body).await?;
        }
        Some(original_id) => {
            client.task_lifecycle_workflows_create(body).await?;
            client.task_lifecycle_workflows_delete(original_id).await?;
        }
        None => {
            client.task_lifecycle_workflows_create(body).await?;
        }
    }
    Ok(())
}

async fn delete_lifecycle_workflow_via_gateway(client: ClientResult, workflow_id: String) -> LifecycleWorkflowListResult {
    if is_builtin_task_workflow_id(&workflow_id) {
        return Err(format!("内置生命周期不能删除: {workflow_id}"));
    }
    let client = client?;
    client.task_lifecycle_workflows_delete(&workflow_id).await?;
    client.task_lifecycle_workflows_list().await
}

async fn reset_lifecycle_workflow_via_gateway(client: ClientResult, workflow_id: String) -> LifecycleWorkflowListResult {
    if !is_builtin_task_workflow_id(&workflow_id) {
        return Err(format!("只有内置生命周期可以重置: {workflow_id}"));
    }
    let client = client?;
    client.task_lifecycle_workflows_reset(&workflow_id).await?;
    client.task_lifecycle_workflows_list().await
}
