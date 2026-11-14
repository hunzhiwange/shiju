//! macOS descriptor-relative publication with atomic no-replace rename.
use super::safety_snapshot::{MAX_SNAPSHOT_BYTES, SafetySnapshotError as Error};
use std::{
    ffi::CString,
    fs::File,
    io::{Read, Write},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::fs::MetadataExt,
    },
    path::Path,
};

fn name(value: &str) -> Result<CString, Error> {
    CString::new(value).map_err(|_| Error::UnsafeStorage)
}
fn private(file: &File, directory: bool) -> Result<(), Error> {
    let metadata = file.metadata().map_err(|_| Error::StorageUnavailable)?;
    // SAFETY: geteuid has no arguments and does not transfer ownership.
    if metadata.uid() != unsafe { libc::geteuid() }
        || metadata.mode() & 0o777 != if directory { 0o700 } else { 0o600 }
        || (directory && !metadata.is_dir())
        || (!directory && (!metadata.is_file() || metadata.nlink() != 1))
    {
        return Err(Error::UnsafeStorage);
    }
    Ok(())
}
pub(super) fn open_root(root: &Path) -> Result<File, Error> {
    if !root.is_absolute() {
        return Err(Error::UnsafeStorage);
    }
    let file = crate::providers::paths::open_beneath(Path::new("/"), root, true)
        .map_err(|_| Error::UnsafeStorage)?;
    private(&file, true)?;
    Ok(file)
}
fn open_at(parent: &File, child: &CString, flags: i32) -> Result<File, Error> {
    // SAFETY: valid descriptor and terminated name; fd is owned exactly once.
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            child.as_ptr(),
            flags | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
            0o600,
        )
    };
    if fd < 0 {
        return Err(Error::StorageUnavailable);
    }
    Ok(unsafe { File::from_raw_fd(fd) })
}
pub(super) fn archive_name(id: &str) -> Result<CString, Error> {
    if id.is_empty()
        || id.len() > 100
        || !id.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
    {
        return Err(Error::UnsafeStorage);
    }
    name(&format!("{id}.local-backup"))
}
pub(super) fn open_backup(parent: &File, id: &str) -> Result<File, Error> {
    let file = open_at(parent, &archive_name(id)?, libc::O_RDONLY)?;
    private(&file, false)?;
    if file.metadata().map_err(|_| Error::StorageUnavailable)?.len() > MAX_SNAPSHOT_BYTES as u64 {
        return Err(Error::TooLarge);
    }
    Ok(file)
}
struct Pending<'a> {
    parent: &'a File,
    child: CString,
}
impl Drop for Pending<'_> {
    fn drop(&mut self) {
        // SAFETY: only our create-new temporary name is removed, never a final
        // archive. After successful rename this name no longer exists.
        unsafe {
            libc::unlinkat(self.parent.as_raw_fd(), self.child.as_ptr(), 0);
        }
    }
}
pub(super) fn publish(
    parent: &File,
    base: &str,
    bytes: &[u8],
    verify: impl FnOnce(&[u8]) -> Result<(), Error>,
) -> Result<String, Error> {
    archive_name(base)?;
    let child = name(&format!(".backup-publish-{}", uuid::Uuid::new_v4()))?;
    let mut file = open_at(parent, &child, libc::O_RDWR | libc::O_CREAT | libc::O_EXCL)?;
    let pending = Pending { parent, child };
    private(&file, false)?;
    file.write_all(bytes).map_err(|_| Error::StorageUnavailable)?;
    file.sync_all().map_err(|_| Error::StorageUnavailable)?;
    // Reopen through the pinned parent and read the complete file before rename.
    let reader = open_at(parent, &pending.child, libc::O_RDONLY)?;
    private(&reader, false)?;
    let mut reread = Vec::new();
    reader
        .take(MAX_SNAPSHOT_BYTES as u64 + 1)
        .read_to_end(&mut reread)
        .map_err(|_| Error::StorageUnavailable)?;
    if reread != bytes {
        return Err(Error::Corrupt);
    }
    verify(&reread)?;
    for suffix in 0..1024 {
        let id = if suffix == 0 { base.to_owned() } else { format!("{base}-{suffix}") };
        let final_name = archive_name(&id)?;
        // SAFETY: both names are relative to the live parent descriptor.
        // RENAME_EXCL never replaces files, directories or case-equivalent names.
        if unsafe {
            libc::renameatx_np(
                parent.as_raw_fd(),
                pending.child.as_ptr(),
                parent.as_raw_fd(),
                final_name.as_ptr(),
                libc::RENAME_EXCL,
            )
        } == 0
        {
            // Failure to sync is reported honestly, but the committed archive is
            // retained: deleting it could destroy a caller's recovery evidence.
            parent.sync_all().map_err(|_| Error::StorageUnavailable)?;
            return Ok(id);
        }
        let error = std::io::Error::last_os_error();
        if error.kind() != std::io::ErrorKind::AlreadyExists {
            return Err(Error::StorageUnavailable);
        }
    }
    Err(Error::Conflict)
}

/// Enumerate the pinned directory, never reopen the caller's path.
pub(super) fn backup_ids(parent: &File) -> Result<Vec<String>, Error> {
    use std::os::fd::IntoRawFd;
    let directory = open_at(parent, &name(".")?, libc::O_RDONLY | libc::O_DIRECTORY)?;
    let fd = directory.into_raw_fd();
    // SAFETY: fd is an owned directory; fdopendir takes ownership on success.
    let stream = unsafe { libc::fdopendir(fd) };
    if stream.is_null() {
        unsafe { libc::close(fd) };
        return Err(Error::StorageUnavailable);
    }
    struct Stream(*mut libc::DIR);
    impl Drop for Stream {
        fn drop(&mut self) {
            // SAFETY: this guard uniquely owns the directory stream.
            unsafe { libc::closedir(self.0) };
        }
    }
    let stream = Stream(stream);
    let mut ids = Vec::new();
    loop {
        // SAFETY: macOS thread-local errno and live, uniquely owned DIR pointer.
        unsafe { *libc::__error() = 0 };
        let entry = unsafe { libc::readdir(stream.0) };
        if entry.is_null() {
            if unsafe { *libc::__error() } != 0 {
                return Err(Error::StorageUnavailable);
            }
            break;
        }
        // SAFETY: readdir's name remains valid until the next stream operation.
        let name = unsafe { std::ffi::CStr::from_ptr((*entry).d_name.as_ptr()) };
        let Ok(name) = name.to_str() else { continue };
        if let Some(id) = name.strip_suffix(".local-backup") {
            if archive_name(id).is_ok() {
                ids.push(id.to_owned());
            }
        }
    }
    ids.sort();
    Ok(ids)
}

/// Nonblocking advisory lock shared by source readers and deletion. Closing the
/// descriptor releases it, including on error or process exit.
pub(super) fn lock_backup(file: &File, exclusive: bool) -> Result<(), Error> {
    let mode = if exclusive { libc::LOCK_EX } else { libc::LOCK_SH };
    // SAFETY: file owns a live descriptor; flock does not transfer ownership.
    if unsafe { libc::flock(file.as_raw_fd(), mode | libc::LOCK_NB) } != 0 {
        return Err(if std::io::Error::last_os_error().kind() == std::io::ErrorKind::WouldBlock {
            Error::Conflict
        } else {
            Error::StorageUnavailable
        });
    }
    // A reader may have opened the file before a deleter unlinked it.
    if file.metadata().map_err(|_| Error::StorageUnavailable)?.nlink() != 1 {
        return Err(Error::Conflict);
    }
    Ok(())
}
