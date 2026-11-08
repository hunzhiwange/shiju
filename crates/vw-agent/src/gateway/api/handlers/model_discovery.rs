//! Scoped model discovery tasks. Cancellation is ordered against result publication.
use super::{agent_apps::provider_discovery_host, model_catalog::registered_instance};
use crate::{global_database::GlobalDatabase, project::store::ProjectStore};
use axum::{
    Extension, Json, Router,
    extract::{
        DefaultBodyLimit, Path, Query,
        rejection::{JsonRejection, QueryRejection},
    },
    http::{HeaderMap, StatusCode},
    routing::{get, post},
};
use futures_util::FutureExt;
use serde::Deserialize;
use std::{
    collections::HashMap,
    panic::AssertUnwindSafe,
    path::PathBuf,
    sync::{Arc, Mutex, MutexGuard},
};
use tokio_util::sync::CancellationToken;
use vw_agent_apps::{
    apps::{discovery::read::LocalReadEnvironment, lifecycle::operations::Operation},
    providers,
};
use vw_api_types::{
    id::{ProjectId, TaskId, TenantScope},
    model_catalog::{
        ModelCatalogIdentity, ModelDiscoveryFailure, ModelDiscoveryRequest, ModelDiscoveryState,
        ModelDiscoveryStatus,
    },
};

#[path = "model_discovery/source.rs"]
mod source;

const MAX_TASKS: usize = 128;
const MAX_RUNNING: usize = 16;

#[derive(Clone)]
struct Service {
    policy: Option<PathBuf>,
    tasks: Arc<Mutex<HashMap<String, Arc<Task>>>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Target {
    project_id: ProjectId,
    directory: String,
    target_host_id: String,
}

#[derive(PartialEq, Eq)]
struct Owner {
    tenant: TenantScope,
    principal: String,
    host: String,
    project: ProjectId,
    directory: String,
}

struct Task {
    id: TaskId,
    owner: Owner,
    deployment: ModelCatalogIdentity,
    state: Mutex<ModelDiscoveryState>,
    cancellation: CancellationToken,
    operation: Operation,
}

impl Task {
    fn state(&self) -> MutexGuard<'_, ModelDiscoveryState> {
        self.state.lock().unwrap_or_else(|error| error.into_inner())
    }

    fn snapshot(&self) -> ModelDiscoveryStatus {
        ModelDiscoveryStatus {
            task_id: self.id.clone(),
            deployment: self.deployment.clone(),
            state: self.state().clone(),
        }
    }

    async fn run(&self, work: source::Work) -> Result<(), String> {
        let result = AssertUnwindSafe(async {
            if self.cancellation.is_cancelled() {
                return Err(ModelDiscoveryFailure::Unavailable);
            }
            *self.state() = ModelDiscoveryState::Discovering;
            let ids = work.discover(&self.cancellation).await?;
            *self.state() = ModelDiscoveryState::Parsing;
            source::entries(&self.owner.tenant, &self.deployment, ids)
        })
        .catch_unwind()
        .await
        .unwrap_or(Err(ModelDiscoveryFailure::Unavailable));

        let mut state = self.state();
        if self.cancellation.is_cancelled() {
            *state = ModelDiscoveryState::Cancelled;
            return Err("operation cancelled".into());
        }
        match result {
            Ok(models) => {
                *state = ModelDiscoveryState::Completed { models };
                Ok(())
            }
            Err(reason) => {
                *state = ModelDiscoveryState::Failed { reason };
                Err("model discovery failed".into())
            }
        }
    }

    async fn cancel(&self) -> ModelDiscoveryStatus {
        {
            let state = self.state();
            if active(&state) {
                // The same lock guards terminal publication, so a winning cancellation
                // cannot be followed by a completed or failed result.
                self.cancellation.cancel();
            }
        }
        self.operation.wait().await;
        self.snapshot()
    }
}

fn active(state: &ModelDiscoveryState) -> bool {
    matches!(
        state,
        ModelDiscoveryState::Queued
            | ModelDiscoveryState::Discovering
            | ModelDiscoveryState::Parsing
    )
}

pub(super) fn router_with_policy(policy: Option<PathBuf>) -> Router {
    Router::new()
        .route("/model-catalog/discoveries", post(submit).layer(DefaultBodyLimit::max(8192)))
        .route("/model-catalog/discoveries/{id}", get(read).delete(cancel))
        .layer(Extension(Service { policy, tasks: Arc::default() }))
}

fn authorize(
    policy: Option<PathBuf>,
    headers: HeaderMap,
    tenant: TenantScope,
    target: Target,
    database: Option<Extension<GlobalDatabase>>,
) -> Result<(Owner, LocalReadEnvironment, GlobalDatabase), StatusCode> {
    let (host, principal) =
        provider_discovery_host(policy.as_deref(), &headers, &tenant, &target.target_host_id)?;
    let database = match database {
        Some(Extension(database)) => database,
        None => GlobalDatabase::for_home(&crate::global::paths().home)
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
    };
    let projects =
        ProjectStore::new(database.clone()).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let context = registered_instance(&projects, &tenant, &target.project_id, &target.directory)?;
    Ok((
        Owner {
            tenant,
            principal,
            host: host.host_id.clone(),
            project: target.project_id,
            directory: context.directory,
        },
        host,
        database,
    ))
}

async fn submit(
    Extension(service): Extension<Service>,
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    query: Result<Query<Target>, QueryRejection>,
    body: Result<Json<ModelDiscoveryRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<ModelDiscoveryStatus>), StatusCode> {
    let Query(target) = query.map_err(|_| StatusCode::BAD_REQUEST)?;
    let Json(input) = body.map_err(|_| StatusCode::BAD_REQUEST)?;
    if target.target_host_id != input.deployment.target_host_id {
        return Err(StatusCode::BAD_REQUEST);
    }
    providers::validate_identity(&input.deployment.provider_id)
        .and_then(|_| providers::validation::endpoint(&input.deployment.endpoint))
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let policy = service.policy.clone();
    let (owner, work, deployment) = tokio::task::spawn_blocking(move || {
        let (owner, host, database) = authorize(policy, headers, tenant, target, database)?;
        let work = source::prepare(&database, &owner.tenant, host, &input)?;
        Ok::<_, StatusCode>((owner, work, input.deployment))
    })
    .await
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)??;

    let mut tasks = service.tasks.lock().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    if tasks.len() >= MAX_TASKS {
        tasks.retain(|_, task| active(&task.state()));
    }
    if tasks.values().filter(|task| active(&task.state())).count() >= MAX_RUNNING {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }
    let id = uuid::Uuid::new_v4().to_string();
    let task = Arc::new(Task {
        id: TaskId::from(id.clone()),
        owner,
        deployment,
        state: Mutex::new(ModelDiscoveryState::Queued),
        cancellation: CancellationToken::new(),
        operation: Operation::new(),
    });
    let response = task.snapshot();
    let worker = task.clone();
    task.operation
        .start_action(async move { worker.run(work).await })
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    tasks.insert(id, task);
    Ok((StatusCode::ACCEPTED, Json(response)))
}

async fn owned(
    service: Service,
    tenant: TenantScope,
    database: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    query: Result<Query<Target>, QueryRejection>,
    id: String,
) -> Result<Arc<Task>, StatusCode> {
    let Query(target) = query.map_err(|_| StatusCode::BAD_REQUEST)?;
    let policy = service.policy.clone();
    let owner = tokio::task::spawn_blocking(move || {
        authorize(policy, headers, tenant, target, database).map(|(owner, _, _)| owner)
    })
    .await
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)??;
    let tasks = service.tasks.lock().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    tasks.get(&id).filter(|task| task.owner == owner).cloned().ok_or(StatusCode::NOT_FOUND)
}

async fn read(
    Extension(service): Extension<Service>,
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    query: Result<Query<Target>, QueryRejection>,
    Path(id): Path<String>,
) -> Result<Json<ModelDiscoveryStatus>, StatusCode> {
    Ok(Json(owned(service, tenant, database, headers, query, id).await?.snapshot()))
}

async fn cancel(
    Extension(service): Extension<Service>,
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    query: Result<Query<Target>, QueryRejection>,
    Path(id): Path<String>,
) -> Result<Json<ModelDiscoveryStatus>, StatusCode> {
    let task = owned(service, tenant, database, headers, query, id).await?;
    Ok(Json(task.cancel().await))
}
