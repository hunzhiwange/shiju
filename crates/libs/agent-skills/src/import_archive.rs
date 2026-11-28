//! Bounded ZIP extraction. No archive entry ever creates an operating-system link.
//!
//! Adapted from the preserved cc-switch 4.0.25 skill service (MIT). Callers keep
//! the returned guard alive until installation/publication has finished.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use tempfile::TempDir;

const MAX_ARCHIVE_ENTRIES: usize = 30_000;
const MAX_ARCHIVE_BYTES: u64 = 512 * 1024 * 1024;
const MAX_ARCHIVE_TOTAL_BYTES: u64 = 512 * 1024 * 1024;
const MAX_SYMLINK_TARGET_BYTES: u64 = 4 * 1024;
const BLOCK_BYTES: u64 = 4096;

#[derive(Debug)]
enum Entry {
    Directory,
    File,
    Link(PathBuf),
}

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

pub(super) fn extract_local_zip(path: &Path) -> io::Result<TempDir> {
    extract(File::open(path)?, None)
}

fn extract<R: Read + Seek>(mut reader: R, temporary_parent: Option<&Path>) -> io::Result<TempDir> {
    if reader.seek(SeekFrom::End(0))? > MAX_ARCHIVE_BYTES {
        return Err(invalid("skill archive exceeds the download budget"));
    }
    reader.seek(SeekFrom::Start(0))?;
    let mut archive = zip::ZipArchive::new(reader).map_err(io::Error::from)?;
    if archive.is_empty() {
        return Err(invalid("skill archive is empty"));
    }
    if archive.len() > MAX_ARCHIVE_ENTRIES {
        return Err(invalid("skill archive has too many entries"));
    }
    let guard = match temporary_parent {
        Some(parent) => tempfile::tempdir_in(parent)?,
        None => tempfile::tempdir()?,
    };
    let mut entries = BTreeMap::new();
    entries.insert(PathBuf::new(), Entry::Directory);
    let mut explicit = BTreeSet::new();
    let mut total = 0;
    for index in 0..archive.len() {
        let mut file = archive.by_index(index).map_err(io::Error::from)?;
        let parts = safe_entry_parts(&file.name().map_err(io::Error::from)?)?;
        let path: PathBuf = parts.iter().collect();
        if !explicit.insert(path.clone()) {
            return Err(invalid("skill archive contains duplicate paths"));
        }
        let mode = file.unix_mode().unwrap_or(0) & 0o170000;
        if !matches!(mode, 0 | 0o040000 | 0o100000 | 0o120000) {
            return Err(invalid("skill archive contains a special file"));
        }
        ensure_parents(&path, &mut entries, guard.path(), &mut total)?;
        if file.is_symlink() {
            if entries.contains_key(&path) {
                return Err(invalid("skill archive link conflicts with a directory"));
            }
            let target = read_symlink_target(&mut file, &mut total)?;
            let target = resolve_target(path.parent().unwrap_or(Path::new("")), &target)?;
            entries.insert(path, Entry::Link(target));
        } else if file.is_dir() {
            if let Some(entry) = entries.get(&path) {
                if !matches!(entry, Entry::Directory) {
                    return Err(invalid("skill archive directory conflicts with a file"));
                }
            } else {
                charge(&mut total, BLOCK_BYTES)?;
                fs::create_dir(guard.path().join(&path))?;
                entries.insert(path, Entry::Directory);
            }
        } else {
            if entries.contains_key(&path) {
                return Err(invalid("skill archive file conflicts with a directory"));
            }
            write_file(&mut file, &guard.path().join(&path), &mut total)?;
            entries.insert(path, Entry::File);
        }
    }
    let mut children: BTreeMap<PathBuf, Vec<PathBuf>> = BTreeMap::new();
    for path in entries.keys().filter(|path| !path.as_os_str().is_empty()) {
        children.entry(path.parent().unwrap().to_path_buf()).or_default().push(path.clone());
    }
    // Resolve against the immutable archive tree, never against earlier copies.
    // This makes link chains independent of ZIP order and detects directory cycles.
    for (path, entry) in &entries {
        if matches!(entry, Entry::Link(_)) {
            materialize(path, &entries, &children, guard.path(), &mut total)?;
        }
    }
    Ok(guard)
}

fn safe_entry_parts(name: &str) -> io::Result<Vec<String>> {
    if name.is_empty() || name.starts_with('/') || name.contains(['\\', '\0', ':']) {
        return Err(invalid("skill archive contains an unsafe path"));
    }
    let name = name.strip_suffix('/').unwrap_or(name);
    let parts: Vec<_> = name.split('/').map(str::to_owned).collect();
    if parts.iter().any(|part| part.is_empty() || part == "." || part == "..") {
        return Err(invalid("skill archive contains an unsafe path component"));
    }
    Ok(parts)
}

fn ensure_parents(
    path: &Path,
    entries: &mut BTreeMap<PathBuf, Entry>,
    root: &Path,
    total: &mut u64,
) -> io::Result<()> {
    let mut parent = PathBuf::new();
    let components: Vec<_> = path.components().collect();
    for component in &components[..components.len() - 1] {
        parent.push(component.as_os_str());
        match entries.get(&parent) {
            Some(Entry::Directory) => {}
            Some(_) => return Err(invalid("skill archive parent is not a directory")),
            None => {
                charge(total, BLOCK_BYTES)?;
                fs::create_dir(root.join(&parent))?;
                entries.insert(parent.clone(), Entry::Directory);
            }
        }
    }
    Ok(())
}

fn resolve_target(parent: &Path, target: &str) -> io::Result<PathBuf> {
    if target.is_empty() || target.starts_with('/') || target.contains(['\\', '\0', ':']) {
        return Err(invalid("skill archive contains an unsafe link target"));
    }
    let mut resolved = parent.to_path_buf();
    for part in target.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if !resolved.pop() {
                    return Err(invalid("skill archive link escapes its root"));
                }
            }
            part => resolved.push(part),
        }
    }
    Ok(resolved)
}

fn read_symlink_target<R: Read>(reader: &mut R, total: &mut u64) -> io::Result<String> {
    let mut bytes = Vec::new();
    reader.take(MAX_SYMLINK_TARGET_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_SYMLINK_TARGET_BYTES {
        return Err(invalid("skill archive link target exceeds 4 KiB"));
    }
    charge(total, bytes.len() as u64)?;
    String::from_utf8(bytes).map_err(|_| invalid("skill archive link target is not UTF-8"))
}

fn charge(total: &mut u64, amount: u64) -> io::Result<()> {
    let next = total
        .checked_add(amount)
        .filter(|next| *next <= MAX_ARCHIVE_TOTAL_BYTES)
        .ok_or_else(|| invalid("skill archive exceeds the extraction budget"))?;
    *total = next;
    Ok(())
}

fn write_file<R: Read>(reader: &mut R, path: &Path, total: &mut u64) -> io::Result<()> {
    // Never overwrite an alias on a case-insensitive or Unicode-normalizing volume.
    let mut writer = OpenOptions::new().write(true).create_new(true).open(path)?;
    let before = *total;
    let mut buffer = [0; 16 * 1024];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        charge(total, read as u64)?;
        writer.write_all(&buffer[..read])?;
    }
    let minimum_charge = BLOCK_BYTES.saturating_sub(*total - before);
    charge(total, minimum_charge)
}

fn materialize(
    link: &Path,
    entries: &BTreeMap<PathBuf, Entry>,
    children: &BTreeMap<PathBuf, Vec<PathBuf>>,
    root: &Path,
    total: &mut u64,
) -> io::Result<()> {
    enum Work {
        Copy(PathBuf, PathBuf),
        Leave(PathBuf),
    }
    let mut work = vec![Work::Copy(link.to_path_buf(), link.to_path_buf())];
    let mut active = BTreeSet::new();
    while let Some(item) = work.pop() {
        let (source, destination) = match item {
            Work::Leave(path) => {
                active.remove(&path);
                continue;
            }
            Work::Copy(source, destination) => (source, destination),
        };
        if !active.insert(source.clone()) {
            return Err(invalid("skill archive contains a cyclic link"));
        }
        work.push(Work::Leave(source.clone()));
        match entries.get(&source) {
            Some(Entry::Link(target)) => {
                work.push(Work::Copy(target.clone(), destination));
            }
            Some(Entry::File) => {
                write_file(&mut File::open(root.join(source))?, &root.join(destination), total)?;
            }
            Some(Entry::Directory) => {
                charge(total, BLOCK_BYTES)?;
                fs::create_dir(root.join(&destination))?;
                if let Some(paths) = children.get(&source) {
                    for child in paths.iter().rev() {
                        work.push(Work::Copy(
                            child.clone(),
                            destination.join(child.file_name().unwrap()),
                        ));
                    }
                }
            }
            None => {
                // A link can also occur in an intermediate target component.
                let prefix = source
                    .ancestors()
                    .skip(1)
                    .find(|prefix| matches!(entries.get(*prefix), Some(Entry::Link(_))));
                let Some(prefix) = prefix else {
                    return Err(invalid("skill archive contains a dangling link"));
                };
                if !active.insert(prefix.to_path_buf()) {
                    return Err(invalid("skill archive contains a cyclic link"));
                }
                let Entry::Link(target) = &entries[prefix] else {
                    return Err(invalid("skill archive link resolution failed"));
                };
                let suffix = source.strip_prefix(prefix).unwrap();
                work.push(Work::Leave(prefix.to_path_buf()));
                work.push(Work::Copy(target.join(suffix), destination));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "import_archive_tests.rs"]
mod tests;
