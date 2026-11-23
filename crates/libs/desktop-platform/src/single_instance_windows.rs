//! The Windows shell launches the executable with dropped file paths as arguments.
//! Publish each request atomically in the current user's LocalAppData directory.
use super::MAX_REQUEST;
use std::fs::{self, File, OpenOptions, TryLockError};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

static REQUESTS: OnceLock<PathBuf> = OnceLock::new();

pub struct InstanceGuard {
    _lock: File,
}

pub(super) fn acquire_or_forward(args: Vec<String>) -> io::Result<Option<InstanceGuard>> {
    let local_app_data = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "无法定位 LocalAppData"))?;
    let requests = local_app_data.join("ShiJu").join("DesktopLaunch");
    fs::create_dir_all(&requests)?;
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(requests.join("instance.lock"))?;
    match lock.try_lock() {
        Ok(()) => {
            super::receive_arguments(args)?;
            REQUESTS.set(requests).map_err(|_| io::Error::other("实例已初始化"))?;
            Ok(Some(InstanceGuard { _lock: lock }))
        }
        Err(TryLockError::WouldBlock) => {
            let files: Vec<_> =
                args.into_iter().filter(|arg| Path::new(arg).is_absolute()).collect();
            let payload = serde_json::to_vec(&files)?;
            if payload.len() > MAX_REQUEST {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, "启动参数超过 16 KiB"));
            }
            let mut pending = tempfile::Builder::new().prefix("pending-").tempfile_in(requests)?;
            pending.write_all(&payload)?;
            let published = pending.path().with_extension("request");
            pending.persist_noclobber(published)?;
            Ok(None)
        }
        Err(TryLockError::Error(error)) => Err(error),
    }
}

pub(super) fn poll() -> Vec<Vec<String>> {
    let Some(requests) = REQUESTS.get() else { return Vec::new() };
    let Ok(entries) = fs::read_dir(requests) else { return Vec::new() };
    let mut received = Vec::new();
    for entry in entries
        .flatten()
        .filter(|entry| {
            entry.path().extension().is_some_and(|ext| ext == "request")
                && entry.file_type().is_ok_and(|file_type| file_type.is_file())
        })
        .take(16)
    {
        match consume(&entry.path()) {
            Ok(args) => received.push(args),
            Err(_) => tracing::warn!("failed to receive desktop launch request"),
        }
    }
    received
}

fn consume(path: &Path) -> io::Result<Vec<String>> {
    let mut payload = Vec::new();
    File::open(path)?.take((MAX_REQUEST + 1) as u64).read_to_end(&mut payload)?;
    fs::remove_file(path)?;
    if payload.len() > MAX_REQUEST {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "启动参数超过 16 KiB"));
    }
    Ok(serde_json::from_slice(&payload)?)
}
