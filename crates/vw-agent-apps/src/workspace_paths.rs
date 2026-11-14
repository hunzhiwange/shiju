//! Descriptor-relative workspace IO on the authorized macOS executor.
//! Callers authorize tenant/project/action before constructing this boundary. Host
//! identities and HOME come from authenticated dispatch, never request path data.
//! Resolved workspace/application roots come from the native application resolver.
use crate::providers::{
    live::write_guard::{FilePrivacy, lock_directory},
    paths,
};
use std::{
    ffi::CString,
    fs::File,
    io::Read,
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::{ffi::OsStrExt, fs::MetadataExt},
    },
    path::{Component, Path},
};
use vw_api_types::agent_apps::AppId;

#[derive(Debug, Clone, Copy)]
pub enum WorkspaceArea {
    OpenClawWorkspace,
    OpenClawMemory,
    HermesMemories,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathError {
    UnauthorizedHost,
    InvalidPath,
    UnsafeEntry,
    Io,
}

impl std::fmt::Display for PathError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::UnauthorizedHost => "workspace executor is not the authorized host",
            Self::InvalidPath => "invalid workspace-relative path",
            Self::UnsafeEntry => "workspace entry is not a safe regular file or directory",
            Self::Io => "workspace IO failed",
        })
    }
}
impl std::error::Error for PathError {}

/// Retains directories rather than returning paths that callers can reopen unsafely.
/// Filename allowlists and action permissions remain the owning domain's concern.
#[derive(Debug)]
pub struct WorkspaceDirectory {
    directory: File,
    app: AppId,
}

impl WorkspaceDirectory {
    /// `resolved_root` is the resolved OpenClaw workspace, or Hermes application
    /// directory. It must be below the authorized executor HOME. Creation is only
    /// requested after write authorization; reads must pass `create = false`.
    pub fn open(
        authorized_host: &str,
        executor_host: &str,
        home: &Path,
        resolved_root: &Path,
        area: WorkspaceArea,
        create: bool,
    ) -> Result<Option<Self>, PathError> {
        if authorized_host.is_empty() || authorized_host != executor_host {
            return Err(PathError::UnauthorizedHost);
        }
        let relative = resolved_root.strip_prefix(home).map_err(|_| PathError::InvalidPath)?;
        check_relative(relative)?;
        let relative = match area {
            WorkspaceArea::OpenClawWorkspace => relative.to_path_buf(),
            WorkspaceArea::OpenClawMemory => relative.join("memory"),
            WorkspaceArea::HermesMemories => relative.join("memories"),
        };
        let mut directory =
            paths::open_beneath(home, home, true).map_err(|_| PathError::UnsafeEntry)?;
        for part in relative.components() {
            let Component::Normal(part) = part else { return Err(PathError::InvalidPath) };
            let name = CString::new(part.as_bytes()).map_err(|_| PathError::InvalidPath)?;
            let mut next = open_child(&directory, &name, true);
            if matches!(&next, Err(e) if e.kind() == std::io::ErrorKind::NotFound) {
                if !create {
                    return Ok(None);
                }
                // SAFETY: live directory descriptor and a single validated component.
                let result = unsafe { libc::mkdirat(directory.as_raw_fd(), name.as_ptr(), 0o700) };
                if result < 0
                    && std::io::Error::last_os_error().kind() != std::io::ErrorKind::AlreadyExists
                {
                    return Err(PathError::Io);
                }
                next = open_child(&directory, &name, true);
            }
            directory = next.map_err(|_| PathError::UnsafeEntry)?;
        }
        Ok(Some(Self {
            directory,
            app: match area {
                WorkspaceArea::OpenClawWorkspace | WorkspaceArea::OpenClawMemory => AppId::OpenClaw,
                WorkspaceArea::HermesMemories => AppId::Hermes,
            },
        }))
    }

    /// Open Finder on the held inode, using its descriptor as the child cwd.
    /// No caller-controlled pathname is passed to the system opener.
    pub fn open_in_finder(&self) -> Result<(), PathError> {
        let status = std::process::Command::new("/usr/bin/open")
            .args(["-a", "Finder", "."])
            .current_dir(format!("/dev/fd/{}", self.directory.as_raw_fd()))
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map_err(|_| PathError::Io)?;
        if !status.success() { return Err(PathError::Io); }
        Ok(())
    }

    /// Enumerates the held directory without reopening its path. A fresh open
    /// description keeps repeated enumerations independent of directory offsets.
    pub(crate) fn filenames(&self) -> Result<Vec<String>, PathError> {
        let directory = open_child(&self.directory, &CString::new(".").unwrap(), true)
            .map_err(|_| PathError::Io)?;
        use std::os::fd::IntoRawFd;
        let fd = directory.into_raw_fd();
        // SAFETY: fd is owned here; fdopendir takes ownership on success.
        let stream = unsafe { libc::fdopendir(fd) };
        if stream.is_null() {
            unsafe { libc::close(fd) };
            return Err(PathError::Io);
        }
        struct DirectoryStream(*mut libc::DIR);
        impl Drop for DirectoryStream {
            fn drop(&mut self) {
                // SAFETY: this guard owns the live stream exactly once.
                unsafe { libc::closedir(self.0) };
            }
        }
        let stream = DirectoryStream(stream);
        let mut names = Vec::new();
        loop {
            // SAFETY: macOS thread-local errno and a live owned stream. Consume
            // each name before the next readdir invalidates its storage.
            let entry = unsafe {
                *libc::__error() = 0;
                libc::readdir(stream.0)
            };
            if entry.is_null() {
                if unsafe { *libc::__error() } != 0 {
                    return Err(PathError::Io);
                }
                break;
            }
            let name = unsafe { std::ffi::CStr::from_ptr((*entry).d_name.as_ptr()) };
            if let Ok(name) = name.to_str() {
                if name != "." && name != ".." {
                    names.push(name.to_owned());
                }
            }
        }
        Ok(names)
    }

    /// Skip links and special entries before opening; recheck the opened inode
    /// through file() so a concurrent replacement cannot broaden access.
    pub(crate) fn regular_file(&self, name: &str) -> Result<Option<File>, PathError> {
        let name = filename(name)?;
        let mut metadata = std::mem::MaybeUninit::<libc::stat>::uninit();
        let result = unsafe {
            libc::fstatat(
                self.directory.as_raw_fd(), name.as_ptr(), metadata.as_mut_ptr(),
                libc::AT_SYMLINK_NOFOLLOW,
            )
        };
        if result < 0 {
            return Err(PathError::Io);
        }
        // SAFETY: successful fstatat initialized metadata.
        let metadata = unsafe { metadata.assume_init() };
        if metadata.st_mode & libc::S_IFMT != libc::S_IFREG || metadata.st_nlink != 1 {
            return Ok(None);
        }
        self.file(&name)
    }

    pub fn read(&self, name: &str) -> Result<Option<String>, PathError> {
        let name = filename(name)?;
        let Some(mut file) = self.file(&name)? else { return Ok(None) };
        let mut content = String::new();
        file.read_to_string(&mut content).map_err(|_| PathError::Io)?;
        Ok(Some(content))
    }

    pub fn write(&self, name: &str, content: &str) -> Result<(), PathError> {
        let checked = filename(name)?;
        let mut guard =
            lock_directory(self.app, self.directory.try_clone().map_err(|_| PathError::Io)?)
                .map_err(|_| PathError::Io)?;
        self.file(&checked)?;
        guard
            .write(std::ffi::OsStr::new(name), content.as_bytes(), FilePrivacy::Private)
            .map_err(|_| PathError::Io)
    }

    pub fn delete(&self, name: &str) -> Result<(), PathError> {
        let name = filename(name)?;
        let _guard =
            lock_directory(self.app, self.directory.try_clone().map_err(|_| PathError::Io)?)
                .map_err(|_| PathError::Io)?;
        if self.file(&name)?.is_none() {
            return Ok(());
        }
        // SAFETY: unlink a single child without following links or reopening parents.
        let result = unsafe { libc::unlinkat(self.directory.as_raw_fd(), name.as_ptr(), 0) };
        if result < 0 && std::io::Error::last_os_error().kind() != std::io::ErrorKind::NotFound {
            return Err(PathError::Io);
        }
        self.directory.sync_all().map_err(|_| PathError::Io)
    }

    fn file(&self, name: &CString) -> Result<Option<File>, PathError> {
        match open_child(&self.directory, name, false) {
            Ok(file) => {
                let meta = file.metadata().map_err(|_| PathError::Io)?;
                if !meta.is_file() || meta.nlink() != 1 {
                    return Err(PathError::UnsafeEntry);
                }
                Ok(Some(file))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(_) => Err(PathError::UnsafeEntry),
        }
    }
}

fn check_relative(path: &Path) -> Result<(), PathError> {
    if path.as_os_str().is_empty() || path.components().any(|p| !matches!(p, Component::Normal(_)))
    {
        return Err(PathError::InvalidPath);
    }
    Ok(())
}
fn filename(name: &str) -> Result<CString, PathError> {
    check_relative(Path::new(name))?;
    if name.contains(['/', '\\']) || name.chars().any(char::is_control) {
        return Err(PathError::InvalidPath);
    }
    CString::new(name).map_err(|_| PathError::InvalidPath)
}
fn open_child(parent: &File, name: &CString, directory: bool) -> std::io::Result<File> {
    let flags = libc::O_RDONLY
        | libc::O_NOFOLLOW
        | libc::O_CLOEXEC
        | libc::O_NONBLOCK
        | if directory { libc::O_DIRECTORY } else { 0 };
    // SAFETY: parent owns a live descriptor and name is NUL-terminated.
    let fd = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) };
    if fd < 0 {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: newly opened descriptor transferred exactly once.
    Ok(unsafe { File::from_raw_fd(fd) })
}
