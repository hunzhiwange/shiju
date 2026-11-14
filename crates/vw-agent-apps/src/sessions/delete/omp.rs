//! Oh My Pi file and owned sidecar deletion on the authorized execution host.
use std::{fs, io, path::{Path, PathBuf}};
use crate::sessions::{cache::TranscriptCache, providers::omp::verify_omp_session_id, source::{SourceRegistry, ValidatedSource}};
use vw_api_types::external_session::{ExternalSessionIdentity, SessionProvider, SessionSource};

/// Requires gateway authorization, explicit confirmation, and a caller-held
/// application lifecycle guard. Activity must come from trusted host state.
/// Partial cleanup is an error and invalidates the transcript cache.
pub fn delete_session<F>(
    registry: &SourceRegistry,
    identity: &ExternalSessionIdentity,
    cache: &TranscriptCache,
    mut is_active: F,
) -> io::Result<()>
where F: FnMut(&ExternalSessionIdentity) -> io::Result<bool> {
    if identity.provider != SessionProvider::Omp
        || !matches!(&identity.source, SessionSource::File { .. })
    { return Err(io::Error::new(io::ErrorKind::InvalidInput, "expected an OMP file session")); }
    let source = registry.resolve(identity)?;
    if is_active(identity)? { return Err(active()); }
    verify(&source)?;
    let artifacts = source.path().with_extension("");
    if artifacts == source.root() || !artifacts.starts_with(source.root()) {
        return Err(forbidden());
    }
    check_optional_tree(&artifacts)?;
    let backups = backups(source.path())?;
    let current = registry.resolve(identity)?;
    if current.path() != source.path() || current.root() != source.root() {
        return Err(forbidden());
    }
    verify(&current)?;
    if is_active(identity)? { return Err(active()); }
    cache.remove(identity).map_err(|_| io::Error::other("session cache unavailable"))?;
    registry.resolve(identity)?;
    verify(&source)?;
    if check_optional_tree(&artifacts)? { remove_tree(&artifacts, 0)?; }
    for backup in backups {
        registry.resolve(identity)?;
        // Backups are sibling files, never directories or links.
        let meta = fs::symlink_metadata(&backup).map_err(storage)?;
        if !meta.is_file() || meta.file_type().is_symlink() { return Err(forbidden()); }
        fs::remove_file(backup).map_err(storage)?;
    }
    let current = registry.resolve(identity)?;
    verify(&current)?;
    fs::remove_file(current.path()).map_err(storage)?;
    cache.remove(identity).map_err(|_| io::Error::other("session cache unavailable"))
}

fn verify(source: &ValidatedSource) -> io::Result<()> {
    verify_omp_session_id(source.path(), &source.identity().session_id.0)
        .map_err(|_| forbidden())
}
fn backups(source: &Path) -> io::Result<Vec<PathBuf>> {
    let filename = source.file_name().and_then(|s| s.to_str()).ok_or_else(forbidden)?;
    let prefix = format!("{filename}.");
    let mut paths = Vec::new();
    for entry in fs::read_dir(source.parent().ok_or_else(forbidden)?).map_err(storage)? {
        let entry = entry.map_err(storage)?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue; };
        if !name.starts_with(&prefix) || !name.ends_with(".bak") { continue; }
        let kind = entry.file_type().map_err(storage)?;
        if kind.is_symlink() { return Err(forbidden()); }
        if kind.is_file() { paths.push(entry.path()); }
    }
    Ok(paths)
}
fn check_optional_tree(path: &Path) -> io::Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(_) => { check_tree(path, 0)?; Ok(true) }
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(storage(e)),
    }
}
fn check_tree(path: &Path, depth: usize) -> io::Result<()> {
    if depth > 64 { return Err(forbidden()); }
    let meta = fs::symlink_metadata(path).map_err(storage)?;
    if meta.file_type().is_symlink() || (!meta.is_dir() && !meta.is_file()) {
        return Err(forbidden());
    }
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
fn forbidden() -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, "OMP session source mismatch or unsafe sidecar")
}
fn active() -> io::Error {
    io::Error::new(io::ErrorKind::WouldBlock, "active session cannot be deleted")
}
fn storage(error: io::Error) -> io::Error {
    io::Error::new(error.kind(), "OMP session storage operation failed")
}
