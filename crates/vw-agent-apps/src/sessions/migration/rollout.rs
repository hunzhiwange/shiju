//! Narrow Codex bucket rewrite. Authorization, lifecycle exclusion and durable
//! backup ownership belong to the maintenance operation invoking this primitive.
use crate::{
    providers::{
        live::write_guard::{WriteError, lock_directory, prepare_private_child},
        paths::open_beneath,
    },
    sessions::{
        rollout,
        source::{SourceRegistry, ValidatedSource},
    },
};
use serde_json::Value;
use std::{
    collections::BTreeSet,
    fs::{File, Metadata},
    io::{self, Read},
    os::unix::fs::MetadataExt,
    path::Path,
};
use vw_api_types::{
    agent_apps::AppId,
    external_session::{ExternalSessionIdentity, SessionProvider, SessionSource},
};

// Fail explicitly rather than allocate unbounded decoded history on a worker.
const MAX_BYTES: u64 = 256 * 1024 * 1024;

/// Rewrite one freshly registered Codex rollout. The caller authorizes maintenance
/// and retains its application lifecycle exclusion (including native writers).
/// `source_ids` must come from trusted provider definitions, never request values.
/// `backup` must durably retain the *original physical bytes* under its scoped
/// ledger before returning success; it must not start another app write operation.
/// This function acquires the Codex write guard itself; callers must not already
/// hold that non-reentrant guard (a lifecycle guard is separate).
/// No backup path, global completion marker or provider configuration is written
/// here. A false return is an idempotent no-op and creates no backup.
pub fn migrate(
    registry: &SourceRegistry,
    identity: &ExternalSessionIdentity,
    source_ids: &BTreeSet<String>,
    target_id: &str,
    mut backup: impl FnMut(&ValidatedSource, &[u8]) -> io::Result<()>,
) -> io::Result<bool> {
    if identity.provider != SessionProvider::Codex
        || !matches!(&identity.source, SessionSource::File { .. })
        || source_ids.is_empty()
    {
        return Err(invalid());
    }
    for id in source_ids.iter().map(String::as_str).chain(std::iter::once(target_id)) {
        crate::providers::validate_identity(id).map_err(|_| invalid())?;
    }
    let source = registry.resolve(identity)?;
    if !rollout::is_rollout_file(source.path()) {
        return Err(invalid());
    }
    let parent = source.path().parent().ok_or_else(invalid)?;
    // Walk every ancestor with no-follow opens, retaining the physical parent
    // through staging/publication instead of trusting a checked path later.
    let directory = open_beneath(Path::new("/"), parent, true).map_err(|_| forbidden())?;
    let _guard = lock_directory(AppId::Codex, directory.try_clone()?).map_err(write_error)?;
    let file = open_beneath(Path::new("/"), source.path(), false).map_err(|_| forbidden())?;
    let before = file.metadata()?;
    let modified = before.modified()?;
    let original = read_bounded(file)?;
    let decoded = if rollout::is_compressed(source.path()) {
        read_bounded(zstd::stream::read::Decoder::new(original.as_slice())?)?
    } else {
        original.clone()
    };
    let Some(rewritten) = rewrite(&decoded, &identity.session_id.0, source_ids, target_id)? else {
        verify(registry, identity, &source, &directory, &before, &original)?;
        return Ok(false);
    };
    let encoded = if rollout::is_compressed(source.path()) {
        zstd::stream::encode_all(rewritten.as_slice(), 3)?
    } else {
        rewritten
    };
    verify(registry, identity, &source, &directory, &before, &original)?;
    backup(&source, &original)?;
    let staged =
        prepare_private_child(&directory, source.path().file_name().ok_or_else(invalid)?, &encoded)
            .map_err(write_error)?;
    staged.set_modified(modified).map_err(write_error)?;
    verify(registry, identity, &source, &directory, &before, &original)?;
    staged.commit().map_err(write_error)?;
    directory.sync_all()?;
    Ok(true)
}

fn rewrite(
    bytes: &[u8],
    session_id: &str,
    sources: &BTreeSet<String>,
    target: &str,
) -> io::Result<Option<Vec<u8>>> {
    let mut output = Vec::with_capacity(bytes.len());
    let mut found = false;
    let mut changed = false;
    for segment in bytes.split_inclusive(|b| *b == b'\n') {
        let line = segment.strip_suffix(b"\n").unwrap_or(segment);
        let Ok(mut value) = serde_json::from_slice::<Value>(line) else {
            output.extend_from_slice(segment);
            continue;
        };
        if value["type"].as_str() != Some("session_meta") {
            output.extend_from_slice(segment);
            continue;
        }
        // Ambiguous or mismatched native IDs cannot authorize a rewrite.
        if found || value["payload"]["id"].as_str() != Some(session_id) {
            return Err(forbidden());
        }
        found = true;
        let provider = value["payload"]["model_provider"].as_str();
        if provider.is_some_and(|id| id != target && sources.contains(id)) {
            value["payload"]["model_provider"] = Value::String(target.to_owned());
            output.extend(serde_json::to_vec(&value).map_err(|_| invalid())?);
            if line.ends_with(b"\r") {
                output.push(b'\r');
            }
            if segment.ends_with(b"\n") {
                output.push(b'\n');
            }
            changed = true;
        } else {
            output.extend_from_slice(segment);
        }
    }
    if !found {
        return Err(forbidden());
    }
    Ok(changed.then_some(output))
}

fn read_bounded(reader: impl Read) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader.take(MAX_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err(invalid());
    }
    Ok(bytes)
}
fn revision(meta: &Metadata) -> (u64, u64, u64, i64, i64, i64, i64) {
    (
        meta.dev(),
        meta.ino(),
        meta.len(),
        meta.mtime(),
        meta.mtime_nsec(),
        meta.ctime(),
        meta.ctime_nsec(),
    )
}
fn verify(
    registry: &SourceRegistry,
    identity: &ExternalSessionIdentity,
    source: &ValidatedSource,
    directory: &File,
    before: &Metadata,
    bytes: &[u8],
) -> io::Result<()> {
    let current = registry.resolve(identity)?;
    if current.path() != source.path() || current.root() != source.root() {
        return Err(conflict());
    }
    let parent = open_beneath(Path::new("/"), current.path().parent().ok_or_else(invalid)?, true)
        .map_err(|_| conflict())?;
    let a = directory.metadata()?;
    let b = parent.metadata()?;
    if (a.dev(), a.ino()) != (b.dev(), b.ino()) {
        return Err(conflict());
    }
    let file = open_beneath(Path::new("/"), current.path(), false).map_err(|_| conflict())?;
    if revision(&file.metadata()?) != revision(before)
        || read_bounded(&file)? != bytes
        || revision(&file.metadata()?) != revision(before)
    {
        return Err(conflict());
    }
    Ok(())
}
fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, "invalid or oversized Codex rollout migration")
}
fn forbidden() -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, "Codex rollout identity mismatch")
}
fn conflict() -> io::Error {
    io::Error::new(io::ErrorKind::WouldBlock, "Codex rollout changed during migration")
}
fn write_error(error: WriteError) -> io::Error {
    match error {
        WriteError::Conflict | WriteError::Recovered => conflict(),
        WriteError::InvalidResource => invalid(),
        WriteError::Io => io::Error::other("Codex rollout publication failed"),
    }
}
