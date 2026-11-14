//! Incremental JSONL scanning over host-authorized, already-open log files.
//!
//! This module never opens a client-supplied path. The host must resolve the
//! application whitelist and scope before supplying a file and stable identity.
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};

use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use vw_api_types::id::ProjectScope;

use super::store::{
    StoredUsageRecord, UsageSourceIdentity, UsageStoreError, save_usage_record_in_transaction,
};

const TAIL_BYTES: u64 = 4096;

/// File identity is supplied by the host registry, never a display path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageCursorKey {
    pub target_host: String,
    pub app: String,
    pub instance_id: String,
    pub file_identity: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageCursor {
    pub byte_offset: u64,
    pub line_offset: u64,
    pub tail_fingerprint: Vec<u8>,
    /// Parser context at the committed complete-line boundary only.
    pub parser_state: Option<String>,
}

#[derive(Debug)]
pub enum UsageCursorError {
    Storage(UsageStoreError),
    Io(std::io::Error),
    InvalidCursor,
    ScanDisabled,
    InvalidRecord,
    SourceChangedDuringScan,
}

impl std::fmt::Display for UsageCursorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Do not expose file paths, log contents or parser state.
        f.write_str(match self {
            Self::Storage(_) => "Usage cursor storage unavailable",
            Self::Io(_) => "Usage source read failed",
            Self::ScanDisabled => "Usage source scanning is disabled",
            Self::InvalidCursor => "Invalid usage cursor",
            Self::InvalidRecord => "Invalid usage source record",
            Self::SourceChangedDuringScan => "Usage source changed during scan",
        })
    }
}
impl std::error::Error for UsageCursorError {}
impl From<std::io::Error> for UsageCursorError {
    fn from(value: std::io::Error) -> Self { Self::Io(value) }
}
impl From<UsageStoreError> for UsageCursorError {
    fn from(value: UsageStoreError) -> Self { Self::Storage(value) }
}
fn storage(_: rusqlite::Error) -> UsageCursorError {
    UsageCursorError::Storage(UsageStoreError::StorageUnavailable)
}

/// Invoke from the host schema migration, after initializing the ledger.
pub fn initialize_usage_cursors(connection: &Connection) -> Result<(), UsageCursorError> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS usage_scan_cursors (
            cursor_key TEXT PRIMARY KEY NOT NULL,
            payload_json TEXT NOT NULL
        );",
    ).map_err(storage)
}

fn key(scope: &ProjectScope, source: &UsageCursorKey) -> Result<String, UsageCursorError> {
    if [&source.target_host, &source.app, &source.instance_id, &source.file_identity]
        .into_iter().any(|s| s.trim().is_empty() || s.contains('\0')) {
        return Err(UsageCursorError::InvalidCursor);
    }
    serde_json::to_string(&(scope, source)).map_err(|_| UsageCursorError::InvalidCursor)
}

/// Missing cursor is distinct from an unreadable or corrupt cursor. Never fall
/// back to scanning from zero after a storage error.
pub fn load_usage_cursor(
    connection: &Connection,
    scope: &ProjectScope,
    source: &UsageCursorKey,
) -> Result<Option<UsageCursor>, UsageCursorError> {
    let payload: Option<String> = connection.query_row(
        "SELECT payload_json FROM usage_scan_cursors WHERE cursor_key=?1",
        [key(scope, source)?], |row| row.get(0),
    ).optional().map_err(storage)?;
    payload.map(|value| {
        let cursor: UsageCursor = serde_json::from_str(&value)
            .map_err(|_| UsageCursorError::InvalidCursor)?;
        if cursor.tail_fingerprint.len() != 32 || cursor.line_offset > cursor.byte_offset {
            return Err(UsageCursorError::InvalidCursor);
        }
        Ok(cursor)
    }).transpose()
}

fn fingerprint(file: &mut File, end: u64) -> Result<Vec<u8>, UsageCursorError> {
    let length = end.min(TAIL_BYTES);
    file.seek(SeekFrom::Start(end - length))?;
    let mut bytes = vec![0; length as usize];
    file.read_exact(&mut bytes)?;
    let mut digest = Sha256::new();
    digest.update(b"shiju-usage-cursor-v1");
    digest.update(&bytes);
    Ok(digest.finalize().to_vec())
}

#[derive(Debug)]
pub struct UsageScanOutcome {
    pub cursor: UsageCursor,
    pub records: usize,
    /// Truncated/rewritten history was pinned without replay, preserving archived
    /// accounting. The host must surface this as a source warning.
    pub history_changed: bool,
}

/// Parse complete lines and commit records, parser context and cursor atomically.
/// `parse` receives bytes including the newline and may emit no usage record.
/// Parser errors abort the entire transaction. It must not perform external writes.
///
/// A non-append mutation pins to the last complete line without importing history:
/// blindly replaying it could double count records already rolled into archives.
/// Concurrent scanners serialize on the caller's existing SQLite database.
pub fn scan_usage_file<F>(
    connection: &mut Connection,
    scope: &ProjectScope,
    source: &UsageCursorKey,
    file: &mut File,
    parse: F,
) -> Result<UsageScanOutcome, UsageCursorError>
where
    F: FnMut(&[u8], &mut Option<String>)
        -> Result<Vec<(UsageSourceIdentity, StoredUsageRecord)>, UsageCursorError>,
{
    scan_usage_file_with_finish(connection, scope, source, file, parse, |_| Ok(Vec::new()))
}

/// Variant for snapshot-based formats: select representative records after all
/// complete lines have been parsed, inside the same cursor/ledger transaction.
/// `finish` is not called when rewritten history is pinned without replay.
pub fn scan_usage_file_with_finish<F, G>(
    connection: &mut Connection,
    scope: &ProjectScope,
    source: &UsageCursorKey,
    file: &mut File,
    mut parse: F,
    finish: G,
) -> Result<UsageScanOutcome, UsageCursorError>
where
    F: FnMut(&[u8], &mut Option<String>)
        -> Result<Vec<(UsageSourceIdentity, StoredUsageRecord)>, UsageCursorError>,
    G: FnOnce(&mut Option<String>)
        -> Result<Vec<(UsageSourceIdentity, StoredUsageRecord)>, UsageCursorError>,
{
    let cursor_key = key(scope, source)?;
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage)?;
    // Preferences may restrict a host-authorized source, never grant access.
    let settings = super::source_settings::load_source_settings(
        &transaction, scope, &source.target_host, &source.app,
    ).map_err(|_| UsageStoreError::StorageUnavailable)?;
    if !settings.settings.scan_enabled {
        return Err(UsageCursorError::ScanDisabled);
    }
    let old = load_usage_cursor(&transaction, scope, source)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() { return Err(UsageCursorError::InvalidRecord); }
    let length = metadata.len();
    let history_changed = match &old {
        Some(cursor) if cursor.byte_offset > length => true,
        Some(cursor) => fingerprint(file, cursor.byte_offset)? != cursor.tail_fingerprint,
        None => false,
    };
    let mut cursor = if history_changed {
        UsageCursor { byte_offset: 0, line_offset: 0, tail_fingerprint: Vec::new(), parser_state: None }
    } else {
        old.clone().unwrap_or(UsageCursor {
            byte_offset: 0, line_offset: 0, tail_fingerprint: Vec::new(), parser_state: None,
        })
    };
    file.seek(SeekFrom::Start(cursor.byte_offset))?;
    let mut records = 0;
    // Bound this scan to the observed EOF; an actively appended partial line is
    // retried next time, with the parser context unchanged.
    {
        let mut reader = BufReader::new((&mut *file).take(length - cursor.byte_offset));
        let mut line = Vec::new();
        loop {
            line.clear();
            if reader.read_until(b'\n', &mut line)? == 0 || line.last() != Some(&b'\n') {
                break;
            }
            if !history_changed {
                for (identity, record) in parse(&line, &mut cursor.parser_state)? {
                    if identity.target_host != source.target_host || identity.app != source.app
                        || identity.instance_id != source.instance_id
                        || identity.file_identity != source.file_identity {
                        return Err(UsageCursorError::InvalidRecord);
                    }
                    save_usage_record_in_transaction(&transaction, scope, &identity, &record)?;
                    records += 1;
                }
            }
            cursor.byte_offset += line.len() as u64;
            cursor.line_offset += 1;
        }
    }
    if !history_changed {
        for (identity, record) in finish(&mut cursor.parser_state)? {
            if identity.target_host != source.target_host || identity.app != source.app
                || identity.instance_id != source.instance_id
                || identity.file_identity != source.file_identity {
                return Err(UsageCursorError::InvalidRecord);
            }
            save_usage_record_in_transaction(&transaction, scope, &identity, &record)?;
            records += 1;
        }
    }
    // Detect common concurrent truncation/rewrite races before publishing a cursor.
    if file.metadata()?.len() < length || file.metadata()?.modified()? != metadata.modified()? {
        return Err(UsageCursorError::SourceChangedDuringScan);
    }
    cursor.tail_fingerprint = fingerprint(file, cursor.byte_offset)?;
    if !history_changed {
        if let Some(old) = old {
            if fingerprint(file, old.byte_offset)? != old.tail_fingerprint {
                return Err(UsageCursorError::SourceChangedDuringScan);
            }
        }
    }
    let payload = serde_json::to_string(&cursor).map_err(|_| UsageCursorError::InvalidCursor)?;
    transaction.execute(
        "INSERT INTO usage_scan_cursors(cursor_key,payload_json) VALUES (?1,?2)
         ON CONFLICT(cursor_key) DO UPDATE SET payload_json=excluded.payload_json",
        params![cursor_key, payload],
    ).map_err(storage)?;
    transaction.commit().map_err(storage)?;
    Ok(UsageScanOutcome { cursor, records, history_changed })
}
