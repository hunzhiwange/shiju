//! Kimi native session-directory deletion on the authorized execution host.
use std::{
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

use crate::sessions::{
    cache::TranscriptCache,
    source::{SourceRegistry, ValidatedSource},
};
use vw_api_types::external_session::{ExternalSessionIdentity, SessionProvider, SessionSource};

/// Requires gateway authorization, explicit confirmation and a caller-held
/// application lifecycle guard. Activity must come from trusted runtime state.
/// Index/dirty-marker failure after deletion is reported as partial cleanup;
/// the cache is invalidated before any mutation, including partial failures.
pub fn delete_session<F>(
    registry: &SourceRegistry,
    identity: &ExternalSessionIdentity,
    cache: &TranscriptCache,
    mut is_active: F,
) -> io::Result<()>
where
    F: FnMut(&ExternalSessionIdentity) -> io::Result<bool>,
{
    if identity.provider != SessionProvider::Kimi
        || !matches!(&identity.source, SessionSource::File { .. })
    {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "expected a Kimi file session"));
    }
    let source = registry.resolve(identity)?;
    if is_active(identity)? { return Err(active()); }
    let directory = verify(&source)?;
    check_tree(source.root(), directory, 0)?;
    let index = source.root().join("session_index.jsonl");
    let dirty = source.root().join("sessions/.index-dirty");
    let original = read_index(source.root(), &index)?;
    optional_path(source.root(), &dirty, true)?;

    // Stage an index replacement instead of appending through a possible hard
    // link. Keep the original index intact until the session has been removed.
    let mut staged = tempfile::NamedTempFile::new_in(source.root()).map_err(storage)?;
    if let Some(bytes) = &original {
        staged.as_file().set_permissions(fs::metadata(&index).map_err(storage)?.permissions())
            .map_err(storage)?;
        staged.write_all(bytes).map_err(storage)?;
        if !bytes.is_empty() && !bytes.ends_with(b"\n") {
            staged.write_all(b"\n").map_err(storage)?;
        }
    }
    writeln!(staged, "{}", serde_json::json!({
        "sessionId": identity.session_id.0, "deleted": true
    })).map_err(storage)?;
    staged.as_file().sync_all().map_err(storage)?;

    let current = registry.resolve(identity)?;
    if current.path() != source.path() || current.root() != source.root() {
        return Err(forbidden());
    }
    verify(&current)?;
    if is_active(identity)? { return Err(active()); }
    if read_index(source.root(), &index)? != original { return Err(changed()); }
    optional_path(source.root(), &dirty, true)?;
    cache.remove(identity).map_err(|_| io::Error::other("session cache unavailable"))?;
    registry.resolve(identity)?;
    verify(&source)?;
    remove_tree(source.root(), directory, 0)?;

    if read_index(source.root(), &index)? != original { return Err(changed()); }
    staged.persist(&index).map_err(|error| storage(error.error))?;
    if !optional_path(source.root(), &dirty, true)? {
        fs::create_dir(&dirty).map_err(storage)?;
    }
    optional_path(source.root(), &dirty, true)?;
    let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| io::Error::other("Kimi deletion clock unavailable"))?.as_millis();
    let marker = dirty.join(format!("{}.{timestamp}", identity.session_id.0));
    fs::OpenOptions::new().write(true).create_new(true).open(marker).map_err(storage)?;
    cache.remove(identity).map_err(|_| io::Error::other("session cache unavailable"))
}

fn verify(source: &ValidatedSource) -> io::Result<&Path> {
    let id = &source.identity().session_id.0;
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_')) {
        return Err(forbidden());
    }
    let relative = source.path().strip_prefix(source.root()).map_err(|_| forbidden())?;
    let parts: Vec<_> = relative.components().map(|part| part.as_os_str()).collect();
    // Kimi discovery emits paths relative to the instance's data directory.
    if parts.len() != 6 || parts[0] != "sessions" || parts[2] != id.as_str()
        || parts[3..] != ["agents", "main", "wire.jsonl"]
    {
        return Err(forbidden());
    }
    source.path().ancestors().nth(3).ok_or_else(forbidden)
}

/// Check every ancestor even when the leaf is missing. Only the leaf may be
/// absent; no auxiliary path can escape the same authorized instance root.
fn optional_path(root: &Path, path: &Path, directory: bool) -> io::Result<bool> {
    if path == root || !path.starts_with(root) { return Err(forbidden()); }
    let mut ancestor = PathBuf::new();
    for component in path.components() {
        ancestor.push(component.as_os_str());
        match fs::symlink_metadata(&ancestor) {
            Ok(meta) => {
                let wrong_kind = if ancestor == path && !directory {
                    !meta.is_file()
                } else {
                    !meta.is_dir()
                };
                if meta.file_type().is_symlink() || wrong_kind {
                    return Err(forbidden());
                }
            }
            Err(error) if ancestor == path && error.kind() == io::ErrorKind::NotFound => return Ok(false),
            Err(error) => return Err(storage(error)),
        }
    }
    Ok(true)
}

fn read_index(root: &Path, path: &Path) -> io::Result<Option<Vec<u8>>> {
    if !optional_path(root, path, false)? { return Ok(None); }
    let mut bytes = Vec::new();
    fs::File::open(path).map_err(storage)?.take(32 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes).map_err(storage)?;
    if bytes.len() > 32 * 1024 * 1024 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Kimi session index exceeds deletion limit"));
    }
    Ok(Some(bytes))
}

fn check_tree(root: &Path, path: &Path, depth: usize) -> io::Result<()> {
    if depth > 64 { return Err(forbidden()); }
    let meta = fs::symlink_metadata(path).map_err(storage)?;
    if meta.file_type().is_symlink() || (!meta.is_dir() && !meta.is_file()) { return Err(forbidden()); }
    optional_path(root, path, meta.is_dir())?;
    if meta.is_dir() {
        for entry in fs::read_dir(path).map_err(storage)? {
            check_tree(root, &entry.map_err(storage)?.path(), depth + 1)?;
        }
    }
    Ok(())
}

fn remove_tree(root: &Path, path: &Path, depth: usize) -> io::Result<()> {
    check_tree(root, path, depth)?;
    if fs::symlink_metadata(path).map_err(storage)?.is_dir() {
        for entry in fs::read_dir(path).map_err(storage)? {
            remove_tree(root, &entry.map_err(storage)?.path(), depth + 1)?;
        }
        optional_path(root, path, true)?;
        fs::remove_dir(path).map_err(storage)
    } else {
        optional_path(root, path, false)?;
        fs::remove_file(path).map_err(storage)
    }
}

fn active() -> io::Error {
    io::Error::new(io::ErrorKind::WouldBlock, "active session cannot be deleted")
}
fn changed() -> io::Error {
    io::Error::new(io::ErrorKind::WouldBlock, "Kimi session index changed during deletion")
}
fn forbidden() -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, "Kimi session source mismatch or unsafe cleanup path")
}
fn storage(error: io::Error) -> io::Error {
    io::Error::new(error.kind(), "Kimi session storage operation failed")
}
