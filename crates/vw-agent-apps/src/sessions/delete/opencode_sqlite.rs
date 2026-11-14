//! Transactional OpenCode SQLite deletion on the authorized execution host.
use std::{io, time::Duration};
use rusqlite::{Connection, OpenFlags, TransactionBehavior};
use crate::sessions::{cache::{source_fingerprint, TranscriptCache}, source::SourceRegistry};
use vw_api_types::external_session::{ExternalSessionIdentity, SessionProvider, SessionSource};

/// Requires delete authorization, confirmation and a caller-held application
/// lifecycle guard. Activity must be queried from trusted runtime state.
/// All known V1/V2 rows are deleted atomically; auxiliary failures roll back.
pub fn delete_session<F>(
    registry: &SourceRegistry,
    identity: &ExternalSessionIdentity,
    cache: &TranscriptCache,
    mut is_active: F,
) -> io::Result<()>
where F: FnMut(&ExternalSessionIdentity) -> io::Result<bool> {
    if identity.provider != SessionProvider::Opencode
        || !matches!(&identity.source, SessionSource::Sqlite { database, record_id }
            if database == "opencode.db" && record_id == &identity.session_id.0)
    {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "expected an OpenCode SQLite session"));
    }
    let source = registry.resolve(identity)?;
    if is_active(identity)? { return Err(active()); }
    let fingerprint = source_fingerprint(&source)?;
    if is_active(identity)? { return Err(active()); }
    let current = registry.resolve(identity)?;
    if current.path() != source.path() || current.root() != source.root()
        || source_fingerprint(&current)? != fingerprint
    {
        return Err(io::Error::new(io::ErrorKind::WouldBlock, "OpenCode database changed during deletion"));
    }
    // Validate optional SQLite sidecars before SQLite can open/write them.
    for suffix in ["-wal", "-shm", "-journal"] {
        let path = current.path().with_file_name(format!("opencode.db{suffix}"));
        match std::fs::symlink_metadata(path) {
            Ok(meta) if meta.file_type().is_symlink() || !meta.is_file() =>
                return Err(io::Error::new(io::ErrorKind::PermissionDenied, "unsafe OpenCode database sidecar")),
            Ok(_) => {},
            Err(error) if error.kind() == io::ErrorKind::NotFound => {},
            Err(error) => return Err(error),
        }
    }
    let mut conn = Connection::open_with_flags(current.path(), OpenFlags::SQLITE_OPEN_READ_WRITE)
        .map_err(storage)?;
    conn.busy_timeout(Duration::from_secs(2)).map_err(storage)?;
    conn.pragma_update(None, "foreign_keys", true).map_err(storage)?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate).map_err(storage)?;
    let v1 = table_exists(&tx, "session")?;
    let v2 = table_exists(&tx, "session_v2")?;
    let id = identity.session_id.0.as_str();
    let mut found = false;
    for (table, present) in [("session", v1), ("session_v2", v2)] {
        if !present { continue; }
        let exists: bool = tx.query_row(&format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE id=?1)"),
            [id], |r| r.get(0)).map_err(storage)?;
        found |= exists;
        // A parent session may own forked sessions through a self foreign key.
        // Never allow SQLite CASCADE/SET NULL to mutate another session.
        let has_parent: bool = tx.query_row(
            &format!("SELECT EXISTS(SELECT 1 FROM pragma_table_info('{table}') WHERE name='parent_id')"),
            [], |r| r.get(0)).map_err(storage)?;
        if exists && has_parent {
            let children: bool = tx.query_row(
                &format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE parent_id=?1 AND id<>?1)"),
                [id], |r| r.get(0)).map_err(storage)?;
            if children { return Err(io::Error::new(io::ErrorKind::WouldBlock, "OpenCode session has child sessions")); }
        }
    }
    if !found { return Err(io::Error::new(io::ErrorKind::NotFound, "OpenCode session not found")); }
    cache.remove(identity).map_err(|_| io::Error::other("session cache unavailable"))?;
    // Static allowlist only; SQL identifiers never come from the request.
    for table in ["session_pending", "session_inbox", "instruction_entry", "instruction_state",
        "session_message", "part", "message"] {
        if table_exists(&tx, table)? {
            tx.execute(&format!("DELETE FROM {table} WHERE session_id=?1"), [id]).map_err(storage)?;
        }
    }
    if table_exists(&tx, "event_sequence")? {
        tx.execute("DELETE FROM event_sequence WHERE aggregate_id=?1", [id]).map_err(storage)?;
    }
    if v2 { tx.execute("DELETE FROM session_v2 WHERE id=?1", [id]).map_err(storage)?; }
    if v1 { tx.execute("DELETE FROM session WHERE id=?1", [id]).map_err(storage)?; }
    tx.commit().map_err(storage)?;
    cache.remove(identity).map_err(|_| io::Error::other("session cache unavailable"))
}

fn table_exists(conn: &Connection, table: &str) -> io::Result<bool> {
    conn.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
        [table], |r| r.get(0)).map_err(storage)
}
fn storage(_: rusqlite::Error) -> io::Error {
    io::Error::other("OpenCode SQLite deletion failed; transaction was not committed")
}
fn active() -> io::Error {
    io::Error::new(io::ErrorKind::WouldBlock, "active session cannot be deleted")
}
