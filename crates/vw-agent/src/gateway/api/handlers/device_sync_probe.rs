//! Draft-only connection probes on an explicitly authorized execution host.
use axum::{
    Extension, Json, Router,
    extract::{DefaultBodyLimit, Path, Query, rejection::JsonRejection},
    http::{HeaderMap, StatusCode}, routing::post,
};
use serde::Deserialize;
use std::path::PathBuf;
use vw_api_types::{
    device_sync_config::{DeviceSyncTransport, UpdateS3SyncConfigRequest, UpdateWebDavSyncConfigRequest},
    id::{ProjectId, ProjectScope, TenantScope},
};
use crate::{device_sync::{s3_settings::S3SettingsStore, webdav_settings::WebDavSettingsStore},
    global_database::GlobalDatabase};

#[derive(Clone)]
struct Policy(Option<PathBuf>);
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ScopeQuery {
    project_id: ProjectId,
    target_host_id: String,
}

pub(super) fn router() -> Router {
    Router::new()
        .route("/agent-apps/device-sync/{transport}/test", post(probe))
        .layer(Extension(Policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from))))
        .layer(DefaultBodyLimit::max(16 * 1024))
        .layer(axum::middleware::map_response(async |mut response: axum::response::Response| {
            response.headers_mut().insert(axum::http::header::CACHE_CONTROL,
                axum::http::HeaderValue::from_static("no-store"));
            response
        }))
}

enum Connection {
    Webdav(vw_agent_apps::device_sync::webdav_config::WebDavConfig),
    S3(vw_agent_apps::device_sync::s3_config::S3Config),
}

async fn probe(
    Extension(policy): Extension<Policy>,
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    Path(transport): Path<DeviceSyncTransport>,
    Query(query): Query<ScopeQuery>,
    body: Result<Json<serde_json::Value>, JsonRejection>,
) -> Result<StatusCode, StatusCode> {
    let project = ProjectScope::new(tenant.clone(), query.project_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let Json(body) = body.map_err(|_| StatusCode::BAD_REQUEST)?;
    let connection = tokio::task::spawn_blocking(move || {
        // Authorize before reading credentials or constructing any network client.
        let root = super::agent_apps::authorize_device_sync_probe(
            policy.0.as_deref(), &headers, &project, &query.target_host_id)?;
        let database = match database {
            Some(Extension(database)) => database,
            None => GlobalDatabase::for_home(&crate::global::paths().home)
                .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
        };
        match transport {
            DeviceSyncTransport::Webdav => {
                let draft: UpdateWebDavSyncConfigRequest = serde_json::from_value(body)
                    .map_err(|_| StatusCode::BAD_REQUEST)?;
                let store = WebDavSettingsStore::new(database, &root)
                    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
                store.resolve_probe(&tenant, &query.target_host_id, draft)
                    .map(Connection::Webdav).map_err(|_| StatusCode::BAD_REQUEST)
            }
            DeviceSyncTransport::S3 => {
                let draft: UpdateS3SyncConfigRequest = serde_json::from_value(body)
                    .map_err(|_| StatusCode::BAD_REQUEST)?;
                let store = S3SettingsStore::new(database, &root)
                    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
                store.resolve_probe(&tenant, &query.target_host_id, draft)
                    .map(Connection::S3).map_err(|_| StatusCode::BAD_REQUEST)
            }
        }
    }).await.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)??;
    match connection {
        Connection::Webdav(config) => vw_agent_apps::device_sync::webdav_transport::test_connection(config)
            .await.map_err(|_| StatusCode::BAD_GATEWAY)?,
        Connection::S3(config) => vw_agent_apps::device_sync::s3_transport::test_connection(config)
            .await.map_err(|_| StatusCode::BAD_GATEWAY)?,
    }
    Ok(StatusCode::NO_CONTENT)
}
