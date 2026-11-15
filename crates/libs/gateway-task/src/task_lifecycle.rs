//! 任务生命周期工作流的租户级增删改查与重置路由。
//!
//! 工作流数据绑定请求租户和全局数据库，重置只恢复存储层定义的初始生命周期状态。

use std::path::{Path as FilePath, PathBuf};
use std::sync::Arc;

use axum::{
    Extension, Json, Router,
    extract::Path,
    routing::{get, post},
};
use vw_api_transport::TenantScope;
use vw_api_task_lifecycle::{
    TaskLifecycleWorkflowDeleteResponse, TaskLifecycleWorkflowDto, TaskLifecycleWorkflowUpsertBody,
};

use gateway_types::ApiError;
use agent_database::GlobalDatabase;
use task_lifecycle::store;

/// 构建任务生命周期工作流管理路由。
pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    router_with_paths(StorePaths::default())
}

fn router_with_paths<S>(paths: StorePaths) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/task-lifecycle/workflows", get(workflow_list).post(workflow_create))
        .route(
            "/task-lifecycle/workflows/{id}",
            get(workflow_get).put(workflow_update).delete(workflow_delete),
        )
        .route("/task-lifecycle/workflows/{id}/reset", post(workflow_reset))
        .layer(Extension(Arc::new(paths)))
}

#[derive(Debug, Clone)]
struct StorePaths {
    home: PathBuf,
}

impl Default for StorePaths {
    fn default() -> Self {
        Self { home: agent_paths::paths().home.clone() }
    }
}

async fn workflow_list(
    Extension(paths): Extension<Arc<StorePaths>>,
    Extension(tenant): Extension<TenantScope>,
) -> Result<Json<Vec<TaskLifecycleWorkflowDto>>, ApiError> {
    let workflows = store::list_workflows(task_lifecycle_database(&paths.home)?, tenant).await.map_err(map_store_error)?;
    Ok(Json(workflows))
}

async fn workflow_get(
    Extension(paths): Extension<Arc<StorePaths>>,
    Extension(tenant): Extension<TenantScope>,
    Path(id): Path<String>,
) -> Result<Json<TaskLifecycleWorkflowDto>, ApiError> {
    let workflow = store::workflow_by_id(task_lifecycle_database(&paths.home)?, tenant, id).await.map_err(map_store_error)?;
    Ok(Json(workflow))
}

async fn workflow_create(
    Extension(paths): Extension<Arc<StorePaths>>,
    Extension(tenant): Extension<TenantScope>,
    Json(body): Json<TaskLifecycleWorkflowUpsertBody>,
) -> Result<Json<TaskLifecycleWorkflowDto>, ApiError> {
    let workflow =
        store::create_workflow(task_lifecycle_database(&paths.home)?, tenant, body).await.map_err(map_store_error)?;
    Ok(Json(workflow))
}

async fn workflow_update(
    Extension(paths): Extension<Arc<StorePaths>>,
    Extension(tenant): Extension<TenantScope>,
    Path(id): Path<String>,
    Json(body): Json<TaskLifecycleWorkflowUpsertBody>,
) -> Result<Json<TaskLifecycleWorkflowDto>, ApiError> {
    let workflow =
        store::update_workflow(task_lifecycle_database(&paths.home)?, tenant, id, body).await.map_err(map_store_error)?;
    Ok(Json(workflow))
}

async fn workflow_delete(
    Extension(paths): Extension<Arc<StorePaths>>,
    Extension(tenant): Extension<TenantScope>,
    Path(id): Path<String>,
) -> Result<Json<TaskLifecycleWorkflowDeleteResponse>, ApiError> {
    let response =
        store::delete_workflow(task_lifecycle_database(&paths.home)?, tenant, id).await.map_err(map_store_error)?;
    Ok(Json(response))
}

async fn workflow_reset(
    Extension(paths): Extension<Arc<StorePaths>>,
    Extension(tenant): Extension<TenantScope>,
    Path(id): Path<String>,
) -> Result<Json<TaskLifecycleWorkflowDto>, ApiError> {
    let workflow = store::reset_workflow(task_lifecycle_database(&paths.home)?, tenant, id).await.map_err(map_store_error)?;
    Ok(Json(workflow))
}

fn task_lifecycle_database(home: &FilePath) -> Result<GlobalDatabase, ApiError> {
    GlobalDatabase::for_home(home).map_err(|error| {
        ApiError::internal(format!("initialize task lifecycle global database failed: {error:#}"))
    })
}

#[cfg(test)]
#[path = "task_lifecycle_tests.rs"]
mod task_lifecycle_tests;

fn map_store_error(error: task_lifecycle::TaskLifecycleError) -> ApiError {
    match error {
        task_lifecycle::TaskLifecycleError::BadRequest(message) => ApiError::bad_request(message),
        task_lifecycle::TaskLifecycleError::NotFound(message) => ApiError::not_found(message),
        task_lifecycle::TaskLifecycleError::Internal(message) => ApiError::internal(message),
    }
}
