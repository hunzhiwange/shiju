//! Descriptor-relative writes for the trusted desktop workspace.

use std::io;
use std::path::{Component, Path};

pub(super) fn ordinary_path(relative: &Path) -> bool {
    relative.file_name().is_some()
        && relative.components().all(|component| match component {
            // Hidden entries include repository metadata, agent settings and secrets.
            Component::Normal(name) => name.to_str().is_some_and(|name| !name.starts_with('.')),
            _ => false,
        })
}

#[cfg(unix)]
pub(super) fn write(root: &Path, relative: &Path, content: &[u8]) -> io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::MetadataExt;

    if !root.is_absolute() {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    let mut directory = std::fs::File::open("/")?;
    for component in root.components() {
        if let Component::Normal(name) = component {
            directory = open_directory(&directory, name, false)?;
        }
    }
    let parent = relative.parent().expect("validated workspace file has a parent");
    for component in parent.components() {
        directory = open_directory(&directory, component.as_os_str(), true)?;
    }
    let name = relative.file_name().expect("validated workspace file has a name");
    // Open without truncation. Inspect and write the same inode, even if the name
    // is replaced after open. Nonblocking open prevents a FIFO from hanging the actor.
    let mut file = open_at(&directory, name, libc::O_WRONLY | libc::O_CREAT | libc::O_NONBLOCK)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.nlink() != 1 {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "expected a regular file without hard links",
        ));
    }
    file.set_len(0)?;
    file.write_all(content)
}

#[cfg(unix)]
fn open_directory(
    parent: &std::fs::File,
    name: &std::ffi::OsStr,
    create: bool,
) -> io::Result<std::fs::File> {
    use std::os::fd::AsRawFd;

    let flags = libc::O_RDONLY | libc::O_DIRECTORY;
    match open_at(parent, name, flags) {
        Err(error) if create && error.kind() == io::ErrorKind::NotFound => {
            let name_c = name_c_string(name)?;
            // SAFETY: The descriptor and C string remain live during this call.
            let result = unsafe { libc::mkdirat(parent.as_raw_fd(), name_c.as_ptr(), 0o777) };
            if result < 0 {
                let error = io::Error::last_os_error();
                if error.kind() != io::ErrorKind::AlreadyExists {
                    return Err(error);
                }
            }
            open_at(parent, name, flags)
        }
        result => result,
    }
}

#[cfg(unix)]
fn open_at(
    parent: &std::fs::File,
    name: &std::ffi::OsStr,
    flags: libc::c_int,
) -> io::Result<std::fs::File> {
    use std::os::fd::{AsRawFd, FromRawFd};

    let name = name_c_string(name)?;
    // SAFETY: The descriptor and C string remain live. A successful result is
    // a new descriptor owned by this function and transferred to File.
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            flags | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            0o666,
        )
    };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(unsafe { std::fs::File::from_raw_fd(fd) })
}

#[cfg(unix)]
fn name_c_string(name: &std::ffi::OsStr) -> io::Result<std::ffi::CString> {
    use std::os::unix::ffi::OsStrExt;

    std::ffi::CString::new(name.as_bytes()).map_err(|_| io::ErrorKind::InvalidInput.into())
}

#[cfg(not(unix))]
pub(super) fn write(_root: &Path, _relative: &Path, _content: &[u8]) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "descriptor-relative ACP workspace writes are unavailable on this platform",
    ))
}
