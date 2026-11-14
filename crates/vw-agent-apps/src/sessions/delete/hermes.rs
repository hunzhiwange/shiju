//! Hermes file-session deletion on the authorized execution host.
use std::{fs, io, path::Path};
use crate::sessions::{
    cache::TranscriptCache,
    providers::hermes::parse_jsonl_session,
    source::{SourceRegistry, ValidatedSource},
};
use vw_api_types::external_session::{ExternalSessionIdentity, SessionProvider, SessionSource};

/// Requires gateway delete authorization, explicit confirmation and a caller-held
/// application lifecycle guard. Activity comes from trusted runtime state and
/// errors fail closed. Native Hermes deletes only the transcript: sibling files,
/// directories and state.db are not owned by this operation.
pub fn delete_session<F>(
    registry: &SourceRegistry,
    identity: &ExternalSessionIdentity,
    cache: &TranscriptCache,
    mut is_active: F,
) -> io::Result<()>
where F: FnMut(&ExternalSessionIdentity) -> io::Result<bool> {
    if identity.provider != SessionProvider::Hermes
        || !matches!(&identity.source, SessionSource::File { .. })
    {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "expected a Hermes file session"));
    }
    let source = registry.resolve(identity)?;
    if is_active(identity)? { return Err(active()); }
    verify(&source)?;
    let current = registry.resolve(identity)?;
    if current.path() != source.path() || current.root() != source.root() {
        return Err(forbidden());
    }
    verify(&current)?;
    if is_active(identity)? { return Err(active()); }
    cache.remove(identity).map_err(|_| io::Error::other("session cache unavailable"))?;
    // The activity callback can reenter host state. Resolve and verify again
    // afterwards, immediately before the destructive filesystem operation.
    let current = registry.resolve(identity)?;
    verify(&current)?;
    fs::remove_file(current.path()).map_err(|error| io::Error::new(
        error.kind(), "Hermes session deletion failed"))?;
    cache.remove(identity).map_err(|_| io::Error::other("session cache unavailable"))
}

fn verify(source: &ValidatedSource) -> io::Result<()> {
    let SessionSource::File { relative_path } = &source.identity().source else {
        return Err(forbidden());
    };
    let parts: Vec<_> = Path::new(relative_path).components().collect();
    if parts.len() != 2 || parts[0].as_os_str() != "sessions"
        || !matches!(source.path().extension().and_then(|v| v.to_str()), Some("jsonl" | "json"))
    {
        return Err(forbidden());
    }
    // Share discovery's native metadata ID and filename fallback semantics.
    let meta = parse_jsonl_session(source.path(), source.identity()).ok_or_else(forbidden)?;
    if meta.identity != *source.identity() { return Err(forbidden()); }
    Ok(())
}
fn active() -> io::Error {
    io::Error::new(io::ErrorKind::WouldBlock, "active session cannot be deleted")
}
fn forbidden() -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, "Hermes session source mismatch or unsafe path")
}
