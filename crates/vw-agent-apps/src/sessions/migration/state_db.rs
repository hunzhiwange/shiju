//! Executor-only Codex state bucket migration; never accepts a client path.
use crate::providers::{
    live::write_guard::{WriteError, lock_directory},
    paths::open_beneath,
};
use rusqlite::{Connection, OpenFlags, TransactionBehavior, params_from_iter};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::{self, Read},
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    time::Duration,
};
use vw_api_types::agent_apps::AppId;

const ID_CHUNK: usize = 500;
const MAX_BACKUP_BYTES: u64 = 512 * 1024 * 1024;

/// Original physical database and WAL bytes. A missing WAL must be recorded as
/// absent, so a restore cannot accidentally replay a later WAL over this database.
/// SHM is a transient WAL index and must be discarded when restoring offline.
pub struct StateDbBackup {
    pub database: Vec<u8>,
    pub wal: Option<Vec<u8>>,
}

/// `db_path` must be selected using `sessions::codex_state::codex_state_db_paths`
/// and authorized on the target host for this maintenance operation. Thread IDs
/// must come from the freshly resolved scoped history, provider IDs from trusted
/// definitions. No request paths, unscoped thread IDs or ambient HOME are used.
///
/// The caller must retain lifecycle exclusion covering native Codex writers and
/// other executor processes. This primitive takes the non-reentrant Codex app
/// write guard itself; callers and backup callbacks must not already hold it.
/// `backup` must durably retain both physical images under the operation's scoped
/// ledger before returning success. It must not mutate the source database.
/// No completion marker or provider configuration is written here. A missing
/// database or no matching threads returns zero without invoking the callback.
/// All chunks commit together; any error drops and rolls back the transaction.
pub fn migrate(
    db_path: &Path,
    thread_ids: &BTreeSet<String>,
    source_ids: &BTreeSet<String>,
    target_id: &str,
    mut backup: impl FnMut(&Path, &StateDbBackup) -> io::Result<()>,
) -> io::Result<usize> {
    if db_path.file_name().and_then(|s| s.to_str())
        != Some(crate::sessions::codex_state::CODEX_STATE_DB_FILENAME)
    {
        return Err(invalid());
    }
    for id in source_ids.iter().map(String::as_str).chain(std::iter::once(target_id)) {
        crate::providers::validate_identity(id).map_err(|_| invalid())?;
    }
    if thread_ids
        .iter()
        .any(|id| id.is_empty() || id.len() > 4096 || id.chars().any(char::is_control))
    {
        return Err(invalid());
    }
    if thread_ids.is_empty() || source_ids.is_empty() {
        return Ok(0);
    }
    let parent = db_path.parent().ok_or_else(invalid)?;
    let directory = open(parent, true)?;
    let _guard = lock_directory(AppId::Codex, directory.try_clone()?).map_err(write_error)?;
    let file = match open(db_path, false) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(0),
        Err(error) => return Err(error),
    };
    let physical = physical_id(&file)?;
    let wal_path = sidecar(db_path, "-wal");
    // SQLite can open these itself; reject links before giving it a pathname.
    for suffix in ["-wal", "-shm", "-journal"] {
        let path = sidecar(db_path, suffix);
        match open(&path, false) {
            Ok(file) if !file.metadata()?.is_file() => return Err(invalid()),
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    let mut connection = Connection::open_with_flags(
        db_path,
        OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_NO_MUTEX
            | OpenFlags::SQLITE_OPEN_NOFOLLOW,
    )
    .map_err(database_error)?;
    connection.busy_timeout(Duration::from_secs(5)).map_err(database_error)?;
    let tx = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    verify_location(db_path, &directory, physical)?;
    // Capture after SQLite has acquired the write reservation and performed any
    // native recovery, but before our first update. Opening WAL mode may create
    // an empty WAL; its presence is part of this physical backup.
    let original = StateDbBackup { database: read_bounded(file)?, wal: read_optional(&wal_path)? };
    // Explicitly require the native schema, instead of treating an unsupported
    // database or broken schema as a successful empty migration.
    let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let ids: Vec<_> = thread_ids.iter().collect();
    for chunk in ids.chunks(ID_CHUNK) {
        let sql = format!(
            "SELECT id, model_provider FROM threads WHERE id IN ({})",
            placeholders(chunk.len())
        );
        let mut query = tx.prepare(&sql).map_err(database_error)?;
        let rows = query
            .query_map(params_from_iter(chunk.iter().copied()), |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
            })
            .map_err(database_error)?;
        for row in rows {
            let (id, provider) = row.map_err(database_error)?;
            if let Some(provider) = provider.filter(|p| p != target_id && source_ids.contains(p)) {
                groups.entry(provider).or_default().push(id);
            }
        }
    }
    if groups.is_empty() {
        return Ok(0);
    }
    backup(db_path, &original)?;
    verify(db_path, &directory, physical, &original)?;
    let mut changed = 0;
    for (provider, ids) in groups {
        for chunk in ids.chunks(ID_CHUNK) {
            let sql = format!(
                "UPDATE threads SET model_provider = ? WHERE model_provider = ? AND id IN ({})",
                placeholders(chunk.len())
            );
            let values = std::iter::once(target_id)
                .chain(std::iter::once(provider.as_str()))
                .chain(chunk.iter().map(String::as_str));
            let count = tx.execute(&sql, params_from_iter(values)).map_err(database_error)?;
            if count != chunk.len() {
                return Err(conflict());
            }
            changed += count;
        }
    }
    // Verify the pathname still refers to the retained file, without comparing
    // contents now that SQLite owns the transaction's changed pages/WAL.
    verify_location(db_path, &directory, physical)?;
    tx.commit().map_err(database_error)?;
    Ok(changed)
}

fn placeholders(count: usize) -> String {
    std::iter::repeat_n("?", count).collect::<Vec<_>>().join(",")
}
fn sidecar(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(suffix);
    PathBuf::from(name)
}
fn physical_id(file: &File) -> io::Result<(u64, u64)> {
    let meta = file.metadata()?;
    if !meta.is_file() {
        return Err(invalid());
    }
    Ok((meta.dev(), meta.ino()))
}
fn read_bounded(file: File) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    file.take(MAX_BACKUP_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_BACKUP_BYTES {
        return Err(invalid());
    }
    Ok(bytes)
}
fn read_optional(path: &Path) -> io::Result<Option<Vec<u8>>> {
    match open(path, false) {
        Ok(file) => read_bounded(file).map(Some),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}
fn verify_location(path: &Path, directory: &File, physical: (u64, u64)) -> io::Result<File> {
    let parent = open(path.parent().ok_or_else(invalid)?, true)?;
    let before = directory.metadata()?;
    let now = parent.metadata()?;
    if (before.dev(), before.ino()) != (now.dev(), now.ino()) {
        return Err(conflict());
    }
    let current = open(path, false)?;
    if physical_id(&current)? != physical {
        return Err(conflict());
    }
    Ok(current)
}
fn verify(
    path: &Path,
    directory: &File,
    physical: (u64, u64),
    original: &StateDbBackup,
) -> io::Result<()> {
    let current = verify_location(path, directory, physical)?;
    if read_bounded(current)? != original.database
        || read_optional(&sidecar(path, "-wal"))? != original.wal
    {
        return Err(conflict());
    }
    Ok(())
}
fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, "invalid or oversized Codex state migration")
}
fn conflict() -> io::Error {
    io::Error::new(io::ErrorKind::WouldBlock, "Codex state database changed during migration")
}
fn database_error(error: rusqlite::Error) -> io::Error {
    match error {
        rusqlite::Error::SqliteFailure(code, _)
            if matches!(
                code.code,
                rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked
            ) =>
        {
            conflict()
        }
        _ => io::Error::other("Codex state database migration failed; transaction not committed"),
    }
}
fn write_error(error: WriteError) -> io::Error {
    match error {
        WriteError::Conflict | WriteError::Recovered => conflict(),
        WriteError::InvalidResource => invalid(),
        WriteError::Io => io::Error::other("Codex state migration write guard unavailable"),
    }
}

fn open(path: &Path, directory: bool) -> io::Result<File> {
    use vw_api_types::app_providers::paths::ConfigPathState;
    open_beneath(Path::new("/"), path, directory).map_err(|state| match state {
        ConfigPathState::Absent => {
            io::Error::new(io::ErrorKind::NotFound, "Codex state source absent")
        }
        _ => io::Error::new(io::ErrorKind::PermissionDenied, "Codex state source inaccessible"),
    })
}
