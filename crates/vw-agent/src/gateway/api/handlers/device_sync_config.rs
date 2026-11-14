//! Host-local WebDAV and S3 configuration. Authentication middleware supplies the tenant;
//! membership policy supplies the principal and local execution-host authority.
use axum::{Extension, Json, Router, extract::{DefaultBodyLimit, Query},
    http::{HeaderMap, StatusCode}, routing::get};
use serde::Deserialize;
use vw_api_types::{ProjectId, ProjectScope, TenantScope,
    agent_apps::DataManagementTarget,
    device_sync_config::{DeviceSyncTransport, SyncSecretUpdate,
        UpdateWebDavSyncConfigRequest, WebDavSyncConfigResponse,
        UpdateS3SyncConfigRequest, S3SyncConfig, S3SyncConfigResponse}};
use crate::{device_sync::{status, webdav_settings::WebDavSettingsStore, s3_settings::S3SettingsStore},
    global_database::GlobalDatabase};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Context { project_id: ProjectId, target_host_id: String }

pub(super) fn router() -> Router {
    Router::new()
        .route("/agent-apps/device-sync/webdav/config", get(read).put(write).delete(clear))
        .route("/agent-apps/device-sync/s3/config", get(read_s3).put(write_s3))
        .layer(DefaultBodyLimit::max(64 * 1024))
        .layer(axum::middleware::map_response(async |mut response: axum::response::Response| {
            response.headers_mut().insert(axum::http::header::CACHE_CONTROL,
                axum::http::HeaderValue::from_static("no-store"));
            response
        }))
}

fn authorize(tenant: TenantScope, headers: &HeaderMap, query: Context)
    -> Result<DataManagementTarget, StatusCode> {
    let scope = ProjectScope::new(tenant, query.project_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let policy = std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(std::path::PathBuf::from);
    let principal_id = super::agent_apps::authorize_device_sync_config(
        policy.as_deref(), headers, &scope, &query.target_host_id)?;
    Ok(DataManagementTarget { scope, target_host_id: query.target_host_id, principal_id })
}
fn database(injected: Option<Extension<GlobalDatabase>>) -> Result<GlobalDatabase, StatusCode> {
    match injected {
        Some(Extension(database)) => Ok(database),
        None => GlobalDatabase::for_home(&crate::global::paths().home)
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE),
    }
}
fn response(database: GlobalDatabase, target: &DataManagementTarget,
    config: Option<vw_api_types::device_sync_config::WebDavSyncConfig>)
    -> Result<Json<WebDavSyncConfigResponse>, StatusCode> {
    let (last_status, active_task_id) = status::connection_status(database, target,
        DeviceSyncTransport::Webdav).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    Ok(Json(WebDavSyncConfigResponse { config, last_status, active_task_id }))
}
async fn read(Extension(tenant): Extension<TenantScope>,
    injected: Option<Extension<GlobalDatabase>>, headers: HeaderMap, Query(query): Query<Context>)
    -> Result<Json<WebDavSyncConfigResponse>, StatusCode> {
    let target = authorize(tenant, &headers, query)?;
    let database = database(injected)?;
    let store = WebDavSettingsStore::new(database.clone(), &crate::global::paths().home)
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let config = store.read(target.scope.tenant_scope(), &target.target_host_id)
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    response(database, &target, config)
}
async fn write(Extension(tenant): Extension<TenantScope>,
    injected: Option<Extension<GlobalDatabase>>, headers: HeaderMap, Query(query): Query<Context>,
    Json(request): Json<UpdateWebDavSyncConfigRequest>)
    -> Result<Json<WebDavSyncConfigResponse>, StatusCode> {
    let target = authorize(tenant, &headers, query)?;
    // Validate the entire draft before opening the settings/credential stores.
    vw_agent_apps::device_sync::webdav_config::WebDavConfig {
        enabled: request.enabled, auto_sync: request.auto_sync,
        base_url: request.base_url.clone(), username: request.username.clone(),
        password: String::new(), remote_root: request.remote_root.clone(), profile: request.profile.clone(),
    }.normalize().map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
    if matches!(&request.secret, SyncSecretUpdate::Replace { value } if value.is_empty()) {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let database = database(injected)?;
    let store = WebDavSettingsStore::new(database.clone(), &crate::global::paths().home)
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let config = store.save(target.scope.tenant_scope(), &target.target_host_id, request,
        &tokio_util::sync::CancellationToken::new()).await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    response(database, &target, Some(config))
}

fn s3_response(database: GlobalDatabase, target: &DataManagementTarget,
    config: Option<S3SyncConfig>) -> Result<Json<S3SyncConfigResponse>, StatusCode> {
    let (last_status, active_task_id) = status::connection_status(database, target,
        DeviceSyncTransport::S3).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    Ok(Json(S3SyncConfigResponse { config, last_status, active_task_id }))
}

async fn read_s3(Extension(tenant): Extension<TenantScope>,
    injected: Option<Extension<GlobalDatabase>>, headers: HeaderMap, Query(query): Query<Context>)
    -> Result<Json<S3SyncConfigResponse>, StatusCode> {
    let target = authorize(tenant, &headers, query)?;
    let database = database(injected)?;
    let store = S3SettingsStore::new(database.clone(), &crate::global::paths().home)
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let config = store.read(target.scope.tenant_scope(), &target.target_host_id)
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    s3_response(database, &target, config)
}

async fn write_s3(Extension(tenant): Extension<TenantScope>,
    injected: Option<Extension<GlobalDatabase>>, headers: HeaderMap, Query(query): Query<Context>,
    Json(request): Json<UpdateS3SyncConfigRequest>)
    -> Result<Json<S3SyncConfigResponse>, StatusCode> {
    let target = authorize(tenant, &headers, query)?;
    // Validate metadata without manufacturing or resolving a plaintext credential.
    let normalized = vw_agent_apps::device_sync::s3_config::S3Config {
        enabled: false, auto_sync: request.auto_sync,
        bucket: request.bucket.clone(), region: request.region.clone(),
        access_key_id: request.access_key_id.clone(), endpoint: request.endpoint.clone(),
        remote_root: request.remote_root.clone(), profile: request.profile.clone(),
        secret_access_key: String::new(),
    }.normalize().map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
    if matches!(&request.secret, SyncSecretUpdate::Replace { value } if value.is_empty()) {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    if request.enabled && (normalized.bucket.is_empty() || normalized.region.is_empty()
        || normalized.access_key_id.is_empty() || matches!(&request.secret, SyncSecretUpdate::Clear {})) {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let database = database(injected)?;
    let store = S3SettingsStore::new(database.clone(), &crate::global::paths().home)
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    if request.enabled && matches!(&request.secret, SyncSecretUpdate::Keep {}) {
        let previous = store.read(target.scope.tenant_scope(), &target.target_host_id)
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        let valid = previous.is_some_and(|old| old.secret_present
            && old.bucket == normalized.bucket && old.region == normalized.region
            && old.access_key_id == normalized.access_key_id && old.endpoint == normalized.endpoint
            && old.remote_root == normalized.remote_root && old.profile == normalized.profile);
        if !valid { return Err(StatusCode::UNPROCESSABLE_ENTITY); }
    }
    let config = store.save(target.scope.tenant_scope(), &target.target_host_id, request,
        &tokio_util::sync::CancellationToken::new()).await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    s3_response(database, &target, Some(config))
}

/// Clearing is idempotent and never touches remote objects or another transport.
async fn clear(Extension(tenant): Extension<TenantScope>,
    injected: Option<Extension<GlobalDatabase>>, headers: HeaderMap, Query(query): Query<Context>)
    -> Result<StatusCode, StatusCode> {
    let target = authorize(tenant, &headers, query)?;
    let store = WebDavSettingsStore::new(database(injected)?, &crate::global::paths().home)
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    store.clear(target.scope.tenant_scope(), &target.target_host_id,
        &tokio_util::sync::CancellationToken::new()).await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    Ok(StatusCode::NO_CONTENT)
}
