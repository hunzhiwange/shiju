//! Untrusted CC SQL executes only in a bounded, disposable memory database.
use std::collections::BTreeMap;
use rusqlite::{Connection, hooks::{AuthAction, AuthContext, Authorization}, limits::Limit};
use sha2::{Digest, Sha256};
use super::source::{self, SourceError, SourceFormat, SourceScope, SourceSnapshot, SourceSummary};

const HEADER: &str = "-- CC Switch SQLite 导出";
const MAX_SQL_BYTES: usize = 64 * 1024 * 1024;

/// Caller authorizes the uploaded bytes and binds the authenticated scope.
/// No target connection or path is accepted, and migrations affect only the disposable copy.
/// The returned records still require domain validation before any target write.
pub fn stage_sql(scope: &SourceScope, sql: &str) -> Result<SourceSnapshot, SourceError> {
    if sql.len() > MAX_SQL_BYTES { return Err(SourceError::TooLarge); }
    let text = sql.trim_start_matches('\u{feff}').trim_start();
    if text.lines().next().map(str::trim_end) != Some(HEADER) {
        return Err(SourceError::UnknownFormat);
    }
    let connection = Connection::open_in_memory().map_err(|_| SourceError::InvalidData)?;
    connection.execute_batch(
        "PRAGMA trusted_schema=OFF; PRAGMA temp_store=MEMORY; PRAGMA page_size=4096; PRAGMA max_page_count=65536;"
    ).map_err(|_| SourceError::InvalidData)?;
    connection.set_limit(Limit::SQLITE_LIMIT_LENGTH, 16 * 1024 * 1024);
    connection.set_limit(Limit::SQLITE_LIMIT_SQL_LENGTH, MAX_SQL_BYTES as i32);
    connection.set_limit(Limit::SQLITE_LIMIT_ATTACHED, 0);
    // Bound VM work as well as the size of input and database pages. Keep the
    // handler active during schema checks and record extraction, too.
    let mut ticks = 0usize;
    connection.progress_handler(1000, Some(move || { ticks += 1; ticks > 100_000 }));
    connection.authorizer(Some(import_authorizer));
    connection.execute_batch(text).map_err(|_| SourceError::InvalidData)?;
    if !connection.is_autocommit() { return Err(SourceError::InvalidData); }
    connection.authorizer(None::<fn(AuthContext<'_>) -> Authorization>);
    // Check what the input actually created, before any downstream conversion.
    for table in ["providers", "provider_endpoints", "mcp_servers", "prompts", "skills", "skill_repos", "settings"] {
        let exists: bool = connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_schema WHERE type='table' AND name=?1)",
            [table], |row| row.get(0),
        ).map_err(|_| SourceError::InvalidData)?;
        if !exists { return Err(SourceError::InvalidData); }
    }
    let integrity: String = connection.query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .map_err(|_| SourceError::InvalidData)?;
    if integrity != "ok" { return Err(SourceError::InvalidData); }
    super::source_upgrade::upgrade_staged_copy(&connection)?;
    connection.execute_batch("PRAGMA query_only=ON;").map_err(|_| SourceError::InvalidData)?;
    let mut records = Vec::new();
    let version = source::read_connection(&connection, &mut records)?;
    let mut counts = BTreeMap::new();
    for record in &records { *counts.entry(record.category).or_insert(0) += 1; }
    Ok(SourceSnapshot {
        scope: scope.clone(),
        summary: SourceSummary {
            format: SourceFormat::Sql, version, main_sha256: Sha256::digest(sql.as_bytes()).into(),
            wal_sha256: None, counts,
        },
        records,
    })
}

fn import_authorizer(context: AuthContext<'_>) -> Authorization {
    // A positive allowlist fails closed for extensions and future SQLite actions.
    // No functions are needed by the literal INSERT statements in CC dumps.
    let allowed = match context.action {
        AuthAction::Pragma { pragma_name, .. } => ["foreign_keys", "user_version"]
            .iter().any(|name| pragma_name.eq_ignore_ascii_case(name)),
        AuthAction::CreateTable { .. } | AuthAction::CreateIndex { .. }
        | AuthAction::CreateTrigger { .. } | AuthAction::CreateView { .. }
        | AuthAction::Insert { .. } | AuthAction::Delete { .. } | AuthAction::Update { .. }
        | AuthAction::Read { .. } | AuthAction::Select | AuthAction::Transaction { .. }
        | AuthAction::Reindex { .. } => true,
        _ => false,
    };
    if allowed { Authorization::Allow } else { Authorization::Deny }
}
