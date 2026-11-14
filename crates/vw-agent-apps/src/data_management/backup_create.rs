//! Host-local archives of owner-captured snapshots, never portable downloads.
//! Callers authorize scope and retain the domain capture guards until this call
//! returns. Consistency across owners remains the safety snapshot contract.
use super::backup_create_files as files;
use super::safety_snapshot::{
    self, MAX_SNAPSHOT_BYTES, OwnerPreimage, SafetySnapshotError as Error, SafetySnapshotScope,
    SnapshotTarget,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{io::Read, path::Path};

/// Safe metadata only; never use the receipt as authorization to read a backup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalBackupReceipt {
    id: String,
    digest: [u8; 32],
}
impl LocalBackupReceipt {
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn digest(&self) -> &[u8; 32] {
        &self.digest
    }
}

/// Deliberately no Debug: complete local preimages may contain credentials.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalBackup {
    format_version: u32,
    kind: vw_api_types::agent_apps::data_management::DataArtifactKind,
    name: String,
    created_at_ms: i64,
    scope: SafetySnapshotScope,
    preimages: Vec<OwnerPreimage>,
}
impl LocalBackup {
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn created_at_ms(&self) -> i64 {
        self.created_at_ms
    }
    pub fn scope(&self) -> &SafetySnapshotScope {
        &self.scope
    }
    pub fn preimages(&self) -> &[OwnerPreimage] {
        &self.preimages
    }
}

fn validate(backup: &LocalBackup, scope: &SafetySnapshotScope) -> Result<(), Error> {
    use vw_api_types::agent_apps::data_management::DataArtifactKind;
    if backup.format_version != 1 {
        return Err(Error::VersionUnsupported);
    }
    if backup.kind != DataArtifactKind::LocalRollback {
        return Err(Error::Corrupt);
    }
    if &backup.scope != scope {
        return Err(Error::ScopeMismatch);
    }
    if scope.host_id.trim().is_empty()
        || scope.principal_id.trim().is_empty()
        || backup.name.trim().is_empty()
        || backup.name.len() > 256
        || backup.name.chars().any(char::is_control)
        || backup.preimages.is_empty()
        || backup.preimages.len() > 10_000
    {
        return Err(Error::InvalidSelection);
    }
    let mut seen = std::collections::HashSet::new();
    let mut present = false;
    for image in &backup.preimages {
        if image.target.object_id.trim().is_empty()
            || image.target.object_id.len() > 4096
            || image.target.object_id.contains('\0')
            || !seen.insert(&image.target)
        {
            return Err(Error::Corrupt);
        }
        if let Some(state) = &image.state {
            present = true;
            if state.revision.trim().is_empty()
                || state.revision.len() > 4096
                || state.revision.contains('\0')
            {
                return Err(Error::Corrupt);
            }
        }
    }
    if !present {
        return Err(Error::IncompleteCapture);
    }
    Ok(())
}

fn decode(
    bytes: &[u8],
    receipt: &LocalBackupReceipt,
    scope: &SafetySnapshotScope,
) -> Result<LocalBackup, Error> {
    if bytes.len() > MAX_SNAPSHOT_BYTES {
        return Err(Error::TooLarge);
    }
    if <[u8; 32]>::from(Sha256::digest(bytes)) != receipt.digest {
        return Err(Error::Corrupt);
    }
    let backup: LocalBackup = serde_json::from_slice(bytes).map_err(|_| Error::Corrupt)?;
    validate(&backup, scope)?;
    Ok(backup)
}

/// Root is an existing private 0700 executor-owned directory, not a request path.
/// Capture must return every selected owner bundle, including explicit absence.
/// All absent targets return None, with no visible archive. Unsupported capture
/// returns an error rather than substituting absence. No host database is copied.
pub fn create_local_backup(
    root: &Path,
    scope: &SafetySnapshotScope,
    targets: &[SnapshotTarget],
    name: &str,
    capture: impl FnOnce() -> Result<Vec<OwnerPreimage>, Error>,
) -> Result<Option<LocalBackupReceipt>, Error> {
    let directory = files::open_root(root)?;
    // Reject invalid labels before sensitive capture or creating staging files.
    if name.trim().is_empty() || name.len() > 256 || name.chars().any(char::is_control) {
        return Err(Error::InvalidSelection);
    }
    // Reuse the owner's complete capture and verified readback, not a second
    // snapshot format/reader. The private staging tree is never a visible backup.
    let staging = tempfile::Builder::new()
        .prefix(".backup-capture-")
        .tempdir_in(root)
        .map_err(|_| Error::StorageUnavailable)?;
    let receipt = safety_snapshot::create_safety_snapshot(staging.path(), scope, targets, capture)?;
    let snapshot = safety_snapshot::open_safety_snapshot(staging.path(), &receipt, scope, targets)?;
    if snapshot.preimages().iter().all(|image| image.state.is_none()) {
        return Ok(None);
    }
    let created_at_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| Error::StorageUnavailable)?
        .as_millis()
        .try_into()
        .map_err(|_| Error::StorageUnavailable)?;
    let backup = LocalBackup {
        format_version: 1,
        kind: vw_api_types::agent_apps::data_management::DataArtifactKind::LocalRollback,
        name: name.into(),
        created_at_ms,
        scope: scope.clone(),
        preimages: snapshot.preimages().to_vec(),
    };
    validate(&backup, scope)?;
    let bytes = serde_json::to_vec(&backup).map_err(|_| Error::Corrupt)?;
    if bytes.len() > MAX_SNAPSHOT_BYTES {
        return Err(Error::TooLarge);
    }
    let mut result = LocalBackupReceipt {
        id: uuid::Uuid::new_v4().to_string(),
        digest: Sha256::digest(&bytes).into(),
    };
    // Complete readback and validation happen before the only visible publish.
    result.id = files::publish(&directory, &result.id, &bytes, |reread| {
        decode(reread, &result, scope).map(|_| ())
    })?;
    Ok(Some(result))
}

/// Independently authorize the original tenant/project/principal/host first.
/// Restoring these bundles is a separate owner operation with version guards;
/// this reader never interprets owner bytes as SQL, paths or commands.
pub fn open_local_backup(
    root: &Path,
    receipt: &LocalBackupReceipt,
    scope: &SafetySnapshotScope,
) -> Result<LocalBackup, Error> {
    let mut file = files::open_backup(&files::open_root(root)?, &receipt.id)?;
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take(MAX_SNAPSHOT_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| Error::StorageUnavailable)?;
    decode(&bytes, receipt, scope)
}
