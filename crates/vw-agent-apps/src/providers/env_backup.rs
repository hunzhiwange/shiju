//! Private environment backup format shared by cleanup and restoration.
use super::env_diagnose::macos::{open_root, read_source};
use sha2::{Digest, Sha256};
use std::fs::Metadata;
use std::{
    ffi::CString,
    fs::File,
    io::{Read, Write},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::fs::MetadataExt,
    },
    path::Path,
};
use vw_api_types::{agent_apps::AppId, app_providers::env_diagnose::UserShellFile};
pub(super) fn random_id() -> Result<String, &'static str> {
    let mut bytes = [0u8; 32];
    File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut bytes))
        .map_err(|_| UNAVAILABLE)?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}
pub(super) fn identity(m: &Metadata) -> (u64, u64) {
    (m.dev(), m.ino())
}
pub(super) fn safe(m: &Metadata) -> bool {
    // SAFETY: geteuid has no pointer arguments or side effects.
    m.uid() == unsafe { libc::geteuid() } && m.mode() & 0o022 == 0
}
pub(super) fn base(m: &Metadata, content: &str) -> String {
    let mut hash = Sha256::new();
    for v in [
        m.dev(),
        m.ino(),
        m.len(),
        m.mode() as u64,
        m.uid() as u64,
        m.nlink(),
        m.mtime() as u64,
        m.mtime_nsec() as u64,
        m.ctime() as u64,
        m.ctime_nsec() as u64,
    ] {
        hash.update(v.to_le_bytes());
    }
    hash.update(content.as_bytes());
    hash.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

const UNAVAILABLE: &str = "environment backup unavailable";
const CONFLICT: &str = "environment backup changed";
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Backup {
    pub version: u32,
    pub scope: String,
    pub app_id: AppId,
    pub home_identity: (u64, u64),
    pub files: Vec<BackupFile>,
}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct BackupFile {
    pub file: UserShellFile,
    pub base_digest: String,
    pub original: String,
    pub replacement: String,
    pub cleaned_digest: Option<String>,
    pub restored_digest: Option<String>,
}
pub(super) fn valid_id(id: &str) -> bool {
    id.len() == 64 && id.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn private(file: &File) -> Result<(), &'static str> {
    let m = file.metadata().map_err(|_| UNAVAILABLE)?;
    if !safe(&m) || m.mode() & 0o077 != 0 {
        return Err(UNAVAILABLE);
    }
    Ok(())
}
pub(super) fn directory(root: &Path, create: bool) -> Result<File, &'static str> {
    let root = open_root(root)?;
    private(&root)?;
    // SAFETY: fixed direct child under a retained, authorized private root.
    if create
        && unsafe { libc::mkdirat(root.as_raw_fd(), c"environment-backups".as_ptr(), 0o700) } != 0
        && std::io::Error::last_os_error().kind() != std::io::ErrorKind::AlreadyExists
    {
        return Err(UNAVAILABLE);
    }
    // SAFETY: no symlink following; returned descriptor is uniquely owned.
    let fd = unsafe {
        libc::openat(
            root.as_raw_fd(),
            c"environment-backups".as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        return Err(UNAVAILABLE);
    }
    let dir = unsafe { File::from_raw_fd(fd) };
    private(&dir)?;
    root.sync_all().map_err(|_| UNAVAILABLE)?;
    Ok(dir)
}
pub(super) fn load(dir: &File, id: &str) -> Result<Backup, &'static str> {
    if !valid_id(id) {
        return Err("invalid environment backup identifier");
    }
    let name = CString::new(format!("{id}.json")).map_err(|_| CONFLICT)?;
    // SAFETY: validated ASCII identifier and live directory descriptor, no links or blocking devices.
    let fd = unsafe {
        libc::openat(
            dir.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        return Err(CONFLICT);
    }
    let file = unsafe { File::from_raw_fd(fd) };
    private(&file)?;
    let m = file.metadata().map_err(|_| UNAVAILABLE)?;
    const MAX: u64 = 64 * 1024 * 1024;
    if !m.is_file() || m.nlink() != 1 || m.len() > MAX {
        return Err(CONFLICT);
    }
    let mut bytes = Vec::new();
    file.take(MAX + 1).read_to_end(&mut bytes).map_err(|_| UNAVAILABLE)?;
    if bytes.len() as u64 > MAX {
        return Err(CONFLICT);
    }
    serde_json::from_slice(&bytes).map_err(|_| CONFLICT)
}
/// Original bytes are durable before cleanup. Checkpoint replacements are atomic;
/// losing a checkpoint fails closed without losing the previous complete backup.
pub(super) fn store(
    dir: &File,
    id: &str,
    backup: &Backup,
    replace: bool,
) -> Result<(), &'static str> {
    if !valid_id(id) {
        return Err("invalid environment backup identifier");
    }
    let final_name = CString::new(format!("{id}.json")).map_err(|_| UNAVAILABLE)?;
    let temp = if replace {
        CString::new(format!("{id}.{}.tmp", random_id()?)).map_err(|_| UNAVAILABLE)?
    } else {
        final_name.clone()
    };
    let bytes = serde_json::to_vec(backup).map_err(|_| UNAVAILABLE)?;
    // SAFETY: generated direct child; exclusive creation never truncates another file.
    let fd = unsafe {
        libc::openat(
            dir.as_raw_fd(),
            temp.as_ptr(),
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            0o600,
        )
    };
    if fd < 0 {
        return Err(UNAVAILABLE);
    }
    let mut file = unsafe { File::from_raw_fd(fd) };
    let result = file.write_all(&bytes).and_then(|_| file.sync_all()).and_then(|_| {
        // SAFETY: both names are generated direct children of the retained directory.
        if replace
            && unsafe {
                libc::renameat(dir.as_raw_fd(), temp.as_ptr(), dir.as_raw_fd(), final_name.as_ptr())
            } != 0
        {
            return Err(std::io::Error::last_os_error());
        }
        dir.sync_all()
    });
    if result.is_err() {
        // SAFETY: remove only this invocation's exclusive staging name.
        unsafe {
            libc::unlinkat(dir.as_raw_fd(), temp.as_ptr(), 0);
        }
        return Err(UNAVAILABLE);
    }
    Ok(())
}

pub(crate) fn fingerprint(root: &File, name: &str, expected: &str) -> Result<String, &'static str> {
    let (bytes, m) = read_source(root, name)?.ok_or(CONFLICT)?;
    if bytes != expected || !safe(&m) || m.nlink() != 1 {
        return Err(CONFLICT);
    }
    Ok(base(&m, &bytes))
}
