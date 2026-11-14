//! Transactional Hermes state.db deletion on the authorized execution host.
use std::{io, time::Duration};
use rusqlite::{Connection, OpenFlags, TransactionBehavior};
use crate::sessions::{cache::{source_fingerprint, TranscriptCache}, source::SourceRegistry};
use vw_api_types::external_session::{ExternalSessionIdentity, SessionProvider, SessionSource};

/// Requires delete authorization, confirmation and a caller-held application
/// lifecycle guard. Activity is queried from trusted host runtime state.
/// Message and session deletion either commit together or roll back together.
pub fn delete_session<F>(
    registry: &SourceRegistry,
    identity: &ExternalSessionIdentity,
    cache: &TranscriptCache,
    mut is_active: F,
) -> io::Result<()>
where F: FnMut(&ExternalSessionIdentity) -> io::Result<bool> {
    if identity.provider != SessionProvider::Hermes
        || !matches!(&identity.source, SessionSource::Sqlite { database, record_id }
            if database == "state.db" && record_id == &identity.session_id.0)
    {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "expected a Hermes SQLite session"));
    }
    let source = registry.resolve(identity)?;
    if is_active(identity)? { return Err(active()); }
    let fingerprint = source_fingerprint(&source)?;
    if is_active(identity)? { return Err(active()); }
    // The activity callback can reenter host state; repeat validation afterwards.
    let current = registry.resolve(identity)?;
    if current.path() != source.path() || current.root() != source.root()
        || source_fingerprint(&current)? != fingerprint
    {
        return Err(io::Error::new(io::ErrorKind::WouldBlock, "Hermes database changed during deletion"));
    }
    for suffix in ["-wal", "-shm", "-journal"] {
        let path = current.path().with_file_name(format!("state.db{suffix}"));
        match std::fs::symlink_metadata(path) {
            Ok(meta) if meta.file_type().is_symlink() || !meta.is_file() =>
                return Err(io::Error::new(io::ErrorKind::PermissionDenied, "unsafe Hermes database sidecar")),
            Ok(_) => {},
            Err(error) if error.kind() == io::ErrorKind::NotFound => {},
            Err(error) => return Err(error),
        }
    }
    // Never create a missing database or silently disable foreign key enforcement.
    let mut conn = Connection::open_with_flags(current.path(), OpenFlags::SQLITE_OPEN_READ_WRITE)
        .map_err(storage)?;
    conn.busy_timeout(Duration::from_secs(2)).map_err(storage)?;
    conn.pragma_update(None, "foreign_keys", true).map_err(storage)?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate).map_err(storage)?;
    let id = identity.session_id.0.as_str();
    let exists: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM sessions WHERE id=?1)",
        [id], |row| row.get(0)).map_err(storage)?;
    if !exists { return Err(io::Error::new(io::ErrorKind::NotFound, "Hermes session not found")); }
    // Self-referencing foreign keys can CASCADE or SET NULL in another session.
    // Refuse such deletion, including when the dependent session is inactive.
    let mut stmt = tx.prepare("SELECT \"from\", \"to\" FROM pragma_foreign_key_list('sessions') WHERE \"table\"='sessions'")
        .map_err(storage)?;
    let references = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)))
        .map_err(storage)?.collect::<Result<Vec<_>, _>>().map_err(storage)?;
    drop(stmt);
    for (column, target) in references {
        if target.as_deref().is_some_and(|target| target != "id") {
            return Err(io::Error::new(io::ErrorKind::Unsupported, "unsupported Hermes session relationship"));
        }
        // Identifiers are schema-owned; quote embedded quotes rather than treating
        // a database column name as executable SQL.
        let column = column.replace('"', "\"\"");
        let dependent: bool = tx.query_row(
            &format!("SELECT EXISTS(SELECT 1 FROM sessions WHERE \"{column}\"=?1 AND id<>?1)"),
            [id], |row| row.get(0)).map_err(storage)?;
        if dependent {
            return Err(io::Error::new(io::ErrorKind::WouldBlock, "Hermes session has child sessions"));
        }
    }
    cache.remove(identity).map_err(|_| io::Error::other("session cache unavailable"))?;
    // Missing or incompatible message schemas fail explicitly. No ignored child
    // errors, recursive filesystem removal, or fallback to JSONL deletion.
    tx.execute("DELETE FROM messages WHERE session_id=?1", [id]).map_err(storage)?;
    let deleted = tx.execute("DELETE FROM sessions WHERE id=?1", [id]).map_err(storage)?;
    if deleted != 1 { return Err(io::Error::other("Hermes session deletion did not affect exactly one session")); }
    tx.commit().map_err(storage)?;
    cache.remove(identity).map_err(|_| io::Error::other("session cache unavailable"))
}

fn storage(_: rusqlite::Error) -> io::Error {
    io::Error::other("Hermes SQLite deletion failed; transaction was not committed")
}
fn active() -> io::Error {
    io::Error::new(io::ErrorKind::WouldBlock, "active session cannot be deleted")
}
