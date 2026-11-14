//! Transactional host-local storage migration. Old roots are always retained.
use std::{fs::{self, File}, os::{fd::AsRawFd, unix::fs::{symlink, MetadataExt}}, path::{Path, PathBuf}};
use serde::{Deserialize, Serialize};
use super::{copy_deploy::{CopyBinding, binding_fingerprint},
    local_install::{copy_skill_tree, rename_exclusive, validate_directory, LocalInstallError}};
use crate::providers::paths::open_beneath;

/// Validate an existing destination beneath the authorized execution-host HOME.
pub fn validate_destination(home: &Path, destination: &Path) -> std::result::Result<(), LocalInstallError> {
    open_beneath(home, destination, true).map_err(|_| LocalInstallError::UnsafeSource)?;
    Ok(())
}

type Result<T> = std::result::Result<T, LocalInstallError>;
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct StorageMoveResult {
    pub migrated: Vec<String>,
    pub skipped: Vec<String>,
    pub errors: Vec<String>,
    pub switched: bool,
}

/// Holds physical root locks until settings commit. Drop restores publications.
pub struct StorageMove {
    pub result: StorageMoveResult,
    pub destination: PathBuf,
    _locks: Vec<File>,
    copies: Vec<(File, tempfile::TempDir, String, (u64, u64))>,
    links: Vec<(File, tempfile::TempDir, String, PathBuf)>,
    committed: bool,
}
impl StorageMove {
    /// All roots must be existing, authorized execution-host paths. The caller
    /// restricts destination to its runtime root or HOME/.agents/skills.
    pub fn prepare(old: &Path, destination: &Path, directories: &[String],
        app_roots: &[PathBuf]) -> Result<Self> {
        if !old.is_absolute() || !destination.is_absolute() { return Err(LocalInstallError::UnsafeSource); }
        let old = fs::canonicalize(old)?;
        let destination = fs::canonicalize(destination)?;
        let mut result = StorageMoveResult::default();
        if old == destination {
            result.skipped = directories.to_vec();
            return Ok(Self { result, destination, _locks: vec![], copies: vec![], links: vec![], committed: false });
        }
        if old.starts_with(&destination) || destination.starts_with(&old) {
            return Err(LocalInstallError::UnsafeSource);
        }
        let mut roots = vec![old.clone(), destination.clone()];
        for app in app_roots {
            let app = fs::canonicalize(app)?;
            if destination.starts_with(&app) || app.starts_with(&destination)
                || old.starts_with(&app) || app.starts_with(&old) {
                return Err(LocalInstallError::UnsafeSource);
            }
            roots.push(app);
        }
        roots.sort(); roots.dedup();
        let mut locks = Vec::new();
        for path in roots {
            let file = open_beneath(&path, &path, true).map_err(|_| LocalInstallError::UnsafeSource)?;
            // Nonblocking acquisition avoids deadlocks with existing deployers.
            if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
                return Err(std::io::Error::last_os_error().into());
            }
            locks.push(file);
        }
        let mut migration = Self { result, destination, _locks: locks,
            copies: vec![], links: vec![], committed: false };
        for name in directories {
            if migration.copy_one(&old, name).is_err() {
                migration.result.errors.push(format!("{name}: storage copy rejected or failed"));
            }
        }
        Ok(migration)
    }
    fn copy_one(&mut self, old: &Path, name: &str) -> Result<()> {
        validate_directory(name)?;
        let source = old.join(name);
        open_beneath(old, &source, true).map_err(|_| LocalInstallError::UnsafeSource)?;
        match fs::symlink_metadata(self.destination.join(name)) {
            Ok(_) => return Err(LocalInstallError::Conflict),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {},
            Err(e) => return Err(e.into()),
        }
        let stage = tempfile::Builder::new().prefix(".skill-move-").tempdir_in(&self.destination)?;
        let tree = stage.path().join("tree");
        copy_skill_tree(&source, &tree)?;
        if binding_fingerprint(&source, None)? != binding_fingerprint(&tree, None)? { return Err(LocalInstallError::Conflict); }
        let root = open_beneath(&self.destination, &self.destination, true).map_err(|_| LocalInstallError::UnsafeSource)?;
        let staged = File::open(stage.path())?;
        let identity = fs::symlink_metadata(&tree)?;
        rename_exclusive(&staged, "tree", &root, name)?;
        self.copies.push((root, stage, name.to_owned(), (identity.dev(), identity.ino())));
        self.result.migrated.push(name.to_owned());
        Ok(())
    }
    /// Replace only recorded symlinks with unchanged content. Copies need no
    /// retargeting. Application roots must have been authorized and locked above.
    pub fn retarget(&mut self, old: &Path, binding: &CopyBinding) -> Result<CopyBinding> {
        let Some(previous) = &binding.link_target else { return Ok(binding.clone()); };
        validate_directory(&binding.directory)?;
        if previous != &old.join(&binding.directory)
            || fs::read_link(&binding.destination)? != *previous
            || binding_fingerprint(&binding.destination, Some(previous))? != binding.fingerprint {
            return Err(LocalInstallError::Conflict);
        }
        let parent = binding.destination.parent().ok_or(LocalInstallError::UnsafeSource)?;
        let root = open_beneath(parent, parent, true).map_err(|_| LocalInstallError::UnsafeSource)?;
        if !self._locks.iter().any(|locked| {
            use std::os::unix::fs::MetadataExt;
            locked.metadata().ok().zip(root.metadata().ok()).is_some_and(|(a,b)| a.dev() == b.dev() && a.ino() == b.ino())
        }) { return Err(LocalInstallError::UnsafeSource); }
        let stage = tempfile::Builder::new().prefix(".skill-move-link-").tempdir_in(parent)?;
        let staged = File::open(stage.path())?;
        let next = self.destination.join(&binding.directory);
        symlink(&next, stage.path().join("next"))?;
        let fingerprint = binding_fingerprint(&stage.path().join("next"), Some(&next))?;
        rename_exclusive(&root, &binding.directory, &staged, "previous")?;
        // Register rollback before publication, including the failure path.
        self.links.push((root, stage, binding.directory.clone(), next.clone()));
        let (root, stage, name, _) = self.links.last().ok_or(LocalInstallError::Conflict)?;
        if binding_fingerprint(&stage.path().join("previous"), Some(previous))? != binding.fingerprint {
            return Err(LocalInstallError::Conflict);
        }
        rename_exclusive(&File::open(stage.path())?, "next", root, name)?;
        let mut updated = binding.clone(); updated.link_target = Some(next);
        updated.fingerprint = fingerprint;
        Ok(updated)
    }
    pub fn finish(mut self) -> StorageMoveResult {
        self.committed = true;
        self.result.switched = true;
        self.result.clone()
    }
    pub fn rollback(&mut self) -> Result<()> {
        while let Some((root, stage, name, next)) = self.links.last() {
            let parent = stage.path().parent().ok_or(LocalInstallError::UnsafeSource)?;
            match fs::read_link(parent.join(name)) {
                Ok(target) if target == *next => {
                    rename_exclusive(root, name, &File::open(stage.path())?, "reverted")?;
                },
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {},
                _ => return Err(LocalInstallError::Conflict),
            }
            rename_exclusive(&File::open(stage.path())?, "previous", root, name)?;
            self.links.pop();
        }
        while let Some((root, stage, name, identity)) = self.copies.last() {
            let current = fs::symlink_metadata(self.destination.join(name))?;
            if (current.dev(), current.ino()) != *identity { return Err(LocalInstallError::Conflict); }
            rename_exclusive(root, name, &File::open(stage.path())?, "tree")?;
            self.copies.pop();
        }
        Ok(())
    }
}
impl Drop for StorageMove {
    fn drop(&mut self) {
        if !self.committed && self.rollback().is_err() {
            // A displaced link/tree may be the only recoverable copy. Never let
            // TempDir cleanup erase it when an outside writer blocks rollback.
            for (_, stage, _, _) in self.links.drain(..) { let _ = stage.keep(); }
            for (_, stage, _, _) in self.copies.drain(..) { let _ = stage.keep(); }
        }
    }
}
