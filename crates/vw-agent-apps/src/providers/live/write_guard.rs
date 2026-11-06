//! Host-local supplier configuration writes, independent of tenant identity.
//!
//! Callers must authorize the directory before locking and retain the guard for
//! the entire read/prepare/commit transaction. External processes and other
//! executor processes do not participate. No network/file API is exposed here.
use std::{
    collections::HashSet,
    ffi::{CString, OsStr},
    fs::{self, File, OpenOptions},
    io::Write,
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::{
            ffi::OsStrExt,
            fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
        },
    },
    path::{Component, Path},
    sync::{
        Condvar, Mutex, OnceLock,
        atomic::{AtomicU64, Ordering},
    },
};
use vw_api_types::agent_apps::AppId;

#[derive(Clone, Copy, Debug)]
pub enum FilePrivacy {
    Private,
    Shared,
}

/// Fixed errors deliberately exclude credentials, input bytes and host paths.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WriteError {
    InvalidResource,
    Io,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum LockKey {
    // This static table belongs to the actual local executor, not a client host ID.
    App(AppId),
    // Aliased paths/host IDs/tenants cannot bypass physical directory ownership.
    Directory(u64, u64),
}

type LockTable = (Mutex<HashSet<LockKey>>, Condvar);
fn locks() -> &'static LockTable {
    static LOCKS: OnceLock<LockTable> = OnceLock::new();
    LOCKS.get_or_init(|| (Mutex::new(HashSet::new()), Condvar::new()))
}

#[derive(Debug)]
pub struct AppWriteGuard {
    directory: File,
    keys: [LockKey; 2],
}

/// Non-reentrant, blocking lock. Call outside async runtime workers. Same-app
/// writes across directories and different apps sharing a directory serialize.
/// The already-authorized directory must exist; the final component cannot be
/// a symlink. Parent traversal/host authorization remain the caller's boundary.
pub fn lock_app(app: AppId, directory: &Path) -> Result<AppWriteGuard, WriteError> {
    let directory = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(directory)
        .map_err(|_| WriteError::InvalidResource)?;
    let metadata = directory.metadata().map_err(|_| WriteError::Io)?;
    let keys = [LockKey::App(app), LockKey::Directory(metadata.dev(), metadata.ino())];
    let (held, released) = locks();
    let mut held = held.lock().unwrap_or_else(|e| e.into_inner());
    while keys.iter().any(|key| held.contains(key)) {
        held = released.wait(held).unwrap_or_else(|e| e.into_inner());
    }
    held.extend(keys.iter().cloned());
    Ok(AppWriteGuard { directory, keys })
}

impl Drop for AppWriteGuard {
    fn drop(&mut self) {
        let (held, released) = locks();
        let mut held = held.lock().unwrap_or_else(|e| e.into_inner());
        for key in &self.keys {
            held.remove(key);
        }
        released.notify_all();
    }
}

impl AppWriteGuard {
    /// Atomically replaces one direct child of the authorized directory. Private
    /// files become exactly 0600; shared files retain existing rwx permissions,
    /// new shared files start 0600. No directory creation or arbitrary paths.
    /// A failed preparation/rename preserves the destination and removes staging.
    /// Mutable access prevents sharing a guard for concurrent writes.
    /// Durability/pending and external-write conflict checks belong to transactions.
    pub fn write(
        &mut self,
        name: &OsStr,
        bytes: &[u8],
        privacy: FilePrivacy,
    ) -> Result<(), WriteError> {
        let mut components = Path::new(name).components();
        if !matches!(components.next(), Some(Component::Normal(part)) if part == name)
            || components.next().is_some()
        {
            return Err(WriteError::InvalidResource);
        }
        let name = CString::new(name.as_bytes()).map_err(|_| WriteError::InvalidResource)?;
        let fd = self.directory.as_raw_fd();
        let mut metadata = std::mem::MaybeUninit::<libc::stat>::uninit();
        // SAFETY: owned directory fd and valid C string; fstatat initializes stat on success.
        let result = unsafe {
            libc::fstatat(fd, name.as_ptr(), metadata.as_mut_ptr(), libc::AT_SYMLINK_NOFOLLOW)
        };
        let existing = if result == 0 {
            // SAFETY: successful fstatat initialized the entire stat.
            let metadata = unsafe { metadata.assume_init() };
            if metadata.st_mode & libc::S_IFMT != libc::S_IFREG {
                return Err(WriteError::InvalidResource);
            }
            Some(u32::from(metadata.st_mode) & 0o777)
        } else if std::io::Error::last_os_error().kind() == std::io::ErrorKind::NotFound {
            None
        } else {
            return Err(WriteError::Io);
        };
        let mode = match privacy {
            FilePrivacy::Private => 0o600,
            FilePrivacy::Shared => existing.unwrap_or(0o600),
        };
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        // create_new relative to the retained directory fd; never follow a temp symlink.
        for _ in 0..32 {
            let temp = CString::new(format!(
                ".shiju-live-{}-{}",
                std::process::id(),
                COUNTER.fetch_add(1, Ordering::Relaxed)
            ))
            .expect("ASCII name");
            if temp == name {
                continue;
            }
            // SAFETY: both C string and fd are valid; returned fd is uniquely owned below.
            let raw = unsafe {
                libc::openat(
                    fd,
                    temp.as_ptr(),
                    libc::O_WRONLY
                        | libc::O_CREAT
                        | libc::O_EXCL
                        | libc::O_CLOEXEC
                        | libc::O_NOFOLLOW,
                    0o600 as libc::c_uint,
                )
            };
            if raw < 0 {
                if std::io::Error::last_os_error().kind() == std::io::ErrorKind::AlreadyExists {
                    continue;
                }
                return Err(WriteError::Io);
            }
            // SAFETY: openat returned a newly owned file descriptor.
            let mut file = unsafe { File::from_raw_fd(raw) };
            let staged = file
                .write_all(bytes)
                .and_then(|()| file.set_permissions(fs::Permissions::from_mode(mode)))
                .and_then(|()| file.sync_all());
            let result = staged.map_err(|_| WriteError::Io).and_then(|()| {
                // SAFETY: renameat uses the same owned directory fd and valid names.
                if unsafe { libc::renameat(fd, temp.as_ptr(), fd, name.as_ptr()) } == 0 {
                    Ok(())
                } else {
                    Err(WriteError::Io)
                }
            });
            if result.is_err() {
                // SAFETY: unlinkat removes only this call's exclusive temporary file.
                unsafe {
                    libc::unlinkat(fd, temp.as_ptr(), 0);
                }
            }
            return result;
        }
        Err(WriteError::Io)
    }
}
