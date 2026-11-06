//! Descriptor-relative access to fixed login/profile/journal children.
use crate::providers::live::write_guard::WriteError;
use std::sync::atomic::{AtomicU64, Ordering};
use std::{
    ffi::CString,
    fs::File,
    io::{Read, Write},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::fs::{MetadataExt, OpenOptionsExt},
    },
    path::Path,
};
pub(crate) struct Directory(pub File);
impl Directory {
    pub fn open(path: &Path) -> Result<Self, WriteError> {
        let file = File::options()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(path)
            .map_err(|_| WriteError::InvalidResource)?;
        Self::check(&file)?;
        Ok(Self(file))
    }
    fn check(file: &File) -> Result<(), WriteError> {
        let m = file.metadata().map_err(|_| WriteError::Io)?;
        if m.uid() != unsafe { libc::geteuid() } || m.mode() & 0o022 != 0 {
            return Err(WriteError::InvalidResource);
        }
        Ok(())
    }
    pub fn read(&self, name: &str) -> Result<Option<Vec<u8>>, WriteError> {
        let name = CString::new(name).map_err(|_| WriteError::InvalidResource)?;
        let fd = unsafe {
            libc::openat(
                self.0.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            return if std::io::Error::last_os_error().kind() == std::io::ErrorKind::NotFound {
                Ok(None)
            } else {
                Err(WriteError::InvalidResource)
            };
        }
        let file = unsafe { File::from_raw_fd(fd) };
        Self::check(&file)?;
        let m = file.metadata().map_err(|_| WriteError::Io)?;
        if (name.as_bytes() == b".credentials.json" || name.as_bytes() == b".shiju-login.json")
            && m.mode() & 0o077 != 0
        {
            return Err(WriteError::InvalidResource);
        }
        if !m.is_file() || m.len() > 1000000 {
            return Err(WriteError::InvalidResource);
        }
        let mut bytes = Vec::new();
        file.take(1000001).read_to_end(&mut bytes).map_err(|_| WriteError::Io)?;
        if bytes.len() > 1000000 {
            return Err(WriteError::InvalidResource);
        }
        Ok(Some(bytes))
    }
    pub fn write(&self, name: &str, bytes: Option<&[u8]>) -> Result<(), WriteError> {
        let name = CString::new(name).map_err(|_| WriteError::InvalidResource)?;
        if let Some(bytes) = bytes {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let temp = CString::new(format!(
                ".shiju-login-{}-{}.tmp",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ))
            .map_err(|_| WriteError::Io)?;
            let fd = unsafe {
                libc::openat(
                    self.0.as_raw_fd(),
                    temp.as_ptr(),
                    libc::O_WRONLY
                        | libc::O_CREAT
                        | libc::O_EXCL
                        | libc::O_NOFOLLOW
                        | libc::O_CLOEXEC,
                    0o600 as libc::c_uint,
                )
            };
            if fd < 0 {
                return Err(WriteError::Io);
            }
            let mut file = unsafe { File::from_raw_fd(fd) };
            let result = (|| {
                file.write_all(bytes).and_then(|_| file.sync_all()).map_err(|_| WriteError::Io)?;
                if unsafe {
                    libc::renameat(
                        self.0.as_raw_fd(),
                        temp.as_ptr(),
                        self.0.as_raw_fd(),
                        name.as_ptr(),
                    )
                } != 0
                {
                    return Err(WriteError::Io);
                }
                Ok(())
            })();
            unsafe { libc::unlinkat(self.0.as_raw_fd(), temp.as_ptr(), 0) };
            result?;
        } else if unsafe { libc::unlinkat(self.0.as_raw_fd(), name.as_ptr(), 0) } != 0
            && std::io::Error::last_os_error().kind() != std::io::ErrorKind::NotFound
        {
            return Err(WriteError::Io);
        }
        self.0.sync_all().map_err(|_| WriteError::Io)
    }
    pub fn lock(&self) -> Result<File, WriteError> {
        let fd = unsafe {
            libc::openat(
                self.0.as_raw_fd(),
                c".shiju-login.lock".as_ptr(),
                libc::O_RDWR
                    | libc::O_CREAT
                    | libc::O_NOFOLLOW
                    | libc::O_NONBLOCK
                    | libc::O_CLOEXEC,
                0o600 as libc::c_uint,
            )
        };
        if fd < 0 {
            return Err(WriteError::Io);
        }
        let file = unsafe { File::from_raw_fd(fd) };
        Self::check(&file)?;
        if !file.metadata().map_err(|_| WriteError::Io)?.is_file()
            || unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) } != 0
        {
            return Err(WriteError::Conflict);
        }
        Ok(file)
    }
}

/// Cooperates with Claude's proper-lockfile directories. Only our dead PID markers
/// can be reclaimed; unknown or live CLI locks are never removed.
pub(crate) struct CliLock {
    parent: File,
    name: CString,
    directory: File,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    heartbeat: Option<std::thread::JoinHandle<()>>,
}
impl Directory {
    pub fn cli_lock(&self, name: &str) -> Result<CliLock, WriteError> {
        let name = CString::new(name).map_err(|_| WriteError::InvalidResource)?;
        let create = || unsafe { libc::mkdirat(self.0.as_raw_fd(), name.as_ptr(), 0o700) };
        if create() != 0 {
            let fd = unsafe {
                libc::openat(
                    self.0.as_raw_fd(),
                    name.as_ptr(),
                    libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                )
            };
            if fd < 0 {
                return Err(WriteError::Conflict);
            }
            let old = Directory(unsafe { File::from_raw_fd(fd) });
            Self::check(&old.0)?;
            let bytes = old.read("shiju-owner.pid")?.ok_or(WriteError::Conflict)?;
            let pid = std::str::from_utf8(&bytes)
                .ok()
                .and_then(|p| p.parse::<libc::pid_t>().ok())
                .filter(|p| *p > 0)
                .ok_or(WriteError::Conflict)?;
            if unsafe { libc::kill(pid, 0) } == 0
                || std::io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH)
            {
                return Err(WriteError::Conflict);
            }
            old.write("shiju-owner.pid", None)?;
            if unsafe { libc::unlinkat(self.0.as_raw_fd(), name.as_ptr(), libc::AT_REMOVEDIR) } != 0
                || create() != 0
            {
                return Err(WriteError::Conflict);
            }
        }
        let fd = unsafe {
            libc::openat(
                self.0.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            return Err(WriteError::Io);
        }
        let directory = Directory(unsafe { File::from_raw_fd(fd) });
        directory.write("shiju-owner.pid", Some(std::process::id().to_string().as_bytes()))?;
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let thread_stop = stop.clone();
        let file = directory.0.try_clone().map_err(|_| WriteError::Io)?;
        let heartbeat = std::thread::spawn(move || {
            while !thread_stop.load(std::sync::atomic::Ordering::Acquire) {
                let times = [libc::timespec { tv_sec: 0, tv_nsec: libc::UTIME_NOW }; 2];
                unsafe { libc::futimens(file.as_raw_fd(), times.as_ptr()) };
                std::thread::park_timeout(std::time::Duration::from_secs(1));
            }
        });
        Ok(CliLock {
            parent: self.0.try_clone().map_err(|_| WriteError::Io)?,
            name,
            directory: directory.0,
            stop,
            heartbeat: Some(heartbeat),
        })
    }
}
impl Drop for CliLock {
    fn drop(&mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::Release);
        if let Some(thread) = self.heartbeat.take() {
            thread.thread().unpark();
            let _ = thread.join();
        }
        let mut status: libc::stat = unsafe { std::mem::zeroed() };
        let Ok(own) = self.directory.metadata() else {
            return;
        };
        if unsafe {
            libc::fstatat(
                self.parent.as_raw_fd(),
                self.name.as_ptr(),
                &mut status,
                libc::AT_SYMLINK_NOFOLLOW,
            )
        } != 0
            || status.st_dev as u64 != own.dev()
            || status.st_ino != own.ino()
        {
            return;
        }
        unsafe {
            libc::unlinkat(self.directory.as_raw_fd(), c"shiju-owner.pid".as_ptr(), 0);
            libc::unlinkat(self.parent.as_raw_fd(), self.name.as_ptr(), libc::AT_REMOVEDIR);
        }
    }
}
