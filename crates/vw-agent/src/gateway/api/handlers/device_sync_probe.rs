//! Connection probes and saved-connection previews on an authorized execution host.
use axum::{
    Extension, Json, Router,
    extract::{DefaultBodyLimit, Path, Query, rejection::JsonRejection},
    http::{HeaderMap, StatusCode}, routing::{get, post},
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
        .route("/agent-apps/device-sync/{transport}/remote", get(remote))
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

/// Read only the saved manifest. Authorization precedes lock, storage and secrets;
/// the shared lock keeps the saved identity stable throughout the network read.
async fn remote(
    Extension(policy): Extension<Policy>,
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    Path(transport): Path<DeviceSyncTransport>,
    Query(query): Query<ScopeQuery>,
) -> Result<Json<vw_api_types::device_sync_config::DeviceSyncRemoteResponse>, (StatusCode, &'static str)> {
    use vw_agent_apps::{
        data_management::shared_snapshot::DATA_COMPAT_VERSION,
        device_sync::webdav_remote::{RemoteCompatibility, RemoteSnapshotQuery},
    };
    use vw_api_types::device_sync_config::{DeviceSyncRemoteCompatibility, DeviceSyncRemoteResponse};
    let project = ProjectScope::new(tenant.clone(), query.project_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid_scope"))?;
    let host = query.target_host_id;
    let auth_host = host.clone();
    let root = tokio::task::spawn_blocking(move || {
        super::agent_apps::authorize_device_sync_probe(
            policy.0.as_deref(), &headers, &project, &auth_host)
    }).await.map_err(|_| (StatusCode::SERVICE_UNAVAILABLE, "authorization_unavailable"))?
        .map_err(|status| (status, "authorization_denied"))?;
    let lock_tenant = serde_json::to_string(&(
        tenant.platform_id().as_ref(), tenant.company_id().as_ref(),
    )).map_err(|_| (StatusCode::SERVICE_UNAVAILABLE, "scope_unavailable"))?;
    let cancellation = tokio_util::sync::CancellationToken::new();
    let _guard = crate::device_sync::operation_lock::acquire_sync_lock(
        &lock_tenant, &host, &cancellation,
    ).await.map_err(|_| (StatusCode::SERVICE_UNAVAILABLE, "lock_unavailable"))?;
    let connection = tokio::task::spawn_blocking(move || {
        let unavailable = || (StatusCode::SERVICE_UNAVAILABLE, "connection_unavailable");
        let database = match database {
            Some(Extension(database)) => database,
            None => GlobalDatabase::for_home(&crate::global::paths().home)
                .map_err(|_| unavailable())?,
        };
        match transport {
            DeviceSyncTransport::Webdav => {
                let store = WebDavSettingsStore::new(database, &root).map_err(|_| unavailable())?;
                let config = store.read(&tenant, &host).map_err(|_| unavailable())?
                    .ok_or((StatusCode::NOT_FOUND, "not_configured"))?;
                if !config.enabled { return Err((StatusCode::CONFLICT, "disabled")); }
                store.connection(&tenant, &host).map(Connection::Webdav).map_err(|_| unavailable())
            }
            DeviceSyncTransport::S3 => {
                let store = S3SettingsStore::new(database, &root).map_err(|_| unavailable())?;
                let config = store.read(&tenant, &host).map_err(|_| unavailable())?
                    .ok_or((StatusCode::NOT_FOUND, "not_configured"))?;
                if !config.enabled { return Err((StatusCode::CONFLICT, "disabled")); }
                store.connection(&tenant, &host).map(Connection::S3).map_err(|_| unavailable())
            }
        }
    }).await.map_err(|_| (StatusCode::SERVICE_UNAVAILABLE, "connection_unavailable"))??;
    let result = match connection {
        Connection::Webdav(config) => vw_agent_apps::device_sync::webdav_remote::query_remote_snapshot(
            config, DATA_COMPAT_VERSION,
        ).await.map_err(|_| (StatusCode::BAD_GATEWAY, "remote_unavailable"))?,
        Connection::S3(config) => vw_agent_apps::device_sync::s3_remote::query_remote_snapshot(
            config, DATA_COMPAT_VERSION,
        ).await.map_err(|_| (StatusCode::BAD_GATEWAY, "remote_unavailable"))?,
    };
    match result {
        RemoteSnapshotQuery::Empty => Ok(Json(DeviceSyncRemoteResponse::Empty)),
        RemoteSnapshotQuery::InvalidManifest { .. } => Err((StatusCode::BAD_GATEWAY, "invalid_manifest")),
        RemoteSnapshotQuery::Snapshot(snapshot) => Ok(Json(DeviceSyncRemoteResponse::Snapshot {
            device_name: snapshot.device_name,
            created_at: snapshot.created_at,
            snapshot_id: snapshot.snapshot_id,
            protocol_version: snapshot.protocol_version,
            data_compat_version: snapshot.data_compat_version,
            compatibility: match snapshot.compatibility {
                RemoteCompatibility::Compatible => DeviceSyncRemoteCompatibility::Compatible,
                RemoteCompatibility::UnsupportedProtocol => DeviceSyncRemoteCompatibility::UnsupportedProtocol,
                RemoteCompatibility::UnsupportedDataVersion => DeviceSyncRemoteCompatibility::UnsupportedDataVersion,
            },
        })),
    }
}
