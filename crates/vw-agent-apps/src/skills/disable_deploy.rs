//! Quarantine only a saved, unchanged deployment until settings commit.
use std::{fs::{self, File}, io, os::fd::AsRawFd, path::Path};

use super::{
    app_paths::AppSkillsPath,
    copy_deploy::{CopyBinding, binding_fingerprint},
    local_install::{LocalInstallError, rename_exclusive, validate_directory},
};
use crate::providers::paths::open_beneath;

#[derive(Debug)]
pub enum DisableError {
    /// Ownership could not be proved. Keep both the path and saved binding.
    Preserved,
    Operation(LocalInstallError),
}

impl std::fmt::Display for DisableError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Preserved => "skill path preserved: deployment ownership could not be verified",
            Self::Operation(_) => "skill disable failed; inspect deployment recovery state",
        })
    }
}
impl std::error::Error for DisableError {}
impl From<LocalInstallError> for DisableError {
    fn from(error: LocalInstallError) -> Self { Self::Operation(error) }
}
impl From<io::Error> for DisableError {
    fn from(error: io::Error) -> Self { Self::Operation(error.into()) }
}

pub struct DisableDeploy {
    root: File,
    stage: Option<tempfile::TempDir>,
    stage_dir: File,
    binding: CopyBinding,
    removed: bool,
}

impl DisableDeploy {
    /// The caller must authorize this installation on the execution host.
    /// Every app uses the same ownership checks, including aliases of native roots.
    pub fn prepare(ssot: &Path, target: &AppSkillsPath, directory: &str,
        saved: Option<&CopyBinding>) -> Result<Self, DisableError> {
        validate_directory(directory)?;
        target.require_distribution().map_err(|_| DisableError::Preserved)?;
        if !ssot.is_absolute() || !target.directory.is_absolute() {
            return Err(DisableError::Preserved);
        }
        let binding = saved.ok_or(DisableError::Preserved)?;
        let source_root = fs::canonicalize(ssot)?;
        let destination_root = fs::canonicalize(&target.directory)?;
        if source_root.starts_with(&destination_root) || destination_root.starts_with(&source_root)
            || binding.directory != directory || binding.destination != destination_root.join(directory)
            || binding.link_target.as_ref().is_some_and(|source| source != &source_root.join(directory)) {
            return Err(DisableError::Preserved);
        }
        open_beneath(ssot, ssot, true).map_err(|_| DisableError::Preserved)?;
        open_beneath(&target.directory, &target.directory, true)
            .map_err(|_| DisableError::Preserved)?;
        Self::prepare_tree(&destination_root, binding)
    }

    /// SSOT callers must first establish a persisted installation record.
    pub(super) fn prepare_tree(directory: &Path, binding: &CopyBinding) -> Result<Self, DisableError> {
        Self::prepare_tree_locked(directory, binding, None)
    }

    pub(crate) fn prepare_tree_locked(directory: &Path, binding: &CopyBinding,
        locked: Option<&File>) -> Result<Self, DisableError> {
        validate_directory(&binding.directory)?;
        if binding.destination != directory.join(&binding.directory) {
            return Err(DisableError::Preserved);
        }
        let destination_root = directory;
        let root = match locked {
            Some(file) => file.try_clone()?,
            None => open_beneath(directory, directory, true).map_err(|_| DisableError::Preserved)?,
        };
        if unsafe { libc::flock(root.as_raw_fd(), libc::LOCK_EX) } != 0 {
            return Err(io::Error::last_os_error().into());
        }
        if binding_fingerprint(&binding.destination, binding.link_target.as_deref())
            .map_err(|_| DisableError::Preserved)? != binding.fingerprint {
            return Err(DisableError::Preserved);
        }
        let stage = tempfile::Builder::new().prefix(".skill-disable-").tempdir_in(&destination_root)?;
        let stage_dir = open_beneath(&destination_root, stage.path(), true)
            .map_err(|_| DisableError::Preserved)?;
        Ok(Self { root, stage: Some(stage), stage_dir, binding: binding.clone(), removed: false })
    }

    /// Recovery location retained if cleanup or rollback fails.
    pub fn recovery_path(&self) -> Option<std::path::PathBuf> {
        self.stage.as_ref().map(|stage| stage.path().join("tree"))
    }

    pub fn publish(&mut self) -> Result<(), DisableError> {
        if self.removed { return Err(DisableError::Preserved); }
        if binding_fingerprint(&self.binding.destination, self.binding.link_target.as_deref())
            .map_err(|_| DisableError::Preserved)? != self.binding.fingerprint {
            return Err(DisableError::Preserved);
        }
        rename_exclusive(&self.root, &self.binding.directory, &self.stage_dir, "tree")?;
        self.removed = true;
        // Verify the displaced entry, so an external replacement during rename is
        // restored rather than deleted. Links are inspected without following them.
        let tree = self.stage.as_ref().ok_or(DisableError::Preserved)?.path().join("tree");
        if binding_fingerprint(&tree, self.binding.link_target.as_deref())
            .map_or(true, |hash| hash != self.binding.fingerprint) {
            self.rollback()?;
            return Err(DisableError::Preserved);
        }
        Ok(())
    }

    pub fn rollback(&mut self) -> Result<(), DisableError> {
        if self.removed {
            // Never overwrite an entry another writer created after quarantine.
            rename_exclusive(&self.stage_dir, "tree", &self.root, &self.binding.directory)?;
            self.removed = false;
        }
        Ok(())
    }

    /// Call only after settings commit. Failed cleanup retains the quarantine.
    pub fn finish(mut self) -> Result<(), DisableError> {
        if !self.removed { return Err(DisableError::Preserved); }
        let stage = self.stage.take().ok_or(DisableError::Preserved)?;
        let tree = stage.path().join("tree");
        self.removed = false;
        if binding_fingerprint(&tree, self.binding.link_target.as_deref())
            .map_or(true, |hash| hash != self.binding.fingerprint) {
            let _ = stage.keep();
            return Err(DisableError::Preserved);
        }
        let result = if self.binding.link_target.is_some() { fs::remove_file(&tree) }
            else { fs::remove_dir_all(&tree) };
        if let Err(error) = result {
            let _ = stage.keep();
            return Err(error.into());
        }
        stage.close()?;
        Ok(())
    }
}

impl Drop for DisableDeploy {
    fn drop(&mut self) {
        if self.rollback().is_err() {
            if let Some(stage) = self.stage.take() { let _ = stage.keep(); }
        }
    }
}
