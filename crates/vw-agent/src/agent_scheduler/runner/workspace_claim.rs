//! Canonical hierarchical OS locks, outside business directories.
//! Shared ancestor locks allow siblings to overlap; the leaf is exclusive.
use std::collections::HashMap;
use std::fs::{File, OpenOptions, TryLockError};
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};

use anyhow::{Context, Result, ensure};
use sha2::{Digest, Sha256};

pub(super) struct WorkspaceClaim {
    files: Vec<File>,
    owner: String,
}

type ClaimKey = (PathBuf, String);
static STARTED: LazyLock<Mutex<HashMap<ClaimKey, WorkspaceClaim>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

impl WorkspaceClaim {
    pub(super) fn acquire(root: &Path, path: &Path, owner: String) -> Result<Option<Self>> {
        let path = path.canonicalize().context("resolve execution directory for claim")?;
        ensure!(path.is_dir(), "execution directory is not a directory");
        std::fs::create_dir_all(root).context("create workspace lock directory")?;
        let mut ancestors: Vec<_> = path.ancestors().collect();
        ancestors.reverse();
        let mut files = Vec::new();
        for ancestor in ancestors {
            let key = hex::encode(Sha256::digest(ancestor.as_os_str().as_encoded_bytes()));
            let file = OpenOptions::new()
                .create(true)
                .truncate(false)
                .read(true)
                .write(true)
                .open(root.join(key))
                .context("open workspace lock")?;
            let locked = if ancestor == path { file.try_lock() } else { file.try_lock_shared() };
            match locked {
                Ok(()) => files.push(file),
                Err(TryLockError::WouldBlock) => return Ok(None),
                Err(TryLockError::Error(error)) => {
                    return Err(error).context("workspace OS locking unavailable");
                }
            }
        }
        Ok(Some(Self { files, owner }))
    }

    pub(super) fn retain(self, path: &Path) -> Result<()> {
        let key = (path.canonicalize()?, self.owner.clone());
        let mut claims =
            STARTED.lock().map_err(|_| anyhow::anyhow!("workspace claims poisoned"))?;
        ensure!(!claims.contains_key(&key), "workspace claim already retained");
        claims.insert(key, self);
        Ok(())
    }

    pub(super) fn take(path: &Path, owner: &str) -> Result<Option<Self>> {
        Ok(STARTED
            .lock()
            .map_err(|_| anyhow::anyhow!("workspace claims poisoned"))?
            .remove(&(path.canonicalize()?, owner.to_owned())))
    }

    pub(super) fn release(&mut self, owner: &str) -> Result<()> {
        ensure!(self.owner == owner, "workspace claim owner mismatch");
        for file in &self.files {
            file.unlock().context("release workspace OS lock")?;
        }
        self.files.clear();
        Ok(())
    }
}

impl Drop for WorkspaceClaim {
    fn drop(&mut self) {
        // Explicit unlock also releases a handle temporarily inherited during fork/exec.
        for file in &self.files {
            if let Err(error) = file.unlock() {
                tracing::warn!(%error, "failed to unlock workspace claim");
            }
        }
    }
}
