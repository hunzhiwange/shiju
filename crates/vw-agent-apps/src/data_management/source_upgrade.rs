//! Frozen CC schema 18–24 bridge, used only on disposable staging connections.
use rusqlite::Connection;

use super::source::{MAX_SCHEMA_VERSION, SourceError};

/// Earlier versions require data-rewriting migrations and are explicitly refused.
/// Never pass a source or target database connection here.
pub(super) fn upgrade_staged_copy(connection: &Connection) -> Result<(), SourceError> {
    let version: u32 = connection.pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(|_| SourceError::InvalidData)?;
    if version > MAX_SCHEMA_VERSION {
        return Err(SourceError::FutureVersion { found: version, supported: MAX_SCHEMA_VERSION });
    }
    if version == MAX_SCHEMA_VERSION { return Ok(()); }
    if version < 18 { return Err(SourceError::UnsupportedVersion); }
    // Do not turn a truncated export into an apparently complete database.
    for (table, columns) in [
        ("providers", "id,app_type,name,settings_config"),
        ("provider_endpoints", "provider_id,app_type,url"),
        ("mcp_servers", "id,name,server_config,enabled_claude"),
        ("prompts", "id,app_type,name,content"),
        ("skills", "id,name,directory,enabled_claude"),
        ("skill_repos", "owner,name,branch,enabled"),
        ("settings", "key,value"),
        ("session_log_sync", "file_path,last_modified,last_line_offset,last_synced_at,last_byte_offset,last_tail_fingerprint"),
    ] {
        let exists: bool = connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_schema WHERE type='table' AND name=?1)",
            [table], |row| row.get(0),
        ).map_err(|_| SourceError::InvalidData)?;
        if !exists { return Err(SourceError::InvalidData); }
        connection.prepare(&format!("SELECT {columns} FROM {table} LIMIT 0"))
            .map_err(|_| SourceError::InvalidData)?;
    }
    let transaction = connection.unchecked_transaction().map_err(|_| SourceError::InvalidData)?;
    // Same steps as frozen database/schema.rs::apply_schema_migrations_on_conn.
    for step in version..MAX_SCHEMA_VERSION {
        match step {
            18 => add_flags(&transaction, "enabled_mcode", true)?,
            19 => add_flags(&transaction, "enabled_pi", false)?,
            20 => add_flags(&transaction, "enabled_agy", true)?,
            21 => add_flags(&transaction, "enabled_kimi", true)?,
            22 => add_flags(&transaction, "enabled_omp", false)?,
            23 => {
                add_flags(&transaction, "enabled_dsh", true)?;
                add_column(&transaction, "session_log_sync", "parser_state", "TEXT")?;
            }
            _ => return Err(SourceError::UnsupportedVersion),
        }
        transaction.pragma_update(None, "user_version", step + 1)
            .map_err(|_| SourceError::InvalidData)?;
    }
    transaction.commit().map_err(|_| SourceError::InvalidData)
}

fn add_flags(connection: &Connection, column: &str, skills: bool) -> Result<(), SourceError> {
    add_column(connection, "mcp_servers", column, "BOOLEAN NOT NULL DEFAULT 0")?;
    if skills { add_column(connection, "skills", column, "BOOLEAN NOT NULL DEFAULT 0")?; }
    Ok(())
}

fn add_column(connection: &Connection, table: &str, column: &str, definition: &str) -> Result<(), SourceError> {
    let mut statement = connection.prepare(&format!("PRAGMA table_info({table})"))
        .map_err(|_| SourceError::InvalidData)?;
    let names = statement.query_map([], |row| row.get::<_, String>(1))
        .map_err(|_| SourceError::InvalidData)?;
    for name in names {
        if name.map_err(|_| SourceError::InvalidData)? == column { return Ok(()); }
    }
    connection.execute_batch(&format!("ALTER TABLE {table} ADD COLUMN {column} {definition}"))
        .map_err(|_| SourceError::InvalidData)
}
