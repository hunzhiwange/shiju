//! Guarded whole-file replacement for executor-owned configuration patches.
use super::{stage::digest, write_guard::WriteError};

/// Rejects replacement when the planning snapshot no longer matches. Capture
/// before staging; pass `apply` through the existing FileChange patch boundary.
pub struct Guarded {
    pub expected_pre: Option<String>,
    pub bytes: Vec<u8>,
}
impl Guarded {
    pub fn new(before: Option<&[u8]>, bytes: Vec<u8>) -> Self {
        Self { expected_pre: before.map(digest), bytes }
    }
    pub fn apply(&self, before: Option<&[u8]>) -> Result<Vec<u8>, WriteError> {
        if before.map(digest) != self.expected_pre {
            return Err(WriteError::Conflict);
        }
        Ok(self.bytes.clone())
    }
}

use super::{
    commit::{Pending, Phase, ProviderTransactions, checkpoint},
    stage::{FileChange, StagedBatch},
    write_guard::{prepare, read_from},
};
use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

const MAX_REPLANS: usize = 3;

impl ProviderTransactions {
    /// The guard and durable intent stay owned by commit. Each rename is preceded
    /// by a descriptor-relative reread. External writers do not share our lock:
    /// this bounds detected races, but cannot provide a filesystem-wide CAS.
    pub(super) fn publish_checked(
        &self,
        batch: &mut StagedBatch<'_>,
        changes: &[FileChange<'_>],
        pending: &mut Pending,
    ) -> Result<(), WriteError> {
        let mut published_any = false;
        let result = (|| {
            for index in 0..batch.writes.len() {
                let mut replans = 0;
                loop {
                    checkpoint(self, pending.app, Phase::BeforePublish)?;
                    let write = &batch.writes[index];
                    let name = OsStr::from_bytes(write.name.as_bytes());
                    let current = read_from(write.directory, name)?;
                    let pre = current.as_deref().map(digest);
                    if pre == pending.files[index].pre {
                        if pre != pending.files[index].planned {
                            write.publish()?;
                            published_any = true;
                            write.directory.sync_all().map_err(|_| WriteError::Io)?;
                        }
                        break;
                    }
                    if replans == MAX_REPLANS {
                        return Err(WriteError::Conflict);
                    }
                    replans += 1;
                    let change = changes
                        .iter()
                        .find(|change| change.name == name)
                        .ok_or(WriteError::InvalidResource)?;
                    // Guarded patches reject here on their first changed snapshot.
                    let after = change.patch.apply(current.as_deref())?;
                    if after.as_ref().is_some_and(|bytes| bytes.len() > 1024 * 1024) {
                        return Err(WriteError::InvalidResource);
                    }
                    let mut replacement = prepare(
                        write.directory,
                        name,
                        after.as_deref().unwrap_or_default(),
                        change.privacy,
                    )?;
                    replacement.delete = after.is_none();
                    replacement.directory.sync_all().map_err(|_| WriteError::Io)?;
                    let mut updated = pending.clone();
                    updated.files[index].pre = pre;
                    updated.files[index].planned = after.as_deref().map(digest);
                    if after.is_none() {
                        updated.version = 2;
                    }
                    updated.files[index].staged = replacement.temp.as_bytes().to_vec();
                    // save can rename pending then fail its directory fsync. Keep
                    // both candidates on error so either durable record is valid.
                    replacement.retain = true;
                    self.save(pending.app, "pending", &updated)?;
                    *pending = updated;
                    let mut old = std::mem::replace(&mut batch.writes[index], replacement);
                    old.retain = false;
                    drop(old);
                    batch.writes[index].directory.sync_all().map_err(|_| WriteError::Io)?;
                }
                // A replan can discover the external writer already produced
                // the desired bytes. Its unused temporary is no longer durable
                // recovery input and must be removed on normal return/unwind.
                batch.writes[index].retain = false;
                checkpoint(self, pending.app, Phase::FilePublished)?;
            }
            Ok(())
        })();
        if result.is_err() && !published_any {
            // A clean rejection must never later settle this target via recovery.
            // Persist discard semantics before removing intent or staged bytes.
            pending.published = false;
            self.save(pending.app, "pending", pending)?;
            self.clear_pending(pending.app)?;
            for write in &mut batch.writes {
                write.retain = false;
            }
        }
        result
    }
}
