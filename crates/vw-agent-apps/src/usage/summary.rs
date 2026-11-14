//! Scope-bound summaries over canonical ledger rows and disjoint archived rows.
//! Source links are filters, never additional requests. Resolve ranges once in the caller.
use std::{
    collections::{BTreeMap, BTreeSet},
    str::FromStr,
};

use rusqlite::Connection;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use vw_api_types::{id::ProjectScope, usage::*};

use super::{
    retention::{UsageDailyRollup, UsageRollupDimensions, load_daily_rollups},
    store::StoredUsageRecord,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageSummary {
    pub counts: UsageRequestCounts,
    pub succeeded_requests: u64,
    pub failed_requests: u64,
    pub cancelled_requests: u64,
    pub unknown_status_requests: u64,
    pub known_cost_usd: String,
    /// Groups keep input/reasoning semantics and missing-counter shapes separate.
    /// Never sum cache creation's 1h subset or included reasoning a second time.
    pub token_groups: Vec<UsageTokens>,
    pub requests_without_tokens: u64,
    /// UTC day boundary, including history whose details have been pruned.
    pub first_usage_date: Option<UsageUnixSeconds>,
}

impl Default for UsageSummary {
    fn default() -> Self {
        Self {
            counts: UsageRequestCounts {
                total_requests: 0,
                billable_requests: 0,
                unknown_cost_requests: 0,
            },
            succeeded_requests: 0,
            failed_requests: 0,
            cancelled_requests: 0,
            unknown_status_requests: 0,
            known_cost_usd: "0".into(),
            token_groups: Vec::new(),
            requests_without_tokens: 0,
            first_usage_date: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsageSummaryError {
    InvalidInput,
    StorageUnavailable,
    /// Legacy day-only archives cannot answer a partial-day query exactly.
    ArchivePrecisionUnavailable,
}
impl std::fmt::Display for UsageSummaryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for UsageSummaryError {}
type Result<T> = std::result::Result<T, UsageSummaryError>;
fn storage<E>(_: E) -> UsageSummaryError {
    UsageSummaryError::StorageUnavailable
}
fn add(a: &mut u64, b: u64) -> Result<()> {
    *a = a.checked_add(b).ok_or(UsageSummaryError::StorageUnavailable)?;
    Ok(())
}

fn source_matches(source: &UsageSourceRef, filter: &UsageFilter) -> bool {
    filter.app.as_ref().is_none_or(|app| app == &source.app)
        && filter.source_kind.is_none_or(|kind| kind == source.kind)
        && filter
            .native_session_id
            .as_ref()
            .is_none_or(|id| source.native_session_id.as_ref() == Some(id))
}
fn matches(d: &UsageRollupDimensions, f: &UsageFilter) -> bool {
    f.provider.as_ref().is_none_or(|v| v == &d.provider)
        && f.request_model.as_ref().is_none_or(|v| d.models.request_model.as_ref() == Some(v))
        && f.response_model.as_ref().is_none_or(|v| d.models.response_model.as_ref() == Some(v))
        && f.pricing_model.as_ref().is_none_or(|v| d.models.pricing_model.as_ref() == Some(v))
        && f.status.is_none_or(|v| v == d.status)
        && f.billable.is_none_or(|v| v == d.billable)
        && ((f.app.is_none() && f.source_kind.is_none() && f.native_session_id.is_none())
            || d.sources.iter().any(|s| source_matches(s, f)))
}
fn in_range(d: &UsageRollupDimensions, r: &UsageResolvedRange) -> Result<bool> {
    let start = r.start_unix_seconds.map(|v| v.0);
    let end = r.end_unix_seconds.0;
    if let Some(at) = d.started_at_unix_seconds {
        return Ok(start.is_none_or(|s| at >= s) && at < end);
    }
    let day = d.day_unix_seconds;
    let next = day.checked_add(86400).ok_or(UsageSummaryError::StorageUnavailable)?;
    if start.is_some_and(|s| s >= next) || end <= day {
        return Ok(false);
    }
    if start.is_some_and(|s| s > day) || end < next {
        return Err(UsageSummaryError::ArchivePrecisionUnavailable);
    }
    Ok(true)
}

/// One SQLite read snapshot prevents a concurrent prune from omitting or doubling history.
pub(super) fn load(
    connection: &Connection,
    scope: &ProjectScope,
    host: &str,
    range: &UsageResolvedRange,
    filter: &UsageFilter,
) -> Result<Vec<UsageDailyRollup>> {
    if host.trim().is_empty()
        || host.contains('\0')
        || range.start_unix_seconds.is_some_and(|s| s.0 >= range.end_unix_seconds.0)
    {
        return Err(UsageSummaryError::InvalidInput);
    }
    let key = serde_json::to_string(&(scope, host)).map_err(storage)?;
    let tx = connection.unchecked_transaction().map_err(storage)?;
    let mut values = load_daily_rollups(&tx, scope, host).map_err(storage)?;
    {
        let mut stmt = tx
            .prepare("SELECT record_id,payload_json FROM usage_records WHERE scope_key=?1")
            .map_err(storage)?;
        let mut rows = stmt.query([key]).map_err(storage)?;
        while let Some(row) = rows.next().map_err(storage)? {
            let id: String = row.get(0).map_err(storage)?;
            let payload: String = row.get(1).map_err(storage)?;
            let stored: StoredUsageRecord = serde_json::from_str(&payload).map_err(storage)?;
            let r = stored.record;
            if r.id != id {
                return Err(UsageSummaryError::StorageUnavailable);
            }
            let (known_cost_usd, unknown_cost_requests) = match (r.cost.source, r.cost.amount_usd) {
                (UsageCostSource::Unknown, None) => ("0".into(), 1),
                (UsageCostSource::Unknown, Some(_)) | (_, None) => {
                    return Err(UsageSummaryError::StorageUnavailable);
                }
                (_, Some(amount)) => (amount, 0),
            };
            values.push(UsageDailyRollup {
                dimensions: UsageRollupDimensions {
                    day_unix_seconds: r
                        .started_at_unix_seconds
                        .0
                        .div_euclid(86400)
                        .checked_mul(86400)
                        .ok_or(UsageSummaryError::StorageUnavailable)?,
                    started_at_unix_seconds: Some(r.started_at_unix_seconds.0),
                    provider: r.provider,
                    sources: r.sources,
                    models: r.models,
                    status: r.status,
                    billable: r.billable,
                    cost_source: r.cost.source,
                    token_shape: None,
                },
                request_count: 1,
                tokens: r.tokens,
                known_cost_usd,
                unknown_cost_requests,
            });
        }
    }
    let mut selected = Vec::new();
    for value in values {
        if matches(&value.dimensions, filter) && in_range(&value.dimensions, range)? {
            selected.push(value);
        }
    }
    tx.commit().map_err(storage)?;
    Ok(selected)
}

pub(super) fn accumulate(summary: &mut UsageSummary, value: &UsageDailyRollup) -> Result<()> {
    if value.unknown_cost_requests > value.request_count {
        return Err(UsageSummaryError::StorageUnavailable);
    }
    add(&mut summary.counts.total_requests, value.request_count)?;
    if value.dimensions.billable {
        add(&mut summary.counts.billable_requests, value.request_count)?;
    }
    add(&mut summary.counts.unknown_cost_requests, value.unknown_cost_requests)?;
    let count = match value.dimensions.status {
        UsageRequestStatus::Succeeded => &mut summary.succeeded_requests,
        UsageRequestStatus::Failed => &mut summary.failed_requests,
        UsageRequestStatus::Cancelled => &mut summary.cancelled_requests,
        UsageRequestStatus::Unknown => &mut summary.unknown_status_requests,
    };
    add(count, value.request_count)?;
    let amount = Decimal::from_str(&value.known_cost_usd).map_err(storage)?;
    if amount < Decimal::ZERO {
        return Err(UsageSummaryError::StorageUnavailable);
    }
    summary.known_cost_usd = Decimal::from_str(&summary.known_cost_usd)
        .map_err(storage)?
        .checked_add(amount)
        .ok_or(UsageSummaryError::StorageUnavailable)?
        .normalize()
        .to_string();
    let day = value.dimensions.day_unix_seconds;
    if summary.first_usage_date.is_none_or(|old| day < old.0) {
        summary.first_usage_date = Some(UsageUnixSeconds(day));
    }
    let Some(tokens) = &value.tokens else {
        return add(&mut summary.requests_without_tokens, value.request_count);
    };
    let same_shape = |t: &&mut UsageTokens| {
        t.input_semantics == tokens.input_semantics
            && t.reasoning_semantics == tokens.reasoning_semantics
            && [t.input, t.output, t.reasoning, t.cache_read, t.cache_creation, t.cache_creation_1h]
                .map(|v| v.is_some())
                == [
                    tokens.input,
                    tokens.output,
                    tokens.reasoning,
                    tokens.cache_read,
                    tokens.cache_creation,
                    tokens.cache_creation_1h,
                ]
                .map(|v| v.is_some())
    };
    if let Some(sum) = summary.token_groups.iter_mut().find(same_shape) {
        for (sum, value) in [
            (&mut sum.input, tokens.input),
            (&mut sum.output, tokens.output),
            (&mut sum.reasoning, tokens.reasoning),
            (&mut sum.cache_read, tokens.cache_read),
            (&mut sum.cache_creation, tokens.cache_creation),
            (&mut sum.cache_creation_1h, tokens.cache_creation_1h),
        ] {
            if let (Some(sum), Some(value)) = (sum.as_mut(), value) {
                add(sum, value)?;
            }
        }
    } else {
        summary.token_groups.push(tokens.clone());
    }
    Ok(())
}

pub fn get_usage_summary(
    connection: &Connection,
    scope: &ProjectScope,
    host: &str,
    range: &UsageResolvedRange,
    filter: &UsageFilter,
) -> Result<UsageSummary> {
    let mut summary = UsageSummary::default();
    for value in load(connection, scope, host, range, filter)? {
        accumulate(&mut summary, &value)?;
    }
    Ok(summary)
}

/// A canonical request contributes once per matching app, regardless of source-link count.
pub fn get_usage_summary_by_app(
    connection: &Connection,
    scope: &ProjectScope,
    host: &str,
    range: &UsageResolvedRange,
    filter: &UsageFilter,
) -> Result<BTreeMap<String, UsageSummary>> {
    let mut groups = BTreeMap::new();
    for value in load(connection, scope, host, range, filter)? {
        let apps: BTreeSet<_> = value
            .dimensions
            .sources
            .iter()
            .filter(|s| source_matches(s, filter))
            .map(|s| s.app.clone())
            .collect();
        for app in apps {
            accumulate(groups.entry(app).or_default(), &value)?;
        }
    }
    Ok(groups)
}

/// Session identity includes the app; equal native IDs across apps never collapse.
/// Requests without a session remain in global/app totals only.
pub fn get_session_usage_summary(
    connection: &Connection,
    scope: &ProjectScope,
    host: &str,
    range: &UsageResolvedRange,
    filter: &UsageFilter,
) -> Result<BTreeMap<(String, String), UsageSummary>> {
    let mut groups = BTreeMap::new();
    for value in load(connection, scope, host, range, filter)? {
        let sessions: BTreeSet<_> = value
            .dimensions
            .sources
            .iter()
            .filter(|s| source_matches(s, filter))
            .filter_map(|s| s.native_session_id.as_ref().map(|id| (s.app.clone(), id.clone())))
            .collect();
        for session in sessions {
            accumulate(groups.entry(session).or_default(), &value)?;
        }
    }
    Ok(groups)
}

/// Uses exactly the same range and filters as the summary.
pub fn get_first_usage_date(
    connection: &Connection,
    scope: &ProjectScope,
    host: &str,
    range: &UsageResolvedRange,
    filter: &UsageFilter,
) -> Result<Option<UsageUnixSeconds>> {
    Ok(get_usage_summary(connection, scope, host, range, filter)?.first_usage_date)
}
