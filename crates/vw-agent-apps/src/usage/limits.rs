//! Read-only limits projection over the authorized usage cache and canonical ledger.
//! No credentials are refreshed and no parallel quota counters are maintained here.
use std::{str::FromStr, time::Instant};

use rusqlite::{Connection, params};
use rust_decimal::Decimal;
use serde::Serialize;
use vw_api_types::{
    apps_usage::{UsageCredentialState, UsageQueryData, UsageQueryOutcome, UsageQuotaWindow},
    usage::{
        UsageCostSource, UsageInputSemantics, UsageProvider, UsageReasoningSemantics, UsageTokens,
    },
};

use super::{
    cache::{UsageCache, UsageCacheKey, UsageCacheResult},
    store::{StoredUsageRecord, UsageStoreError},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LimitState {
    Unknown,
    Available,
    Exhausted,
    Failed,
}

#[derive(Debug, Clone, Serialize)]
pub struct LimitWindow {
    pub name: String,
    pub unit: String,
    pub used: Option<String>,
    pub limit: Option<String>,
    pub resets_at: Option<String>,
    pub state: LimitState,
}

/// Period boundaries resolved by the service in the configured calendar timezone.
/// This is trusted provider configuration, never a client-supplied usage count.
pub struct LedgerLimitPeriod {
    pub name: String,
    pub start_unix_seconds: i64,
    pub reset_unix_seconds: i64,
    pub usd: Option<Decimal>,
    pub tokens: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderLimitStatus {
    pub provider_id: String,
    pub state: LimitState,
    pub windows: Vec<LimitWindow>,
}

fn combine(states: impl Iterator<Item = LimitState>) -> LimitState {
    let states: Vec<_> = states.collect();
    if states.contains(&LimitState::Exhausted) {
        LimitState::Exhausted
    } else if states.contains(&LimitState::Failed) {
        LimitState::Failed
    } else if states.is_empty() || states.contains(&LimitState::Unknown) {
        LimitState::Unknown
    } else {
        LimitState::Available
    }
}

fn quota_window(window: &UsageQuotaWindow, now: i64) -> LimitWindow {
    let current = match window.resets_at.as_deref() {
        None => true,
        Some(reset) => {
            chrono::DateTime::parse_from_rfc3339(reset).is_ok_and(|reset| reset.timestamp() > now)
        }
    };
    let remaining = window.remaining.map(f64::from).or_else(|| {
        window.total.zip(window.used).map(|(total, used)| f64::from(total) - f64::from(used))
    });
    let state = if !current {
        LimitState::Unknown
    } else if remaining.is_some_and(|v| v <= 0.0)
        || window.utilization.is_some_and(|v| f64::from(v) >= 100.0)
    {
        LimitState::Exhausted
    } else if remaining.is_some_and(|v| v > 0.0)
        || window.utilization.is_some_and(|v| (0.0..100.0).contains(&f64::from(v)))
    {
        LimitState::Available
    } else {
        LimitState::Unknown
    };
    LimitWindow {
        name: window.name.clone(),
        unit: window.unit.clone().unwrap_or_default(),
        used: window.used.map(|v| f64::from(v).to_string()),
        limit: window.total.map(|v| f64::from(v).to_string()),
        resets_at: window.resets_at.clone(),
        state,
    }
}

fn total_tokens(tokens: &UsageTokens) -> Option<u64> {
    let input = match tokens.input_semantics {
        UsageInputSemantics::Total => tokens.input?,
        UsageInputSemantics::Fresh => {
            tokens.input?.checked_add(tokens.cache_read?)?.checked_add(tokens.cache_creation?)?
        }
        UsageInputSemantics::Unknown => return None,
    };
    let output = match tokens.reasoning_semantics {
        UsageReasoningSemantics::IncludedInOutput => tokens.output?,
        UsageReasoningSemantics::SeparateFromOutput => {
            tokens.output?.checked_add(tokens.reasoning?)?
        }
        UsageReasoningSemantics::Unknown => return None,
    };
    input.checked_add(output)
}

/// The host must authorize and resolve `key` (including account/version) before
/// calling. Its project/host identity also selects the existing ledger partition.
/// No snapshot or expired evidence is interpreted as an unlimited allowance.
pub fn check_provider_limits(
    connection: &Connection,
    cache: &UsageCache,
    key: &UsageCacheKey,
    periods: &[LedgerLimitPeriod],
    now: i64,
    monotonic_now: Instant,
) -> Result<ProviderLimitStatus, UsageStoreError> {
    if periods.iter().any(|p| {
        p.start_unix_seconds > now
            || p.reset_unix_seconds <= now
            || p.usd.is_some_and(|v| v < Decimal::ZERO)
    }) {
        return Err(UsageStoreError::InvalidInput);
    }
    let mut windows = Vec::new();
    let quota_state = match cache.snapshot(key, monotonic_now) {
        Some(UsageCacheResult::Current(result))
            if result.credential_state == UsageCredentialState::Valid =>
        {
            // Cache snapshots do not themselves expire successful publications.
            let fresh = result.queried_at.is_some_and(|at| {
                now.checked_mul(1000)
                    .and_then(|now| now.checked_sub(at.0))
                    .is_some_and(|age| (0..60_000).contains(&age))
            });
            match result.outcome {
                UsageQueryOutcome::Success { data } if fresh => {
                    match data {
                        UsageQueryData::OfficialSubscription(data) => {
                            windows.extend(data.windows.iter().map(|w| quota_window(w, now)))
                        }
                        UsageQueryData::CodingPlan(data) => {
                            windows.extend(data.iter().map(|w| quota_window(w, now)))
                        }
                        // Currency balances and scripts are not token quota evidence.
                        _ => {}
                    }
                    combine(windows.iter().map(|w| w.state))
                }
                UsageQueryOutcome::Failure { .. } => LimitState::Failed,
                _ => LimitState::Unknown,
            }
        }
        Some(UsageCacheResult::Stale { .. }) | Some(UsageCacheResult::Current(_)) => {
            LimitState::Failed
        }
        None => LimitState::Unknown,
    };
    let scope = serde_json::to_string(&(&key.project, &key.host))
        .map_err(|_| UsageStoreError::InvalidInput)?;
    let app = serde_json::to_value(key.app).map_err(|_| UsageStoreError::InvalidInput)?;
    let app = app.as_str().ok_or(UsageStoreError::InvalidInput)?;
    let mut statement = connection
        .prepare("SELECT payload_json FROM usage_records WHERE scope_key=?1")
        .map_err(|_| UsageStoreError::StorageUnavailable)?;
    let rows = statement
        .query_map(params![scope], |row| row.get::<_, String>(0))
        .map_err(|_| UsageStoreError::StorageUnavailable)?;
    let mut records = Vec::new();
    for row in rows {
        let record: StoredUsageRecord =
            serde_json::from_str(&row.map_err(|_| UsageStoreError::StorageUnavailable)?)
                .map_err(|_| UsageStoreError::StorageUnavailable)?;
        if matches!(&record.record.provider, UsageProvider::Known { provider_id } if provider_id == &key.provider)
            && record.record.sources.iter().any(|s| s.app == app)
            && record.record.billable
        {
            records.push(record.record);
        }
    }
    let archived = super::retention::load_daily_rollups(connection, &key.project, &key.host)
        .map_err(|_| UsageStoreError::StorageUnavailable)?;
    for period in periods {
        let mut usd = Some(Decimal::ZERO);
        let mut tokens = Some(0_u64);
        for record in records.iter().filter(|r| {
            r.started_at_unix_seconds.0 >= period.start_unix_seconds
                && r.started_at_unix_seconds.0 <= now
        }) {
            let cost = match record.cost.source {
                UsageCostSource::Native | UsageCostSource::Calculated => record
                    .cost
                    .amount_usd
                    .as_deref()
                    .and_then(|s| Decimal::from_str(s).ok())
                    .filter(|v| *v >= Decimal::ZERO),
                _ => None,
            };
            usd = usd.zip(cost).and_then(|(sum, v)| sum.checked_add(v));
            tokens = tokens
                .zip(record.tokens.as_ref().and_then(total_tokens))
                .and_then(|(sum, v)| sum.checked_add(v));
        }
        for rollup in archived.iter().filter(|r| {
            r.dimensions.billable
                && matches!(&r.dimensions.provider, UsageProvider::Known { provider_id } if provider_id == &key.provider)
                && r.dimensions.sources.iter().any(|source| source.app == app)
        }) {
            let day = rollup.dimensions.day_unix_seconds;
            let end = day.checked_add(86400).ok_or(UsageStoreError::StorageUnavailable)?;
            if end <= period.start_unix_seconds || day > now { continue; }
            // Daily archives cannot resolve a partially overlapping quota window.
            if day < period.start_unix_seconds || end.saturating_sub(1) > now {
                usd = None;
                tokens = None;
                continue;
            }
            let cost = if rollup.unknown_cost_requests == 0 && matches!(
                rollup.dimensions.cost_source, UsageCostSource::Native | UsageCostSource::Calculated
            ) {
                Decimal::from_str(&rollup.known_cost_usd).ok()
            } else { None };
            usd = usd.zip(cost).and_then(|(sum, value)| sum.checked_add(value));
            tokens = tokens.zip(rollup.tokens.as_ref().and_then(total_tokens))
                .and_then(|(sum, value)| sum.checked_add(value));
        }
        for (unit, used, limit) in [
            ("USD", usd, period.usd),
            ("tokens", tokens.map(Decimal::from), period.tokens.map(Decimal::from)),
        ] {
            if let Some(limit) = limit {
                windows.push(LimitWindow {
                    name: period.name.clone(),
                    unit: unit.into(),
                    used: used.map(|v| v.to_string()),
                    limit: Some(limit.to_string()),
                    resets_at: chrono::DateTime::from_timestamp(period.reset_unix_seconds, 0)
                        .map(|v| v.to_rfc3339()),
                    state: match used {
                        Some(used) if used >= limit => LimitState::Exhausted,
                        Some(_) => LimitState::Available,
                        None => LimitState::Unknown,
                    },
                });
            }
        }
    }
    Ok(ProviderLimitStatus {
        provider_id: key.provider.clone(),
        state: combine(windows.iter().map(|w| w.state).chain(std::iter::once(quota_state))),
        windows,
    })
}
