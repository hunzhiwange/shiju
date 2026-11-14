//! OpenClaw transcript deletion and agent-local sessions index cleanup.
use crate::sessions::{cache::TranscriptCache, providers, source::SourceRegistry};
use std::{fs, io::{self, Write}, path::{Path, PathBuf}};
use vw_api_types::external_session::{ExternalSessionIdentity, SessionProvider, SessionSource};

/// Requires explicit gateway authorization/confirmation. The caller holds its
/// application lifecycle guard throughout; the activity callback uses trusted
/// runtime state and fails closed. Index cleanup may precede a failed unlink.
pub fn delete_session<F>(
    registry: &SourceRegistry,
    identity: &ExternalSessionIdentity,
    cache: &TranscriptCache,
    mut is_active: F,
) -> io::Result<()>
where F: FnMut(&ExternalSessionIdentity) -> io::Result<bool> {
    if identity.provider != SessionProvider::Openclaw
        || !matches!(&identity.source, SessionSource::File { .. })
    {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "expected an OpenClaw file session"));
    }
    let source = registry.resolve(identity)?;
    if is_active(identity)? { return Err(active()); }
    verify(source.path(), identity)?;
    let index_path = source.path().parent().ok_or_else(forbidden)?.join("sessions.json");
    let original = read_index(source.root(), &index_path)?;
    let replacement = match &original {
        Some(bytes) => {
            let mut index: serde_json::Map<String, serde_json::Value> =
                serde_json::from_slice(bytes).map_err(|_| io::Error::new(
                    io::ErrorKind::InvalidData, "invalid OpenClaw sessions index"))?;
            let path = source.path().to_str().ok_or_else(forbidden)?;
            index.retain(|_, entry| {
                entry.get("sessionId").and_then(serde_json::Value::as_str)
                    != Some(identity.session_id.0.as_str())
                    && entry.get("sessionFile").and_then(serde_json::Value::as_str) != Some(path)
            });
            Some(serde_json::to_vec_pretty(&index).map_err(|_| io::Error::other(
                "OpenClaw index serialization failed"))?)
        }
        None => None,
    };
    let current = registry.resolve(identity)?;
    if current.path() != source.path() || current.root() != source.root() { return Err(forbidden()); }
    verify(current.path(), identity)?;
    if is_active(identity)? { return Err(active()); }
    if read_index(source.root(), &index_path)? != original { return Err(changed()); }
    cache.remove(identity).map_err(|_| io::Error::other("session cache unavailable"))?;
    if let Some(bytes) = replacement {
        // Stage in the same directory and atomically replace: never truncate the
        // shared index or follow an index symlink. Preserve native permissions.
        let permissions = fs::metadata(&index_path).map_err(storage)?.permissions();
        let mut staged = tempfile::NamedTempFile::new_in(index_path.parent().ok_or_else(forbidden)?)
            .map_err(storage)?;
        staged.as_file().set_permissions(permissions).map_err(storage)?;
        staged.write_all(&bytes).map_err(storage)?;
        staged.as_file().sync_all().map_err(storage)?;
        registry.resolve(identity)?;
        if read_index(source.root(), &index_path)? != original { return Err(changed()); }
        staged.persist(&index_path).map_err(|e| storage(e.error))?;
    }
    registry.resolve(identity)?;
    verify(source.path(), identity)?;
    fs::remove_file(source.path()).map_err(storage)?;
    cache.remove(identity).map_err(|_| io::Error::other("session cache unavailable"))
}

fn verify(path: &Path, identity: &ExternalSessionIdentity) -> io::Result<()> {
    let SessionSource::File { relative_path } = &identity.source else { return Err(forbidden()); };
    let parts: Vec<_> = Path::new(relative_path).components().collect();
    if parts.len() != 4 || parts[0].as_os_str() != "agents"
        || parts[2].as_os_str() != "sessions"
        || path.extension().and_then(|s| s.to_str()) != Some("jsonl")
    { return Err(forbidden()); }
    let meta = providers::openclaw::parse_session(path, None, identity).ok_or_else(forbidden)?;
    if meta.identity != *identity { return Err(forbidden()); }
    Ok(())
}

fn read_index(root: &Path, path: &Path) -> io::Result<Option<Vec<u8>>> {
    if !path.starts_with(root) || path == root { return Err(forbidden()); }
    let mut ancestor = PathBuf::new();
    for component in path.components() {
        ancestor.push(component.as_os_str());
        match fs::symlink_metadata(&ancestor) {
            Ok(meta) => {
                if meta.file_type().is_symlink() || (ancestor == path && !meta.is_file()) {
                    return Err(forbidden());
                }
            }
            Err(e) if ancestor == path && e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(storage(e)),
        }
    }
    fs::read(path).map(Some).map_err(storage)
}
fn active() -> io::Error {
    io::Error::new(io::ErrorKind::WouldBlock, "active session cannot be deleted")
}
fn changed() -> io::Error {
    io::Error::new(io::ErrorKind::WouldBlock, "OpenClaw sessions index changed")
}
fn forbidden() -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, "OpenClaw session source mismatch or unsafe index")
}
fn storage(error: io::Error) -> io::Error {
    io::Error::new(error.kind(), "OpenClaw session storage operation failed")
}
