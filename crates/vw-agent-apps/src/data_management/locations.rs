//! Executor-owned, scope-bound backup usage. No ambient HOME discovery.
use super::{
    backup_list::list_local_backups,
    safety_snapshot::{SafetySnapshotError as Error, SafetySnapshotScope},
};
use serde::Serialize;
use std::{
    ffi::{CStr, CString},
    fs::File,
    os::{
        fd::{AsRawFd, FromRawFd, IntoRawFd},
        unix::ffi::OsStrExt,
    },
    path::{Component, Path},
    time::SystemTime,
};

/// These roots must be selected by the executor after authorization, never from
/// request paths. Each root contains only the supplied tenant/project/host scope.
/// Legacy CC files are measured only after explicit placement in this scope.
pub struct BackupLocationRoots<'a> {
    pub scope: &'a SafetySnapshotScope,
    pub local_backups: &'a Path,
    pub app: &'a Path,
    pub device: &'a Path,
    pub environment: &'a Path,
}

#[derive(Debug, Clone, Serialize)]
pub struct BackupLocation {
    pub id: &'static str,
    pub size_bytes: u64,
    /// Whole-directory categories count immediate entries, file categories count files.
    pub item_count: u64,
    pub last_modified: Option<SystemTime>,
    pub deletable: bool,
}

fn open_at(parent: &File, name: &CStr, directory: bool) -> Result<File, std::io::Error> {
    // SAFETY: borrowed fd and terminated basename; returned fd has one owner.
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY
                | libc::O_NOFOLLOW
                | libc::O_CLOEXEC
                | libc::O_NONBLOCK
                | if directory { libc::O_DIRECTORY } else { 0 },
        )
    };
    if fd < 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(unsafe { File::from_raw_fd(fd) })
}

// Walk every absolute-path component without following links, including ancestors.
fn directory(path: &Path) -> Result<Option<File>, Error> {
    if !path.is_absolute() {
        return Err(Error::UnsafeStorage);
    }
    let mut dir = File::open("/").map_err(|_| Error::StorageUnavailable)?;
    for part in path.components() {
        let Component::Normal(part) = part else {
            if part == Component::RootDir {
                continue;
            }
            return Err(Error::UnsafeStorage);
        };
        let name = CString::new(part.as_bytes()).map_err(|_| Error::UnsafeStorage)?;
        match open_at(&dir, &name, true) {
            Ok(next) => dir = next,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(Error::StorageUnavailable),
        }
    }
    Ok(Some(dir))
}

fn names(dir: &File) -> Result<Vec<CString>, Error> {
    let fd = open_at(dir, c".", true).map_err(|_| Error::StorageUnavailable)?.into_raw_fd();
    // SAFETY: fdopendir takes ownership on success, otherwise close it ourselves.
    let stream = unsafe { libc::fdopendir(fd) };
    if stream.is_null() {
        unsafe {
            libc::close(fd);
        }
        return Err(Error::StorageUnavailable);
    }
    struct Stream(*mut libc::DIR);
    impl Drop for Stream {
        fn drop(&mut self) {
            unsafe {
                libc::closedir(self.0);
            }
        }
    }
    let stream = Stream(stream);
    let mut result = Vec::new();
    loop {
        // SAFETY: macOS thread-local errno and uniquely owned stream.
        unsafe {
            *libc::__error() = 0;
        }
        let entry = unsafe { libc::readdir(stream.0) };
        if entry.is_null() {
            if unsafe { *libc::__error() } != 0 {
                return Err(Error::StorageUnavailable);
            }
            break;
        }
        let name = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) };
        if !matches!(name.to_bytes(), b"." | b".." | b".DS_Store") {
            result.push(name.to_owned());
        }
    }
    Ok(result)
}

fn walk(
    dir: &File,
    usage: &mut BackupLocation,
    matcher: Option<fn(&str) -> bool>,
    depth: usize,
) -> Result<(), Error> {
    if depth > 128 {
        return Err(Error::TooLarge);
    }
    for name in names(dir)? {
        if matcher.is_some_and(|m| !name.to_str().is_ok_and(m)) {
            continue;
        }
        // Inspect without following links; openat below also refuses replacement links.
        let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
        if unsafe {
            libc::fstatat(
                dir.as_raw_fd(),
                name.as_ptr(),
                stat.as_mut_ptr(),
                libc::AT_SYMLINK_NOFOLLOW,
            )
        } != 0
        {
            return Err(Error::StorageUnavailable);
        }
        let stat = unsafe { stat.assume_init() };
        let kind = stat.st_mode & libc::S_IFMT;
        if kind == libc::S_IFLNK {
            continue;
        }
        if kind != libc::S_IFREG && kind != libc::S_IFDIR {
            return Err(Error::UnsafeStorage);
        }
        if matcher.is_some() && kind != libc::S_IFREG {
            continue;
        }
        let file =
            open_at(dir, &name, kind == libc::S_IFDIR).map_err(|_| Error::StorageUnavailable)?;
        let meta = file.metadata().map_err(|_| Error::StorageUnavailable)?;
        if depth == 0 {
            usage.item_count = usage.item_count.checked_add(1).ok_or(Error::TooLarge)?;
        }
        if meta.is_dir() {
            walk(&file, usage, None, depth + 1)?;
        } else if meta.is_file() {
            usage.size_bytes = usage.size_bytes.checked_add(meta.len()).ok_or(Error::TooLarge)?;
            let modified = meta.modified().map_err(|_| Error::StorageUnavailable)?;
            usage.last_modified =
                Some(usage.last_modified.map_or(modified, |old| old.max(modified)));
        } else {
            return Err(Error::UnsafeStorage);
        }
    }
    Ok(())
}

/// Missing categories return zero. Unreadable categories fail explicitly, never
/// masquerade as zero. Paths and foreign archive diagnostics are not returned.
/// Database usage reuses task 23's scope-filtered native archive listing.
pub fn list_locations(
    roots: &BackupLocationRoots<'_>,
    scope: &SafetySnapshotScope,
) -> Result<Vec<BackupLocation>, Error> {
    if roots.scope != scope {
        return Err(Error::ScopeMismatch);
    }
    if scope.host_id.trim().is_empty() || scope.principal_id.trim().is_empty() {
        return Err(Error::InvalidSelection);
    }
    let mut database = BackupLocation {
        id: "database",
        size_bytes: 0,
        item_count: 0,
        last_modified: None,
        deletable: false,
    };
    if directory(roots.local_backups)?.is_some() {
        for backup in list_local_backups(roots.local_backups, scope)? {
            database.size_bytes =
                database.size_bytes.checked_add(backup.size_bytes).ok_or(Error::TooLarge)?;
            database.item_count += 1;
            if let Ok(ms) = u64::try_from(backup.created_at_ms) {
                let time = SystemTime::UNIX_EPOCH
                    .checked_add(std::time::Duration::from_millis(ms))
                    .ok_or(Error::Corrupt)?;
                database.last_modified =
                    Some(database.last_modified.map_or(time, |old| old.max(time)));
            }
        }
    }
    type Matcher = Option<fn(&str) -> bool>;
    let categories: [(_, _, Matcher); 13] = [
        ("skills", roots.app.join("skill-backups"), None),
        ("codexHistoryUnify", roots.app.join("backups/codex-official-history-unify-v1"), None),
        (
            "codexHistoryUnifyRestore",
            roots.app.join("backups/codex-official-history-unify-restore-v1"),
            None,
        ),
        (
            "codexHistoryProviderMigration",
            roots.app.join("backups/codex-history-provider-migration-v1"),
            None,
        ),
        ("liveFirstWrite", roots.device.join("backups/live-first-write"), None),
        ("hermes", roots.app.join("backups/hermes"), None),
        ("openclaw", roots.app.join("backups/openclaw"), None),
        ("proxyLiveBackup", roots.device.join("backups/proxy-live-backup"), None),
        (
            "envVars",
            roots.environment.to_owned(),
            Some(|n| n.starts_with("env-backup-") && n.ends_with(".json")),
        ),
        (
            "legacyConfigBackups",
            roots.app.join("backups"),
            Some(|n| n.starts_with("backup_") && n.ends_with(".json")),
        ),
        ("legacyConfigArchive", roots.app.to_owned(), Some(|n| n == "config.json.migrated")),
        (
            "legacyDbUpgrade",
            roots.app.to_owned(),
            Some(|n| n.starts_with("cc-switch.db.") && n.contains("-backup-")),
        ),
        ("legacyToolSearch", roots.app.join("toolsearch-backups"), None),
    ];
    let mut result = vec![database];
    for (id, path, matcher) in categories {
        let mut usage = BackupLocation {
            id,
            size_bytes: 0,
            item_count: 0,
            last_modified: None,
            deletable: true,
        };
        if let Some(dir) = directory(&path)? {
            walk(&dir, &mut usage, matcher, 0)?;
        }
        result.push(usage);
    }
    Ok(result)
}
