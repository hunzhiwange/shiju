//! macOS GUI instance ownership and bounded, user-private launch argument delivery.

use std::fs::{File, OpenOptions};
use std::io;
use std::os::fd::AsRawFd;
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt};
use std::os::unix::net::UnixDatagram;
use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const MAX_REQUEST: usize = 16 * 1024;
static REQUESTS: Mutex<Option<UnixDatagram>> = Mutex::new(None);

/// Keeps the advisory lock held until the event loop and gateway have exited.
/// The lock file is deliberately never removed, preventing inode replacement races.
pub struct InstanceGuard {
    _lock: File,
}

/// `None` means the existing GUI has received the launch request: exit before booting App.
pub fn acquire_or_forward() -> io::Result<Option<InstanceGuard>> {
    let uid = unsafe { libc::geteuid() };
    let directory = std::path::PathBuf::from(format!("/tmp/shiju-desktop-{uid}"));
    match std::fs::DirBuilder::new().mode(0o700).create(&directory) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error),
    }
    let metadata = std::fs::symlink_metadata(&directory)?;
    if !metadata.is_dir() || metadata.uid() != uid || metadata.mode() & 0o077 != 0 {
        return Err(io::Error::new(io::ErrorKind::PermissionDenied, "实例目录不安全"));
    }
    acquire_at(&directory, std::env::args().skip(1).collect())
}

fn acquire_at(directory: &Path, args: Vec<String>) -> io::Result<Option<InstanceGuard>> {
    let payload = serde_json::to_vec(&args)?;
    if payload.len() > MAX_REQUEST {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "启动参数超过 16 KiB"));
    }
    let lock = OpenOptions::new().read(true).write(true).create(true).mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC).open(directory.join("instance.lock"))?;
    let socket_path = directory.join("launch.sock");
    if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        let error = io::Error::last_os_error();
        if error.kind() != io::ErrorKind::WouldBlock {
            return Err(error);
        }
        let sender = UnixDatagram::unbound()?;
        sender.set_nonblocking(true)?;
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            match sender.send_to(&payload, &socket_path) {
                Ok(_) => return Ok(None),
                Err(error) if Instant::now() < deadline && matches!(error.kind(),
                    io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused | io::ErrorKind::WouldBlock) => {
                    std::thread::sleep(Duration::from_millis(25));
                }
                Err(error) => return Err(error),
            }
        }
    }
    // Only the lock owner may clean up a stale socket left after a crash.
    match std::fs::remove_file(&socket_path) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    let receiver = UnixDatagram::bind(socket_path)?;
    receiver.set_nonblocking(true)?;
    *REQUESTS.lock().map_err(|_| io::Error::other("实例接收锁不可用"))? = Some(receiver);
    super::deep_links::receive_arguments(&args)?;
    Ok(Some(InstanceGuard { _lock: lock }))
}

pub(crate) fn poll() -> Vec<Vec<String>> {
    let Ok(guard) = REQUESTS.lock() else { return Vec::new() };
    let Some(receiver) = guard.as_ref() else { return Vec::new() };
    let mut requests = Vec::new();
    let mut buffer = [0; MAX_REQUEST + 1];
    // Bound work per UI tick even when another process floods the socket.
    for _ in 0..16 {
        match receiver.recv(&mut buffer) {
            Ok(size) if size <= MAX_REQUEST => {
                if let Ok(args) = serde_json::from_slice::<Vec<String>>(&buffer[..size]) {
                    requests.push(args);
                }
            }
            Ok(_) => tracing::warn!("discarded oversized desktop launch request"),
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
            Err(error) => {
                tracing::warn!(%error, "desktop launch request receive failed");
                break;
            }
        }
    }
    requests
}
