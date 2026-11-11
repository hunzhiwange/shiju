//! Authorized reads and writes of application failover configuration.
use super::lifecycle_api::{ControlError, RoutingControl};
use crate::{
    app_providers::AppProviderStore,
    gateway::{
        agent_apps_high_availability_service::{self as service, QueueChange, QueueMutationError},
        agent_apps_high_availability_store::{FailoverQueueStore, FailoverQueueVersionConflict},
    },
    global_database::GlobalDatabase,
};
use axum::{
    Extension, Json, Router,
    extract::{
        DefaultBodyLimit, Query,
        rejection::{JsonRejection, QueryRejection},
    },
    http::{HeaderMap, HeaderValue, Method, StatusCode, header},
    middleware,
    response::{IntoResponse, Response},
    routing::{get, post, put},
};
use serde_json::json;
use std::collections::HashMap;
use vw_agent_apps::{
    high_availability::queue::Queue, providers::validate_identity, routing::supports_app,
};
use vw_api_types::{
    agent_apps_high_availability::{
        FailoverCandidatesResponse, FailoverQueueEnabledRequest, FailoverQueueEntry,
        FailoverQueueMemberRequest, FailoverQueueQuery, FailoverQueueReorderRequest,
        FailoverQueueResponse, FailoverQueueScope, FailoverQueueVersion,
    },
    error::{ApiErrorBody, ApiErrorDetail},
    id::{ProjectScope, TenantScope},
};

#[derive(Clone, Copy)]
enum Resource {
    Queue,
    Candidates,
}

pub(crate) fn router() -> Router {
    Router::new()
        .route("/agent-apps/high-availability/queue/members", post(member).delete(member))
        .route("/agent-apps/high-availability/queue/order", put(order))
        .route("/agent-apps/high-availability/enabled", put(enabled))
        .layer(DefaultBodyLimit::max(16 * 1024))
        .route("/agent-apps/high-availability/queue", get(read).layer(Extension(Resource::Queue)))
        .route(
            "/agent-apps/high-availability/candidates",
            get(read).layer(Extension(Resource::Candidates)),
        )
        .layer(middleware::map_response(async |mut response: Response| {
            response
                .headers_mut()
                .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
            response
        }))
}

async fn read(
    control: Option<Extension<RoutingControl>>,
    Extension(tenant): Extension<TenantScope>,
    Extension(resource): Extension<Resource>,
    database: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    query: Result<Query<FailoverQueueQuery>, QueryRejection>,
) -> Result<Response, ReadError> {
    let Query(query) = query.map_err(|_| ReadError::InvalidInput)?;
    validate_identity(&query.target_host_id).map_err(|_| ReadError::InvalidInput)?;
    let project = ProjectScope::new(tenant.clone(), query.project_id.clone())
        .map_err(|_| ReadError::InvalidInput)?;
    RoutingControl::authorize(
        control,
        headers,
        tenant,
        query.target_host_id.clone(),
        Some((query.project_id, query.app_id)),
        false,
    )
    .await
    .map_err(|error| match error {
        ControlError::Unauthenticated => ReadError::Unauthenticated,
        ControlError::Forbidden => ReadError::Forbidden,
        ControlError::NotFound => ReadError::NotFound,
        _ => ReadError::Unavailable,
    })?;
    if !supports_app(query.app_id) {
        return Err(ReadError::UnsupportedMode);
    }
    let scope =
        FailoverQueueScope { project, target_host_id: query.target_host_id, app_id: query.app_id };
    tokio::task::spawn_blocking(move || {
        let db = match database {
            Some(Extension(database)) => database,
            None => GlobalDatabase::for_home(&crate::global::paths().home)
                .map_err(|_| ReadError::Unavailable)?,
        };
        read_resource(db, scope, resource)
    })
    .await
    .map_err(|_| ReadError::Unavailable)?
}

fn read_resource(
    database: GlobalDatabase,
    scope: FailoverQueueScope,
    resource: Resource,
) -> Result<Response, ReadError> {
    let queue = FailoverQueueStore::new(database.clone())
        .and_then(|store| store.get(&scope))
        .map_err(|_| ReadError::Unavailable)?;
    let providers = AppProviderStore::new(database)
        .map_err(|_| ReadError::Unavailable)?
        .failover_providers(&scope.project, scope.app_id)
        .map_err(|_| ReadError::Unavailable)?;
    match resource {
        Resource::Queue => {
            let names: HashMap<_, _> = providers
                .iter()
                .map(|entry| (entry.provider.id.as_str(), entry.provider.name.as_str()))
                .collect();
            let entries = queue
                .members
                .into_iter()
                .map(|id| FailoverQueueEntry {
                    // Preserve saved positions so deleted profiles can still be removed or reordered.
                    name: names.get(id.as_str()).copied().unwrap_or(&id).to_owned(),
                    provider_id: id.into(),
                })
                .collect();
            Ok(Json(FailoverQueueResponse {
                scope,
                version: queue.version,
                enabled: queue.enabled,
                entries,
            })
            .into_response())
        }
        Resource::Candidates => {
            let candidates = Queue::new(scope.app_id, queue.members)
                .candidates(&providers)
                .map_err(|_| ReadError::UnsupportedMode)?
                .into_iter()
                .map(|provider| FailoverQueueEntry {
                    provider_id: provider.id.as_str().into(),
                    name: provider.name.clone(),
                })
                .collect();
            Ok(Json(FailoverCandidatesResponse { scope, candidates }).into_response())
        }
    }
}

enum Change {
    Queue(FailoverQueueVersion, QueueChange),
    Enabled(FailoverQueueEnabledRequest),
}

async fn member(
    control: Option<Extension<RoutingControl>>,
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    method: Method,
    query: Result<Query<FailoverQueueQuery>, QueryRejection>,
    body: Result<Json<FailoverQueueMemberRequest>, JsonRejection>,
) -> Result<Response, ReadError> {
    let Json(body) = body.map_err(|_| ReadError::InvalidInput)?;
    validate_identity(body.provider_id.as_ref()).map_err(|_| ReadError::InvalidInput)?;
    let change = if method == Method::POST {
        QueueChange::Add(body.provider_id.0)
    } else {
        QueueChange::Remove(body.provider_id.0)
    };
    write(control, tenant, database, headers, query, Change::Queue(body.expected_version, change))
        .await
}

async fn order(
    control: Option<Extension<RoutingControl>>,
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    query: Result<Query<FailoverQueueQuery>, QueryRejection>,
    body: Result<Json<FailoverQueueReorderRequest>, JsonRejection>,
) -> Result<Response, ReadError> {
    let Json(body) = body.map_err(|_| ReadError::InvalidInput)?;
    for id in &body.provider_ids {
        validate_identity(id.as_ref()).map_err(|_| ReadError::InvalidInput)?;
    }
    let change = QueueChange::Reorder(body.provider_ids.into_iter().map(|id| id.0).collect());
    write(control, tenant, database, headers, query, Change::Queue(body.expected_version, change))
        .await
}

async fn enabled(
    control: Option<Extension<RoutingControl>>,
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    query: Result<Query<FailoverQueueQuery>, QueryRejection>,
    body: Result<Json<FailoverQueueEnabledRequest>, JsonRejection>,
) -> Result<Response, ReadError> {
    let Json(body) = body.map_err(|_| ReadError::InvalidInput)?;
    write(control, tenant, database, headers, query, Change::Enabled(body)).await
}

async fn write(
    control: Option<Extension<RoutingControl>>,
    tenant: TenantScope,
    database: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    query: Result<Query<FailoverQueueQuery>, QueryRejection>,
    change: Change,
) -> Result<Response, ReadError> {
    let Query(query) = query.map_err(|_| ReadError::InvalidInput)?;
    validate_identity(&query.target_host_id).map_err(|_| ReadError::InvalidInput)?;
    let project = ProjectScope::new(tenant.clone(), query.project_id.clone())
        .map_err(|_| ReadError::InvalidInput)?;
    let (control, _) = RoutingControl::authorize(
        control,
        headers.clone(),
        tenant.clone(),
        query.target_host_id.clone(),
        Some((query.project_id, query.app_id)),
        true,
    )
    .await
    .map_err(|error| match error {
        ControlError::Unauthenticated => ReadError::Unauthenticated,
        ControlError::Forbidden => ReadError::Forbidden,
        ControlError::NotFound => ReadError::NotFound,
        _ => ReadError::Unavailable,
    })?;
    if !supports_app(query.app_id) {
        return Err(ReadError::UnsupportedMode);
    }
    let scope =
        FailoverQueueScope { project, target_host_id: query.target_host_id, app_id: query.app_id };
    tokio::task::spawn_blocking(move || {
        let (root, _) = control
            .authorize_mode(&headers, &scope.project, &scope.target_host_id, scope.app_id)
            .map_err(ReadError::from_status)?;
        let database = match database {
            Some(Extension(database)) => database,
            None => GlobalDatabase::for_home(&crate::global::paths().home)
                .map_err(|_| ReadError::Unavailable)?,
        };
        let providers = AppProviderStore::new(database.clone())
            .map_err(|_| ReadError::Unavailable)?
            .failover_providers(&scope.project, scope.app_id)
            .map_err(|_| ReadError::Unavailable)?;
        let result = match change {
            Change::Queue(version, change) => {
                service::update_queue(database.clone(), &scope, version, change)
            }
            Change::Enabled(body) => {
                let policy =
                    std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(std::path::PathBuf::from);
                let (home, provider_root) = super::super::agent_apps::provider_editor_context(
                    policy.as_deref(),
                    &headers,
                    &tenant,
                    &scope.target_host_id,
                    true,
                )
                .map_err(ReadError::from_status)?;
                if root != provider_root {
                    return Err(ReadError::Conflict);
                }
                service::set_enabled(
                    database.clone(),
                    &control.routing,
                    &home,
                    &root,
                    &scope,
                    body.expected_version,
                    body.enabled,
                )
            }
        };
        let state = result.map_err(|error| {
            if error.is::<FailoverQueueVersionConflict>() {
                ReadError::VersionConflict
            } else if let Some(error) = error.downcast_ref::<QueueMutationError>() {
                use vw_agent_apps::high_availability::queue::QueueError;
                match error.0 {
                    QueueError::ProviderNotFound => ReadError::NotFound,
                    QueueError::InvalidPermutation => ReadError::InvalidInput,
                    QueueError::IneligibleProvider => ReadError::IneligibleProvider,
                    QueueError::UnsupportedApp => ReadError::UnsupportedMode,
                }
            } else {
                ReadError::Conflict
            }
        })?;
        // Return the committed revision, even if a subsequent writer has already
        // changed the queue. Reading again would misattribute that writer's result.
        let entries = state
            .members
            .into_iter()
            .map(|id| {
                let name = providers
                    .iter()
                    .find(|entry| entry.provider.id == id)
                    .map(|entry| entry.provider.name.clone())
                    .unwrap_or_else(|| id.clone());
                FailoverQueueEntry { provider_id: id.into(), name }
            })
            .collect();
        Ok(Json(FailoverQueueResponse {
            scope,
            version: state.version,
            enabled: state.enabled,
            entries,
        })
        .into_response())
    })
    .await
    .map_err(|_| ReadError::Unavailable)?
}

impl ReadError {
    fn from_status(status: StatusCode) -> Self {
        match status {
            StatusCode::UNAUTHORIZED => Self::Unauthenticated,
            StatusCode::FORBIDDEN => Self::Forbidden,
            StatusCode::NOT_FOUND => Self::NotFound,
            _ => Self::Unavailable,
        }
    }
}

enum ReadError {
    InvalidInput,
    Unauthenticated,
    Forbidden,
    NotFound,
    UnsupportedMode,
    Unavailable,
    Conflict,
    VersionConflict,
    IneligibleProvider,
}

impl IntoResponse for ReadError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::IneligibleProvider => (
                StatusCode::CONFLICT,
                "ineligible_provider",
                "Provider is not eligible for failover",
            ),
            Self::Conflict => (
                StatusCode::CONFLICT,
                "conflict",
                "Failover configuration changed or mode is unavailable",
            ),
            Self::VersionConflict => {
                (StatusCode::CONFLICT, "version_conflict", "Failover queue version conflict")
            }
            Self::InvalidInput => {
                (StatusCode::BAD_REQUEST, "invalid_input", "Invalid failover request")
            }
            Self::Unauthenticated => {
                (StatusCode::UNAUTHORIZED, "unauthenticated", "Application membership is required")
            }
            Self::Forbidden => (StatusCode::FORBIDDEN, "forbidden", "Failover permission denied"),
            Self::NotFound => (StatusCode::NOT_FOUND, "not_found", "Failover resource not found"),
            Self::UnsupportedMode => {
                (StatusCode::CONFLICT, "unsupported_mode", "Application does not support failover")
            }
            Self::Unavailable => (
                StatusCode::SERVICE_UNAVAILABLE,
                "unavailable",
                "Failover configuration unavailable",
            ),
        };
        (
            status,
            Json(ApiErrorBody {
                error: ApiErrorDetail {
                    code: code.into(),
                    message: message.into(),
                    details: Some(json!({ "correlationId": uuid::Uuid::new_v4().to_string() })),
                },
            }),
        )
            .into_response()
    }
}
