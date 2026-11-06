//! Tenant icon library API.
use super::agent_apps_error::AgentAppsApiError;
use crate::{agent_apps::store::IconLibraryStore, global_database::GlobalDatabase};
use axum::{
    Extension, Json, Router,
    extract::{DefaultBodyLimit, Path, rejection::JsonRejection},
    routing::{get, patch},
};
use vw_api_types::{
    agent_apps::{
        AddCustomIconRequest, AddCustomIconResult, AgentAppsErrorCode, CustomIconEntry,
        RenameCustomIconRequest,
    },
    id::TenantScope,
};

pub(crate) fn router() -> Router {
    Router::new()
        .merge(read::router())
        .merge(lifecycle::router())
        .route(
            "/agent-apps/custom-icons",
            get(read_icons).post(add_icon).layer(DefaultBodyLimit::max(128 * 1024)),
        )
        .route(
            "/agent-apps/custom-icons/{id}",
            patch(rename_icon).delete(delete_icon).layer(DefaultBodyLimit::max(128 * 1024)),
        )
}

async fn read_icons(
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
) -> Result<Json<Vec<CustomIconEntry>>, AgentAppsApiError> {
    tokio::task::spawn_blocking(move || {
        let database = match database {
            Some(Extension(database)) => database,
            None => GlobalDatabase::for_home(&crate::global::paths().home)
                .map_err(AgentAppsApiError::storage_unavailable)?,
        };
        let store =
            IconLibraryStore::new(database).map_err(AgentAppsApiError::storage_unavailable)?;
        store.read_icons(&tenant).map(Json).map_err(AgentAppsApiError::storage_unavailable)
    })
    .await
    .map_err(|error| AgentAppsApiError::storage_unavailable(error.into()))?
}

async fn add_icon(
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    request: Result<Json<AddCustomIconRequest>, JsonRejection>,
) -> Result<Json<AddCustomIconResult>, axum::response::Response> {
    use axum::response::IntoResponse;
    let Json(request) = request.map_err(|error| {
        if error.status() == axum::http::StatusCode::PAYLOAD_TOO_LARGE {
            error.into_response()
        } else {
            AgentAppsApiError::from(AgentAppsErrorCode::InvalidInput).into_response()
        }
    })?;
    tokio::task::spawn_blocking(move || -> Result<_, AgentAppsApiError> {
        let database = match database {
            Some(Extension(database)) => database,
            None => GlobalDatabase::for_home(&crate::global::paths().home)
                .map_err(AgentAppsApiError::storage_unavailable)?,
        };
        let store =
            IconLibraryStore::new(database).map_err(AgentAppsApiError::storage_unavailable)?;
        store
            .add_icon(
                &tenant,
                &request.name,
                &request.data,
                &uuid::Uuid::new_v4().to_string(),
                chrono::Utc::now().timestamp_millis(),
            )
            .map(Json)
            .map_err(AgentAppsApiError::from)
    })
    .await
    .map_err(|error| AgentAppsApiError::storage_unavailable(error.into()).into_response())?
    .map_err(IntoResponse::into_response)
}

async fn rename_icon(
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    Path(id): Path<String>,
    request: Result<Json<RenameCustomIconRequest>, JsonRejection>,
) -> Result<Json<CustomIconEntry>, axum::response::Response> {
    use axum::response::IntoResponse;
    let Json(request) = request.map_err(|error| {
        if error.status() == axum::http::StatusCode::PAYLOAD_TOO_LARGE {
            error.into_response()
        } else {
            AgentAppsApiError::from(AgentAppsErrorCode::InvalidInput).into_response()
        }
    })?;
    tokio::task::spawn_blocking(move || -> Result<_, AgentAppsApiError> {
        let database = match database {
            Some(Extension(database)) => database,
            None => GlobalDatabase::for_home(&crate::global::paths().home)
                .map_err(AgentAppsApiError::storage_unavailable)?,
        };
        let store =
            IconLibraryStore::new(database).map_err(AgentAppsApiError::storage_unavailable)?;
        store.rename_icon(&tenant, &id, &request.name).map(Json).map_err(AgentAppsApiError::from)
    })
    .await
    .map_err(|error| AgentAppsApiError::storage_unavailable(error.into()).into_response())?
    .map_err(IntoResponse::into_response)
}

async fn delete_icon(
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    Path(id): Path<String>,
) -> Result<axum::http::StatusCode, AgentAppsApiError> {
    tokio::task::spawn_blocking(move || {
        let database = match database {
            Some(Extension(database)) => database,
            None => GlobalDatabase::for_home(&crate::global::paths().home)
                .map_err(AgentAppsApiError::storage_unavailable)?,
        };
        let store =
            IconLibraryStore::new(database).map_err(AgentAppsApiError::storage_unavailable)?;
        store.delete_icon(&tenant, &id).map_err(AgentAppsApiError::from)?;
        Ok(axum::http::StatusCode::NO_CONTENT)
    })
    .await
    .map_err(|error| AgentAppsApiError::storage_unavailable(error.into()))?
}

mod read;
#[cfg(all(test, unix))]
mod read_tests;

mod access;

mod lifecycle;
#[cfg(all(test, unix))]
mod lifecycle_tests;
