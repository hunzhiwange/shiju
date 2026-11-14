//! Explicit manual receive; request versions are optimistic preconditions.
use super::device_sync_jobs::Policy;
#[cfg(target_os = "macos")]
use super::device_sync_jobs::{configuration_revision, http_error};
use axum::{Extension, Json, extract::Path, http::{HeaderMap, StatusCode}};
use vw_api_types::{device_sync_config::DeviceSyncTransport, device_sync_task::*, id::TenantScope};
use crate::{device_sync::tasks, global_database::GlobalDatabase};

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct DownloadRequest {
    target: vw_api_types::agent_apps::DataManagementTarget,
    request_id: String,
    expected_remote_snapshot: SyncRemoteSnapshot,
    expected_local_snapshot: String,
    connection_revision: String,
    selection: crate::agent_apps::shared_snapshot::SnapshotSelection,
}

pub(super) async fn download(
    Extension(policy): Extension<Policy>, Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>, Path(transport): Path<String>,
    headers: HeaderMap, Json(request): Json<DownloadRequest>,
) -> Result<(StatusCode, Json<SyncTaskStatus>), StatusCode> {
    let transport = match transport.as_str() {
        "webdav" => DeviceSyncTransport::Webdav, "s3" => DeviceSyncTransport::S3,
        _ => return Err(StatusCode::BAD_REQUEST),
    };
    if request.target.scope.tenant_scope() != &tenant { return Err(StatusCode::FORBIDDEN); }
    let roots = super::agent_apps::authorize_device_sync_download(policy.0.as_deref(), &headers, &request.target)?;
    // Resource application requires a complete saved-binding recovery owner.
    // Never silently omit requested skills or publish unprotected filesystem data.
    if !request.selection.skills.is_empty() { return Err(StatusCode::NOT_IMPLEMENTED); }
    #[cfg(not(target_os = "macos"))]
    { let _ = (database, transport, roots); Err(StatusCode::NOT_IMPLEMENTED) }
    #[cfg(target_os = "macos")]
    {
        use sha2::{Digest, Sha256};
        let database = database.ok_or(StatusCode::SERVICE_UNAVAILABLE)?.0;
        let revision = configuration_revision(&database, &roots.0, &request.target, transport).map_err(http_error)?;
        if revision != serde_json::Value::String(request.connection_revision.clone()) { return Err(StatusCode::CONFLICT); }
        tasks::initialize(database.clone(), &request.target).map_err(http_error)?;
        let parameters = serde_json::to_vec(&(&request.selection, &revision, &request.expected_local_snapshot))
            .map_err(|_| StatusCode::BAD_REQUEST)?;
        let status = tasks::accept_upload(SyncStartRequest {
            target: request.target.clone(), transport, direction: SyncDirection::Download,
            request_id: request.request_id.clone(), expected_remote_snapshot: Some(SyncExpectedRemote::Snapshot {
                snapshot: request.expected_remote_snapshot.clone() }),
        }, hex::encode(Sha256::digest(parameters))).map_err(http_error)?;
        let id = status.task_id.clone();
        tokio::spawn(async move {
            let target = request.target.clone();
            let _ = tasks::execute(&target, &id, |execution| async move {
                run(database, policy, headers, request, transport, execution).await
            }).await;
        });
        Ok((StatusCode::ACCEPTED, Json(status)))
    }
}

#[cfg(target_os = "macos")]
async fn run(database: GlobalDatabase, policy: Policy, headers: HeaderMap, request: DownloadRequest,
    transport: DeviceSyncTransport, execution: tasks::SyncTaskExecution,
) -> Result<SyncRemoteSnapshot, SyncErrorCode> {
    use crate::device_sync::{operation_lock, webdav_settings::WebDavSettingsStore, s3_settings::S3SettingsStore};
    use vw_agent_apps::device_sync::{manifest::{SyncManifest, SyncArtifacts}, conflict::{SyncPreview, SyncDecision, SyncBaseline}};
    use sha2::{Digest, Sha256};
    let tenant = request.target.scope.tenant_scope();
    let key = serde_json::to_string(&(tenant.platform_id().as_ref(), tenant.company_id().as_ref())).map_err(|_| SyncErrorCode::InvalidRequest)?;
    let cancellation = tokio_util::sync::CancellationToken::new();
    let guard = operation_lock::acquire_sync_lock(&key, &request.target.target_host_id, &cancellation).await
        .map_err(|_| SyncErrorCode::SnapshotUnavailable)?;
    let (credentials, skills, recovery) = super::agent_apps::authorize_device_sync_download(
        policy.0.as_deref(), &headers, &request.target).map_err(|_| SyncErrorCode::Unauthorized)?;
    if configuration_revision(&database, &credentials, &request.target, transport)? != serde_json::Value::String(request.connection_revision.clone()) {
        return Err(SyncErrorCode::Conflict);
    }
    execution.checkpoint(SyncTaskPhase::InspectingRemote, 0, None)?;
    let scope = crate::agent_apps::shared_snapshot::SnapshotScope {
        project: &request.target.scope, principal: &request.target.principal_id, host: &request.target.target_host_id,
    };
    let local = crate::device_sync::export::export_shared_snapshot(&database, scope, &request.selection, &skills)
        .map_err(|_| SyncErrorCode::SnapshotUnavailable)?;
    let local_id = SyncArtifacts { config: local.config_artifact, skills: local.skills.artifact }.snapshot_id()
        .map_err(|_| SyncErrorCode::Integrity)?;
    if local_id != request.expected_local_snapshot { return Err(SyncErrorCode::Conflict); }
    let capture = crate::device_sync::import::preview_shared_snapshot(&database, scope, &request.selection, &local.config_json)
        .map_err(|_| SyncErrorCode::SnapshotUnavailable)?.recovery_preimages();
    let targets = capture.iter().map(|p| p.target.clone()).collect::<Vec<_>>();
    let preview = SyncPreview { decision: SyncDecision::Download,
        expected_remote_snapshot: Some(request.expected_remote_snapshot.snapshot_id.clone()) };
    let baseline = SyncBaseline { snapshot_id: local_id };
    // Saved metadata commits independently from native publication. Surface the
    // pending owner reconciliation rather than claiming native files refreshed.
    if !request.selection.providers.is_empty() || !request.selection.mcp.is_empty() || !request.selection.prompts.is_empty() {
        execution.warn(SyncTaskWarning::ProjectionFailed)?;
    }
    let version = vw_agent_apps::data_management::shared_snapshot::DATA_COMPAT_VERSION;
    macro_rules! receive {
        ($store:expr, $reader:ty, $connection:ident, $body:expr) => {{
            let $connection = $store.connection(tenant, &request.target.target_host_id).map_err(|_| SyncErrorCode::NotConfigured)?;
            let reader = <$reader>::new($connection.clone(), version).map_err(|_| SyncErrorCode::Transport)?;
            let observed = reader.download_manifest().await.map_err(|_| SyncErrorCode::Transport)?.ok_or(SyncErrorCode::RemoteEmpty)?;
            let manifest = SyncManifest::from_slice(&observed.bytes, version).map_err(|_| SyncErrorCode::Incompatible)?;
            let identity = SyncRemoteSnapshot { snapshot_id: manifest.snapshot_id,
                manifest_sha256: hex::encode(Sha256::digest(&observed.bytes)), etag: Some(observed.metadata.etag) };
            if identity != request.expected_remote_snapshot { return Err(SyncErrorCode::Conflict); }
            let applied = $body.map_err(|e| if e.downcast_ref::<vw_agent_apps::device_sync::conflict::SyncConflict>().is_some() {
                SyncErrorCode::Conflict
            } else { SyncErrorCode::ApplyFailed })?;
            let applied = applied.map_err(|e| if e.rollback_failed { SyncErrorCode::RollbackFailed } else { SyncErrorCode::ApplyFailed })?;
            if !applied.warnings.is_empty() { execution.warn(SyncTaskWarning::ProjectionFailed)?; }
            Ok(identity)
        }};
    }
    match transport {
        DeviceSyncTransport::Webdav => receive!(WebDavSettingsStore::new(database.clone(), &credentials).map_err(|_| SyncErrorCode::NotConfigured)?,
            vw_agent_apps::device_sync::webdav_transport::download::WebDavDownloadTarget, connection,
            crate::device_sync::webdav_download::download_webdav_snapshot(&execution, &database, &guard, scope, &request.selection,
                connection, &preview, &recovery, &targets,
                |_, incoming| { anyhow::ensure!(incoming.skills.is_empty(), "skill recovery owner required"); Ok(vec![]) },
                || Ok(capture), vec![]).await.map(|r| r.map(|(applied, _)| applied))),
        DeviceSyncTransport::S3 => receive!(S3SettingsStore::new(database.clone(), &credentials).map_err(|_| SyncErrorCode::NotConfigured)?,
            vw_agent_apps::device_sync::s3_transport::S3DownloadTarget, connection,
            crate::device_sync::s3_download::download_s3_snapshot(&execution, &database, &guard, scope, &request.selection,
                connection, &skills, Some(&baseline), &preview, &recovery, &targets,
                |_, incoming| { anyhow::ensure!(incoming.is_empty(), "skill recovery owner required"); Ok(vec![]) },
                || Ok(capture), vec![]).await.map(|r| r.map(|result| result.applied))),
    }
}
