//! Deterministic skills ZIPs from an explicitly authorized, dedicated skills root.

use std::path::Path;

use super::manifest::SyncArtifact;

/// Bytes and their identity travel together; callers must upload these exact bytes.
pub struct PackedSkillsArchive {
    pub bytes: Vec<u8>,
    pub artifact: SyncArtifact,
}

/// Does not discover roots or authorize the caller. The host must supply a dedicated
/// portable skills root, excluding credentials and device data, and keep its contents
/// stable for the capture. Unsupported platforms fail closed.
pub fn pack_skills_archive(root: &Path) -> Result<PackedSkillsArchive, &'static str> {
    #[cfg(unix)]
    {
        native::pack(root)
    }
    #[cfg(not(unix))]
    {
        let _ = root;
        Err("secure skills archive capture is unsupported on this platform")
    }
}

#[cfg(unix)]
mod native {
    use std::{
        collections::{BTreeMap, BTreeSet},
        ffi::CString,
        fs::{self, File, OpenOptions},
        io::{Cursor, Read, Write},
        os::{fd::{AsRawFd, FromRawFd}, unix::{ffi::OsStrExt, fs::{MetadataExt, OpenOptionsExt}}},
        path::{Component, Path},
    };

    use sha2::{Digest, Sha256};
    use zip::{CompressionMethod, DateTime, ZipWriter, write::SimpleFileOptions};

    use super::{PackedSkillsArchive, SyncArtifact};
    use crate::device_sync::artifact::MAX_ARTIFACT_SIZE;

    const MAX_ENTRIES: usize = 10_000;

    pub(super) fn pack(root: &Path) -> Result<PackedSkillsArchive, &'static str> {
        let root = fs::canonicalize(root).map_err(|_| "cannot resolve skills root")?;
        let anchor = OpenOptions::new().read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&root).map_err(|_| "cannot open skills root")?;
        let mut files = BTreeMap::new();
        let mut visited = BTreeSet::new();
        let mut pending = vec![root.clone()];
        let mut count = 0usize;
        while let Some(directory) = pending.pop() {
            if !visited.insert(directory.clone()) {
                continue;
            }
            for entry in fs::read_dir(directory).map_err(|_| "cannot enumerate skills directory")? {
                count += 1;
                if count > MAX_ENTRIES {
                    return Err("skills archive entry limit exceeded");
                }
                let entry = entry.map_err(|_| "cannot read skills entry")?;
                let target = fs::canonicalize(entry.path())
                    .map_err(|_| "cannot resolve skills entry or symbolic link")?;
                let relative = target.strip_prefix(&root)
                    .map_err(|_| "skills symbolic link escapes authorized root")?;
                if relative.as_os_str().is_empty() {
                    continue;
                }
                let file = open_relative(&anchor, relative)?;
                let metadata = file.metadata().map_err(|_| "cannot inspect skills entry")?;
                if metadata.is_dir() {
                    pending.push(target);
                } else if metadata.is_file() {
                    let name = archive_name(relative)?;
                    files.insert(name, relative.to_path_buf());
                } else {
                    return Err("skills archive supports only regular files and directories");
                }
            }
        }

        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Stored)
            .last_modified_time(DateTime::default())
            .unix_permissions(0o644);
        let mut identities = BTreeSet::new();
        let mut total = 0u64;
        for (name, relative) in files {
            let mut file = open_relative(&anchor, &relative)?;
            let metadata = file.metadata().map_err(|_| "cannot inspect skills file")?;
            if !metadata.is_file() {
                return Err("skills file changed type during capture");
            }
            if !identities.insert((metadata.dev(), metadata.ino())) {
                continue;
            }
            let remaining = MAX_ARTIFACT_SIZE.saturating_sub(total);
            if metadata.len() > remaining {
                return Err("skills content size exceeds limit");
            }
            writer.start_file(name, options).map_err(|_| "cannot start skills ZIP entry")?;
            let mut buffer = [0u8; 64 * 1024];
            loop {
                let length = file.read(&mut buffer).map_err(|_| "cannot read skills file")?;
                if length == 0 { break; }
                total += length as u64;
                // Reserve a bounded allowance for ZIP headers and central directory.
                if total > MAX_ARTIFACT_SIZE.saturating_sub(MAX_ENTRIES as u64 * 1024) {
                    return Err("skills archive size exceeds limit");
                }
                writer.write_all(&buffer[..length]).map_err(|_| "cannot write skills ZIP entry")?;
            }
        }
        let bytes = writer.finish().map_err(|_| "cannot finish skills ZIP")?.into_inner();
        if bytes.len() as u64 > MAX_ARTIFACT_SIZE {
            return Err("skills archive size exceeds limit");
        }
        let artifact = SyncArtifact {
            sha256: format!("{:x}", Sha256::digest(&bytes)),
            size: bytes.len() as u64,
        };
        Ok(PackedSkillsArchive { bytes, artifact })
    }

    fn archive_name(relative: &Path) -> Result<String, &'static str> {
        let mut parts = Vec::new();
        for component in relative.components() {
            let Component::Normal(part) = component else {
                return Err("invalid skills archive path");
            };
            let part = part.to_str().ok_or("skills archive path must be UTF-8")?;
            if part.contains(['\\', ':']) || part.chars().any(char::is_control) {
                return Err("unsafe skills archive path");
            }
            parts.push(part);
        }
        let name = parts.join("/");
        if name.is_empty() || name.len() > 1024 {
            return Err("skills archive path length exceeds limit");
        }
        Ok(name)
    }

    /// Resolve each canonical relative component through directory handles. Concurrent
    /// replacement with a symlink cannot redirect reads outside the anchored root.
    fn open_relative(root: &File, relative: &Path) -> Result<File, &'static str> {
        let mut directory = root.try_clone().map_err(|_| "cannot retain skills root")?;
        let mut components = relative.components().peekable();
        while let Some(component) = components.next() {
            let Component::Normal(name) = component else {
                return Err("invalid skills relative path");
            };
            let name = CString::new(name.as_bytes()).map_err(|_| "invalid skills path")?;
            let mut flags = libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK;
            if components.peek().is_some() { flags |= libc::O_DIRECTORY; }
            // SAFETY: valid directory descriptor and NUL-terminated name; success
            // returns a newly owned descriptor transferred exactly once to File.
            let fd = unsafe { libc::openat(directory.as_raw_fd(), name.as_ptr(), flags) };
            if fd < 0 { return Err("cannot securely open skills entry"); }
            directory = unsafe { File::from_raw_fd(fd) };
        }
        Ok(directory)
    }
}
