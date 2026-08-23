//! 任务生命周期工作流的租户级增删改查与重置路由。

use std::path::{Path as FilePath, PathBuf};
use std::sync::Arc;

use axum::{
    Extension, Json, Router,
    extract::Path,
    routing::{get, post},
};
use vw_api_types::TenantScope;
use vw_api_types::task_lifecycle::{
    TaskLifecycleWorkflowDeleteResponse, TaskLifecycleWorkflowDto, TaskLifecycleWorkflowUpsertBody,
};

use crate::app::agent::gateway::ApiError;
use crate::global_database::GlobalDatabase;
use crate::task_lifecycle::store;

pub(crate) fn router<S>() -> Router<S>
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
        Self { home: crate::global::paths().home.clone() }
    }
}

async fn workflow_list(
    Extension(paths): Extension<Arc<StorePaths>>,
    Extension(tenant): Extension<TenantScope>,
) -> Result<Json<Vec<TaskLifecycleWorkflowDto>>, ApiError> {
    let workflows = store::list_workflows(task_lifecycle_database(&paths.home)?, tenant).await?;
    Ok(Json(workflows))
}

async fn workflow_get(
    Extension(paths): Extension<Arc<StorePaths>>,
    Extension(tenant): Extension<TenantScope>,
    Path(id): Path<String>,
) -> Result<Json<TaskLifecycleWorkflowDto>, ApiError> {
    let workflow = store::workflow_by_id(task_lifecycle_database(&paths.home)?, tenant, id).await?;
    Ok(Json(workflow))
}

async fn workflow_create(
    Extension(paths): Extension<Arc<StorePaths>>,
    Extension(tenant): Extension<TenantScope>,
    Json(body): Json<TaskLifecycleWorkflowUpsertBody>,
) -> Result<Json<TaskLifecycleWorkflowDto>, ApiError> {
    let workflow =
        store::create_workflow(task_lifecycle_database(&paths.home)?, tenant, body).await?;
    Ok(Json(workflow))
}

async fn workflow_update(
    Extension(paths): Extension<Arc<StorePaths>>,
    Extension(tenant): Extension<TenantScope>,
    Path(id): Path<String>,
    Json(body): Json<TaskLifecycleWorkflowUpsertBody>,
) -> Result<Json<TaskLifecycleWorkflowDto>, ApiError> {
    let workflow =
        store::update_workflow(task_lifecycle_database(&paths.home)?, tenant, id, body).await?;
    Ok(Json(workflow))
}

async fn workflow_delete(
    Extension(paths): Extension<Arc<StorePaths>>,
    Extension(tenant): Extension<TenantScope>,
    Path(id): Path<String>,
) -> Result<Json<TaskLifecycleWorkflowDeleteResponse>, ApiError> {
    let response =
        store::delete_workflow(task_lifecycle_database(&paths.home)?, tenant, id).await?;
    Ok(Json(response))
}

async fn workflow_reset(
    Extension(paths): Extension<Arc<StorePaths>>,
    Extension(tenant): Extension<TenantScope>,
    Path(id): Path<String>,
) -> Result<Json<TaskLifecycleWorkflowDto>, ApiError> {
    let workflow = store::reset_workflow(task_lifecycle_database(&paths.home)?, tenant, id).await?;
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
