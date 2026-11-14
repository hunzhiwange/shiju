//! Persisted preferences and statistics generations; quota snapshots remain owned by cache.
use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use vw_api_types::id::ProjectScope;

use super::store::UsageStoreError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UsageSourceSettings {
    /// A preference, never authorization to resolve or open a log path.
    pub scan_enabled: bool,
    pub show_usage: bool,
    /// None disables automatic refresh; manual queries remain available.
    pub refresh_seconds: Option<u32>,
}

impl Default for UsageSourceSettings {
    fn default() -> Self {
        Self { scan_enabled: true, show_usage: true, refresh_seconds: None }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageSourceState {
    pub settings: UsageSourceSettings,
    /// Optimistic concurrency token for preferences only.
    pub settings_revision: u64,
    /// Include this generation and the full scope key in statistics cache keys.
    pub statistics_revision: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceSettingsError {
    InvalidInput,
    RevisionConflict,
    StorageUnavailable,
}
impl std::fmt::Display for SourceSettingsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for SourceSettingsError {}
fn storage(_: rusqlite::Error) -> SourceSettingsError {
    SourceSettingsError::StorageUnavailable
}

/// Use only scope/host/app resolved from the authenticated host context.
/// Structured encoding also isolates projects and delimiter-containing identifiers.
pub fn statistics_scope_key(
    scope: &ProjectScope,
    host: &str,
    app: &str,
) -> Result<String, SourceSettingsError> {
    if [host, app].iter().any(|value| value.trim().is_empty() || value.contains('\0')) {
        return Err(SourceSettingsError::InvalidInput);
    }
    serde_json::to_string(&(scope, host, app)).map_err(|_| SourceSettingsError::InvalidInput)
}

pub fn initialize_source_settings(connection: &Connection) -> Result<(), SourceSettingsError> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS usage_source_settings (
            scope_key TEXT PRIMARY KEY NOT NULL,
            payload_json TEXT NOT NULL,
            settings_revision INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS usage_statistics_revisions (
            scope_key TEXT PRIMARY KEY NOT NULL,
            revision INTEGER NOT NULL
        );",
    ).map_err(storage)
}

pub fn load_source_settings(
    connection: &Connection,
    scope: &ProjectScope,
    host: &str,
    app: &str,
) -> Result<UsageSourceState, SourceSettingsError> {
    let key = statistics_scope_key(scope, host, app)?;
    let row: Option<(String, u64)> = connection.query_row(
        "SELECT payload_json, settings_revision FROM usage_source_settings WHERE scope_key=?1",
        [&key], |row| Ok((row.get(0)?, row.get(1)?)),
    ).optional().map_err(storage)?;
    let (settings, settings_revision) = match row {
        Some((payload, revision)) => (
            serde_json::from_str(&payload).map_err(|_| SourceSettingsError::StorageUnavailable)?,
            revision,
        ),
        None => (UsageSourceSettings::default(), 0),
    };
    let statistics_revision = connection.query_row(
        "SELECT revision FROM usage_statistics_revisions WHERE scope_key=?1",
        [&key], |row| row.get(0),
    ).optional().map_err(storage)?.unwrap_or(0);
    Ok(UsageSourceState { settings, settings_revision, statistics_revision })
}

pub fn save_source_settings(
    connection: &mut Connection,
    scope: &ProjectScope,
    host: &str,
    app: &str,
    expected_revision: u64,
    settings: UsageSourceSettings,
) -> Result<UsageSourceState, SourceSettingsError> {
    if settings.refresh_seconds == Some(0) {
        return Err(SourceSettingsError::InvalidInput);
    }
    let key = statistics_scope_key(scope, host, app)?;
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage)?;
    let old = load_source_settings(&transaction, scope, host, app)?;
    if old.settings_revision != expected_revision {
        return Err(SourceSettingsError::RevisionConflict);
    }
    if old.settings != settings {
        let revision = expected_revision.checked_add(1)
            .filter(|value| *value <= i64::MAX as u64)
            .ok_or(SourceSettingsError::StorageUnavailable)?;
        let payload = serde_json::to_string(&settings)
            .map_err(|_| SourceSettingsError::InvalidInput)?;
        transaction.execute(
            "INSERT INTO usage_source_settings VALUES (?1,?2,?3)
             ON CONFLICT(scope_key) DO UPDATE SET payload_json=excluded.payload_json,
             settings_revision=excluded.settings_revision",
            params![key, payload, revision],
        ).map_err(storage)?;
        advance_statistics_revision(&transaction, scope, host, app)
            .map_err(|_| SourceSettingsError::StorageUnavailable)?;
    }
    let state = load_source_settings(&transaction, scope, host, app)?;
    transaction.commit().map_err(storage)?;
    Ok(state)
}

/// Called in the same transaction as a ledger mutation. Replays do not invalidate.
/// Consumers requery after commit; missed/coalesced notifications are recovered by
/// comparing this durable generation. Never clears another scope or quota cache.
pub fn advance_statistics_revision(
    transaction: &Transaction<'_>,
    scope: &ProjectScope,
    host: &str,
    app: &str,
) -> Result<(), UsageStoreError> {
    let key = statistics_scope_key(scope, host, app).map_err(|_| UsageStoreError::InvalidInput)?;
    let changed = transaction.execute(
        "INSERT INTO usage_statistics_revisions VALUES (?1,1)
         ON CONFLICT(scope_key) DO UPDATE SET revision=revision+1
         WHERE revision < 9223372036854775807",
        [key],
    ).map_err(|_| UsageStoreError::StorageUnavailable)?;
    if changed != 1 { return Err(UsageStoreError::StorageUnavailable); }
    Ok(())
}

/// Consumption contract for upstream `usage-cache-updated`. The authenticated
/// event bridge uses the original quota owner and its account/provider key;
/// these notifications never mutate ledger statistics or execute scripts.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UsageCacheUpdated {
    Script {
        #[serde(rename = "appType")]
        app: String,
        #[serde(rename = "providerId")]
        provider_id: String,
    },
    Subscription {
        #[serde(rename = "appType")]
        app: String,
    },
}
