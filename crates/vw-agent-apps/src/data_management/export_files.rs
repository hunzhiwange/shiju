//! Atomic, exclusive publication in a pinned private directory on macOS.
use super::safety_snapshot::{SafetySnapshotError as Error, MAX_SNAPSHOT_BYTES};
use std::{ffi::CString, fs::File, io::{Read, Write}, os::fd::{AsRawFd, FromRawFd}};

struct Pending<'a> { parent: &'a File, name: CString }
impl Drop for Pending<'_> {
    fn drop(&mut self) {
        // SAFETY: only a name created by this operation in the pinned directory.
        unsafe { libc::unlinkat(self.parent.as_raw_fd(), self.name.as_ptr(), 0); }
    }
}
pub(super) fn publish(
    parent: &File, id: &str, bytes: &[u8], verify: impl FnOnce(&[u8]) -> Result<(), Error>,
) -> Result<(), Error> {
    if uuid::Uuid::parse_str(id).is_err() { return Err(Error::UnsafeStorage); }
    let name = CString::new(format!(".export-{}", uuid::Uuid::new_v4())).unwrap();
    // SAFETY: parent is a live pinned directory; create-new name has no separators.
    let fd = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(),
        libc::O_RDWR | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC, 0o600) };
    if fd < 0 { return Err(Error::StorageUnavailable); }
    let mut pending = Pending { parent, name };
    // SAFETY: the newly created fd is owned exactly once.
    let mut file = unsafe { File::from_raw_fd(fd) };
    file.write_all(bytes).map_err(|_| Error::StorageUnavailable)?;
    file.sync_all().map_err(|_| Error::StorageUnavailable)?;
    use std::io::{Seek, SeekFrom};
    file.seek(SeekFrom::Start(0)).map_err(|_| Error::StorageUnavailable)?;
    let mut reread = Vec::new();
    file.take(MAX_SNAPSHOT_BYTES as u64 + 1).read_to_end(&mut reread)
        .map_err(|_| Error::StorageUnavailable)?;
    if reread != bytes { return Err(Error::Corrupt); }
    verify(&reread)?;
    let final_name = CString::new(format!("{id}.shiju-package")).unwrap();
    // SAFETY: both names are single components in the pinned private directory.
    if unsafe { libc::renameatx_np(parent.as_raw_fd(), pending.name.as_ptr(),
        parent.as_raw_fd(), final_name.as_ptr(), libc::RENAME_EXCL) } != 0 {
        return Err(if std::io::Error::last_os_error().kind() == std::io::ErrorKind::AlreadyExists {
            Error::Conflict
        } else { Error::StorageUnavailable });
    }
    pending.name = final_name;
    // A sync failure removes only this operation's publication. The caller must
    // exclusively own this private export directory throughout publication.
    parent.sync_all().map_err(|_| Error::StorageUnavailable)?;
    std::mem::forget(pending);
    Ok(())
}
