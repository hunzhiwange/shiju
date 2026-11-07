//! Fixed Agy profile provisioning. Retained handles reject path replacement and arbitrary links.
use super::*;
use std::{
    ffi::CString,
    fs::{File, OpenOptions},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::{
            ffi::OsStrExt,
            fs::{MetadataExt, OpenOptionsExt},
        },
    },
};
struct Directory {
    path: PathBuf,
    file: File,
    private: bool,
}
fn checked(file: &File, private: bool) -> Result<(), WriteError> {
    let m = file.metadata().map_err(|_| WriteError::Io)?;
    if !m.is_dir()
        || m.uid() != unsafe { libc::geteuid() }
        || m.mode() & if private { 0o077 } else { 0o022 } != 0
    {
        return Err(WriteError::InvalidResource);
    }
    Ok(())
}
impl Directory {
    fn root(path: &Path, private: bool) -> Result<Self, WriteError> {
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(path)
            .map_err(|_| WriteError::InvalidResource)?;
        checked(&file, private)?;
        Ok(Self { path: path.into(), file, private })
    }
    fn child(&self, name: &str, create: bool, private: bool) -> Result<Self, WriteError> {
        let c = CString::new(name).map_err(|_| WriteError::InvalidResource)?;
        if name.is_empty() || name.contains('/') || name == "." || name == ".." {
            return Err(WriteError::InvalidResource);
        }
        if create
            && unsafe { libc::mkdirat(self.file.as_raw_fd(), c.as_ptr(), 0o700) } != 0
            && std::io::Error::last_os_error().kind() != std::io::ErrorKind::AlreadyExists
        {
            return Err(WriteError::Io);
        }
        let fd = unsafe {
            libc::openat(
                self.file.as_raw_fd(),
                c.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            return Err(WriteError::InvalidResource);
        }
        // SAFETY: fresh descriptor owned once, opened relative to a retained parent without following links.
        let file = unsafe { File::from_raw_fd(fd) };
        checked(&file, private)?;
        if create {
            self.file.sync_all().map_err(|_| WriteError::Io)?;
        }
        Ok(Self { path: self.path.join(name), file, private })
    }
    fn current(&self) -> Result<(), WriteError> {
        let now = Self::root(&self.path, self.private)?;
        let a = now.file.metadata().map_err(|_| WriteError::Io)?;
        let b = self.file.metadata().map_err(|_| WriteError::Io)?;
        if (a.dev(), a.ino()) != (b.dev(), b.ino()) {
            return Err(WriteError::Conflict);
        }
        Ok(())
    }
}
/// Retained until the selected process exits; settings cannot rotate under an active profile.
pub(crate) struct Lease {
    dirs: Vec<Directory>,
    profile: usize,
    settings: usize,
    shared: usize,
    _lock: File,
}
impl Lease {
    pub fn open(scope: &Scope<'_>) -> Result<Self, WriteError> {
        // Read-only native directories must already exist; never provision or modify native config.
        let mut dirs = vec![Directory::root(scope.home, false)?];
        dirs.push(dirs[0].child(".gemini", false, false)?);
        dirs.push(dirs[1].child("config", false, false)?);
        let shared = 2;
        dirs.push(Directory::root(scope.private_root, true)?);
        let relative = scope
            .directory()
            .strip_prefix(scope.private_root)
            .map_err(|_| WriteError::InvalidResource)?
            .to_path_buf();
        for c in relative.components() {
            let std::path::Component::Normal(c) = c else {
                return Err(WriteError::InvalidResource);
            };
            let next = dirs.last().unwrap().child(
                c.to_str().ok_or(WriteError::InvalidResource)?,
                true,
                true,
            )?;
            dirs.push(next);
        }
        let profile = dirs.len() - 1;
        let lockname = c".shiju-agy.lock";
        let fd = unsafe {
            libc::openat(
                dirs[profile].file.as_raw_fd(),
                lockname.as_ptr(),
                libc::O_RDWR
                    | libc::O_CREAT
                    | libc::O_NOFOLLOW
                    | libc::O_NONBLOCK
                    | libc::O_CLOEXEC,
                0o600,
            )
        };
        if fd < 0 {
            return Err(WriteError::InvalidResource);
        }
        // SAFETY: fresh owned lock descriptor. Closing releases flock on every error and exit path.
        let lock = unsafe { File::from_raw_fd(fd) };
        let m = lock.metadata().map_err(|_| WriteError::Io)?;
        if !m.is_file() || m.uid() != unsafe { libc::geteuid() } || m.mode() & 0o077 != 0 {
            return Err(WriteError::InvalidResource);
        }
        if unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err(WriteError::Conflict);
        }
        dirs.push(dirs[profile].child("antigravity-cli", true, true)?);
        let settings = dirs.len() - 1;
        Ok(Self { dirs, profile, settings, shared, _lock: lock })
    }
    pub fn guard(&self) -> Result<AppWriteGuard, WriteError> {
        super::super::live::write_guard::lock_directory(
            AppId::Agy,
            self.dirs[self.settings].file.try_clone().map_err(|_| WriteError::Io)?,
        )
    }
    fn link_target(&self) -> Result<PathBuf, WriteError> {
        let mut b = vec![0u8; 8192];
        let n = unsafe {
            libc::readlinkat(
                self.dirs[self.profile].file.as_raw_fd(),
                c"config".as_ptr(),
                b.as_mut_ptr().cast(),
                b.len(),
            )
        };
        if n < 0 || n as usize == b.len() {
            return Err(WriteError::InvalidResource);
        }
        b.truncate(n as usize);
        Ok(PathBuf::from(OsStr::from_bytes(&b)))
    }
    pub fn ensure_link(&self) -> Result<(), WriteError> {
        for d in &self.dirs {
            d.current()?;
        }
        let target = CString::new(self.dirs[self.shared].path.as_os_str().as_bytes())
            .map_err(|_| WriteError::InvalidResource)?;
        // Create-only, fixed config child to the retained authorized shared directory. Never replace existing entries.
        if unsafe {
            libc::symlinkat(
                target.as_ptr(),
                self.dirs[self.profile].file.as_raw_fd(),
                c"config".as_ptr(),
            )
        } != 0
            && std::io::Error::last_os_error().kind() != std::io::ErrorKind::AlreadyExists
        {
            return Err(WriteError::Io);
        }
        self.dirs[self.profile].file.sync_all().map_err(|_| WriteError::Io)?;
        self.validate()
    }
    pub fn validate(&self) -> Result<(), WriteError> {
        for d in &self.dirs {
            d.current()?;
        }
        if self.link_target()? != self.dirs[self.shared].path {
            return Err(WriteError::Conflict);
        }
        // A replaced lock must never permit two owners to update the same profile.
        let fd = unsafe {
            libc::openat(
                self.dirs[self.profile].file.as_raw_fd(),
                c".shiju-agy.lock".as_ptr(),
                libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
            )
        };
        if fd < 0 {
            return Err(WriteError::Conflict);
        }
        let file = unsafe { File::from_raw_fd(fd) };
        let a = file.metadata().map_err(|_| WriteError::Io)?;
        let b = self._lock.metadata().map_err(|_| WriteError::Io)?;
        if (a.dev(), a.ino()) != (b.dev(), b.ino()) {
            return Err(WriteError::Conflict);
        }
        Ok(())
    }
}

/// Catalog deletion is exclusive with every authorized Agy launch, including native login.
/// A separate control lock never removes or provisions a persistent session/profile.
pub fn catalog_lease(scope: &Scope<'_>, exclusive: bool) -> Result<File, WriteError> {
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    use sha2::{Digest, Sha256};
    super::super::validate_identity(scope.provider).map_err(|_| WriteError::InvalidResource)?;
    let root = Directory::root(scope.private_root, true)?;
    let identity = serde_json::to_vec(&(scope.tenant, scope.provider))
        .map_err(|_| WriteError::InvalidResource)?;
    let name = CString::new(format!(
        ".agy-catalog-{}.lock",
        URL_SAFE_NO_PAD.encode(Sha256::digest(identity))
    ))
    .map_err(|_| WriteError::InvalidResource)?;
    let fd = unsafe {
        libc::openat(
            root.file.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDWR | libc::O_CREAT | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC,
            0o600,
        )
    };
    if fd < 0 {
        return Err(WriteError::InvalidResource);
    }
    // SAFETY: openat returned a new owned descriptor without following a final symlink.
    let lock = unsafe { File::from_raw_fd(fd) };
    let m = lock.metadata().map_err(|_| WriteError::Io)?;
    if !m.is_file()
        || m.uid() != unsafe { libc::geteuid() }
        || m.mode() & 0o077 != 0
        || m.nlink() != 1
    {
        return Err(WriteError::InvalidResource);
    }
    if unsafe {
        libc::flock(fd, (if exclusive { libc::LOCK_EX } else { libc::LOCK_SH }) | libc::LOCK_NB)
    } != 0
    {
        return Err(WriteError::Conflict);
    }
    root.current()?;
    Ok(lock)
}
