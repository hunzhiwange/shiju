//! Explicit manual receive; request versions are optimistic preconditions.
use super::device_sync_jobs::Policy;
#[cfg(target_os = "macos")]
use super::device_sync_jobs::{configuration_revision, http_error};
use axum::{Extension, Json, extract::Path, http::{HeaderMap, StatusCode}};
use vw_api_types::{device_sync_config::DeviceSyncTransport, device_sync_task::*, id::TenantScope};
use crate::{device_sync::tasks, global_database::GlobalDatabase};

#[cfg(target_os = "macos")]
use super::app_routing::RoutingControl as DownloadRouting;
#[cfg(not(target_os = "macos"))]
#[derive(Clone)]
pub(super) struct DownloadRouting;

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
    database: Option<Extension<GlobalDatabase>>, control: Option<Extension<DownloadRouting>>, Path(transport): Path<String>,
    headers: HeaderMap, Json(request): Json<DownloadRequest>,
) -> Result<(StatusCode, Json<SyncTaskStatus>), StatusCode> {
    let transport = match transport.as_str() {
        "webdav" => DeviceSyncTransport::Webdav, "s3" => DeviceSyncTransport::S3,
        _ => return Err(StatusCode::BAD_REQUEST),
    };
    if request.target.scope.tenant_scope() != &tenant { return Err(StatusCode::FORBIDDEN); }
    let roots = super::agent_apps::authorize_device_sync_download(policy.0.as_deref(), &headers, &request.target)?;
    #[cfg(not(target_os = "macos"))]
    { let _ = (database, transport, roots, control); Err(StatusCode::NOT_IMPLEMENTED) }
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
                run(database, policy, headers, request, transport, control.map(|Extension(c)| c), execution).await
            }).await;
        });
        Ok((StatusCode::ACCEPTED, Json(status)))
    }
}

#[cfg(target_os = "macos")]
async fn run(database: GlobalDatabase, policy: Policy, headers: HeaderMap, request: DownloadRequest,
    transport: DeviceSyncTransport, control: Option<DownloadRouting>, execution: tasks::SyncTaskExecution,
) -> Result<SyncRemoteSnapshot, SyncErrorCode> {
    use crate::device_sync::{operation_lock, webdav_settings::WebDavSettingsStore, s3_settings::S3SettingsStore};
    use vw_agent_apps::device_sync::{manifest::{SyncManifest, SyncArtifacts}};
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
    let local_config: vw_agent_apps::data_management::shared_snapshot::SharedSnapshot = serde_json::from_slice(&local.config_json)
        .map_err(|_| SyncErrorCode::Integrity)?;
    let version = vw_agent_apps::data_management::shared_snapshot::DATA_COMPAT_VERSION;
    macro_rules! receive {
        ($store:expr, $reader:ty) => {{
            use vw_agent_apps::device_sync::{artifact::ArtifactDeclaration, manifest};
            let connection = $store.connection(tenant, &request.target.target_host_id).map_err(|_| SyncErrorCode::NotConfigured)?;
            let reader = <$reader>::new(connection, version).map_err(|_| SyncErrorCode::Transport)?;
            let observed = reader.download_manifest().await.map_err(|_| SyncErrorCode::Transport)?.ok_or(SyncErrorCode::RemoteEmpty)?;
            let manifest = SyncManifest::from_slice(&observed.bytes, version).map_err(|_| SyncErrorCode::Incompatible)?;
            let identity = SyncRemoteSnapshot { snapshot_id: manifest.snapshot_id.clone(),
                manifest_sha256: hex::encode(Sha256::digest(&observed.bytes)), etag: Some(observed.metadata.etag.clone()) };
            if identity != request.expected_remote_snapshot { return Err(SyncErrorCode::Conflict); }
            execution.checkpoint(SyncTaskPhase::Transferring, 0, None)?;
            let mut artifacts = Vec::new();
            for (name, artifact) in [(manifest::CONFIG_ARTIFACT, &manifest.artifacts.config), (manifest::SKILLS_ARTIFACT, &manifest.artifacts.skills)] {
                let downloaded = reader.download(ArtifactDeclaration { name: name.into(), sha256: artifact.sha256.clone(), size: artifact.size })
                    .await.map_err(|_| SyncErrorCode::Transport)?.ok_or(SyncErrorCode::Integrity)?;
                artifacts.push(downloaded.bytes);
            }
            execution.checkpoint(SyncTaskPhase::Verifying, 0, None)?;
            let incoming: vw_agent_apps::data_management::shared_snapshot::SharedSnapshot = serde_json::from_slice(&artifacts[0])
                .map_err(|_| SyncErrorCode::Integrity)?;
            let mut import = crate::device_sync::import::preview_shared_snapshot(&database, scope, &request.selection, &artifacts[0])
                .map_err(|_| SyncErrorCode::InvalidRequest)?;
            let staging = vw_agent_apps::device_sync::archive_extract::extract_skills_archive(&artifacts[1]).map_err(|_| SyncErrorCode::UnsafeArchive)?;
            let mut declared = std::collections::BTreeSet::new();
            for skill in &incoming.skills {
                vw_agent_apps::skills::local_install::validate_directory(&skill.directory).map_err(|_| SyncErrorCode::UnsafeArchive)?;
                if !declared.insert(skill.directory.clone()) || vw_agent_apps::skills::inventory::compute_dir_hash(&staging.path().join(&skill.directory))
                    .map_err(|_| SyncErrorCode::Integrity)? != skill.sha256 { return Err(SyncErrorCode::Integrity); }
            }
            for entry in std::fs::read_dir(staging.path()).map_err(|_| SyncErrorCode::UnsafeArchive)? {
                let entry = entry.map_err(|_| SyncErrorCode::UnsafeArchive)?;
                if !entry.file_type().map_err(|_| SyncErrorCode::UnsafeArchive)?.is_dir()
                    || !entry.file_name().to_str().is_some_and(|name| declared.contains(name)) { return Err(SyncErrorCode::UnsafeArchive); }
            }
            let mut resource_capture = None;
            let updates = if incoming.skills.is_empty() { vec![] } else {
                let prepared = crate::device_sync::skill_receive::prepare_sync_receive(&database, &request.target.scope, &request.target.target_host_id, &skills,
                    staging.path(), &incoming.skills, &local_config.skills).map_err(|_| SyncErrorCode::Conflict)?;
                import.attach_skill_state(prepared.key, prepared.before, prepared.after);
                resource_capture = Some(prepared.resources);
                prepared.updates
            };
            let mut capture = import.recovery_preimages();
            capture.extend(resource_capture);
            let targets = capture.iter().map(|p| p.target.clone()).collect::<Vec<_>>();
            // Recheck the complete preview revision after staging and immediately
            // before the recovery/commit boundary. Never infer CAS from content ID.
            let current = reader.download_manifest().await.map_err(|_| SyncErrorCode::Transport)?.ok_or(SyncErrorCode::Conflict)?;
            if current.bytes != observed.bytes || current.metadata.etag != observed.metadata.etag { return Err(SyncErrorCode::Conflict); }
            let fresh = super::agent_apps::authorize_device_sync_download(policy.0.as_deref(), &headers, &request.target)
                .map_err(|_| SyncErrorCode::Unauthorized)?;
            if fresh != (credentials.clone(), skills.clone(), recovery.clone()) || configuration_revision(&database, &credentials,
                &request.target, transport)? != serde_json::Value::String(request.connection_revision.clone()) { return Err(SyncErrorCode::Conflict); }
            for ((namespace, key), expected) in &local.source_versions {
                let current = crate::gateway::settings_store::GatewaySettingsStore::read_existing(&database, tenant, namespace, key)
                    .map_err(|_| SyncErrorCode::SnapshotUnavailable)?;
                let digest: Option<[u8; 32]> = current.as_ref().map(|raw| Sha256::digest(raw.as_bytes()).into());
                if digest != *expected { return Err(SyncErrorCode::Conflict); }
            }
            execution.begin_commit()?;
            let applied = crate::device_sync::apply::apply_snapshot(&database, &guard, scope, import, updates,
                &recovery, &targets, || Ok(capture), vec![]).await.map_err(|_| SyncErrorCode::SnapshotUnavailable)?;
            let applied = match applied {
                Ok(applied) => { execution.mark_committed()?; applied }
                Err(failure) => {
                    if failure.rollback_failed { execution.mark_committed()?; return Err(SyncErrorCode::RollbackFailed); }
                    return Err(SyncErrorCode::ApplyFailed);
                }
            };
            let _ = applied;
            if super::device_sync_projection::refresh(&database, policy.0.as_deref(), &headers,
                &request.target, &request.selection, control.as_ref()).await.is_err() { execution.warn(SyncTaskWarning::ProjectionFailed)?; }
            Ok(identity)
        }};
    }
    match transport {
        DeviceSyncTransport::Webdav => receive!(WebDavSettingsStore::new(database.clone(), &credentials).map_err(|_| SyncErrorCode::NotConfigured)?,
            vw_agent_apps::device_sync::webdav_transport::download::WebDavDownloadTarget),
        DeviceSyncTransport::S3 => receive!(S3SettingsStore::new(database.clone(), &credentials).map_err(|_| SyncErrorCode::NotConfigured)?,
            vw_agent_apps::device_sync::s3_transport::S3DownloadTarget),
    }
}
