//! Verified WebDAV receive pipeline; remote bytes never select local owners.
use std::path::Path;

use anyhow::{Result, ensure};
use vw_agent_apps::{
    data_management::{
        safety_snapshot::{OwnerPreimage, SafetySnapshotError, SnapshotTarget},
        shared_snapshot::{DATA_COMPAT_VERSION, SharedSnapshot},
    },
    device_sync::{
        archive_extract::extract_skills_archive,
        artifact::ArtifactDeclaration,
        conflict::{SyncBaseline, SyncConflict, SyncDecision, SyncPreview},
        manifest::{self, SyncArtifact, SyncManifest},
        webdav_config::WebDavConfig,
        webdav_transport::download::{DownloadedArtifact, WebDavDownloadTarget},
    },
    skills::{inventory::compute_dir_hash, update_apply::SkillUpdate},
};

use crate::{
    agent_apps::shared_snapshot::{SnapshotScope, SnapshotSelection},
    global_database::GlobalDatabase,
};
use super::{apply::{ApplyFailure, apply_snapshot}, projection::{AppliedSnapshot, ProjectionJob}};

/// Run inline under the authorized host's sync lock. The preview must be bound
/// to this destination/profile and the locally checked conflict decision.
/// `prepare_skills` stages updates using saved local identities, hashes and
/// bindings, without publishing. It is called only after both artifacts and the
/// entire archive have passed validation. `capture` and projections follow the
/// same owner contract as apply_snapshot. Never race cancellation against commit.
/// An inner failure retains its durable recovery receipt; no baseline is returned.
#[allow(clippy::too_many_arguments)]
pub async fn download_webdav_snapshot(
    database: &GlobalDatabase,
    sync_lock: &tokio::sync::OwnedMutexGuard<()>,
    scope: SnapshotScope<'_>,
    selection: &SnapshotSelection,
    connection: WebDavConfig,
    preview: &SyncPreview,
    recovery_root: &Path,
    targets: &[SnapshotTarget],
    prepare_skills: impl FnOnce(&Path, &SharedSnapshot) -> Result<Vec<SkillUpdate>>,
    capture: impl FnOnce() -> Result<Vec<OwnerPreimage>, SafetySnapshotError>,
    projections: Vec<ProjectionJob<'_>>,
) -> Result<Result<(AppliedSnapshot, SyncBaseline), ApplyFailure>> {
    ensure!(preview.decision == SyncDecision::Download, SyncConflict);
    let reader = WebDavDownloadTarget::new(connection, DATA_COMPAT_VERSION)?;
    let initial = read_manifest(&reader).await?;
    let manifest = SyncManifest::from_slice(&initial.bytes, DATA_COMPAT_VERSION)
        .map_err(anyhow::Error::msg)?;
    preview.recheck_remote(Some(&manifest.snapshot_id))?;
    let config = read_artifact(&reader, manifest::CONFIG_ARTIFACT, &manifest.artifacts.config).await?;
    let skills = read_artifact(&reader, manifest::SKILLS_ARTIFACT, &manifest.artifacts.skills).await?;
    recheck(&reader, &initial).await?;
    let incoming: SharedSnapshot = serde_json::from_slice(&config)
        .map_err(|_| anyhow::anyhow!("invalid shared snapshot schema"))?;
    let import = super::import::preview_shared_snapshot(database, scope, selection, &config)?;
    let staging = extract_skills_archive(&skills)?;
    // An archive cannot smuggle additional trees outside the configuration's
    // locally authorized selection. Hash each tree using the existing skill owner.
    let mut directories = std::collections::BTreeSet::new();
    for skill in &incoming.skills {
        vw_agent_apps::skills::local_install::validate_directory(&skill.directory)?;
        ensure!(directories.insert(skill.directory.as_str()), "duplicate shared skill");
        ensure!(compute_dir_hash(&staging.path().join(&skill.directory))? == skill.sha256,
            "shared skill content mismatch");
    }
    for entry in std::fs::read_dir(staging.path())? {
        let entry = entry?;
        ensure!(entry.file_type()?.is_dir()
            && entry.file_name().to_str().is_some_and(|name| directories.contains(name)),
            "archive contains undeclared skill content");
    }
    let updates = prepare_skills(staging.path(), &incoming)?;
    ensure!(updates.len() == incoming.skills.len(), "incomplete skill application");
    let mut prepared = std::collections::BTreeSet::new();
    for update in &updates {
        ensure!(prepared.insert(update.skill.directory.as_str())
            && incoming.skills.iter().any(|skill| skill.directory == update.skill.directory
                && skill.sha256 == update.content_hash), "skill application differs from download");
    }
    // Preparation can take time. Reject any manifest change before recovery or
    // publication; staged updates drop without touching the live configuration.
    recheck(&reader, &initial).await?;
    let applied = apply_snapshot(database, sync_lock, scope, import, updates,
        recovery_root, targets, capture, projections).await?;
    Ok(applied.map(|report| (report, SyncBaseline { snapshot_id: manifest.snapshot_id })))
}

async fn read_manifest(reader: &WebDavDownloadTarget) -> Result<DownloadedArtifact> {
    reader.download_manifest().await?.ok_or_else(|| SyncConflict.into())
}

async fn recheck(reader: &WebDavDownloadTarget, initial: &DownloadedArtifact) -> Result<()> {
    let current = read_manifest(reader).await?;
    ensure!(current.metadata.etag == initial.metadata.etag && current.bytes == initial.bytes, SyncConflict);
    Ok(())
}

async fn read_artifact(reader: &WebDavDownloadTarget, name: &str, artifact: &SyncArtifact) -> Result<Vec<u8>> {
    let download = reader.download(ArtifactDeclaration {
        name: name.into(), sha256: artifact.sha256.clone(), size: artifact.size,
    }).await?.ok_or(SyncConflict)?;
    Ok(download.bytes)
}
