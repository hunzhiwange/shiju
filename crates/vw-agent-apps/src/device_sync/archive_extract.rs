//! Strict synchronization ZIP extraction into an owned, private staging tree.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::{self, Cursor, Read, Write};
use std::path::PathBuf;

use tempfile::TempDir;

use super::artifact::MAX_ARTIFACT_SIZE;

const MAX_ENTRIES: usize = 30_000;
const MAX_EXPANDED_BYTES: u64 = 512 * 1024 * 1024;

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

/// Retains layout without publishing or applying any live data. The caller must
/// verify these same bytes against the manifest before calling this function.
/// Every link is rejected, including links whose targets would remain inside.
/// Dropping the returned guard removes staging; any error removes partial output.
/// The guard must remain private until extraction and subsequent validation end.
pub fn extract_skills_archive(bytes: &[u8]) -> io::Result<TempDir> {
    if bytes.len() as u64 > MAX_ARTIFACT_SIZE {
        return Err(invalid("sync archive exceeds download limit"));
    }
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).map_err(io::Error::from)?;
    if archive.len() > MAX_ENTRIES {
        return Err(invalid("sync archive has too many entries"));
    }
    let staging = tempfile::Builder::new().prefix("shiju-sync-").tempdir()?;
    let mut explicit = BTreeSet::new();
    let mut spellings = BTreeMap::new();
    let mut directories = BTreeSet::new();
    let mut total = 0u64;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(io::Error::from)?;
        let name = std::str::from_utf8(entry.name_raw())
            .map_err(|_| invalid("sync archive path must be UTF-8"))?;
        if name.is_empty() || name.starts_with('/') || name.contains(['\\', '\0', ':']) {
            return Err(invalid("sync archive contains an unsafe path"));
        }
        let name = name.strip_suffix('/').unwrap_or(name);
        let parts: Vec<_> = name.split('/').collect();
        if parts.iter().any(|part| {
            part.is_empty() || *part == "." || *part == ".."
                || part.ends_with([' ', '.']) || part.chars().any(char::is_control)
        }) {
            return Err(invalid("sync archive contains an unsafe path component"));
        }
        let path: PathBuf = parts.iter().collect();
        if !explicit.insert(path.clone()) {
            return Err(invalid("sync archive contains duplicate entries"));
        }
        let mode = entry.unix_mode().unwrap_or(0) & 0o170000;
        if entry.is_symlink() || !matches!(mode, 0 | 0o040000 | 0o100000)
            || (mode == 0o040000 && !entry.is_dir())
            || (mode == 0o100000 && entry.is_dir())
        {
            return Err(invalid("sync archive contains a link or special file"));
        }
        let mut prefix = PathBuf::new();
        let mut folded = String::new();
        for (position, part) in parts.iter().enumerate() {
            prefix.push(part);
            folded.push('/');
            folded.extend(part.chars().flat_map(char::to_lowercase));
            if let Some(previous) = spellings.insert(folded.clone(), prefix.clone()) {
                if previous != prefix {
                    return Err(invalid("sync archive contains case-conflicting paths"));
                }
            }
            if position + 1 < parts.len() || entry.is_dir() {
                if directories.insert(prefix.clone()) {
                    // Do not accept AlreadyExists: the filesystem may normalize
                    // distinct Unicode spellings to the same directory name.
                    fs::create_dir(staging.path().join(&prefix))?;
                }
            }
        }
        if entry.is_dir() {
            if entry.size() != 0 {
                return Err(invalid("sync archive directory contains data"));
            }
            continue;
        }
        if entry.size() > MAX_EXPANDED_BYTES.saturating_sub(total) {
            return Err(invalid("sync archive exceeds extraction limit"));
        }
        let mut output = OpenOptions::new().write(true).create_new(true)
            .open(staging.path().join(&path))?;
        let mut buffer = [0u8; 16 * 1024];
        loop {
            let count = entry.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            total = total.checked_add(count as u64)
                .filter(|value| *value <= MAX_EXPANDED_BYTES)
                .ok_or_else(|| invalid("sync archive exceeds extraction limit"))?;
            output.write_all(&buffer[..count])?;
        }
    }
    Ok(staging)
}
