//! Publish a portable capture through signed S3 requests, manifest last.
use std::path::Path;

use anyhow::{Result, ensure};
use vw_agent_apps::device_sync::{
    artifact::ArtifactDeclaration,
    conflict::{self, SyncBaseline, SyncConflict, SyncDecision, SyncPreview},
    manifest::{self, SyncArtifacts, SyncManifest},
    s3_config::S3Config,
    s3_transport::{UploadArtifact, S3UploadTarget, S3DownloadTarget},
};

use crate::{
    agent_apps::shared_snapshot::{SnapshotScope, SnapshotSelection},
    global_database::GlobalDatabase,
};
use super::export::export_shared_snapshot;

/// Caller runs this inline inside run_with_sync_lock, using freshly resolved
/// credentials and an authorized host/network/selection. Baseline and preview
/// must belong to that same destination/profile/version. Only the returned
/// baseline may be persisted, and only after this future succeeds.
/// Fixed remote keys remain best-effort: failure can leave partial artifacts.
/// A later baseline/status persistence failure must not undo or retry this upload;
/// report that failure separately while retaining the successful transfer fact.
#[allow(clippy::too_many_arguments)]
pub async fn upload_s3_snapshot(
    database: &GlobalDatabase,
    scope: SnapshotScope<'_>,
    selection: &SnapshotSelection,
    skills_root: &Path,
    connection: S3Config,
    baseline: Option<&SyncBaseline>,
    preview: &SyncPreview,
    device_name: &str,
) -> Result<SyncBaseline> {
    let snapshot = export_shared_snapshot(database, scope, selection, skills_root)?;
    let version = snapshot.data_compat_version;
    let artifacts = SyncArtifacts {
        config: snapshot.config_artifact,
        skills: snapshot.skills.artifact,
    };
    let snapshot_id = artifacts.snapshot_id().map_err(anyhow::Error::msg)?;
    let manifest = SyncManifest {
        format: manifest::FORMAT.to_owned(),
        protocol_version: manifest::PROTOCOL_VERSION,
        data_compat_version: version,
        device_name: device_name.to_owned(),
        created_at: chrono::Utc::now().to_rfc3339(),
        artifacts,
        snapshot_id: snapshot_id.clone(),
    }.normalize(version).map_err(anyhow::Error::msg)?;
    let manifest_bytes = serde_json::to_vec(&manifest)?;
    let reader = S3DownloadTarget::new(connection.clone(), version)?;
    let writer = S3UploadTarget::new(connection, version)?;
    let remote = read_manifest(&reader, version).await?;
    preview.recheck_remote(remote.as_ref().map(|value| value.snapshot_id.as_str()))?;
    let decision = conflict::preview(&snapshot_id, baseline,
        remote.as_ref().map(|value| value.snapshot_id.as_str()))
        .map_err(anyhow::Error::msg)?;
    // Do not accept a stale local preview or turn a download into a forced upload.
    ensure!(decision == *preview, SyncConflict);
    match decision.decision {
        SyncDecision::NoTransfer => {
            verify_artifacts(&reader, &manifest).await?;
            let actual = read_manifest(&reader, version).await?;
            conflict::verify_published_snapshot(&snapshot_id,
                actual.as_ref().map(|value| value.snapshot_id.as_str()))?;
            return Ok(SyncBaseline { snapshot_id });
        }
        SyncDecision::Upload => {}
        SyncDecision::Download | SyncDecision::Conflict => return Err(SyncConflict.into()),
    }
    // Recheck immediately before the first signed PUT; export and reads may take time.
    let current = read_manifest(&reader, version).await?;
    preview.recheck_remote(current.as_ref().map(|value| value.snapshot_id.as_str()))?;
    writer.upload(UploadArtifact::Config, snapshot.config_json).await?;
    writer.upload(UploadArtifact::Skills, snapshot.skills.bytes).await?;
    writer.upload(UploadArtifact::Manifest, manifest_bytes).await?;
    let published = read_manifest(&reader, version).await?;
    conflict::verify_published_snapshot(&snapshot_id,
        published.as_ref().map(|value| value.snapshot_id.as_str()))?;
    // Same manifest identity alone cannot detect concurrent fixed-key overwrites.
    verify_artifacts(&reader, &manifest).await?;
    let final_manifest = read_manifest(&reader, version).await?;
    conflict::verify_published_snapshot(&snapshot_id,
        final_manifest.as_ref().map(|value| value.snapshot_id.as_str()))?;
    Ok(SyncBaseline { snapshot_id })
}

async fn read_manifest(reader: &S3DownloadTarget, version: u32) -> Result<Option<SyncManifest>> {
    reader.download_manifest().await?.map(|download| {
        SyncManifest::from_slice(&download.bytes, version).map_err(|_| SyncConflict.into())
    }).transpose()
}

async fn verify_artifacts(reader: &S3DownloadTarget, manifest: &SyncManifest) -> Result<()> {
    for (name, artifact) in [
        (manifest::CONFIG_ARTIFACT, &manifest.artifacts.config),
        (manifest::SKILLS_ARTIFACT, &manifest.artifacts.skills),
    ] {
        let downloaded = reader.download(ArtifactDeclaration {
            name: name.to_owned(), sha256: artifact.sha256.clone(), size: artifact.size,
        }).await.map_err(|_| SyncConflict)?;
        ensure!(downloaded.is_some(), SyncConflict);
    }
    Ok(())
}
