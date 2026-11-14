//! Backup catalog on the authorized execution host. The caller owns root authorization.
use std::{collections::BTreeMap, ffi::{CStr, CString}, fs::File, io::{self, Read},
    os::fd::{AsRawFd, FromRawFd}, path::{Path, PathBuf}, time::UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use super::{copy_deploy::CopyBinding, local_install::{validate_directory, LocalSkill}};

const RETAIN_COUNT: usize = 20;
const MAX_METADATA: u64 = 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillBackupEntry {
    pub backup_id: String,
    pub backup_path: PathBuf,
    pub created_at: u64,
    pub skill: LocalSkill,
}

#[derive(Deserialize)]
struct Metadata {
    schema_version: u32,
    skill: LocalSkill,
    bindings: BTreeMap<String, CopyBinding>,
    fingerprint: String,
    #[serde(default)]
    backup_created_at: Option<u64>,
}

fn invalid() -> io::Error { io::Error::new(io::ErrorKind::InvalidData, "invalid skill backup") }
fn open_root(root: &Path) -> io::Result<File> {
    if !root.is_absolute() { return Err(invalid()); }
    crate::providers::paths::open_beneath(root, root, true).map_err(|_| invalid())
}
fn id_name(id: &str) -> io::Result<CString> {
    if !id.starts_with("skill-uninstall-") || id.len() > 255
        || !id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') {
        return Err(invalid());
    }
    CString::new(id).map_err(|_| invalid())
}
fn child(parent: &File, name: &CStr, directory: bool) -> io::Result<File> {
    let flags = libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW
        | if directory { libc::O_DIRECTORY } else { libc::O_NONBLOCK };
    let fd = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) };
    if fd < 0 { return Err(io::Error::last_os_error()); }
    Ok(unsafe { File::from_raw_fd(fd) })
}
fn read_entry(root: &File, path: &Path, id: &str) -> io::Result<SkillBackupEntry> {
    let dir = child(root, &id_name(id)?, true)?;
    let file = child(&dir, c"meta.json", false)?;
    if !file.metadata()?.is_file() { return Err(invalid()); }
    let mut bytes = Vec::new();
    file.take(MAX_METADATA + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_METADATA { return Err(invalid()); }
    let meta: Metadata = serde_json::from_slice(&bytes).map_err(|_| invalid())?;
    if meta.schema_version != 1 || meta.fingerprint.is_empty() { return Err(invalid()); }
    validate_directory(&meta.skill.directory).map_err(|_| invalid())?;
    for binding in meta.bindings.values() {
        if binding.directory != meta.skill.directory { return Err(invalid()); }
    }
    let created_at = match meta.backup_created_at {
        Some(time) => time,
        None => dir.metadata()?.modified()?.duration_since(UNIX_EPOCH).map_err(|_| invalid())?.as_millis()
            .try_into().map_err(|_| invalid())?,
    };
    Ok(SkillBackupEntry { backup_id: id.into(), backup_path: path.join(id), created_at, skill: meta.skill })
}

pub fn list_backups(root: &Path) -> io::Result<Vec<SkillBackupEntry>> {
    let directory = open_root(root)?;
    let mut entries = Vec::new();
    for name in names(&directory)? {
        let Ok(id) = name.to_str() else { continue; };
        // Like the source catalog, unrelated or malformed entries remain untouched.
        if let Ok(entry) = read_entry(&directory, root, id) { entries.push(entry); }
    }
    entries.sort_by(|a, b| b.created_at.cmp(&a.created_at).then(a.backup_id.cmp(&b.backup_id)));
    Ok(entries)
}

pub fn delete_backup(root: &Path, id: &str) -> io::Result<()> {
    let directory = open_root(root)?;
    read_entry(&directory, root, id)?;
    let name = id_name(id)?;
    let backup = child(&directory, &name, true)?;
    // Keep metadata until content removal succeeds, allowing failed deletions to be retried.
    remove_contents(&backup)?;
    unlink(&directory, &name, libc::AT_REMOVEDIR)
}

/// Preserve the source retention count: keep the newest twenty by directory mtime.
/// Only recognized backups participate; malformed directories are never deleted.
pub fn cleanup_old_skill_backups(root: &Path) -> io::Result<()> {
    let directory = open_root(root)?;
    let mut entries = list_backups(root)?.into_iter().map(|entry| {
        let modified = child(&directory, &id_name(&entry.backup_id)?, true)?.metadata()?.modified()?;
        Ok((modified, entry.backup_id))
    }).collect::<io::Result<Vec<_>>>()?;
    entries.sort();
    let count = entries.len().saturating_sub(RETAIN_COUNT);
    for (_, id) in entries.into_iter().take(count) { delete_backup(root, &id)?; }
    Ok(())
}

fn names(directory: &File) -> io::Result<Vec<CString>> {
    // Open a separate directory description so enumeration does not consume the caller's offset.
    let scan = child(directory, c".", true)?;
    let fd = std::os::fd::IntoRawFd::into_raw_fd(scan);
    let stream = unsafe { libc::fdopendir(fd) };
    if stream.is_null() {
        unsafe { libc::close(fd); }
        return Err(io::Error::last_os_error());
    }
    let mut result = Vec::new();
    loop {
        unsafe { *libc::__error() = 0; }
        let entry = unsafe { libc::readdir(stream) };
        if entry.is_null() {
            let error = io::Error::last_os_error();
            unsafe { libc::closedir(stream); }
            return if error.raw_os_error() == Some(0) { Ok(result) } else { Err(error) };
        }
        let name = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) };
        if name != c"." && name != c".." { result.push(name.to_owned()); }
    }
}
fn unlink(parent: &File, name: &CStr, flags: i32) -> io::Result<()> {
    if unsafe { libc::unlinkat(parent.as_raw_fd(), name.as_ptr(), flags) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}
fn remove_contents(directory: &File) -> io::Result<()> {
    let mut children = names(directory)?;
    children.sort_by_key(|name| name.as_c_str() == c"meta.json");
    for name in children {
        match child(directory, &name, true) {
            Ok(child) => { remove_contents(&child)?; unlink(directory, &name, libc::AT_REMOVEDIR)?; }
            Err(error) if matches!(error.raw_os_error(), Some(libc::ENOTDIR | libc::ELOOP)) => {
                // unlinkat removes links themselves and never traverses their targets.
                unlink(directory, &name, 0)?;
            }
            Err(error) => return Err(error),
        }
    }
    Ok(())
}
