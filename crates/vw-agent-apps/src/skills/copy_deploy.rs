//! Authorized host-local copy publication. The caller commits the returned binding
//! in its settings transaction, then finishes; failed commits must roll back.
use std::{fs::{self, File}, io::{self, Read}, os::{fd::AsRawFd, unix::ffi::OsStrExt}, path::{Path, PathBuf}};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use super::{app_paths::AppSkillsPath, local_install::{LocalInstallError, copy_skill_tree, rename_exclusive, validate_directory}};
use crate::providers::paths::open_beneath;

type Result<T> = std::result::Result<T, LocalInstallError>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CopyBinding {
    pub directory: String,
    pub destination: PathBuf,
    pub fingerprint: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link_target: Option<PathBuf>,
}

pub struct CopyDeploy {
    root: File,
    stage: Option<tempfile::TempDir>,
    stage_dir: File,
    binding: CopyBinding,
    previous: Option<String>,
    published: bool,
}

impl CopyDeploy {
    /// All paths must be resolved and authorized on this execution host. The
    /// existing application root is required; no implicit parent creation occurs.
    pub fn prepare(ssot: &Path, target: &AppSkillsPath, directory: &str, saved: Option<&CopyBinding>) -> Result<Self> {
        Self::prepare_method(ssot, target, directory, saved, super::link_deploy::SyncMethod::Copy)
    }

    pub(crate) fn prepare_method(ssot: &Path, target: &AppSkillsPath, directory: &str, saved: Option<&CopyBinding>, method: super::link_deploy::SyncMethod) -> Result<Self> {
        validate_directory(directory)?;
        target.require_distribution().map_err(|_| LocalInstallError::UnsafeSource)?;
        if !ssot.is_absolute() || !target.directory.is_absolute() { return Err(LocalInstallError::UnsafeSource); }
        Self::prepare_tree(ssot, &target.directory, directory, saved, method)
    }

    pub(super) fn prepare_tree(ssot: &Path, destination_root: &Path, directory: &str,
        saved: Option<&CopyBinding>, method: super::link_deploy::SyncMethod) -> Result<Self> {
        Self::prepare_tree_locked(ssot, destination_root, directory, saved, method, None)
    }

    pub(crate) fn prepare_tree_locked(ssot: &Path, destination_root: &Path, directory: &str,
        saved: Option<&CopyBinding>, method: super::link_deploy::SyncMethod,
        locked: Option<&File>) -> Result<Self> {
        use super::link_deploy::SyncMethod;
        validate_directory(directory)?;
        if !ssot.is_absolute() || !destination_root.is_absolute() { return Err(LocalInstallError::UnsafeSource); }
        let source_root = fs::canonicalize(ssot)?;
        let destination_root = fs::canonicalize(destination_root)?;
        if source_root.starts_with(&destination_root) || destination_root.starts_with(&source_root) {
            return Err(LocalInstallError::UnsafeSource);
        }
        // Reject symlink components rather than trusting canonicalization alone.
        let root = match locked {
            Some(file) => file.try_clone()?,
            None => open_beneath(&destination_root, &destination_root, true).map_err(|_| LocalInstallError::UnsafeSource)?,
        };
        let source = source_root.join(directory);
        open_beneath(ssot, &source, true).map_err(|_| LocalInstallError::UnsafeSource)?;
        // Serialize all managed copy changes through the physical app root.
        if unsafe { libc::flock(root.as_raw_fd(), libc::LOCK_EX) } != 0 { return Err(io::Error::last_os_error().into()); }
        let destination = destination_root.join(directory);
        // A missing managed projection may be recreated during resync, but its
        // saved identity and mode still must match the authorized target.
        if let Some(saved) = saved {
            if saved.directory != directory || saved.destination != destination
                || (method == SyncMethod::Copy && saved.link_target.is_some())
                || (method == SyncMethod::Symlink && saved.link_target.is_none())
                || saved.link_target.as_ref().is_some_and(|path| path != &source) {
                return Err(LocalInstallError::Conflict);
            }
        }
        let previous = match fs::symlink_metadata(&destination) {
            Ok(_) => {
                let saved = saved.ok_or(LocalInstallError::Conflict)?;
                let current = binding_fingerprint(&destination, saved.link_target.as_deref())?;
                if current != saved.fingerprint { return Err(LocalInstallError::Conflict); }
                Some(current)
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => None,
            Err(e) => return Err(e.into()),
        };
        let stage = tempfile::Builder::new().prefix(".skill-copy-").tempdir_in(&destination_root)?;
        let stage_dir = open_beneath(&destination_root, stage.path(), true).map_err(|_| LocalInstallError::UnsafeSource)?;
        let tree = stage.path().join("tree");
        // Auto retains an existing binding's actual mode. Only the symlink
        // syscall failure on a new binding may fall back to a checked copy.
        let use_link = saved.map_or(method != SyncMethod::Copy, |binding| binding.link_target.is_some());
        let link_target = if use_link {
            match std::os::unix::fs::symlink(&source, &tree) {
                Ok(()) => Some(source.clone()),
                Err(_) if method == SyncMethod::Auto && saved.is_none() => {
                    copy_skill_tree(&source, &tree)?;
                    None
                }
                Err(error) => return Err(error.into()),
            }
        } else {
            copy_skill_tree(&source, &tree)?;
            None
        };
        let binding = CopyBinding { directory: directory.into(), destination,
            fingerprint: binding_fingerprint(&tree, link_target.as_deref())?, link_target };
        Ok(Self { root, stage: Some(stage), stage_dir, binding, previous, published: false })
    }

    pub fn binding(&self) -> &CopyBinding { &self.binding }

    pub fn publish(&mut self) -> Result<()> {
        if self.published { return Err(LocalInstallError::Conflict); }
        if let Some(previous) = &self.previous {
            if binding_fingerprint(&self.binding.destination, self.binding.link_target.as_deref())? != *previous { return Err(LocalInstallError::Conflict); }
            self.exchange()?;
            self.published = true;
            // Check the displaced tree too: an external writer may have changed
            // it between preflight and the atomic exchange. Restore that content.
            let displaced = self.stage.as_ref().ok_or(LocalInstallError::Conflict)?.path().join("tree");
            if binding_fingerprint(&displaced, self.binding.link_target.as_deref())? != *previous {
                self.rollback()?;
                return Err(LocalInstallError::Conflict);
            }
        } else {
            rename_exclusive(&self.stage_dir, "tree", &self.root, &self.binding.directory)?;
        }
        self.published = true;
        Ok(())
    }

    pub fn rollback(&mut self) -> Result<()> {
        if !self.published { return Ok(()); }
        if binding_fingerprint(&self.binding.destination, self.binding.link_target.as_deref())? != self.binding.fingerprint { return Err(LocalInstallError::Conflict); }
        if self.previous.is_some() { self.exchange()?; }
        else { rename_exclusive(&self.root, &self.binding.directory, &self.stage_dir, "tree")?; }
        self.published = false;
        Ok(())
    }

    pub fn finish(mut self) -> CopyBinding { self.published = false; self.binding.clone() }

    fn exchange(&self) -> Result<()> {
        let name = std::ffi::CString::new(self.binding.directory.as_bytes()).map_err(|_| LocalInstallError::InvalidDirectory)?;
        // SAFETY: descriptors and NUL terminated names are live throughout the call.
        let result = unsafe { libc::renameatx_np(self.stage_dir.as_raw_fd(), c"tree".as_ptr(), self.root.as_raw_fd(), name.as_ptr(), libc::RENAME_SWAP) };
        if result != 0 { return Err(io::Error::last_os_error().into()); }
        Ok(())
    }
}

impl Drop for CopyDeploy {
    fn drop(&mut self) {
        if self.rollback().is_err() {
            // Preserve the previous tree for explicit recovery after outside edits.
            if let Some(stage) = self.stage.take() { let _ = stage.keep(); }
        }
    }
}

/// Includes names, entry kinds and file bytes; links/special files are conflicts.
fn fingerprint(root: &Path) -> Result<String> {
    fn visit(path: &Path, hash: &mut Sha256, entries: &mut usize, bytes: &mut u64, depth: usize) -> Result<()> {
        *entries += 1;
        if *entries > super::archive::MAX_ARCHIVE_ENTRIES || depth > 128 { return Err(LocalInstallError::LimitExceeded); }
        let metadata = fs::symlink_metadata(path)?;
        if metadata.is_dir() {
            hash.update(b"directory");
            let mut children = Vec::new();
            for child in fs::read_dir(path)? {
                children.push(child?);
                if children.len() > super::archive::MAX_ARCHIVE_ENTRIES { return Err(LocalInstallError::LimitExceeded); }
            }
            children.sort_by_key(|child| child.file_name());
            for child in children {
                let name = child.file_name();
                hash.update((name.as_bytes().len() as u64).to_le_bytes());
                hash.update(name.as_bytes());
                visit(&child.path(), hash, entries, bytes, depth + 1)?;
            }
            hash.update(b"end");
        } else if metadata.is_file() {
            hash.update(b"file");
            hash.update(metadata.len().to_le_bytes());
            let mut file = open_beneath(path.parent().ok_or(LocalInstallError::UnsafeSource)?, path, false).map_err(|_| LocalInstallError::UnsafeSource)?;
            let mut buffer = [0; 8192];
            loop {
                let count = file.read(&mut buffer)?;
                if count == 0 { break; }
                *bytes += count as u64;
                if *bytes > super::archive::MAX_ARCHIVE_TOTAL_BYTES { return Err(LocalInstallError::LimitExceeded); }
                hash.update(&buffer[..count]);
            }
        } else { return Err(LocalInstallError::Conflict); }
        Ok(())
    }
    let mut hash = Sha256::new();
    visit(root, &mut hash, &mut 0, &mut 0, 0)?;
    Ok(format!("{:x}", hash.finalize()))
}

// Link ownership covers the link itself, never the mutable SSOT contents.
pub(super) fn binding_fingerprint(path: &Path, link_target: Option<&Path>) -> Result<String> {
    match link_target {
        None => fingerprint(path),
        Some(target) => {
            if !fs::symlink_metadata(path)?.file_type().is_symlink() || fs::read_link(path)? != target {
                return Err(LocalInstallError::Conflict);
            }
            let mut hash = Sha256::new();
            hash.update(b"skill-symlink\0");
            hash.update(target.as_os_str().as_bytes());
            Ok(format!("{:x}", hash.finalize()))
        }
    }
}
