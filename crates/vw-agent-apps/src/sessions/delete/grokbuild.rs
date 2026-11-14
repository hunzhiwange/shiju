//! Grok Build session-directory deletion on the authorized execution host.
use std::{fs, io, io::Read, path::Path};
use crate::sessions::{cache::TranscriptCache, source::{SourceRegistry, ValidatedSource}};
use vw_api_types::external_session::{ExternalSessionIdentity, SessionProvider, SessionSource};

/// Requires gateway authorization and explicit confirmation. The caller holds
/// its application lifecycle guard throughout; activity comes from trusted host
/// runtime state and errors fail closed. Partial cleanup invalidates the cache.
pub fn delete_session<F>(
    registry: &SourceRegistry,
    identity: &ExternalSessionIdentity,
    cache: &TranscriptCache,
    mut is_active: F,
) -> io::Result<()>
where F: FnMut(&ExternalSessionIdentity) -> io::Result<bool> {
    if identity.provider != SessionProvider::Grokbuild
        || !matches!(&identity.source, SessionSource::File { .. })
    {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "expected a Grok Build file session"));
    }
    let source = registry.resolve(identity)?;
    if is_active(identity)? { return Err(active()); }
    let directory = verify(&source)?;
    check_tree(directory, 0)?;
    let current = registry.resolve(identity)?;
    if current.path() != source.path() || current.root() != source.root() {
        return Err(forbidden());
    }
    verify(&current)?;
    if is_active(identity)? { return Err(active()); }
    cache.remove(identity).map_err(|_| io::Error::other("session cache unavailable"))?;
    // Revalidate before cleanup. Keep summary until all sidecars have been
    // removed so a partial failure still has its native session identity.
    registry.resolve(identity)?;
    verify(&source)?;
    check_tree(directory, 0)?;
    for entry in fs::read_dir(directory).map_err(storage)? {
        let path = entry.map_err(storage)?.path();
        if path != source.path() { remove_tree(&path, 1)?; }
    }
    registry.resolve(identity)?;
    verify(&source)?;
    fs::remove_file(source.path()).map_err(storage)?;
    fs::remove_dir(directory).map_err(storage)?;
    cache.remove(identity).map_err(|_| io::Error::other("session cache unavailable"))
}

fn verify(source: &ValidatedSource) -> io::Result<&Path> {
    let SessionSource::File { relative_path } = &source.identity().source else { return Err(forbidden()); };
    let parts: Vec<_> = Path::new(relative_path).components().collect();
    // Discovery binds locators relative to the instance configuration root.
    if parts.len() < 3
        || !matches!(parts[0].as_os_str().to_str(), Some("sessions" | "archived_sessions"))
        || source.path().file_name().and_then(|s| s.to_str()) != Some("summary.json")
    { return Err(forbidden()); }
    let directory = source.path().parent().ok_or_else(forbidden)?;
    if directory == source.root() || !directory.starts_with(source.root())
        || directory.file_name().and_then(|s| s.to_str()) != Some(source.identity().session_id.0.as_str())
    { return Err(forbidden()); }
    let mut bytes = Vec::new();
    fs::File::open(source.path()).map_err(storage)?.take(4 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes).map_err(storage)?;
    if bytes.len() > 4 * 1024 * 1024 { return Err(forbidden()); }
    let summary: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid Grok Build summary"))?;
    if summary.get("info").and_then(|v| v.get("id")).and_then(|v| v.as_str())
        != Some(source.identity().session_id.0.as_str())
    { return Err(forbidden()); }
    Ok(directory)
}

fn check_tree(path: &Path, depth: usize) -> io::Result<()> {
    if depth > 64 { return Err(forbidden()); }
    let meta = fs::symlink_metadata(path).map_err(storage)?;
    if meta.file_type().is_symlink() || (!meta.is_dir() && !meta.is_file()) { return Err(forbidden()); }
    if meta.is_dir() {
        for entry in fs::read_dir(path).map_err(storage)? {
            check_tree(&entry.map_err(storage)?.path(), depth + 1)?;
        }
    }
    Ok(())
}
fn remove_tree(path: &Path, depth: usize) -> io::Result<()> {
    check_tree(path, depth)?;
    if fs::symlink_metadata(path).map_err(storage)?.is_dir() {
        for entry in fs::read_dir(path).map_err(storage)? {
            remove_tree(&entry.map_err(storage)?.path(), depth + 1)?;
        }
        fs::remove_dir(path).map_err(storage)
    } else { fs::remove_file(path).map_err(storage) }
}
fn active() -> io::Error {
    io::Error::new(io::ErrorKind::WouldBlock, "active session cannot be deleted")
}
fn forbidden() -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, "Grok Build session source mismatch or unsafe directory")
}
fn storage(error: io::Error) -> io::Error {
    io::Error::new(error.kind(), "Grok Build session storage operation failed")
}
