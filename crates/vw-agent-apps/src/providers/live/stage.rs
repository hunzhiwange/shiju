//! Multi-file preparation only. Transactions must persist pending before publishing.
use super::write_guard::{AppWriteGuard, FilePrivacy, PreparedWrite, WriteError};
use std::{collections::HashSet, ffi::OsStr};

pub type Patch<'a> = &'a dyn Fn(Option<&[u8]>) -> Result<Vec<u8>, WriteError>;

/// Executor-owned patch; never a client-supplied arbitrary file operation.
pub struct FileChange<'a> {
    pub name: &'a OsStr,
    pub privacy: FilePrivacy,
    pub patch: Patch<'a>,
}

/// Retains the exclusive guard borrow until publication or discard. Drop removes
/// unpublished temporary files. No pointer, backup or pending writes happen here.
pub struct StagedBatch<'a> {
    pub(super) writes: Vec<PreparedWrite<'a>>,
}
impl StagedBatch<'_> {
    pub fn len(&self) -> usize {
        self.writes.len()
    }
    pub fn is_empty(&self) -> bool {
        self.writes.is_empty()
    }
    /// The transaction owner must first persist pending. Publication is sequential,
    /// not multi-file atomic; partial publication recovery belongs to that owner.
    pub fn publish(self) -> Result<(), WriteError> {
        for write in self.writes {
            write.commit()?;
        }
        Ok(())
    }
}
impl AppWriteGuard {
    pub fn stage<'a>(
        &'a mut self,
        changes: &[FileChange<'_>],
    ) -> Result<StagedBatch<'a>, WriteError> {
        // Reject duplicate targets before any patch runs. Patch callbacks must be
        // pure executor code: parse/validate and return bytes without I/O writes.
        let mut names = HashSet::new();
        for change in changes {
            if !names.insert(change.name) {
                return Err(WriteError::InvalidResource);
            }
        }
        let mut plans = Vec::with_capacity(changes.len());
        for change in changes {
            let before = self.read(change.name)?;
            let after = (change.patch)(before.as_deref())?;
            if after.len() > 1024 * 1024 {
                return Err(WriteError::InvalidResource);
            }
            if before.as_deref() != Some(after.as_slice()) {
                plans.push((change, after));
            }
        }
        // RAII discards every earlier temporary if any subsequent staging fails.
        let mut writes = Vec::with_capacity(plans.len());
        for (change, bytes) in plans {
            writes.push(self.stage_one(change.name, &bytes, change.privacy)?);
        }
        Ok(StagedBatch { writes })
    }
}
