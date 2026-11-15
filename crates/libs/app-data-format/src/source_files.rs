//! Bounded file capture without opening the selected database through SQLite.
use std::{fs::{self, File}, io::{Read, Write}, path::{Component, Path, PathBuf}};
use sha2::{Digest, Sha256};
use super::source::SourceError;

const MAX_FILE_BYTES: u64 = 256 * 1024 * 1024;

pub(super) fn selected_path(root: &Path, selected: &Path) -> Result<PathBuf, SourceError> {
    if !root.is_absolute() || !selected.is_absolute() { return Err(SourceError::OutsideAuthorizedRoot); }
    let relative = selected.strip_prefix(root).map_err(|_| SourceError::OutsideAuthorizedRoot)?;
    let mut path = root.canonicalize()?;
    for component in relative.components() {
        let Component::Normal(name) = component else { return Err(SourceError::OutsideAuthorizedRoot); };
        path.push(name);
        if fs::symlink_metadata(&path)?.file_type().is_symlink() { return Err(SourceError::UnsafeFile); }
    }
    if relative.as_os_str().is_empty() { return Err(SourceError::UnsafeFile); }
    Ok(path)
}

pub(super) fn wal_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push("-wal");
    PathBuf::from(name)
}

pub(super) struct Capture {
    pub bytes: Vec<u8>,
    pub digest: [u8; 32],
    metadata: fs::Metadata,
}
impl Capture {
    pub fn verify(&self, path: &Path) -> Result<(), SourceError> {
        let current = capture(path)?;
        if self.digest != current.digest || !same_file(&self.metadata, &current.metadata) {
            return Err(SourceError::Changed);
        }
        Ok(())
    }
}

fn same_file(a: &fs::Metadata, b: &fs::Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        a.dev() == b.dev() && a.ino() == b.ino() && a.len() == b.len()
            && a.mtime() == b.mtime() && a.mtime_nsec() == b.mtime_nsec()
            && a.ctime() == b.ctime() && a.ctime_nsec() == b.ctime_nsec()
    }
    #[cfg(not(unix))]
    { a.len() == b.len() && a.modified().ok() == b.modified().ok() }
}

pub(super) fn capture(path: &Path) -> Result<Capture, SourceError> {
    let before = fs::symlink_metadata(path)?;
    if !before.is_file() || before.file_type().is_symlink() { return Err(SourceError::UnsafeFile); }
    let file = open_no_follow(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() || !same_file(&before, &metadata) { return Err(SourceError::Changed); }
    if metadata.len() > MAX_FILE_BYTES { return Err(SourceError::TooLarge); }
    let mut bytes = Vec::new();
    (&file).take(MAX_FILE_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_FILE_BYTES { return Err(SourceError::TooLarge); }
    if !same_file(&metadata, &file.metadata()?) || !same_file(&metadata, &fs::symlink_metadata(path)?) {
        return Err(SourceError::Changed);
    }
    Ok(Capture { digest: Sha256::digest(&bytes).into(), bytes, metadata })
}

pub(super) fn capture_optional(path: &Path) -> Result<Option<Capture>, SourceError> {
    match capture(path) {
        Ok(value) => Ok(Some(value)),
        Err(SourceError::NotFound) => Ok(None),
        Err(error) => Err(error),
    }
}
pub(super) fn verify_optional(path: &Path, captured: Option<&Capture>) -> Result<(), SourceError> {
    match captured {
        Some(value) => value.verify(path),
        None => match fs::symlink_metadata(path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Ok(_) => Err(SourceError::Changed),
            Err(e) => Err(e.into()),
        },
    }
}
pub(super) fn write_private(path: &Path, bytes: &[u8]) -> Result<(), SourceError> {
    let mut options = File::options();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)?.write_all(bytes)?;
    Ok(())
}

// Walk every component with directory descriptors. O_NOFOLLOW on only the final
// file would still allow a swapped ancestor symlink to escape the authorized root.
#[cfg(unix)]
fn open_no_follow(path: &Path) -> Result<File, SourceError> {
    use std::{ffi::CString, os::{fd::{AsRawFd, FromRawFd}, unix::ffi::OsStrExt}};
    let mut directory = File::open("/")?;
    let names: Vec<_> = path.components().filter_map(|component| match component {
        Component::Normal(name) => Some(name),
        _ => None,
    }).collect();
    for (index, name) in names.iter().enumerate() {
        let name = CString::new(name.as_bytes()).map_err(|_| SourceError::UnsafeFile)?;
        let flags = libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK
            | if index + 1 < names.len() { libc::O_DIRECTORY } else { 0 };
        // SAFETY: directory owns a live fd and name is a NUL-terminated string.
        let fd = unsafe { libc::openat(directory.as_raw_fd(), name.as_ptr(), flags) };
        if fd < 0 { return Err(std::io::Error::last_os_error().into()); }
        // SAFETY: openat returned a new fd owned solely by this File.
        directory = unsafe { File::from_raw_fd(fd) };
    }
    Ok(directory)
}
#[cfg(not(unix))]
fn open_no_follow(path: &Path) -> Result<File, SourceError> {
    fs::OpenOptions::new().read(true).open(path).map_err(Into::into)
}
