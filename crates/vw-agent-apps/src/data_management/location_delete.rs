//! Category cleanup of executor-authorized, isolated backup roots (macOS).
use super::{
    backup_create_files::lock_backup,
    locations::{BackupLocationRoots, Matcher, categories, directory, names, open_at},
    safety_snapshot::{SafetySnapshotError, SafetySnapshotScope},
};
use std::{
    ffi::CStr,
    fs::File,
    os::{fd::AsRawFd, unix::fs::MetadataExt},
    path::PathBuf,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocationDeleteError {
    UnknownCategory,
    DatabaseNotDeletable,
    /// Domain backup guards/cache invalidation have not been exposed by the owner.
    OwnerRequired,
    Storage(SafetySnapshotError),
}
impl From<SafetySnapshotError> for LocationDeleteError {
    fn from(error: SafetySnapshotError) -> Self {
        Self::Storage(error)
    }
}
impl std::fmt::Display for LocationDeleteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "backup category deletion: {self:?}")
    }
}
impl std::error::Error for LocationDeleteError {}

#[derive(Debug, PartialEq, Eq)]
pub struct UndeletedEntry {
    /// Relative to the category root; never an executor filesystem path.
    pub entry: PathBuf,
    pub reason: SafetySnapshotError,
}
#[derive(Debug, Default, PartialEq, Eq)]
pub struct LocationDeleteReport {
    /// Bytes of regular files actually unlinked, including partial cleanup.
    pub freed_bytes: u64,
    pub undeleted: Vec<UndeletedEntry>,
}

fn remove_entry(
    parent: &File,
    name: &CStr,
    relative: PathBuf,
    depth: usize,
    report: &mut LocationDeleteReport,
) -> Result<(), SafetySnapshotError> {
    use SafetySnapshotError as Error;
    if depth > 128 {
        return Err(Error::TooLarge);
    }
    let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
    // SAFETY: live descriptor, terminated basename and writable stat storage.
    if unsafe {
        libc::fstatat(
            parent.as_raw_fd(),
            name.as_ptr(),
            stat.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    } != 0
    {
        return Err(Error::StorageUnavailable);
    }
    let stat = unsafe { stat.assume_init() };
    let is_dir = match stat.st_mode & libc::S_IFMT {
        libc::S_IFDIR => true,
        libc::S_IFREG => false,
        _ => return Err(Error::UnsafeStorage),
    };
    let file = open_at(parent, name, is_dir).map_err(|_| Error::UnsafeStorage)?;
    let meta = file.metadata().map_err(|_| Error::StorageUnavailable)?;
    if meta.ino() != stat.st_ino
        || meta.dev() != stat.st_dev as u64
        || (!is_dir && meta.nlink() != 1)
    {
        return Err(Error::UnsafeStorage);
    }
    // A live restore source holding the shared inode lock is never removed.
    lock_backup(&file, true)?;
    if is_dir {
        for child in names(&file)? {
            let path = relative.join(std::ffi::OsStr::from_bytes(child.to_bytes()));
            if let Err(reason) = remove_entry(&file, &child, path.clone(), depth + 1, report) {
                report.undeleted.push(UndeletedEntry { entry: path, reason });
            }
        }
    }
    // Check the name still addresses the opened inode before unlinking it.
    let mut current = std::mem::MaybeUninit::<libc::stat>::uninit();
    if unsafe {
        libc::fstatat(
            parent.as_raw_fd(),
            name.as_ptr(),
            current.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    } != 0
    {
        return Err(Error::StorageUnavailable);
    }
    let current = unsafe { current.assume_init() };
    if current.st_ino != stat.st_ino || current.st_dev != stat.st_dev {
        return Err(Error::Conflict);
    }
    if unsafe {
        libc::unlinkat(
            parent.as_raw_fd(),
            name.as_ptr(),
            if is_dir { libc::AT_REMOVEDIR } else { 0 },
        )
    } != 0
    {
        return Err(Error::StorageUnavailable);
    }
    if !is_dir {
        report.freed_bytes = report.freed_bytes.saturating_add(meta.len());
    }
    parent.sync_all().map_err(|_| Error::StorageUnavailable)
}
use std::os::unix::ffi::OsStrExt;

/// Authorize the roots and identity first. Roots must contain only this scope.
/// Category writers/readers must serialize cleanup and hold inode locks for live
/// sources. Domain categories fail closed until their owner exposes that protocol.
/// Partial errors are returned in the report; never discard already freed bytes.
pub fn delete_location(
    roots: &BackupLocationRoots<'_>,
    scope: &SafetySnapshotScope,
    id: &str,
) -> Result<LocationDeleteReport, LocationDeleteError> {
    if roots.scope != scope {
        return Err(SafetySnapshotError::ScopeMismatch.into());
    }
    if scope.host_id.trim().is_empty() || scope.principal_id.trim().is_empty() {
        return Err(SafetySnapshotError::InvalidSelection.into());
    }
    if id == "database" {
        return Err(LocationDeleteError::DatabaseNotDeletable);
    }
    let (_, path, matcher) = categories(roots)
        .into_iter()
        .find(|(key, _, _)| *key == id)
        .ok_or(LocationDeleteError::UnknownCategory)?;
    if id == "skills" || id.starts_with("codexHistory") {
        return Err(LocationDeleteError::OwnerRequired);
    }
    let mut report = LocationDeleteReport::default();
    let Some(dir) = directory(&path)? else {
        return Ok(report);
    };
    lock_backup(&dir, true)?;
    delete_contents(&dir, matcher, &mut report)?;
    Ok(report)
}

fn delete_contents(
    dir: &File,
    matcher: Matcher,
    report: &mut LocationDeleteReport,
) -> Result<(), SafetySnapshotError> {
    for name in names(dir)? {
        if matcher.is_some_and(|m| !name.to_str().is_ok_and(m)) {
            continue;
        }
        // File categories never recurse into a directory with a matching name.
        if matcher.is_some() {
            let file = match open_at(dir, &name, false) {
                Ok(file) => file,
                Err(_) => {
                    report.undeleted.push(UndeletedEntry {
                        entry: PathBuf::from(std::ffi::OsStr::from_bytes(name.to_bytes())),
                        reason: SafetySnapshotError::UnsafeStorage,
                    });
                    continue;
                }
            };
            match file.metadata() {
                Ok(meta) if meta.is_file() => {}
                Ok(_) => continue,
                Err(_) => {
                    report.undeleted.push(UndeletedEntry {
                        entry: PathBuf::from(std::ffi::OsStr::from_bytes(name.to_bytes())),
                        reason: SafetySnapshotError::StorageUnavailable,
                    });
                    continue;
                }
            }
        }
        let path = PathBuf::from(std::ffi::OsStr::from_bytes(name.to_bytes()));
        if let Err(reason) = remove_entry(dir, &name, path.clone(), 0, report) {
            report.undeleted.push(UndeletedEntry { entry: path, reason });
        }
    }
    Ok(())
}
