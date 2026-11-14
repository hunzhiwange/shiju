//! Host-local, staged installation. Call only after authorizing both paths on
//! this executor; a remote host ID never authorizes a gateway-local path.

use serde::{Deserialize, Serialize};
use std::{
    ffi::CString,
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    os::{
        fd::AsRawFd,
        unix::fs::MetadataExt,
    },
    path::{Path, PathBuf},
};
use tempfile::TempDir;

use super::archive::{MAX_ARCHIVE_ENTRIES, MAX_ARCHIVE_TOTAL_BYTES};
use crate::providers::paths::open_beneath;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalSkill {
    pub directory: String,
    pub name: String,
    pub description: Option<String>,
    /// Canonical selected directory on the authorized execution host.
    pub source: PathBuf,
}

#[derive(Debug)]
pub enum LocalInstallError {
    InvalidDirectory,
    InvalidSkill,
    UnsafeSource,
    Conflict,
    LimitExceeded,
    Io(io::Error),
}

impl std::fmt::Display for LocalInstallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidDirectory => "invalid skill installation directory",
            Self::InvalidSkill => "invalid skill document or metadata",
            Self::UnsafeSource => "unsafe skill source or overlapping storage",
            Self::Conflict => "skill installation directory already exists",
            Self::LimitExceeded => "skill installation exceeds copy limits",
            Self::Io(_) => "skill filesystem operation failed",
        })
    }
}
impl std::error::Error for LocalInstallError {}
impl From<io::Error> for LocalInstallError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}
type Result<T> = std::result::Result<T, LocalInstallError>;

pub fn validate_directory(name: &str) -> Result<()> {
    if name.is_empty()
        || name.len() > 255
        || name.starts_with('.')
        || name.chars().any(|c| c.is_control() || matches!(c, '/' | '\\' | ':'))
    {
        return Err(LocalInstallError::InvalidDirectory);
    }
    Ok(())
}

/// Keeps an exclusive physical-root lock through settings commit or rollback.
/// The SSOT root must already exist and be resolved by the execution contract.
pub struct LocalInstall {
    root: File,
    _stage: TempDir,
    stage_directory: File,
    identity: (u64, u64),
    skill: LocalSkill,
    published: bool,
}

impl LocalInstall {
    pub fn prepare(ssot: &Path, source: &Path, directory: &str) -> Result<Self> {
        Self::prepare_checked(ssot, source, directory, &|| Ok(()))
    }

    pub fn prepare_checked(ssot: &Path, source: &Path, directory: &str,
        check: &(dyn Fn() -> io::Result<()> + Sync)) -> Result<Self> {
        check()?;
        validate_directory(directory)?;
        if !ssot.is_absolute() || !source.is_absolute() {
            return Err(LocalInstallError::UnsafeSource);
        }
        let ssot = fs::canonicalize(ssot)?;
        let source = fs::canonicalize(source)?;
        if source.starts_with(&ssot) || ssot.starts_with(&source) {
            return Err(LocalInstallError::UnsafeSource);
        }
        let root = open_beneath(&ssot, &ssot, true)
            .map_err(|_| LocalInstallError::UnsafeSource)?;
        // SAFETY: root owns a live directory descriptor, retained until drop.
        loop {
            check()?;
            // Nonblocking attempts keep cancellation responsive while another
            // installation owns the physical root lock.
            if unsafe { libc::flock(root.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0 {
                break;
            }
            let error = io::Error::last_os_error();
            if error.kind() != io::ErrorKind::WouldBlock { return Err(error.into()); }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let destination = ssot.join(directory);
        match fs::symlink_metadata(&destination) {
            Ok(_) => return Err(LocalInstallError::Conflict),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        let stage = tempfile::Builder::new().prefix(".skill-install-").tempdir_in(&ssot)?;
        let stage_directory = open_beneath(&ssot, stage.path(), true)
            .map_err(|_| LocalInstallError::UnsafeSource)?;
        // Reject a root swapped while the temporary directory was created.
        let current = open_beneath(&ssot, &ssot, true)
            .map_err(|_| LocalInstallError::UnsafeSource)?;
        if identity(&root)? != identity(&current)? {
            return Err(LocalInstallError::UnsafeSource);
        }
        let tree = stage.path().join("tree");
        let mut budget = CopyBudget { entries: 0, bytes: 0 };
        copy_tree(&source, &source, &tree, &mut Vec::new(), &mut budget, check)?;
        let skill = read_metadata(&tree, directory, source)?;
        let tree_file = open_beneath(stage.path(), &tree, true)
            .map_err(|_| LocalInstallError::UnsafeSource)?;
        Ok(Self {
            root,
            _stage: stage,
            stage_directory,
            identity: identity(&tree_file)?,
            skill,
            published: false,
        })
    }

    /// Compare the private staged copy to an explicitly selected scan snapshot.
    pub(super) fn matches_snapshot(&self, hash: &str) -> Result<bool> {
        Ok(super::inventory::compute_dir_hash(&self._stage.path().join("tree"))? == hash)
    }

    pub fn skill(&self) -> &LocalSkill {
        &self.skill
    }

    /// Exclusive rename also rejects dangling links and concurrent external creators.
    pub fn publish(&mut self) -> Result<()> {
        if self.published {
            return Err(LocalInstallError::Conflict);
        }
        rename_exclusive(&self.stage_directory, "tree", &self.root, &self.skill.directory)?;
        self.published = true;
        Ok(())
    }

    /// Call after a failed settings transaction; never remove an external replacement.
    pub fn rollback(&mut self) -> Result<()> {
        if !self.published {
            return Ok(());
        }
        let name = CString::new(self.skill.directory.as_bytes())
            .map_err(|_| LocalInstallError::InvalidDirectory)?;
        let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
        // SAFETY: the descriptor and name are live; stat points to writable storage.
        let result = unsafe {
            libc::fstatat(
                self.root.as_raw_fd(), name.as_ptr(), stat.as_mut_ptr(),
                libc::AT_SYMLINK_NOFOLLOW,
            )
        };
        if result != 0 {
            return Err(io::Error::last_os_error().into());
        }
        // SAFETY: fstatat succeeded and initialized stat.
        let stat = unsafe { stat.assume_init() };
        if (stat.st_dev as u64, stat.st_ino) != self.identity {
            return Err(LocalInstallError::Conflict);
        }
        rename_exclusive(&self.root, &self.skill.directory, &self.stage_directory, "tree")?;
        self.published = false;
        Ok(())
    }

    /// Disarm rollback only after the settings transaction has committed.
    pub fn finish(mut self) -> LocalSkill {
        self.published = false;
        self.skill.clone()
    }
}

impl Drop for LocalInstall {
    fn drop(&mut self) {
        // Explicit callers report rollback failures; this is unwind protection.
        let _ = self.rollback();
    }
}

fn identity(file: &File) -> io::Result<(u64, u64)> {
    let meta = file.metadata()?;
    Ok((meta.dev(), meta.ino()))
}

pub(super) fn rename_exclusive(from: &File, name: &str, to: &File, destination: &str) -> Result<()> {
    let name = CString::new(name).map_err(|_| LocalInstallError::InvalidDirectory)?;
    let destination = CString::new(destination).map_err(|_| LocalInstallError::InvalidDirectory)?;
    // SAFETY: both directory descriptors and NUL-terminated child names are live.
    let result = unsafe {
        libc::renameatx_np(
            from.as_raw_fd(), name.as_ptr(), to.as_raw_fd(), destination.as_ptr(),
            libc::RENAME_EXCL,
        )
    };
    if result != 0 {
        let error = io::Error::last_os_error();
        return Err(if error.kind() == io::ErrorKind::AlreadyExists {
            LocalInstallError::Conflict
        } else {
            error.into()
        });
    }
    Ok(())
}

struct CopyBudget {
    entries: usize,
    bytes: u64,
}

fn copy_tree(
    root: &Path,
    source: &Path,
    destination: &Path,
    active: &mut Vec<PathBuf>,
    budget: &mut CopyBudget,
    check: &(dyn Fn() -> io::Result<()> + Sync),
) -> Result<()> {
    check()?;
    budget.entries += 1;
    budget.bytes = budget.bytes.saturating_add(4096);
    if budget.entries > MAX_ARCHIVE_ENTRIES
        || budget.bytes > MAX_ARCHIVE_TOTAL_BYTES
        || active.len() > 128
    {
        return Err(LocalInstallError::LimitExceeded);
    }
    // Internal links become ordinary copies; external, dangling and cyclic links fail.
    let source = fs::canonicalize(source)?;
    if !source.starts_with(root) || active.contains(&source) {
        return Err(LocalInstallError::UnsafeSource);
    }
    let metadata = fs::symlink_metadata(&source)?;
    if metadata.is_dir() {
        open_beneath(root, &source, true).map_err(|_| LocalInstallError::UnsafeSource)?;
        active.push(source.clone());
        fs::create_dir(destination)?;
        let mut entries = Vec::new();
        for entry in fs::read_dir(&source)? {
            if entries.len() >= MAX_ARCHIVE_ENTRIES.saturating_sub(budget.entries) {
                return Err(LocalInstallError::LimitExceeded);
            }
            entries.push(entry?);
        }
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            copy_tree(root, &entry.path(), &destination.join(entry.file_name()), active, budget, check)?;
        }
        active.pop();
    } else if metadata.is_file() {
        let reader = open_beneath(root, &source, false).map_err(|_| LocalInstallError::UnsafeSource)?;
        let mut writer = OpenOptions::new().write(true).create_new(true).open(destination)?;
        let remaining = MAX_ARCHIVE_TOTAL_BYTES.saturating_sub(budget.bytes);
        let mut reader = reader.take(remaining + 1);
        let mut copied = 0;
        let mut buffer = [0; 16 * 1024];
        loop {
            check()?;
            let count = reader.read(&mut buffer)?;
            if count == 0 { break; }
            writer.write_all(&buffer[..count])?;
            copied += count as u64;
        }
        budget.bytes += copied;
        if budget.bytes > MAX_ARCHIVE_TOTAL_BYTES {
            return Err(LocalInstallError::LimitExceeded);
        }
        writer.flush()?;
    } else {
        return Err(LocalInstallError::UnsafeSource);
    }
    Ok(())
}

fn read_metadata(tree: &Path, directory: &str, source: PathBuf) -> Result<LocalSkill> {
    let file = open_beneath(tree, &tree.join("SKILL.md"), false)
        .map_err(|_| LocalInstallError::InvalidSkill)?;
    let mut content = String::new();
    file.take(1024 * 1024 + 1).read_to_string(&mut content)?;
    if content.len() > 1024 * 1024 || content.trim().is_empty() {
        return Err(LocalInstallError::InvalidSkill);
    }
    #[derive(Default, Deserialize)]
    struct Metadata {
        name: Option<String>,
        description: Option<String>,
    }
    let content = content.trim_start_matches('\u{feff}');
    let mut lines = content.lines();
    let metadata: Metadata = if lines.next() == Some("---") {
        let mut yaml = String::new();
        let mut closed = false;
        for line in lines {
            if line == "---" {
                closed = true;
                break;
            }
            yaml.push_str(line);
            yaml.push('\n');
        }
        if !closed {
            return Err(LocalInstallError::InvalidSkill);
        }
        if yaml.trim().is_empty() {
            Metadata::default()
        } else {
            serde_yaml::from_str(&yaml).map_err(|_| LocalInstallError::InvalidSkill)?
        }
    } else {
        Metadata::default()
    };
    let name = metadata.name.unwrap_or_else(|| directory.to_owned());
    if name.trim().is_empty() {
        return Err(LocalInstallError::InvalidSkill);
    }
    Ok(LocalSkill { directory: directory.to_owned(), name, description: metadata.description, source })
}

/// Shared bounded copier for application projections; never executes copied files.
pub(crate) fn copy_skill_tree(source: &Path, destination: &Path) -> Result<()> {
    let source = fs::canonicalize(source)?;
    let mut budget = CopyBudget { entries: 0, bytes: 0 };
    copy_tree(&source, &source, destination, &mut Vec::new(), &mut budget, &|| Ok(()))?;
    read_metadata(destination, "projection", source)?;
    Ok(())
}
