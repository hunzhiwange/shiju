//! Manual portable exports. Authentication is repeated after the shared queue lock.
use axum::{Extension, Json, Router, extract::{Path, DefaultBodyLimit}, http::{HeaderMap, StatusCode}, routing::post};
use std::path::PathBuf;
use vw_api_types::{id::TenantScope, device_sync_task::*};
use crate::{global_database::GlobalDatabase, device_sync::tasks};

#[derive(Clone)]
pub(super) struct Policy(pub(super) Option<PathBuf>);
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct UploadRequest {
    target: vw_api_types::agent_apps::DataManagementTarget,
    request_id: String,
    expected_remote_snapshot: SyncExpectedRemote,
    selection: crate::agent_apps::shared_snapshot::SnapshotSelection,
}

pub(crate) fn router() -> Router {
    Router::new()
        .route("/agent-apps/device-sync/{transport}/upload", post(upload))
        .route("/agent-apps/device-sync/{transport}/download", post(super::device_sync_download::download))
        .layer(Extension(Policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from))))
        .layer(DefaultBodyLimit::max(64 * 1024))
        .layer(axum::middleware::map_response(async |mut response: axum::response::Response| {
            response.headers_mut().insert(axum::http::header::CACHE_CONTROL,
                axum::http::HeaderValue::from_static("no-store"));
            response
        }))
}

async fn upload(
    Extension(policy): Extension<Policy>, Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>, Path(transport): Path<String>,
    headers: HeaderMap, Json(request): Json<UploadRequest>,
) -> Result<(StatusCode, Json<SyncTaskStatus>), StatusCode> {
    use vw_api_types::device_sync_config::DeviceSyncTransport;
    let transport = match transport.as_str() {
        "webdav" => DeviceSyncTransport::Webdav,
        "s3" => DeviceSyncTransport::S3,
        _ => return Err(StatusCode::BAD_REQUEST),
    };
    if request.target.scope.tenant_scope() != &tenant { return Err(StatusCode::FORBIDDEN); }
    let roots = super::agent_apps::authorize_device_sync_upload(policy.0.as_deref(), &headers, &request.target)?;
    #[cfg(not(target_os = "macos"))]
    { let _ = (database, transport); Err(StatusCode::NOT_IMPLEMENTED) }
    #[cfg(target_os = "macos")]
    {
        let database = database.ok_or(StatusCode::SERVICE_UNAVAILABLE)?.0;
        tasks::initialize(database.clone(), &request.target).map_err(http_error)?;
        // Bind the explicit object selection into durable idempotency parameters.
        // The durable record rejects reuse with changed selection or connection.
        let start = SyncStartRequest {
            target: request.target.clone(), transport, direction: SyncDirection::Upload,
            request_id: request.request_id.clone(),
            expected_remote_snapshot: Some(request.expected_remote_snapshot.clone()),
        };
        let revision = configuration_revision(&database, &roots.0, &request.target, transport)
            .map_err(http_error)?;
        use sha2::{Digest, Sha256};
        let parameters = serde_json::to_vec(&(&request.selection, &revision))
            .map_err(|_| StatusCode::BAD_REQUEST)?;
        let status = tasks::accept_upload(start, hex::encode(Sha256::digest(parameters))).map_err(http_error)?;
        let id = status.task_id.clone();
        tokio::spawn(async move {
            let target = request.target.clone();
            let _ = tasks::execute(&target, &id, |execution| async move {
                run(database, policy, headers, request, transport, revision, execution).await
            }).await;
        });
        Ok((StatusCode::ACCEPTED, Json(status)))
    }
}
pub(super) fn http_error(code: SyncErrorCode) -> StatusCode {
    match code {
        SyncErrorCode::InvalidRequest => StatusCode::BAD_REQUEST,
        SyncErrorCode::IdempotencyConflict | SyncErrorCode::Conflict => StatusCode::CONFLICT,
        _ => StatusCode::SERVICE_UNAVAILABLE,
    }
}

#[cfg(target_os = "macos")]
async fn run(database: GlobalDatabase, policy: Policy, headers: HeaderMap, request: UploadRequest,
    transport: vw_api_types::device_sync_config::DeviceSyncTransport,
    revision: serde_json::Value, execution: tasks::SyncTaskExecution,
) -> Result<SyncRemoteSnapshot, SyncErrorCode> {
    use crate::device_sync::{operation_lock, webdav_settings::WebDavSettingsStore, s3_settings::S3SettingsStore};
    use vw_agent_apps::device_sync::{conflict::{SyncBaseline, SyncPreview, SyncDecision, SyncConflict}, manifest::SyncManifest};
    use sha2::{Digest, Sha256};
    let tenant = request.target.scope.tenant_scope();
    let key = serde_json::to_string(&(tenant.platform_id().as_ref(), tenant.company_id().as_ref()))
        .map_err(|_| SyncErrorCode::InvalidRequest)?;
    let cancellation = tokio_util::sync::CancellationToken::new();
    let _guard = operation_lock::acquire_sync_lock(&key, &request.target.target_host_id, &cancellation)
        .await.map_err(|_| SyncErrorCode::SnapshotUnavailable)?;
    let (credentials, skills) = super::agent_apps::authorize_device_sync_upload(
        policy.0.as_deref(), &headers, &request.target).map_err(|_| SyncErrorCode::Unauthorized)?;
    if configuration_revision(&database, &credentials, &request.target, transport)? != revision {
        return Err(SyncErrorCode::Conflict);
    }
    execution.checkpoint(SyncTaskPhase::InspectingRemote, 0, None)?;
    let scope = crate::agent_apps::shared_snapshot::SnapshotScope {
        project: &request.target.scope, principal: &request.target.principal_id, host: &request.target.target_host_id,
    };
    let version = vw_agent_apps::data_management::shared_snapshot::DATA_COMPAT_VERSION;
    // Keep both connection material and all transfer futures inside the lock.
    macro_rules! transfer {
        ($store:expr, $reader:ty, $upload:path) => {{
            let connection = $store.connection(tenant, &request.target.target_host_id)
                .map_err(|_| SyncErrorCode::NotConfigured)?;
            let reader = <$reader>::new(connection.clone(), version).map_err(|_| SyncErrorCode::Transport)?;
            let observed = reader.download_manifest().await.map_err(|_| SyncErrorCode::Transport)?;
            let identity = |bytes: &[u8], etag: &str| -> Result<SyncRemoteSnapshot, SyncErrorCode> {
                let manifest = SyncManifest::from_slice(bytes, version).map_err(|_| SyncErrorCode::Incompatible)?;
                Ok(SyncRemoteSnapshot { snapshot_id: manifest.snapshot_id,
                    manifest_sha256: hex::encode(Sha256::digest(bytes)), etag: Some(etag.to_owned()) })
            };
            let actual = observed.as_ref().map(|d| identity(&d.bytes, &d.metadata.etag)).transpose()?;
            let expected = match &request.expected_remote_snapshot {
                SyncExpectedRemote::Empty => None,
                SyncExpectedRemote::Snapshot { snapshot } => Some(snapshot),
            };
            if actual.as_ref() != expected { return Err(SyncErrorCode::Conflict); }
            let baseline = expected.map(|s| SyncBaseline { snapshot_id: s.snapshot_id.clone() });
            let local = crate::device_sync::export::export_shared_snapshot(&database, scope, &request.selection, &skills)
                .map_err(|_| SyncErrorCode::SnapshotUnavailable)?;
            let local_id = vw_agent_apps::device_sync::manifest::SyncArtifacts {
                config: local.config_artifact, skills: local.skills.artifact,
            }.snapshot_id().map_err(|_| SyncErrorCode::Integrity)?;
            let preview = SyncPreview {
                decision: if expected.is_some_and(|s| s.snapshot_id == local_id) { SyncDecision::NoTransfer } else { SyncDecision::Upload },
                expected_remote_snapshot: expected.map(|s| s.snapshot_id.clone()),
            };
            execution.checkpoint(SyncTaskPhase::Exporting, 0, None)?;
            let uploaded = $upload(&execution, &database, scope, &request.selection, &skills, connection,
                baseline.as_ref(), &preview, &request.target.target_host_id).await
                .map_err(|e| if e.downcast_ref::<SyncConflict>().is_some() { SyncErrorCode::Conflict } else { SyncErrorCode::Transport })?;
            let published = reader.download_manifest().await.map_err(|_| SyncErrorCode::Transport)?
                .ok_or(SyncErrorCode::Integrity)?;
            let result = identity(&published.bytes, &published.metadata.etag)?;
            if result.snapshot_id != uploaded.snapshot_id { return Err(SyncErrorCode::Conflict); }
            Ok(result)
        }};
    }
    match transport {
        vw_api_types::device_sync_config::DeviceSyncTransport::Webdav => transfer!(
            WebDavSettingsStore::new(database.clone(), &credentials).map_err(|_| SyncErrorCode::NotConfigured)?,
            vw_agent_apps::device_sync::webdav_transport::download::WebDavDownloadTarget,
            crate::device_sync::webdav_upload::upload_webdav_snapshot),
        vw_api_types::device_sync_config::DeviceSyncTransport::S3 => transfer!(
            S3SettingsStore::new(database.clone(), &credentials).map_err(|_| SyncErrorCode::NotConfigured)?,
            vw_agent_apps::device_sync::s3_transport::S3DownloadTarget,
            crate::device_sync::s3_upload::upload_s3_snapshot),
    }
}

#[cfg(target_os = "macos")]
pub(super) fn configuration_revision(database: &GlobalDatabase, root: &std::path::Path,
    target: &vw_api_types::agent_apps::DataManagementTarget,
    transport: vw_api_types::device_sync_config::DeviceSyncTransport,
) -> Result<serde_json::Value, SyncErrorCode> {
    use crate::device_sync::{webdav_settings::WebDavSettingsStore, s3_settings::S3SettingsStore};
    let tenant = target.scope.tenant_scope();
    let result = match transport {
        vw_api_types::device_sync_config::DeviceSyncTransport::Webdav =>
            WebDavSettingsStore::new(database.clone(), root).and_then(|s| s.revision(tenant, &target.target_host_id)),
        vw_api_types::device_sync_config::DeviceSyncTransport::S3 =>
            S3SettingsStore::new(database.clone(), root).and_then(|s| s.revision(tenant, &target.target_host_id)),
    };
    result.map(serde_json::Value::String).map_err(|_| SyncErrorCode::NotConfigured)
}
