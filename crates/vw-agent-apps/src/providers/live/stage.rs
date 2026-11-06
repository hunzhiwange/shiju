//! Multi-file preparation only. Transactions must persist pending before publishing.
use super::write_guard::{AppWriteGuard, FilePrivacy, PreparedWrite, WriteError};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use sha2::{Digest, Sha256};
use std::{collections::HashSet, ffi::OsStr};

pub(super) struct FileRevision {
    pub before: Option<String>,
    pub after: Option<String>,
}
pub(super) fn digest(bytes: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(bytes))
}

/// None means remove the file; an empty vector still means an existing empty file.
pub trait FilePatch {
    fn apply(&self, before: Option<&[u8]>) -> Result<Option<Vec<u8>>, WriteError>;
}
impl<F: Fn(Option<&[u8]>) -> Result<Vec<u8>, WriteError>> FilePatch for F {
    fn apply(&self, before: Option<&[u8]>) -> Result<Option<Vec<u8>>, WriteError> {
        self(before).map(Some)
    }
}
/// Executor-owned snapshot guard for whole-file replacement or deletion.
pub struct WholeFile {
    expected: Option<String>,
    after: Option<Vec<u8>>,
}
impl WholeFile {
    pub fn new(before: Option<&[u8]>, after: Option<Vec<u8>>) -> Self {
        Self { expected: before.map(digest), after }
    }
}
impl FilePatch for WholeFile {
    fn apply(&self, before: Option<&[u8]>) -> Result<Option<Vec<u8>>, WriteError> {
        if before.map(digest) != self.expected {
            return Err(WriteError::Conflict);
        }
        Ok(self.after.clone())
    }
}
pub type Patch<'a> = &'a dyn FilePatch;

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
    pub(super) revisions: Vec<FileRevision>,
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
            let after = change.patch.apply(before.as_deref())?;
            if after.as_ref().is_some_and(|bytes| bytes.len() > 1024 * 1024) {
                return Err(WriteError::InvalidResource);
            }
            if before != after {
                let revision = FileRevision {
                    before: before.as_deref().map(digest),
                    after: after.as_deref().map(digest),
                };
                plans.push((change, after, revision));
            }
        }
        // RAII discards every earlier temporary if any subsequent staging fails.
        let mut writes = Vec::with_capacity(plans.len());
        let mut revisions = Vec::with_capacity(plans.len());
        for (change, bytes, revision) in plans {
            let mut write =
                self.stage_one(change.name, bytes.as_deref().unwrap_or_default(), change.privacy)?;
            write.delete = bytes.is_none();
            writes.push(write);
            revisions.push(revision);
        }
        Ok(StagedBatch { writes, revisions })
    }
}
