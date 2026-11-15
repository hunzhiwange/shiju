//! 将独立生命周期服务错误映射为宿主 API 错误；不持有业务状态。

use crate::app::agent::gateway::ApiError;
use crate::global_database::GlobalDatabase;
use vw_api_transport::TenantScope;
use vw_api_task_lifecycle::{
    TaskLifecycleWorkflowDeleteResponse, TaskLifecycleWorkflowDto, TaskLifecycleWorkflowUpsertBody,
};

fn map_store_error(error: task_lifecycle::TaskLifecycleError) -> ApiError {
    match error {
        task_lifecycle::TaskLifecycleError::BadRequest(message) => ApiError::bad_request(message),
        task_lifecycle::TaskLifecycleError::NotFound(message) => ApiError::not_found(message),
        task_lifecycle::TaskLifecycleError::Internal(message) => ApiError::internal(message),
    }
}

pub(crate) async fn list_workflows(
    database: GlobalDatabase,
    tenant: TenantScope,
) -> Result<Vec<TaskLifecycleWorkflowDto>, ApiError> {
    task_lifecycle::store::list_workflows(database, tenant)
        .await
        .map_err(map_store_error)
}

pub(crate) async fn workflow_by_id(
    database: GlobalDatabase,
    tenant: TenantScope,
    id: String,
) -> Result<TaskLifecycleWorkflowDto, ApiError> {
    task_lifecycle::store::workflow_by_id(database, tenant, id)
        .await
        .map_err(map_store_error)
}

pub(crate) async fn create_workflow(
    database: GlobalDatabase,
    tenant: TenantScope,
    body: TaskLifecycleWorkflowUpsertBody,
) -> Result<TaskLifecycleWorkflowDto, ApiError> {
    task_lifecycle::store::create_workflow(database, tenant, body)
        .await
        .map_err(map_store_error)
}

pub(crate) async fn update_workflow(
    database: GlobalDatabase,
    tenant: TenantScope,
    id: String,
    body: TaskLifecycleWorkflowUpsertBody,
) -> Result<TaskLifecycleWorkflowDto, ApiError> {
    task_lifecycle::store::update_workflow(database, tenant, id, body)
        .await
        .map_err(map_store_error)
}

pub(crate) async fn delete_workflow(
    database: GlobalDatabase,
    tenant: TenantScope,
    id: String,
) -> Result<TaskLifecycleWorkflowDeleteResponse, ApiError> {
    task_lifecycle::store::delete_workflow(database, tenant, id)
        .await
        .map_err(map_store_error)
}

pub(crate) async fn reset_workflow(
    database: GlobalDatabase,
    tenant: TenantScope,
    id: String,
) -> Result<TaskLifecycleWorkflowDto, ApiError> {
    task_lifecycle::store::reset_workflow(database, tenant, id)
        .await
        .map_err(map_store_error)
}
