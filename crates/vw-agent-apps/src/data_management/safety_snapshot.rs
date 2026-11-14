//! Host-local rollback preimages, separate from portable exports.
//!
//! Domain owners capture under their transaction/locks and keep those guards until
//! `with_safety_snapshot` returns. This module cannot make independent databases,
//! native files or Keychain reads atomic. Unsupported owners must return an error;
//! never substitute empty data. No SQL, namespace or native path is executed here.
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use vw_api_types::id::ProjectScope;

use super::safety_snapshot_files as files;

pub(super) const MAX_SNAPSHOT_BYTES: usize = 64 * 1024 * 1024;
const FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotCategory {
    Provider,
    Mcp,
    Skill,
    Prompt,
    Profile,
    Preference,
    Usage,
}

/// Constructed by the authorized execution host, never from desktop paths.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SafetySnapshotScope {
    pub project: ProjectScope,
    pub principal_id: String,
    pub host_id: String,
}

/// An owner-issued identity, not a table name, SQL expression or filesystem path.
/// Include dependency records, indexes, native projections and resources that the
/// operation can change. One owner bundle may contain several physical objects.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SnapshotTarget {
    pub category: SnapshotCategory,
    pub object_id: String,
}

/// Sensitive, owner-encoded restoration data. Deliberately has no Debug impl.
/// `None` records absence, so newly created objects can be removed on rollback.
/// Present objects require the owner's revision/file identity and complete bytes,
/// including resources needed for restoration. An ID/reference alone is insufficient.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnerPreimage {
    pub target: SnapshotTarget,
    pub state: Option<PresentPreimage>,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PresentPreimage {
    pub revision: String,
    pub bytes: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SafetySnapshot {
    format_version: u32,
    scope: SafetySnapshotScope,
    preimages: Vec<OwnerPreimage>,
}

impl SafetySnapshot {
    pub fn scope(&self) -> &SafetySnapshotScope {
        &self.scope
    }
    pub fn preimages(&self) -> &[OwnerPreimage] {
        &self.preimages
    }
}

/// Safe task metadata. Keep it with the owning operation on the execution host.
/// The digest detects corruption relative to this receipt, not malicious changes
/// by the host owner. Neither this receipt nor a snapshot ID grants authorization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SafetySnapshotReceipt {
    id: String,
    digest: [u8; 32],
}
impl SafetySnapshotReceipt {
    pub fn id(&self) -> &str {
        &self.id
    }
}

/// Stable errors contain no payloads, native paths or underlying driver messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SafetySnapshotError {
    InvalidSelection,
    CaptureFailed,
    IncompleteCapture,
    StorageUnavailable,
    UnsafeStorage,
    TooLarge,
    Corrupt,
    ScopeMismatch,
    VersionUnsupported,
    Conflict,
}
impl std::fmt::Display for SafetySnapshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "safety snapshot: {self:?}")
    }
}
impl std::error::Error for SafetySnapshotError {}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 4096 && !value.contains('\0')
}
fn validate_selection(
    scope: &SafetySnapshotScope,
    targets: &[SnapshotTarget],
) -> Result<(), SafetySnapshotError> {
    if !valid_id(&scope.principal_id)
        || !valid_id(&scope.host_id)
        || targets.is_empty()
        || targets.len() > 10_000
    {
        return Err(SafetySnapshotError::InvalidSelection);
    }
    let mut seen = std::collections::HashSet::new();
    for target in targets {
        if !valid_id(&target.object_id) || !seen.insert(target) {
            return Err(SafetySnapshotError::InvalidSelection);
        }
    }
    Ok(())
}
fn validate_capture(
    snapshot: &SafetySnapshot,
    targets: &[SnapshotTarget],
) -> Result<(), SafetySnapshotError> {
    if snapshot.format_version != FORMAT_VERSION {
        return Err(SafetySnapshotError::VersionUnsupported);
    }
    let mut remaining: std::collections::HashSet<_> = targets.iter().collect();
    let mut size = 0usize;
    for image in &snapshot.preimages {
        if !remaining.remove(&image.target) {
            return Err(SafetySnapshotError::IncompleteCapture);
        }
        if let Some(state) = &image.state {
            if !valid_id(&state.revision) {
                return Err(SafetySnapshotError::IncompleteCapture);
            }
            size = size.checked_add(state.bytes.len()).ok_or(SafetySnapshotError::TooLarge)?;
            if size > MAX_SNAPSHOT_BYTES {
                return Err(SafetySnapshotError::TooLarge);
            }
        }
    }
    if !remaining.is_empty() {
        return Err(SafetySnapshotError::IncompleteCapture);
    }
    Ok(())
}

/// Root must already be a private, executor-owned directory on the execution host.
/// Capture must observe exactly the selected owner bundles from one consistent
/// boundary. Keep owner locks open, or CAS-check every observed version before
/// committing. This function performs no domain writes and returns only after a
/// durable, bounded, checksum-checked deserialize/readback of the complete archive.
pub fn create_safety_snapshot(
    root: &Path,
    scope: &SafetySnapshotScope,
    targets: &[SnapshotTarget],
    capture: impl FnOnce() -> Result<Vec<OwnerPreimage>, SafetySnapshotError>,
) -> Result<SafetySnapshotReceipt, SafetySnapshotError> {
    validate_selection(scope, targets)?;
    let snapshot = SafetySnapshot {
        format_version: FORMAT_VERSION,
        scope: scope.clone(),
        preimages: capture()?,
    };
    validate_capture(&snapshot, targets)?;
    let bytes = serde_json::to_vec(&snapshot).map_err(|_| SafetySnapshotError::Corrupt)?;
    if bytes.len() > MAX_SNAPSHOT_BYTES {
        return Err(SafetySnapshotError::TooLarge);
    }
    let receipt = SafetySnapshotReceipt {
        id: uuid::Uuid::new_v4().to_string(),
        digest: Sha256::digest(&bytes).into(),
    };
    files::publish(root, &receipt.id, &bytes, |reread| {
        decode(reread, &receipt, scope, targets).map(|_| ())
    })?;
    Ok(receipt)
}

fn decode(
    bytes: &[u8],
    receipt: &SafetySnapshotReceipt,
    scope: &SafetySnapshotScope,
    targets: &[SnapshotTarget],
) -> Result<SafetySnapshot, SafetySnapshotError> {
    if <[u8; 32]>::from(Sha256::digest(bytes)) != receipt.digest {
        return Err(SafetySnapshotError::Corrupt);
    }
    let snapshot: SafetySnapshot =
        serde_json::from_slice(bytes).map_err(|_| SafetySnapshotError::Corrupt)?;
    if &snapshot.scope != scope {
        return Err(SafetySnapshotError::ScopeMismatch);
    }
    validate_capture(&snapshot, targets)?;
    Ok(snapshot)
}

/// Reopen only for an independently authorized scope and exact write selection.
/// Restoration is performed by each existing owner with CAS against the state
/// written by this operation. Never blindly replace a database or native file.
pub fn open_safety_snapshot(
    root: &Path,
    receipt: &SafetySnapshotReceipt,
    scope: &SafetySnapshotScope,
    targets: &[SnapshotTarget],
) -> Result<SafetySnapshot, SafetySnapshotError> {
    validate_selection(scope, targets)?;
    decode(&files::read(root, &receipt.id)?, receipt, scope, targets)
}

/// The only write callback runs after capture, persistence and verified readback.
/// It must revalidate observed versions under owner guards before its first write.
/// On failure the receipt is retained: a failed callback may have committed data.
/// The nested Result preserves that fact instead of reporting a false zero-write
/// failure. Rollback/commit and projection outcomes remain the owners' concern.
pub fn with_safety_snapshot<T, E>(
    root: &Path,
    scope: &SafetySnapshotScope,
    targets: &[SnapshotTarget],
    capture: impl FnOnce() -> Result<Vec<OwnerPreimage>, SafetySnapshotError>,
    write: impl FnOnce(&SafetySnapshotReceipt) -> Result<T, E>,
) -> Result<(SafetySnapshotReceipt, Result<T, E>), SafetySnapshotError> {
    let receipt = create_safety_snapshot(root, scope, targets, capture)?;
    let outcome = write(&receipt);
    Ok((receipt, outcome))
}
