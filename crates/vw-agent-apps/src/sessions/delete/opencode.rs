//! OpenCode legacy storage deletion. SQLite is deliberately a separate operation.
use std::{collections::BTreeSet, fs, io, path::{Path, PathBuf}};
use serde_json::Value;
use crate::sessions::{cache::TranscriptCache, source::{SourceRegistry, ValidatedSource}};
use vw_api_types::external_session::{ExternalSessionIdentity, SessionProvider, SessionSource};

/// Requires gateway authorization, explicit confirmation and a caller-held
/// application lifecycle guard throughout this operation. The callback queries
/// trusted host activity and fails closed. A partial cleanup returns an error.
pub fn delete_session<F>(
    registry: &SourceRegistry,
    identity: &ExternalSessionIdentity,
    cache: &TranscriptCache,
    mut is_active: F,
) -> io::Result<()>
where F: FnMut(&ExternalSessionIdentity) -> io::Result<bool>,
{
    if identity.provider != SessionProvider::Opencode
        || !matches!(&identity.source, SessionSource::File { .. }) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "expected an OpenCode file session"));
    }
    let source = registry.resolve(identity)?;
    if is_active(identity)? { return Err(active()); }
    let plan = prepare(&source)?;
    let current = registry.resolve(identity)?;
    if current.path() != source.path() || current.root() != source.root()
        || prepare(&current)? != plan { return Err(changed()); }
    if is_active(identity)? { return Err(active()); }
    cache.remove(identity).map_err(|_| io::Error::other("session cache unavailable"))?;
    // Activity callbacks may observe/reenter host state; repeat all validation
    // after the final callback and before performing any filesystem mutation.
    registry.resolve(identity)?;
    if prepare(&source)? != plan { return Err(changed()); }
    for (path, bytes) in &plan.files {
        if !safe_path(source.root(), path, false)?
            || fs::read(path).map_err(storage)? != *bytes { return Err(changed()); }
        fs::remove_file(path).map_err(storage)?;
    }
    // Children precede parents; never recurse through a newly introduced entry.
    for path in &plan.directories {
        if !safe_path(source.root(), path, true)? { return Err(changed()); }
        fs::remove_dir(path).map_err(storage)?;
    }
    cache.remove(identity).map_err(|_| io::Error::other("session cache unavailable"))
}

#[derive(PartialEq, Eq)]
struct Cleanup {
    files: Vec<(PathBuf, Vec<u8>)>,
    directories: Vec<PathBuf>,
}

fn prepare(source: &ValidatedSource) -> io::Result<Cleanup> {
    let root = source.root();
    let id = &source.identity().session_id.0;
    if !valid_id(id) || source.path() != root.join("message").join(id) {
        return Err(forbidden());
    }
    let mut messages = Cleanup { files: Vec::new(), directories: Vec::new() };
    collect(root, source.path(), &mut messages, 0)?;
    let mut message_ids = BTreeSet::new();
    for (path, bytes) in &messages.files {
        let value = record(path, bytes)?;
        let message_id = value.get("id").and_then(Value::as_str).filter(|id| valid_id(id))
            .ok_or_else(forbidden)?;
        if value.get("sessionID").and_then(Value::as_str) != Some(id.as_str())
            || path.file_stem().and_then(|s| s.to_str()) != Some(message_id)
            || !message_ids.insert(message_id.to_owned()) { return Err(forbidden()); }
    }
    let mut plan = Cleanup { files: Vec::new(), directories: Vec::new() };
    for message_id in message_ids {
        let mut parts = Cleanup { files: Vec::new(), directories: Vec::new() };
        collect(root, &root.join("part").join(&message_id), &mut parts, 0)?;
        for (path, bytes) in &parts.files {
            let value = record(path, bytes)?;
            if value.get("messageID").and_then(Value::as_str) != Some(message_id.as_str())
                || value.get("sessionID").is_some_and(|v| v.as_str() != Some(id.as_str())) {
                return Err(forbidden());
            }
        }
        plan.files.extend(parts.files);
        plan.directories.extend(parts.directories);
    }
    let diff = root.join("session_diff").join(format!("{id}.json"));
    if safe_path(root, &diff, false)? {
        plan.files.push((diff.clone(), fs::read(&diff).map_err(storage)?));
    }
    plan.files.extend(messages.files);
    plan.directories.extend(messages.directories);
    // Discovery reads session metadata recursively. Refuse duplicate locators
    // instead of selecting the first matching native ID in another project.
    let mut sessions = Cleanup { files: Vec::new(), directories: Vec::new() };
    collect(root, &root.join("session"), &mut sessions, 0)?;
    let filename = format!("{id}.json");
    let mut found = false;
    for (path, bytes) in sessions.files {
        if path.file_name().and_then(|s| s.to_str()) != Some(filename.as_str()) { continue; }
        if found || record(&path, &bytes)?.get("id").and_then(Value::as_str) != Some(id.as_str()) {
            return Err(forbidden());
        }
        found = true;
        plan.files.push((path, bytes));
    }
    Ok(plan)
}

fn valid_id(id: &str) -> bool {
    !id.is_empty() && id != "." && id != ".." && !id.contains(['/', '\\', '\0'])
}

fn record(path: &Path, bytes: &[u8]) -> io::Result<Value> {
    if path.extension().and_then(|s| s.to_str()) != Some("json") { return Err(forbidden()); }
    serde_json::from_slice(bytes).map_err(|_| io::Error::new(
        io::ErrorKind::InvalidData, "invalid OpenCode cleanup record"))
}

/// Validate every existing ancestor, including ancestors of absent optional
/// paths. No symlinks or special files are permitted anywhere in cleanup trees.
fn safe_path(root: &Path, path: &Path, directory: bool) -> io::Result<bool> {
    if path == root || !path.starts_with(root) { return Err(forbidden()); }
    let mut ancestor = PathBuf::new();
    let mut missing = false;
    for component in path.components() {
        if matches!(component, std::path::Component::ParentDir | std::path::Component::CurDir) {
            return Err(forbidden());
        }
        ancestor.push(component.as_os_str());
        if missing { continue; }
        match fs::symlink_metadata(&ancestor) {
            Ok(meta) => {
                if meta.file_type().is_symlink() || if ancestor == path && !directory {
                    !meta.is_file()
                } else { !meta.is_dir() } { return Err(forbidden()); }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => missing = true,
            Err(error) => return Err(storage(error)),
        }
    }
    Ok(!missing)
}

fn collect(root: &Path, path: &Path, plan: &mut Cleanup, depth: usize) -> io::Result<()> {
    if depth > 64 { return Err(forbidden()); }
    if !safe_path(root, path, true)? { return Ok(()); }
    let mut entries = fs::read_dir(path).map_err(storage)?
        .map(|entry| entry.map(|entry| entry.path())).collect::<io::Result<Vec<_>>>().map_err(storage)?;
    entries.sort();
    for path in entries {
        let meta = fs::symlink_metadata(&path).map_err(storage)?;
        if meta.file_type().is_symlink() { return Err(forbidden()); }
        if meta.is_dir() { collect(root, &path, plan, depth + 1)?; }
        else {
            if !safe_path(root, &path, false)? { return Err(changed()); }
            plan.files.push((path.clone(), fs::read(path).map_err(storage)?));
        }
    }
    plan.directories.push(path.to_path_buf());
    Ok(())
}

fn active() -> io::Error {
    io::Error::new(io::ErrorKind::WouldBlock, "active session cannot be deleted")
}
fn changed() -> io::Error {
    io::Error::new(io::ErrorKind::WouldBlock, "OpenCode storage changed during deletion")
}
fn forbidden() -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, "OpenCode session source mismatch or unsafe cleanup path")
}
fn storage(error: io::Error) -> io::Error {
    io::Error::new(error.kind(), "OpenCode session storage operation failed")
}
