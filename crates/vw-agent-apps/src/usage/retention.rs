//! Atomic daily aggregation and detail retention on the host-owned ledger.
//! Replay receipts deliberately outlive details and scan cursors.
use std::{collections::BTreeMap, str::FromStr};

use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use vw_api_types::{id::ProjectScope, usage::*};

use super::store::{StoredUsageRecord, UsageStoreError};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageRollupDimensions {
    pub day_unix_seconds: i64,
    /// Exact range filtering after pruning; absent on legacy daily aggregates.
    #[serde(default)]
    pub started_at_unix_seconds: Option<i64>,
    pub provider: UsageProvider,
    pub sources: Vec<UsageSourceRef>,
    pub models: UsageModels,
    pub status: UsageRequestStatus,
    pub billable: bool,
    pub cost_source: UsageCostSource,
    // Missing counters and different semantics must never collapse into known zero.
    pub token_shape: Option<UsageTokens>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageDailyRollup {
    pub dimensions: UsageRollupDimensions,
    pub request_count: u64,
    pub tokens: Option<UsageTokens>,
    pub known_cost_usd: String,
    pub unknown_cost_requests: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsageRetentionError {
    InvalidInput,
    Cancelled,
    StorageUnavailable,
}
impl std::fmt::Display for UsageRetentionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for UsageRetentionError {}
fn storage(_: rusqlite::Error) -> UsageRetentionError {
    UsageRetentionError::StorageUnavailable
}
fn corrupt() -> UsageRetentionError {
    UsageRetentionError::StorageUnavailable
}

pub(super) fn initialize_retention(connection: &Connection) -> Result<(), UsageStoreError> {
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS usage_daily_rollups (
            scope_key TEXT NOT NULL, dimensions_key TEXT NOT NULL, payload_json TEXT NOT NULL,
            PRIMARY KEY(scope_key, dimensions_key));
        CREATE TABLE IF NOT EXISTS usage_archived_records (
            scope_key TEXT NOT NULL, record_id TEXT NOT NULL,
            PRIMARY KEY(scope_key, record_id));
        CREATE TABLE IF NOT EXISTS usage_archived_identities (
            scope_key TEXT NOT NULL, identity_key TEXT NOT NULL, record_id TEXT NOT NULL,
            PRIMARY KEY(scope_key, identity_key));",
        )
        .map_err(|_| UsageStoreError::StorageUnavailable)
}

fn scope_key(scope: &ProjectScope, host: &str) -> Result<String, UsageRetentionError> {
    if host.trim().is_empty() || host.contains('\0') {
        return Err(UsageRetentionError::InvalidInput);
    }
    serde_json::to_string(&(scope, host)).map_err(|_| UsageRetentionError::InvalidInput)
}

/// Read from this connection every time. No process-global count cache can leak
/// results across connections, scopes or maintenance commits.
pub fn load_daily_rollups(
    connection: &Connection,
    scope: &ProjectScope,
    host: &str,
) -> Result<Vec<UsageDailyRollup>, UsageRetentionError> {
    let key = scope_key(scope, host)?;
    let mut statement = connection.prepare(
        "SELECT payload_json FROM usage_daily_rollups WHERE scope_key=?1 ORDER BY dimensions_key"
    ).map_err(storage)?;
    let rows = statement.query_map([key], |row| row.get::<_, String>(0)).map_err(storage)?;
    rows.map(|row| serde_json::from_str(&row.map_err(storage)?).map_err(|_| corrupt())).collect()
}

/// Retain at least thirty full UTC days. Only complete older days are removed.
/// Cancellation or any malformed amount/counter rolls back the entire operation.
/// Cursor rebuilds must preserve the receipt tables, just as normal rescans do.
pub fn rollup_and_prune(
    connection: &mut Connection,
    scope: &ProjectScope,
    host: &str,
    now_unix_seconds: i64,
    cancelled: impl Fn() -> bool,
) -> Result<u64, UsageRetentionError> {
    let key = scope_key(scope, host)?;
    let cutoff = now_unix_seconds
        .div_euclid(86400)
        .checked_mul(86400)
        .and_then(|day| day.checked_sub(30 * 86400))
        .ok_or(UsageRetentionError::InvalidInput)?;
    let check_cancelled = || if cancelled() { Err(UsageRetentionError::Cancelled) } else { Ok(()) };
    check_cancelled()?;
    let tx =
        connection.transaction_with_behavior(TransactionBehavior::Immediate).map_err(storage)?;
    let mut groups: BTreeMap<String, UsageDailyRollup> = BTreeMap::new();
    let mut statement = tx
        .prepare("SELECT record_id,payload_json FROM usage_records WHERE scope_key=?1")
        .map_err(storage)?;
    let mut rows = statement.query([&key]).map_err(storage)?;
    let mut ids = Vec::new();
    while let Some(row) = rows.next().map_err(storage)? {
        check_cancelled()?;
        let id: String = row.get(0).map_err(storage)?;
        let payload: String = row.get(1).map_err(storage)?;
        let stored: StoredUsageRecord = serde_json::from_str(&payload).map_err(|_| corrupt())?;
        let record = stored.record;
        if record.started_at_unix_seconds.0 >= cutoff {
            continue;
        }
        if record.id != id {
            return Err(corrupt());
        }
        let mut sources = record.sources.clone();
        for source in &mut sources {
            source.source_id.clear();
            source.native_request_id = None;
        }
        let mut shape = record.tokens.clone();
        if let Some(tokens) = &mut shape {
            for count in [
                &mut tokens.input,
                &mut tokens.output,
                &mut tokens.reasoning,
                &mut tokens.cache_read,
                &mut tokens.cache_creation,
                &mut tokens.cache_creation_1h,
            ] {
                if count.is_some() {
                    *count = Some(0);
                }
            }
        }
        let dimensions = UsageRollupDimensions {
            started_at_unix_seconds: Some(record.started_at_unix_seconds.0),
            day_unix_seconds: record
                .started_at_unix_seconds
                .0
                .div_euclid(86400)
                .checked_mul(86400)
                .ok_or_else(corrupt)?,
            provider: record.provider.clone(),
            sources,
            models: record.models.clone(),
            status: record.status,
            billable: record.billable,
            cost_source: record.cost.source,
            token_shape: shape.clone(),
        };
        let group_key = serde_json::to_string(&dimensions).map_err(|_| corrupt())?;
        if !groups.contains_key(&group_key) {
            let old: Option<String> = tx.query_row(
                "SELECT payload_json FROM usage_daily_rollups WHERE scope_key=?1 AND dimensions_key=?2",
                params![key, group_key], |row| row.get(0),
            ).optional().map_err(storage)?;
            let aggregate = match old {
                Some(payload) => serde_json::from_str(&payload).map_err(|_| corrupt())?,
                None => UsageDailyRollup {
                    dimensions,
                    request_count: 0,
                    tokens: shape,
                    known_cost_usd: "0".into(),
                    unknown_cost_requests: 0,
                },
            };
            groups.insert(group_key.clone(), aggregate);
        }
        let aggregate = groups.get_mut(&group_key).ok_or_else(corrupt)?;
        aggregate.request_count = aggregate.request_count.checked_add(1).ok_or_else(corrupt)?;
        if let (Some(sum), Some(tokens)) = (&mut aggregate.tokens, &record.tokens) {
            for (sum, value) in [
                (&mut sum.input, tokens.input),
                (&mut sum.output, tokens.output),
                (&mut sum.reasoning, tokens.reasoning),
                (&mut sum.cache_read, tokens.cache_read),
                (&mut sum.cache_creation, tokens.cache_creation),
                (&mut sum.cache_creation_1h, tokens.cache_creation_1h),
            ] {
                if let (Some(a), Some(b)) = (*sum, value) {
                    *sum = Some(a.checked_add(b).ok_or_else(corrupt)?);
                }
            }
        }
        match (&record.cost.source, &record.cost.amount_usd) {
            (UsageCostSource::Unknown, None) => {
                aggregate.unknown_cost_requests =
                    aggregate.unknown_cost_requests.checked_add(1).ok_or_else(corrupt)?;
            }
            (UsageCostSource::Unknown, Some(_)) | (_, None) => return Err(corrupt()),
            (_, Some(amount)) => {
                let amount = Decimal::from_str(amount).map_err(|_| corrupt())?;
                if amount < Decimal::ZERO {
                    return Err(corrupt());
                }
                aggregate.known_cost_usd = Decimal::from_str(&aggregate.known_cost_usd)
                    .map_err(|_| corrupt())?
                    .checked_add(amount)
                    .ok_or_else(corrupt)?
                    .normalize()
                    .to_string();
            }
        }
        ids.push(id);
    }
    drop(rows);
    drop(statement);
    for (dimensions, aggregate) in &groups {
        check_cancelled()?;
        tx.execute(
            "INSERT INTO usage_daily_rollups VALUES (?1,?2,?3)
            ON CONFLICT(scope_key,dimensions_key) DO UPDATE SET payload_json=excluded.payload_json",
            params![key, dimensions, serde_json::to_string(aggregate).map_err(|_| corrupt())?],
        )
        .map_err(storage)?;
    }
    for id in &ids {
        check_cancelled()?;
        tx.execute("INSERT INTO usage_archived_records VALUES (?1,?2)", params![key, id])
            .map_err(storage)?;
        tx.execute(
            "INSERT INTO usage_archived_identities SELECT scope_key,identity_key,record_id
            FROM usage_source_identities WHERE scope_key=?1 AND record_id=?2",
            params![key, id],
        )
        .map_err(storage)?;
        tx.execute(
            "DELETE FROM usage_source_identities WHERE scope_key=?1 AND record_id=?2",
            params![key, id],
        )
        .map_err(storage)?;
        tx.execute(
            "DELETE FROM usage_records WHERE scope_key=?1 AND record_id=?2",
            params![key, id],
        )
        .map_err(storage)?;
    }
    let apps: std::collections::BTreeSet<_> = groups
        .values()
        .flat_map(|group| group.dimensions.sources.iter().map(|source| source.app.as_str()))
        .collect();
    for app in apps {
        super::source_settings::advance_statistics_revision(&tx, scope, host, app)
            .map_err(|_| corrupt())?;
    }
    check_cancelled()?;
    tx.commit().map_err(storage)?;
    Ok(ids.len() as u64)
}
