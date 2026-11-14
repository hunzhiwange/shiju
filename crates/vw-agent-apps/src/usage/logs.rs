//! Read-only queries over canonical ledger records. Authorization is bound by the
//! caller's project and host, never by query parameters. Archived detail is absent.
use std::collections::BTreeMap;

use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use vw_api_types::{id::ProjectScope, usage::*};

use super::{ranges::resolve_usage_range, store::StoredUsageRecord};

pub const MAX_USAGE_LOG_PAGE_SIZE: u32 = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsageLogsError {
    InvalidInput,
    RevisionConflict,
    StorageUnavailable,
}
impl std::fmt::Display for UsageLogsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for UsageLogsError {}
fn storage(_: rusqlite::Error) -> UsageLogsError {
    UsageLogsError::StorageUnavailable
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageLogDetail {
    pub record: UsageRecord,
    pub status_code: Option<u16>,
    /// Fixed public message from the failure enum; no upstream body is returned.
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageSourceDistribution {
    pub app: String,
    pub kind: UsageSourceKind,
    /// Canonical requests associated with this source, not identity/event count.
    pub counts: UsageRequestCounts,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageLogResult {
    pub page: UsagePage<UsageLogDetail>,
    /// Counts and distribution cover all matched records before pagination.
    pub counts: UsageRequestCounts,
    /// Multi-source canonical records may appear in more than one group.
    pub sources: Vec<UsageSourceDistribution>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Cursor {
    binding: String,
    range: UsageResolvedRange,
    snapshot: String,
    after_time: i64,
    after_id: String,
}

fn key(scope: &ProjectScope, host: &str) -> Result<String, UsageLogsError> {
    if host.trim().is_empty() || host.contains('\0') {
        return Err(UsageLogsError::InvalidInput);
    }
    serde_json::to_string(&(scope, host)).map_err(|_| UsageLogsError::InvalidInput)
}
fn digest(value: &impl Serialize) -> Result<String, UsageLogsError> {
    let bytes = serde_json::to_vec(value).map_err(|_| UsageLogsError::StorageUnavailable)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

/// Shared effective filter for totals, rows and source distribution. The ledger
/// already stores canonical requests; suspected duplicates remain visible until
/// evidence-based reconciliation. Source predicates must match the same source.
pub fn effective_usage_log_filter(
    record: &UsageRecord,
    filter: &UsageFilter,
    range: &UsageResolvedRange,
) -> bool {
    let time = record.started_at_unix_seconds.0;
    range.start_unix_seconds.is_none_or(|start| time >= start.0)
        && time < range.end_unix_seconds.0
        && filter.provider.as_ref().is_none_or(|v| v == &record.provider)
        && filter.request_model.as_ref().is_none_or(|v| Some(v) == record.models.request_model.as_ref())
        && filter.response_model.as_ref().is_none_or(|v| Some(v) == record.models.response_model.as_ref())
        && filter.pricing_model.as_ref().is_none_or(|v| Some(v) == record.models.pricing_model.as_ref())
        && filter.status.is_none_or(|v| v == record.status)
        && filter.billable.is_none_or(|v| v == record.billable)
        && (filter.app.is_none() && filter.source_kind.is_none() && filter.native_session_id.is_none()
            || record.sources.iter().any(|source| source_matches(source, filter)))
}
fn source_matches(source: &UsageSourceRef, filter: &UsageFilter) -> bool {
    filter.app.as_ref().is_none_or(|v| v == &source.app)
        && filter.source_kind.is_none_or(|v| v == source.kind)
        && filter.native_session_id.as_ref().is_none_or(|v| Some(v) == source.native_session_id.as_ref())
}
fn detail(value: StoredUsageRecord) -> UsageLogDetail {
    let error_message = value.failure.map(|failure| match failure {
        super::store::UsageFailure::Upstream => "Upstream request failed",
        super::store::UsageFailure::Timeout => "Request timed out",
        super::store::UsageFailure::Cancelled => "Request cancelled",
        super::store::UsageFailure::PermissionDenied => "Request permission denied",
    }.to_owned());
    UsageLogDetail { record: value.record, status_code: value.status_code, error_message }
}
fn empty_counts() -> UsageRequestCounts {
    UsageRequestCounts { total_requests: 0, billable_requests: 0, unknown_cost_requests: 0 }
}
fn count(counts: &mut UsageRequestCounts, record: &UsageRecord) {
    counts.total_requests += 1;
    counts.billable_requests += u64::from(record.billable);
    counts.unknown_cost_requests += u64::from(record.cost.source == UsageCostSource::Unknown);
}

/// The caller resolves the authenticated host and scope before invoking this API.
/// One SQLite read snapshot keeps count, detail, revision and distribution aligned.
/// Stable ordering is start time descending, then canonical ID descending.
pub fn query_usage_logs(
    connection: &Connection,
    scope: &ProjectScope,
    host: &str,
    query: &UsageQuery,
    time_zone: &str,
    now: UsageUnixSeconds,
) -> Result<UsageLogResult, UsageLogsError> {
    let scope_key = key(scope, host)?;
    let limit = query.limit.unwrap_or(50);
    if limit == 0 || limit > MAX_USAGE_LOG_PAGE_SIZE {
        return Err(UsageLogsError::InvalidInput);
    }
    let binding = digest(&(&scope_key, &query.range, &query.filter, time_zone))?;
    let cursor: Option<Cursor> = query.cursor.as_ref().map(|value| {
        if value.len() > 4096 { return Err(UsageLogsError::InvalidInput); }
        serde_json::from_str(value).map_err(|_| UsageLogsError::InvalidInput)
    }).transpose()?;
    let resolved = resolve_usage_range(&query.range, time_zone, now)
        .map_err(|_| UsageLogsError::InvalidInput)?;
    let range = match &cursor {
        Some(cursor) if cursor.binding == binding => {
            // A cursor may freeze a live range, but cannot introduce invalid bounds.
            let frozen = match cursor.range.start_unix_seconds {
                Some(start) => UsageRange::Absolute {
                    start_unix_seconds: start,
                    end_unix_seconds: cursor.range.end_unix_seconds,
                },
                None => UsageRange::All,
            };
            let checked = resolve_usage_range(
                &frozen, &cursor.range.time_zone, cursor.range.end_unix_seconds,
            ).map_err(|_| UsageLogsError::InvalidInput)?;
            if checked != cursor.range { return Err(UsageLogsError::InvalidInput); }
            cursor.range.clone()
        }
        Some(_) => return Err(UsageLogsError::InvalidInput),
        None => resolved,
    };
    let transaction = connection.unchecked_transaction().map_err(storage)?;
    let mut statement = transaction.prepare(
        "SELECT payload_json FROM usage_records WHERE scope_key=?1 ORDER BY record_id"
    ).map_err(storage)?;
    let rows = statement.query_map([&scope_key], |row| row.get::<_, String>(0)).map_err(storage)?;
    let mut records = Vec::new();
    for row in rows {
        let value: StoredUsageRecord = serde_json::from_str(&row.map_err(storage)?)
            .map_err(|_| UsageLogsError::StorageUnavailable)?;
        if effective_usage_log_filter(&value.record, &query.filter, &range) {
            records.push(detail(value));
        }
    }
    drop(statement);
    let mut revision = 0u64;
    let mut statement = transaction.prepare(
        "SELECT scope_key, revision FROM usage_statistics_revisions"
    ).map_err(storage)?;
    let revisions = statement.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, u64>(1)?)))
        .map_err(storage)?;
    for row in revisions {
        let (encoded, value) = row.map_err(storage)?;
        let (row_scope, row_host, _): (ProjectScope, String, String) = serde_json::from_str(&encoded)
            .map_err(|_| UsageLogsError::StorageUnavailable)?;
        if row_scope == *scope && row_host == host {
            revision = revision.checked_add(value).ok_or(UsageLogsError::StorageUnavailable)?;
        }
    }
    drop(statement);
    transaction.commit().map_err(storage)?;
    records.sort_by(|a, b| b.record.started_at_unix_seconds.0.cmp(&a.record.started_at_unix_seconds.0)
        .then_with(|| b.record.id.cmp(&a.record.id)));
    let snapshot = digest(&(&records, revision))?;
    if cursor.as_ref().is_some_and(|v| v.snapshot != snapshot) {
        return Err(UsageLogsError::RevisionConflict);
    }
    let mut counts = empty_counts();
    let mut groups = BTreeMap::new();
    for value in &records {
        count(&mut counts, &value.record);
        let mut seen = std::collections::HashSet::new();
        for source in value.record.sources.iter().filter(|v| source_matches(v, &query.filter)) {
            let kind = match source.kind { UsageSourceKind::Native => 0, UsageSourceKind::Relay => 1, UsageSourceKind::Shiju => 2 };
            let group_key = (source.app.clone(), kind);
            if !seen.insert(group_key.clone()) { continue; }
            let group = groups.entry(group_key).or_insert_with(|| UsageSourceDistribution {
                app: source.app.clone(), kind: source.kind, counts: empty_counts(),
            });
            count(&mut group.counts, &value.record);
        }
    }
    let mut remaining = records.into_iter().filter(|v| cursor.as_ref().is_none_or(|cursor| {
        (v.record.started_at_unix_seconds.0, v.record.id.as_str()) < (cursor.after_time, cursor.after_id.as_str())
    }));
    let items: Vec<_> = remaining.by_ref().take(limit as usize).collect();
    let next_cursor = if remaining.next().is_some() {
        let last = items.last().ok_or(UsageLogsError::StorageUnavailable)?;
        Some(serde_json::to_string(&Cursor {
            binding, range: range.clone(), snapshot,
            after_time: last.record.started_at_unix_seconds.0, after_id: last.record.id.clone(),
        }).map_err(|_| UsageLogsError::StorageUnavailable)?)
    } else { None };
    Ok(UsageLogResult {
        page: UsagePage { items, next_cursor, resolved_range: range, revision },
        counts, sources: groups.into_values().collect(),
    })
}

/// Missing and out-of-scope IDs both return None. Never queries source file paths
/// or the stored free-form error column, and does not backfill/mutate on reads.
pub fn get_usage_log_detail(
    connection: &Connection,
    scope: &ProjectScope,
    host: &str,
    id: &str,
) -> Result<Option<UsageLogDetail>, UsageLogsError> {
    if id.trim().is_empty() || id.contains('\0') { return Err(UsageLogsError::InvalidInput); }
    let payload: Option<String> = connection.query_row(
        "SELECT payload_json FROM usage_records WHERE scope_key=?1 AND record_id=?2",
        rusqlite::params![key(scope, host)?, id], |row| row.get(0),
    ).optional().map_err(storage)?;
    payload.map(|payload| serde_json::from_str(&payload).map(detail)
        .map_err(|_| UsageLogsError::StorageUnavailable)).transpose()
}
