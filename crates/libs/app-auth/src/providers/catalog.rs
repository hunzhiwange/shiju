//! Bounded no-follow reads for native account storage.
use std::path::Path;
use vw_api_app_provider_edit::catalog::ProviderReadUnavailable;
#[cfg(unix)]
pub fn read_file(
    home: &Path,
    components: &[&str],
) -> Result<Option<Vec<u8>>, ProviderReadUnavailable> {
    use ProviderReadUnavailable::Unreadable;
    use std::{
        fs::OpenOptions,
        io::Read,
        os::{
            fd::{AsRawFd, FromRawFd},
            unix::fs::OpenOptionsExt,
        },
    };
    if !home.is_absolute() {
        return Err(Unreadable);
    }
    let mut directory = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(home)
        .map_err(|_| Unreadable)?;
    for (index, component) in components.iter().enumerate() {
        let last = index + 1 == components.len();
        let name = std::ffi::CString::new(*component).map_err(|_| Unreadable)?;
        let flags = libc::O_RDONLY
            | libc::O_NOFOLLOW
            | libc::O_CLOEXEC
            | libc::O_NONBLOCK
            | if last { 0 } else { libc::O_DIRECTORY };
        // SAFETY: directory is live and name is a fixed single NUL-terminated component.
        let fd = unsafe { libc::openat(directory.as_raw_fd(), name.as_ptr(), flags) };
        if fd < 0 {
            let error = std::io::Error::last_os_error();
            return if error.kind() == std::io::ErrorKind::NotFound {
                Ok(None)
            } else {
                Err(Unreadable)
            };
        }
        // SAFETY: openat returned a new owned descriptor, consumed once.
        let file = unsafe { std::fs::File::from_raw_fd(fd) };
        if last {
            let metadata = file.metadata().map_err(|_| Unreadable)?;
            if !metadata.is_file() || metadata.len() > 1024 * 1024 {
                return Err(Unreadable);
            }
            let mut bytes = Vec::new();
            file.take(1024 * 1024 + 1).read_to_end(&mut bytes).map_err(|_| Unreadable)?;
            if bytes.len() > 1024 * 1024 {
                return Err(Unreadable);
            }
            return Ok(Some(bytes));
        }
        directory = file;
    }
    Err(Unreadable)
}
#[cfg(not(unix))]
pub fn read_file(
    _home: &Path,
    _components: &[&str],
) -> Result<Option<Vec<u8>>, ProviderReadUnavailable> {
    Err(ProviderReadUnavailable::UnsupportedPlatform)
}
