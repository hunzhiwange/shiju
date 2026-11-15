//! Content-based synchronization decisions, independent of transport and storage.

use std::fmt;

/// Only advance this baseline after transfer, apply and read-back have succeeded.
/// Gateway persists it per destination/profile/data compatibility version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncBaseline {
    pub snapshot_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncDecision {
    NoTransfer,
    Upload,
    Download,
    Conflict,
}

/// Retain the observed absence as well as an observed snapshot identity.
/// This is an optimistic precondition, not a distributed lock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncPreview {
    pub decision: SyncDecision,
    pub expected_remote_snapshot: Option<String>,
}

/// Safe error without remote payloads or credentials. API owners map this to 409.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyncConflict;

impl SyncConflict {
    pub const fn status_code(self) -> u16 {
        409
    }
}

impl fmt::Display for SyncConflict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("device synchronization conflict: refresh preview")
    }
}

impl std::error::Error for SyncConflict {}

/// Inputs must come from the same destination/profile and compatibility scope.
/// `None` means confirmed remote absence, never a failed or unauthorized read.
/// Manifest parsing and compatibility checks remain the caller's responsibility.
pub fn preview(
    local_snapshot: &str,
    baseline: Option<&SyncBaseline>,
    remote_snapshot: Option<&str>,
) -> Result<SyncPreview, &'static str> {
    validate_snapshot(local_snapshot)?;
    if let Some(baseline) = baseline {
        validate_snapshot(&baseline.snapshot_id)?;
    }
    if let Some(remote) = remote_snapshot {
        validate_snapshot(remote)?;
    }
    let decision = if remote_snapshot == Some(local_snapshot) {
        SyncDecision::NoTransfer
    } else {
        match (baseline, remote_snapshot) {
            (None, None) => SyncDecision::Upload,
            (None, Some(_)) => SyncDecision::Conflict,
            (Some(base), Some(remote)) => {
                match (local_snapshot != base.snapshot_id, remote != base.snapshot_id) {
                    (true, false) => SyncDecision::Upload,
                    (false, true) => SyncDecision::Download,
                    // Equal local and remote was handled above.
                    _ => SyncDecision::Conflict,
                }
            }
            // Remote deletion is a change, not permission to recreate or delete data.
            (Some(_), None) => SyncDecision::Conflict,
        }
    };
    Ok(SyncPreview {
        decision,
        expected_remote_snapshot: remote_snapshot.map(str::to_owned),
    })
}

/// Re-read the remote manifest immediately before side effects. Errors reading it
/// must propagate before this call. Even a no-transfer preview must be rechecked.
impl SyncPreview {
    pub fn recheck_remote(&self, actual_remote_snapshot: Option<&str>) -> Result<(), SyncConflict> {
        if let Some(actual) = actual_remote_snapshot {
            validate_snapshot(actual).map_err(|_| SyncConflict)?;
        }
        if self.expected_remote_snapshot.as_deref() != actual_remote_snapshot {
            return Err(SyncConflict);
        }
        Ok(())
    }
}

/// Read back the published manifest after upload; do not advance the baseline on
/// mismatch. Concurrent writers can still race after this observation: neither
/// this check nor an ETag provides cross-device mutual exclusion.
pub fn verify_published_snapshot(
    uploaded_snapshot: &str,
    actual_remote_snapshot: Option<&str>,
) -> Result<(), SyncConflict> {
    validate_snapshot(uploaded_snapshot).map_err(|_| SyncConflict)?;
    if actual_remote_snapshot != Some(uploaded_snapshot) {
        return Err(SyncConflict);
    }
    Ok(())
}

fn validate_snapshot(snapshot: &str) -> Result<(), &'static str> {
    if snapshot.len() != 64
        || !snapshot.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err("invalid synchronization snapshot identity");
    }
    Ok(())
}
