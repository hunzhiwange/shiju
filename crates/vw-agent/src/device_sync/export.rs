//! Device synchronization consumes the data-management owner's portable capture.
use anyhow::{Result, ensure};
use sha2::{Digest, Sha256};
use crate::{agent_apps::shared_snapshot::{self, SnapshotScope, SnapshotSelection}, global_database::GlobalDatabase};
use vw_agent_apps::device_sync::{archive_pack::PackedSkillsArchive, manifest::SyncArtifact};

/// Only these exact immutable bytes may be described in a manifest and uploaded.
/// Source versions stay local; they are not part of either portable artifact.
pub struct ExportedSnapshot {
    pub data_compat_version: u32,
    pub config_json: Vec<u8>,
    pub config_artifact: SyncArtifact,
    pub skills: PackedSkillsArchive,
    pub source_versions: std::collections::BTreeMap<(String, String), Option<[u8; 32]>>,
}

/// Call on the authorized executor under run_with_sync_lock. The selection must
/// already authorize all domains, free text and resource contents for export.
/// This entry never initializes storage, activates configurations or reads secrets.
pub fn export_shared_snapshot(
    database: &GlobalDatabase, scope: SnapshotScope<'_>, selection: &SnapshotSelection,
    skills_root: &std::path::Path,
) -> Result<ExportedSnapshot> {
    let capture = shared_snapshot::capture_shared_snapshot(database, scope, selection, skills_root)?;
    let config_json = serde_json::to_vec(&capture.config)?;
    ensure!(config_json.len() as u64 <= vw_agent_apps::device_sync::artifact::MAX_ARTIFACT_SIZE,
        "shared configuration exceeds synchronization limit");
    let config_artifact = SyncArtifact { sha256: format!("{:x}", Sha256::digest(&config_json)), size: config_json.len() as u64 };
    Ok(ExportedSnapshot { data_compat_version: capture.config.data_compat_version,
        config_json, config_artifact, skills: capture.skills, source_versions: capture.source_versions })
}
