//! Claude transcript and sibling sidecar deletion on the authorized host.
use std::{fs, io, path::Path};
use vw_api_types::external_session::{ExternalSessionIdentity, SessionProvider, SessionSource};
use crate::sessions::{cache::TranscriptCache, providers, source::SourceRegistry};

/// The caller must hold its application lifecycle guard throughout this call.
/// `is_active` must query trusted host runtime state under that guard, never a
/// client-provided flag. An unavailable activity check fails closed. Gateway
/// authorization and explicit delete confirmation must precede this operation.
/// Sidecar failures may leave a partial cleanup; the transcript is removed last.
pub fn delete_session<F>(
    registry: &SourceRegistry,
    identity: &ExternalSessionIdentity,
    cache: &TranscriptCache,
    mut is_active: F,
) -> io::Result<()>
where F: FnMut(&ExternalSessionIdentity) -> io::Result<bool> {
    if identity.provider != SessionProvider::Claude
        || !matches!(&identity.source, SessionSource::File { .. })
    {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "expected a Claude file session"));
    }
    let source = registry.resolve(identity)?;
    if is_active(identity)? { return Err(active()); }
    if source.path().extension().and_then(|v| v.to_str()) != Some("jsonl") {
        return Err(forbidden());
    }
    let meta = providers::claude::scan_session_file(source.path(), identity)?
        .ok_or_else(forbidden)?;
    if meta.identity != *identity { return Err(forbidden()); }
    let sidecar = source.path().with_extension("");
    if sidecar == source.root() || !sidecar.starts_with(source.root()) {
        return Err(forbidden());
    }
    let has_sidecar = match fs::symlink_metadata(&sidecar) {
        Ok(_) => { check_tree(&sidecar, 0)?; true }
        Err(e) if e.kind() == io::ErrorKind::NotFound => false,
        Err(e) => return Err(storage(e)),
    };
    // Resolve again immediately before mutation, and refuse a changed transcript.
    let current = registry.resolve(identity)?;
    if current.path() != source.path() || current.root() != source.root()
        || providers::claude::scan_session_file(current.path(), identity)?
            .is_none_or(|meta| meta.identity != *identity)
    { return Err(forbidden()); }
    if is_active(identity)? { return Err(active()); }
    // Invalidate before any mutation as cleanup itself can partially succeed.
    cache.remove(identity).map_err(|_| io::Error::other("session cache unavailable"))?;
    if has_sidecar { remove_tree(&sidecar, 0)?; }
    registry.resolve(identity)?;
    fs::remove_file(source.path()).map_err(storage)?;
    cache.remove(identity).map_err(|_| io::Error::other("session cache unavailable"))
}

fn active() -> io::Error {
    io::Error::new(io::ErrorKind::WouldBlock, "active session cannot be deleted")
}
fn forbidden() -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, "Claude session source mismatch or unsafe sidecar")
}
fn storage(error: io::Error) -> io::Error {
    io::Error::new(error.kind(), "Claude session storage operation failed")
}

// Check descendants before the first deletion. Never follow links, even when
// they target another authorized session. Bound recursion to avoid stack abuse.
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
    // Repeat checks during cleanup rather than using remove_dir_all on an
    // unchecked subtree. Host lifecycle serialization remains the caller's duty.
    check_tree(path, depth)?;
    if fs::symlink_metadata(path).map_err(storage)?.is_dir() {
        for entry in fs::read_dir(path).map_err(storage)? {
            remove_tree(&entry.map_err(storage)?.path(), depth + 1)?;
        }
        fs::remove_dir(path).map_err(storage)
    } else {
        fs::remove_file(path).map_err(storage)
    }
}
