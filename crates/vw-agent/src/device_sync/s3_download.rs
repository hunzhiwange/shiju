//! Fixed-key S3 download into private staging, followed by the shared apply service.
use std::path::Path;

use anyhow::{Result, ensure};
use vw_agent_apps::{
    data_management::{
        safety_snapshot::{OwnerPreimage, SafetySnapshotError, SnapshotTarget},
        shared_snapshot::{DATA_COMPAT_VERSION, SharedSnapshot, SharedSkill},
    },
    device_sync::{
        archive_extract::extract_skills_archive,
        artifact::ArtifactDeclaration,
        conflict::{self, SyncBaseline, SyncDecision, SyncPreview},
        manifest::{self, SyncManifest},
        s3_config::S3Config,
        s3_transport::S3DownloadTarget,
    },
    skills::update_apply::SkillUpdate,
};

use crate::{
    agent_apps::shared_snapshot::{SnapshotScope, SnapshotSelection},
    global_database::GlobalDatabase,
};

use super::{apply::{ApplyFailure, apply_snapshot}, import::preview_shared_snapshot,
    projection::{AppliedSnapshot, ProjectionJob}};

pub struct DownloadApplied {
    pub applied: AppliedSnapshot,
    pub baseline: SyncBaseline,
}

/// Execute inline under the authorized tenant/host lock, without cancellation
/// racing commit. Connection, selection, targets, capture and projection jobs must
/// come from local authorization. `prepare_skills` must use the skill owner to
/// stage exactly the supplied skills against saved local hashes and bindings;
/// it must not publish. Remote paths never select local destinations.
///
/// Persist the returned baseline only in this connection/profile/version scope.
/// The inner error retains recovery material; projection warnings are committed
/// success. Neither failure path returns a baseline or modifies one in storage.
#[allow(clippy::too_many_arguments)]
pub async fn download_s3_snapshot(
    execution: &super::tasks::SyncTaskExecution,
    database: &GlobalDatabase,
    sync_lock: &tokio::sync::OwnedMutexGuard<()>,
    scope: SnapshotScope<'_>,
    selection: &SnapshotSelection,
    connection: S3Config,
    skills_root: &Path,
    baseline: Option<&SyncBaseline>,
    preview: &SyncPreview,
    recovery_root: &Path,
    targets: &[SnapshotTarget],
    prepare_skills: impl FnOnce(&Path, &[SharedSkill]) -> Result<Vec<SkillUpdate>>,
    capture: impl FnOnce() -> Result<Vec<OwnerPreimage>, SafetySnapshotError>,
    projections: Vec<ProjectionJob<'_>>,
) -> Result<Result<DownloadApplied, ApplyFailure>> {
    // Explicit download still requires a confirmed remote identity; conflict
    // resolution belongs to the preview owner rather than this transport path.
    ensure!(preview.decision == SyncDecision::Download, "download requires an approved download preview");
    let reader = S3DownloadTarget::new(connection, DATA_COMPAT_VERSION)?;
    let original = reader.download_manifest().await?
        .ok_or_else(|| anyhow::anyhow!("S3 synchronization manifest is missing"))?;
    let manifest = SyncManifest::from_slice(&original.bytes, DATA_COMPAT_VERSION)
        .map_err(anyhow::Error::msg)?;
    preview.recheck_remote(Some(&manifest.snapshot_id))?;
    let mut artifacts = Vec::new();
    for (name, artifact) in [
        (manifest::CONFIG_ARTIFACT, &manifest.artifacts.config),
        (manifest::SKILLS_ARTIFACT, &manifest.artifacts.skills),
    ] {
        let downloaded = reader.download(ArtifactDeclaration {
            name: name.to_owned(), sha256: artifact.sha256.clone(), size: artifact.size,
        }).await?.ok_or_else(|| anyhow::anyhow!("S3 synchronization artifact is missing"))?;
        artifacts.push(downloaded.bytes);
    }
    let local = super::export::export_shared_snapshot(database, scope, selection, skills_root)?;
    let local_id = manifest::SyncArtifacts {
        config: local.config_artifact, skills: local.skills.artifact,
    }.snapshot_id().map_err(anyhow::Error::msg)?;
    let actual_preview = conflict::preview(&local_id, baseline, Some(&manifest.snapshot_id))
        .map_err(anyhow::Error::msg)?;
    ensure!(actual_preview == *preview, conflict::SyncConflict);
    let config = &artifacts[0];
    let import = preview_shared_snapshot(database, scope, selection, config)?;
    let incoming: SharedSnapshot = serde_json::from_slice(config)
        .map_err(|_| anyhow::anyhow!("invalid shared snapshot schema"))?;
    let staging = extract_skills_archive(&artifacts[1])?;
    // No unlisted top-level tree may reach the resource owner.
    for entry in std::fs::read_dir(staging.path())? {
        let entry = entry?;
        ensure!(incoming.skills.iter().any(|skill| entry.file_name() == skill.directory.as_str()),
            "archive contains an undeclared skill");
    }
    for skill in &incoming.skills {
        vw_agent_apps::skills::local_install::validate_directory(&skill.directory)?;
        let hash = vw_agent_apps::skills::inventory::compute_dir_hash(&staging.path().join(&skill.directory))?;
        ensure!(hash == skill.sha256, "downloaded skill content differs from snapshot");
    }
    let skills = prepare_skills(staging.path(), &incoming.skills)?;
    ensure!(skills.len() == incoming.skills.len(), "incomplete skill staging");
    for skill in &incoming.skills {
        ensure!(skills.iter().filter(|update| update.skill.directory == skill.directory
            && update.content_hash == skill.sha256).count() == 1, "skill staging mismatch");
    }
    // Fixed keys have no distributed lock. Re-read immediately before local
    // publication, rejecting replacement even when a writer reused an identity.
    let current = reader.download_manifest().await?
        .ok_or_else(|| anyhow::anyhow!("S3 synchronization manifest disappeared"))?;
    ensure!(current.bytes == original.bytes && current.metadata.etag == original.metadata.etag,
        "S3 synchronization changed; refresh preview");
    execution.begin_commit().map_err(|_| anyhow::anyhow!("device sync cancelled before local publication"))?;
    let result = apply_snapshot(database, sync_lock, scope, import, skills,
        recovery_root, targets, capture, projections).await?;
    if result.as_ref().map_or_else(|failure| failure.rollback_failed, |_| true) {
        execution.mark_committed().map_err(|_| anyhow::anyhow!("device sync commit status unavailable"))?;
    }
    Ok(result.map(|applied| DownloadApplied {
        applied, baseline: SyncBaseline { snapshot_id: manifest.snapshot_id },
    }))
}
