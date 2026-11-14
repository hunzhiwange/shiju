//! Pi JSONL deletion on the authorized execution host.
use std::{fs, io};
use crate::sessions::{cache::TranscriptCache, providers::pi::verify_pi_session_id, source::{SourceRegistry, ValidatedSource}};
use vw_api_types::external_session::{ExternalSessionIdentity, SessionProvider, SessionSource};

/// Requires gateway authorization and explicit confirmation. The caller holds
/// its application lifecycle guard throughout; activity is queried from trusted
/// host runtime state and errors fail closed. Native Pi deletes only the file:
/// fork/subagent trees and artifacts may belong to other sessions and remain.
pub fn delete_session<F>(
    registry: &SourceRegistry,
    identity: &ExternalSessionIdentity,
    cache: &TranscriptCache,
    mut is_active: F,
) -> io::Result<()>
where F: FnMut(&ExternalSessionIdentity) -> io::Result<bool> {
    if identity.provider != SessionProvider::Pi
        || !matches!(&identity.source, SessionSource::File { .. })
    {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "expected a Pi file session"));
    }
    let source = registry.resolve(identity)?;
    if is_active(identity)? { return Err(active()); }
    verify(&source)?;
    let current = registry.resolve(identity)?;
    if current.path() != source.path() || current.root() != source.root() {
        return Err(io::Error::new(io::ErrorKind::PermissionDenied, "Pi source changed"));
    }
    verify(&current)?;
    if is_active(identity)? { return Err(active()); }
    cache.remove(identity).map_err(|_| io::Error::other("session cache unavailable"))?;
    let current = registry.resolve(identity)?;
    verify(&current)?;
    fs::remove_file(current.path()).map_err(|error| io::Error::new(
        error.kind(), "Pi session deletion failed"))?;
    cache.remove(identity).map_err(|_| io::Error::other("session cache unavailable"))
}

fn verify(source: &ValidatedSource) -> io::Result<()> {
    verify_pi_session_id(source.path(), &source.identity().session_id.0)
        .map_err(|_| io::Error::new(io::ErrorKind::PermissionDenied, "invalid Pi session or native ID mismatch"))
}
fn active() -> io::Error {
    io::Error::new(io::ErrorKind::WouldBlock, "active session cannot be deleted")
}
