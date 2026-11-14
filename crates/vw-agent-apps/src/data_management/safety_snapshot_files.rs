//! Descriptor-relative macOS storage; never follow a symlink in an archive path.
use super::safety_snapshot::{MAX_SNAPSHOT_BYTES, SafetySnapshotError as Error};
use std::{
    ffi::CString,
    fs::File,
    io::{Read, Write},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::{ffi::OsStrExt, fs::MetadataExt},
    },
    path::{Component, Path},
};

fn storage(_: std::io::Error) -> Error {
    Error::StorageUnavailable
}
fn name(bytes: &[u8]) -> Result<CString, Error> {
    CString::new(bytes).map_err(|_| Error::UnsafeStorage)
}
fn open_at(parent: &File, child: &CString, flags: i32) -> Result<File, Error> {
    // SAFETY: a valid parent descriptor and NUL-terminated name are passed; the
    // returned descriptor is owned exactly once. Creation always uses mode 0600.
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            child.as_ptr(),
            flags | libc::O_CLOEXEC | libc::O_NOFOLLOW,
            0o600,
        )
    };
    if fd < 0 {
        return Err(storage(std::io::Error::last_os_error()));
    }
    Ok(unsafe { File::from_raw_fd(fd) })
}
fn private(file: &File, directory: bool) -> Result<(), Error> {
    let meta = file.metadata().map_err(storage)?;
    if meta.uid() != unsafe { libc::geteuid() }
        || meta.mode() & 0o777 != if directory { 0o700 } else { 0o600 }
        || (directory && !meta.is_dir())
        || (!directory && (!meta.is_file() || meta.nlink() != 1))
    {
        return Err(Error::UnsafeStorage);
    }
    Ok(())
}
fn root(path: &Path) -> Result<File, Error> {
    if !path.is_absolute() {
        return Err(Error::UnsafeStorage);
    }
    let mut dir = File::open("/").map_err(storage)?;
    for component in path.components() {
        match component {
            Component::RootDir => (),
            Component::Normal(part) => {
                dir = open_at(&dir, &name(part.as_bytes())?, libc::O_RDONLY | libc::O_DIRECTORY)?;
            }
            _ => return Err(Error::UnsafeStorage),
        }
    }
    private(&dir, true)?;
    Ok(dir)
}
fn snapshot_name(id: &str) -> Result<CString, Error> {
    let parsed = uuid::Uuid::parse_str(id).map_err(|_| Error::UnsafeStorage)?;
    if parsed.to_string() != id {
        return Err(Error::UnsafeStorage);
    }
    name(id.as_bytes())
}
fn read_file(dir: &File) -> Result<Vec<u8>, Error> {
    let file = open_at(dir, &name(b"preimages.json")?, libc::O_RDONLY | libc::O_NONBLOCK)?;
    private(&file, false)?;
    if file.metadata().map_err(storage)?.len() > MAX_SNAPSHOT_BYTES as u64 {
        return Err(Error::TooLarge);
    }
    let mut bytes = Vec::new();
    file.take(MAX_SNAPSHOT_BYTES as u64 + 1).read_to_end(&mut bytes).map_err(storage)?;
    if bytes.len() > MAX_SNAPSHOT_BYTES {
        return Err(Error::TooLarge);
    }
    Ok(bytes)
}

pub(super) fn publish(
    path: &Path,
    id: &str,
    bytes: &[u8],
    verify: impl FnOnce(&[u8]) -> Result<(), Error>,
) -> Result<(), Error> {
    let parent = root(path)?;
    let child = snapshot_name(id)?;
    // SAFETY: parent and child are valid for the duration of mkdirat.
    if unsafe { libc::mkdirat(parent.as_raw_fd(), child.as_ptr(), 0o700) } != 0 {
        return Err(Error::StorageUnavailable);
    }
    let result = (|| {
        let dir = open_at(&parent, &child, libc::O_RDONLY | libc::O_DIRECTORY)?;
        private(&dir, true)?;
        let mut file = open_at(
            &dir,
            &name(b"preimages.json")?,
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL,
        )?;
        private(&file, false)?;
        file.write_all(bytes).map_err(storage)?;
        file.sync_all().map_err(storage)?;
        dir.sync_all().map_err(storage)?;
        parent.sync_all().map_err(storage)?;
        verify(&read_file(&dir)?)
    })();
    if result.is_err() {
        // Best-effort removal only of our newly created child; never recurse into
        // caller paths or delete an existing archive after a name collision.
        if let Ok(dir) = open_at(&parent, &child, libc::O_RDONLY | libc::O_DIRECTORY) {
            if let Ok(file) = name(b"preimages.json") {
                unsafe {
                    libc::unlinkat(dir.as_raw_fd(), file.as_ptr(), 0);
                }
            }
        }
        unsafe {
            libc::unlinkat(parent.as_raw_fd(), child.as_ptr(), libc::AT_REMOVEDIR);
        }
    }
    result
}

pub(super) fn read(path: &Path, id: &str) -> Result<Vec<u8>, Error> {
    let parent = root(path)?;
    let dir = open_at(&parent, &snapshot_name(id)?, libc::O_RDONLY | libc::O_DIRECTORY)?;
    private(&dir, true)?;
    read_file(&dir)
}
