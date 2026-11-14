//! Read-only staging of CC copies. Raw records stay inside the business layer.
use std::{collections::BTreeMap, fmt, path::Path};

use rusqlite::{Connection, OpenFlags, types::ValueRef};
use serde_json::{Map, Value};
use vw_api_types::id::ProjectScope;

use super::{source_files, source_json};

pub const MAX_SCHEMA_VERSION: u32 = 24;
pub const MAX_JSON_VERSION: u32 = 2;
const MAX_RECORDS: usize = 200_000;
const MAX_STAGED_BYTES: usize = 256 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceError {
    NotFound,
    PermissionDenied,
    OutsideAuthorizedRoot,
    InvalidScope,
    UnsafeFile,
    Changed,
    TooLarge,
    UnknownFormat,
    InvalidData,
    FutureVersion { found: u32, supported: u32 },
    UnsupportedVersion,
    Io(std::io::ErrorKind),
}

impl fmt::Display for SourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // No source paths, SQL errors, record names or payloads in errors.
        write!(f, "CC source: {self:?}")
    }
}
impl std::error::Error for SourceError {}
impl From<std::io::Error> for SourceError {
    fn from(error: std::io::Error) -> Self {
        match error.kind() {
            std::io::ErrorKind::NotFound => Self::NotFound,
            std::io::ErrorKind::PermissionDenied => Self::PermissionDenied,
            kind => Self::Io(kind),
        }
    }
}

/// Supplied by authenticated host policy, never inferred from the source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceScope {
    project: ProjectScope,
    host_id: String,
    principal_id: String,
}
impl SourceScope {
    pub fn new(project: ProjectScope, host_id: String, principal_id: String) -> Result<Self, SourceError> {
        if host_id.trim().is_empty() || principal_id.trim().is_empty() {
            return Err(SourceError::InvalidScope);
        }
        Ok(Self { project, host_id, principal_id })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SourceCategory {
    Providers,
    Mcp,
    Skills,
    Rules,
    Profiles,
    Preferences,
    Usage,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceFormat { Sqlite, Json, Sql }

/// Safe summary only. No raw values, filesystem paths or credentials.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSummary {
    pub format: SourceFormat,
    pub version: u32,
    pub main_sha256: [u8; 32],
    pub wal_sha256: Option<[u8; 32]>,
    pub counts: BTreeMap<SourceCategory, usize>,
}

/// Deliberately neither Debug nor Serialize: fields can contain credentials.
pub struct SourceRecord {
    pub category: SourceCategory,
    /// A source table name or JSON pointer, not a destination namespace.
    pub origin: String,
    pub fields: Value,
}

/// Owned, immutable staging. It does not keep a connection to the source.
pub struct SourceSnapshot {
    pub(super) scope: SourceScope,
    pub(super) summary: SourceSummary,
    pub(super) records: Vec<SourceRecord>,
}
impl SourceSnapshot {
    pub fn summary(&self, scope: &SourceScope) -> Result<&SourceSummary, SourceError> {
        self.authorize(scope)?;
        Ok(&self.summary)
    }
    /// Domain preview must still validate/convert these source fields. Reading
    /// them does not grant activation, installation or target-write authority.
    pub fn records(&self, scope: &SourceScope) -> Result<&[SourceRecord], SourceError> {
        self.authorize(scope)?;
        Ok(&self.records)
    }
    fn authorize(&self, scope: &SourceScope) -> Result<(), SourceError> {
        if &self.scope != scope { return Err(SourceError::PermissionDenied); }
        Ok(())
    }
}

/// Read only a user-selected copy under a server-authorized root. The caller
/// must authorize the sensitive read before calling; this is not an HTTP API.
/// No HOME discovery, source writes, checkpoints or target writes occur.
/// Supported old SQLite schemas are upgraded only in the disposable copy.
pub fn read_selected_source(
    scope: &SourceScope,
    authorized_root: &Path,
    selected: &Path,
) -> Result<SourceSnapshot, SourceError> {
    let path = source_files::selected_path(authorized_root, selected)?;
    let main = source_files::capture(&path)?;
    let mut records = Vec::new();
    let (format, version, wal_hash) = if main.bytes.starts_with(b"SQLite format 3\0") {
        let wal_path = source_files::wal_path(&path);
        let wal = source_files::capture_optional(&wal_path)?;
        // Check the whole set before SQLite can see it. The original is never
        // opened by SQLite: even a read-only connection may write to its SHM.
        main.verify(&path)?;
        source_files::verify_optional(&wal_path, wal.as_ref())?;
        let version = read_sqlite(&main.bytes, wal.as_ref().map(|v| v.bytes.as_slice()), &mut records)?;
        (SourceFormat::Sqlite, version, wal.map(|v| v.digest))
    } else {
        main.verify(&path)?;
        let version = source_json::read(&main.bytes, &mut records)?;
        (SourceFormat::Json, version, None)
    };
    let mut counts = BTreeMap::new();
    for record in &records { *counts.entry(record.category).or_insert(0) += 1; }
    Ok(SourceSnapshot {
        scope: scope.clone(),
        summary: SourceSummary { format, version, main_sha256: main.digest, wal_sha256: wal_hash, counts },
        records,
    })
}

pub(super) fn push_record(records: &mut Vec<SourceRecord>, record: SourceRecord) -> Result<(), SourceError> {
    if records.len() >= MAX_RECORDS { return Err(SourceError::TooLarge); }
    records.push(record);
    Ok(())
}

// Literal allowlist. Never accept tables/queries supplied by the caller. Related
// source records remain separate; no join drops fields or changes source IDs.
const TABLES: &[(&str, SourceCategory)] = &[
    ("providers", SourceCategory::Providers),
    ("provider_endpoints", SourceCategory::Providers),
    ("mcp_servers", SourceCategory::Mcp),
    ("skills", SourceCategory::Skills),
    ("skill_repos", SourceCategory::Skills),
    ("prompts", SourceCategory::Rules),
    ("profiles", SourceCategory::Profiles),
    ("settings", SourceCategory::Preferences),
    ("proxy_request_logs", SourceCategory::Usage),
    ("usage_daily_rollups", SourceCategory::Usage),
    ("session_log_sync", SourceCategory::Usage),
    ("session_usage_dedup", SourceCategory::Usage),
    ("model_pricing", SourceCategory::Usage),
];

fn read_sqlite(main: &[u8], wal: Option<&[u8]>, records: &mut Vec<SourceRecord>) -> Result<u32, SourceError> {
    let directory = tempfile::tempdir()?; // private 0700, dropped on every exit
    let path = directory.path().join("source.db");
    source_files::write_private(&path, main)?;
    if let Some(wal) = wal { source_files::write_private(&source_files::wal_path(&path), wal)?; }
    let connection = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_WRITE)
        .map_err(|_| SourceError::InvalidData)?;
    connection.execute_batch("PRAGMA trusted_schema=OFF;")
        .map_err(|_| SourceError::InvalidData)?;
    super::source_upgrade::upgrade_staged_copy(&connection)?;
    connection.execute_batch("PRAGMA query_only=ON; BEGIN DEFERRED;")
        .map_err(|_| SourceError::InvalidData)?;
    read_connection(&connection, records)
}

pub(super) fn read_connection(connection: &Connection, records: &mut Vec<SourceRecord>) -> Result<u32, SourceError> {
    let version: u32 = connection.pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(|_| SourceError::InvalidData)?;
    if version > MAX_SCHEMA_VERSION {
        return Err(SourceError::FutureVersion { found: version, supported: MAX_SCHEMA_VERSION });
    }
    // user_version alone does not identify a CC database. Older schemas are
    // accepted here only after the caller has applied the staging bridge.
    for (table, columns) in [
        ("providers", &["id", "app_type", "name", "settings_config"][..]),
        ("settings", &["key", "value"][..]),
    ] {
        if !is_table(&connection, table)? { return Err(SourceError::UnknownFormat); }
        let sql = format!("SELECT {} FROM {table} LIMIT 0", columns.join(","));
        connection.prepare(&sql).map_err(|_| SourceError::UnknownFormat)?;
    }
    let mut budget = MAX_STAGED_BYTES;
    for &(table, category) in TABLES {
        if !is_table(&connection, table)? { continue; }
        let mut statement = connection.prepare(&format!("SELECT * FROM {table}"))
            .map_err(|_| SourceError::InvalidData)?;
        let columns: Vec<String> = statement.column_names().iter().map(|s| (*s).to_owned()).collect();
        let mut rows = statement.query([]).map_err(|_| SourceError::InvalidData)?;
        while let Some(row) = rows.next().map_err(|_| SourceError::InvalidData)? {
            let mut fields = Map::new();
            for (index, column) in columns.iter().enumerate() {
                let raw = row.get_ref(index).map_err(|_| SourceError::InvalidData)?;
                let size = column.len() + match raw { ValueRef::Text(v) | ValueRef::Blob(v) => v.len(), _ => 16 };
                budget = budget.checked_sub(size).ok_or(SourceError::TooLarge)?;
                let value = match raw {
                    ValueRef::Null => Value::Null,
                    ValueRef::Integer(v) => Value::from(v),
                    ValueRef::Real(v) => serde_json::Number::from_f64(v).map(Value::Number).ok_or(SourceError::InvalidData)?,
                    ValueRef::Text(v) => Value::String(std::str::from_utf8(v).map_err(|_| SourceError::InvalidData)?.to_owned()),
                    ValueRef::Blob(_) => return Err(SourceError::InvalidData),
                };
                fields.insert(column.clone(), value);
            }
            push_record(records, SourceRecord { category, origin: table.into(), fields: Value::Object(fields) })?;
        }
    }
    // Connection drops before the private directory; no source checkpoint.
    Ok(version)
}

fn is_table(connection: &Connection, table: &str) -> Result<bool, SourceError> {
    connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_schema WHERE name=?1 AND type='table')",
        [table], |row| row.get(0),
    ).map_err(|_| SourceError::InvalidData)
}
