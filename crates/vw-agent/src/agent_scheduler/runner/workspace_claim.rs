//! Canonical hierarchical OS locks, outside business directories.
//! Shared ancestor locks allow siblings to overlap; the leaf is exclusive.
use std::collections::HashMap;
use std::fs::{File, OpenOptions, TryLockError};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};

use anyhow::{Context, Result, ensure};
use sha2::{Digest, Sha256};

pub(crate) struct WorkspaceClaim {
    files: Vec<File>,
    owner: String,
    shutdown_unconfirmed: bool,
    path: PathBuf,
    marker: PathBuf,
    recorded: bool,
}

type ClaimKey = (PathBuf, String);
static STARTED: LazyLock<Mutex<HashMap<ClaimKey, WorkspaceClaim>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

impl WorkspaceClaim {
    pub(crate) fn acquire(root: &Path, path: &Path, owner: String) -> Result<Option<Self>> {
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
        // A crashed scheduler can leave child writers behind after its OS locks
        // disappear. Never infer their shutdown from an unlocked file or a TTL.
        for entry in std::fs::read_dir(root)? {
            let entry = entry?;
            if !entry.file_name().to_string_lossy().starts_with("active-") {
                continue;
            }
            let record = match std::fs::read(entry.path()) {
                Ok(record) => record,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => return Err(error).context("read retained workspace claim"),
            };
            let (occupied, run): (PathBuf, String) =
                serde_json::from_slice(&record).with_context(|| {
                    format!("invalid retained workspace claim: {}", entry.path().display())
                })?;
            ensure!(
                !path.starts_with(&occupied) && !occupied.starts_with(&path),
                "execution shutdown unconfirmed for run {run}; retained claim: {}",
                entry.path().display()
            );
        }
        let key = hex::encode(Sha256::digest(path.as_os_str().as_encoded_bytes()));
        Ok(Some(Self {
            files,
            owner,
            shutdown_unconfirmed: false,
            path,
            marker: root.join(format!("active-{key}")),
            recorded: false,
        }))
    }

    pub(crate) fn retain(mut self, path: &Path) -> Result<()> {
        let path = path.canonicalize()?;
        ensure!(path == self.path, "workspace claim directory mismatch");
        let key = (path, self.owner.clone());
        let mut claims =
            STARTED.lock().map_err(|_| anyhow::anyhow!("workspace claims poisoned"))?;
        ensure!(!claims.contains_key(&key), "workspace claim already retained");
        if !self.recorded {
            let pending = self.marker.with_file_name(format!(
                "pending-{}-{}-{}",
                std::process::id(),
                self.marker.file_name().unwrap().to_string_lossy(),
                hex::encode(Sha256::digest(self.owner.as_bytes()))
            ));
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let mut file =
                options.open(&pending).context("create pending workspace ownership record")?;
            file.write_all(&serde_json::to_vec(&(&self.path, &self.owner))?)?;
            file.sync_all()?;
            std::fs::rename(&pending, &self.marker)
                .context("persist workspace ownership record")?;
            self.recorded = true;
        }
        claims.insert(key, self);
        Ok(())
    }

    pub(crate) fn retain_unconfirmed(mut self, path: &Path) -> Result<()> {
        self.shutdown_unconfirmed = true;
        self.retain(path)
    }

    pub(crate) fn take(path: &Path, owner: &str) -> Result<Option<Self>> {
        Self::take_canonical(&path.canonicalize()?, owner)
    }

    /// Use the canonical path captured at acquisition, even after explicit removal.
    pub(crate) fn take_canonical(path: &Path, owner: &str) -> Result<Option<Self>> {
        let key = (path.to_path_buf(), owner.to_owned());
        let mut claims =
            STARTED.lock().map_err(|_| anyhow::anyhow!("workspace claims poisoned"))?;
        if claims.get(&key).is_some_and(|claim| claim.shutdown_unconfirmed) {
            return Ok(None);
        }
        Ok(claims.remove(&key))
    }

    pub(crate) fn release(&mut self, owner: &str) -> Result<()> {
        ensure!(self.owner == owner, "workspace claim owner mismatch");
        self.clear_record()?;
        for file in &self.files {
            file.unlock().context("release workspace OS lock")?;
        }
        self.files.clear();
        Ok(())
    }

    fn clear_record(&mut self) -> Result<()> {
        if self.recorded && !self.shutdown_unconfirmed {
            std::fs::remove_file(&self.marker)
                .context("clear confirmed workspace ownership record")?;
            self.recorded = false;
        }
        Ok(())
    }
}

impl Drop for WorkspaceClaim {
    fn drop(&mut self) {
        if let Err(error) = self.clear_record() {
            tracing::error!(%error, marker = %self.marker.display(), "workspace ownership record retained");
        }
        // Explicit unlock also releases a handle temporarily inherited during fork/exec.
        for file in &self.files {
            if let Err(error) = file.unlock() {
                tracing::warn!(%error, "failed to unlock workspace claim");
            }
        }
    }
}

pub(crate) fn lock_root() -> Result<PathBuf> {
    Ok(directories::BaseDirs::new()
        .context("resolve workspace lock home")?
        .data_local_dir()
        .join("shiju/workspace-claims"))
}
