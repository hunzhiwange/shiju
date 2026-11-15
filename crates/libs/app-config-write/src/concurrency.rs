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
