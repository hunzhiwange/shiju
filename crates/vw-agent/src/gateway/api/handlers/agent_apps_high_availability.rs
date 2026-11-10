//! Authorized reads of application failover configuration.
use super::lifecycle_api::{ControlError, RoutingControl};
use crate::{
    app_providers::AppProviderStore,
    gateway::agent_apps_high_availability_store::FailoverQueueStore,
    global_database::GlobalDatabase,
};
use axum::{
    Extension, Json, Router,
    extract::{Query, rejection::QueryRejection},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    middleware,
    response::{IntoResponse, Response},
    routing::get,
};
use serde_json::json;
use std::collections::HashMap;
use vw_agent_apps::{
    high_availability::queue::Queue, providers::validate_identity, routing::supports_app,
};
use vw_api_types::{
    agent_apps_high_availability::{
        FailoverCandidatesResponse, FailoverQueueEntry, FailoverQueueQuery, FailoverQueueResponse,
        FailoverQueueScope,
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

enum ReadError {
    InvalidInput,
    Unauthenticated,
    Forbidden,
    NotFound,
    UnsupportedMode,
    Unavailable,
}

impl IntoResponse for ReadError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::InvalidInput => {
                (StatusCode::BAD_REQUEST, "invalid_input", "Invalid failover scope")
            }
            Self::Unauthenticated => {
                (StatusCode::UNAUTHORIZED, "unauthenticated", "Application membership is required")
            }
            Self::Forbidden => {
                (StatusCode::FORBIDDEN, "forbidden", "Failover read permission denied")
            }
            Self::NotFound => (StatusCode::NOT_FOUND, "not_found", "Execution host not found"),
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
