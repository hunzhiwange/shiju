//! Read-only inventory on the already authorized execution host.
use std::{fs, io::{self, Read}, path::{Path, PathBuf}};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{archive::{MAX_ARCHIVE_ENTRIES, MAX_ARCHIVE_TOTAL_BYTES}, local_install::{LocalSkill, validate_directory}};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledSkillDetail {
    pub skill: LocalSkill,
    pub present: bool,
    /// Content identity, never a semantic version.
    pub content_hash: Option<String>,
    pub version: Option<String>,
    pub markdown: Option<String>,
    pub manifest: Option<String>,
    pub resources: Vec<PathBuf>,
}

fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "invalid or unsafe installed skill")
}

fn open_safe(root: &Path, path: &Path, directory: bool) -> io::Result<fs::File> {
    crate::providers::paths::open_beneath(root, path, directory).map_err(|_| invalid())
}

/// Persisted records own identity; marketplace metadata must not add inventory rows.
pub fn get_all_installed(root: &Path, saved: &[LocalSkill]) -> io::Result<Vec<InstalledSkillDetail>> {
    if !root.is_absolute() { return Err(invalid()); }
    open_safe(root, root, true)?;
    let mut seen = std::collections::BTreeSet::new();
    let mut result = Vec::new();
    for skill in saved {
        validate_directory(&skill.directory).map_err(|_| invalid())?;
        if !seen.insert(skill.directory.to_ascii_lowercase()) { return Err(invalid()); }
        let tree = root.join(&skill.directory);
        let mut detail = InstalledSkillDetail {
            skill: skill.clone(), present: false, content_hash: None, version: None,
            markdown: None, manifest: None, resources: Vec::new(),
        };
        match fs::symlink_metadata(&tree) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => { result.push(detail); continue; }
            Err(error) => return Err(error),
            Ok(metadata) if !metadata.is_dir() => return Err(invalid()),
            Ok(_) => {}
        }
        open_safe(root, &tree, true)?;
        detail.markdown = read_document(root, &tree.join("SKILL.md"))?;
        detail.manifest = read_document(root, &tree.join("SKILL.toml"))?;
        #[derive(Default, Deserialize)]
        struct Metadata { name: Option<String>, description: Option<String>, version: Option<String> }
        let metadata: Metadata = if let Some(manifest) = &detail.manifest {
            toml::from_str(manifest).map_err(|_| invalid())?
        } else if let Some(markdown) = &detail.markdown {
            let mut lines = markdown.trim_start_matches('\u{feff}').lines();
            if lines.next() == Some("---") {
                let mut yaml = String::new();
                let mut closed = false;
                for line in lines {
                    if line == "---" { closed = true; break; }
                    yaml.push_str(line); yaml.push('\n');
                }
                if !closed { return Err(invalid()); }
                if yaml.trim().is_empty() { Metadata::default() }
                else { serde_yaml::from_str(&yaml).map_err(|_| invalid())? }
            } else { Metadata::default() }
        } else { return Err(invalid()); };
        if let Some(name) = metadata.name {
            if name.trim().is_empty() { return Err(invalid()); }
            detail.skill.name = name;
        }
        detail.skill.description = metadata.description;
        detail.version = metadata.version;
        let files = collect_files(&tree)?;
        detail.content_hash = Some(hash_files(&tree, &files)?);
        detail.resources = files.into_iter().filter(|path| path != Path::new("SKILL.md") && path != Path::new("SKILL.toml")).collect();
        detail.present = true;
        result.push(detail);
    }
    result.sort_by(|a, b| a.skill.directory.cmp(&b.skill.directory));
    Ok(result)
}

fn read_document(root: &Path, path: &Path) -> io::Result<Option<String>> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
        Ok(_) => {}
    }
    let mut text = String::new();
    open_safe(root, path, false)?.take(1024 * 1024 + 1).read_to_string(&mut text)?;
    if text.len() > 1024 * 1024 || text.trim().is_empty() { return Err(invalid()); }
    Ok(Some(text))
}

/// Matches upstream update identity: sorted relative file names and bytes,
/// excluding hidden entries. Links and special files are rejected.
pub fn compute_dir_hash(root: &Path) -> io::Result<String> {
    let files = collect_files(root)?;
    hash_files(root, &files)
}

fn collect_files(root: &Path) -> io::Result<Vec<PathBuf>> {
    open_safe(root, root, true)?;
    let mut pending = vec![root.to_owned()];
    let mut files = Vec::new();
    let mut count = 0;
    while let Some(directory) = pending.pop() {
        open_safe(root, &directory, true)?;
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            count += 1;
            if count > MAX_ARCHIVE_ENTRIES { return Err(invalid()); }
            let name = entry.file_name();
            let name = name.to_str().ok_or_else(invalid)?;
            if name.starts_with('.') { continue; }
            let path = entry.path();
            let kind = entry.file_type()?;
            if kind.is_dir() {
                if path.components().count() > root.components().count() + 128 { return Err(invalid()); }
                pending.push(path);
            } else if kind.is_file() {
                files.push(path.strip_prefix(root).map_err(|_| invalid())?.to_owned());
            } else { return Err(invalid()); }
        }
    }
    files.sort();
    Ok(files)
}

fn hash_files(root: &Path, files: &[PathBuf]) -> io::Result<String> {
    let mut hash = Sha256::new();
    let mut remaining = MAX_ARCHIVE_TOTAL_BYTES;
    let mut buffer = [0u8; 8192];
    for relative in files {
        hash.update(relative.to_str().ok_or_else(invalid)?.replace('\\', "/").as_bytes());
        hash.update(b"\0");
        let mut file = open_safe(root, &root.join(relative), false)?;
        loop {
            let read = file.read(&mut buffer)?;
            if read == 0 { break; }
            remaining = remaining.checked_sub(read as u64).ok_or_else(invalid)?;
            hash.update(&buffer[..read]);
        }
        hash.update(b"\0");
    }
    Ok(format!("{:x}", hash.finalize()))
}

/// Missing directories invalidate cached hashes. Existing trees are read afresh.
pub fn local_hash_for_update_check(root: &Path, directory: &str) -> io::Result<Option<String>> {
    validate_directory(directory).map_err(|_| invalid())?;
    let path = root.join(directory);
    match fs::symlink_metadata(&path) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
        Ok(metadata) if metadata.is_dir() => {
            open_safe(root, &path, true)?;
            compute_dir_hash(&path).map(Some)
        }
        Ok(_) => Err(invalid()),
    }
}
