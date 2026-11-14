//! Provider/model statistics over the canonical ledger and retained rollups.
//! Performance is a per-request arithmetic mean, split by evidence kind. Pruned
//! records have no retained performance samples and never enter those denominators.
use std::{collections::BTreeMap, str::FromStr};

use rusqlite::Connection;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use vw_api_types::{id::ProjectScope, usage::*};

use super::{retention::load_daily_rollups, store::StoredUsageRecord};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MetricSamples {
    pub samples: u64,
    pub mean: Option<f64>,
}

impl MetricSamples {
    fn add(&mut self, value: f64) -> Result<(), String> {
        if !value.is_finite() || value < 0.0 {
            return Err("invalid usage performance measurement".into());
        }
        self.samples = add(self.samples, 1)?;
        let previous = self.mean.unwrap_or(0.0);
        self.mean = Some(previous + (value - previous) / self.samples as f64);
        Ok(())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PerformanceStats {
    pub first_token_ms: MetricSamples,
    pub latency_ms: MetricSamples,
    pub output_tokens_per_second: MetricSamples,
}

/// Token buckets preserve missing fields and fresh/total and reasoning semantics.
/// Cache creation 1h is a subset, never added again to cache creation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenStats {
    pub requests: u64,
    pub tokens: UsageTokens,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostStats {
    pub source: UsageCostSource,
    pub requests: u64,
    pub amount_usd: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupStats {
    pub counts: UsageRequestCounts,
    pub succeeded: u64,
    pub failed: u64,
    pub cancelled: u64,
    pub unknown_status: u64,
    /// Missing-status requests are excluded from the success-rate denominator.
    pub success_rate: Option<f64>,
    pub tokens: Vec<TokenStats>,
    /// Unknown costs are counted in counts, never represented as a zero amount.
    pub costs: Vec<CostStats>,
    pub measured: PerformanceStats,
    pub estimated: PerformanceStats,
}

impl Default for GroupStats {
    fn default() -> Self {
        Self {
            counts: UsageRequestCounts {
                total_requests: 0,
                billable_requests: 0,
                unknown_cost_requests: 0,
            },
            succeeded: 0,
            failed: 0,
            cancelled: 0,
            unknown_status: 0,
            success_rate: None,
            tokens: Vec::new(),
            costs: Vec::new(),
            measured: PerformanceStats::default(),
            estimated: PerformanceStats::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStats {
    pub provider: UsageProvider,
    pub stats: GroupStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelStats {
    /// Pricing model, falling back to response then request model; None is unknown.
    pub model: Option<String>,
    pub stats: GroupStats,
}

pub fn get_provider_stats(
    connection: &Connection,
    scope: &ProjectScope,
    host: &str,
    range: &UsageResolvedRange,
    filter: &UsageFilter,
) -> Result<Vec<ProviderStats>, String> {
    let rows = grouped(connection, scope, host, range, filter, false)?;
    rows.into_iter()
        .map(|(key, stats)| {
            Ok(ProviderStats {
                provider: serde_json::from_str(&key).map_err(|_| "invalid provider group")?,
                stats,
            })
        })
        .collect()
}

pub fn get_model_stats(
    connection: &Connection,
    scope: &ProjectScope,
    host: &str,
    range: &UsageResolvedRange,
    filter: &UsageFilter,
) -> Result<Vec<ModelStats>, String> {
    let rows = grouped(connection, scope, host, range, filter, true)?;
    rows.into_iter()
        .map(|(key, stats)| {
            Ok(ModelStats {
                model: serde_json::from_str(&key).map_err(|_| "invalid model group")?,
                stats,
            })
        })
        .collect()
}

fn add(a: u64, b: u64) -> Result<u64, String> {
    a.checked_add(b).ok_or_else(|| "usage count overflow".into())
}

fn matches(record: &UsageRecord, range: &UsageResolvedRange, f: &UsageFilter) -> bool {
    let t = record.started_at_unix_seconds.0;
    t < range.end_unix_seconds.0
        && range.start_unix_seconds.is_none_or(|start| t >= start.0)
        && f.provider.as_ref().is_none_or(|v| v == &record.provider)
        && f.request_model.as_ref().is_none_or(|v| Some(v) == record.models.request_model.as_ref())
        && f.response_model
            .as_ref()
            .is_none_or(|v| Some(v) == record.models.response_model.as_ref())
        && f.pricing_model.as_ref().is_none_or(|v| Some(v) == record.models.pricing_model.as_ref())
        && f.status.is_none_or(|v| v == record.status)
        && f.billable.is_none_or(|v| v == record.billable)
        && ((f.app.is_none() && f.source_kind.is_none() && f.native_session_id.is_none())
            || record.sources.iter().any(|source| {
                f.app.as_ref().is_none_or(|v| v == &source.app)
                    && f.source_kind.is_none_or(|v| v == source.kind)
                    && f.native_session_id
                        .as_ref()
                        .is_none_or(|v| Some(v) == source.native_session_id.as_ref())
            }))
}

fn grouped(
    connection: &Connection,
    scope: &ProjectScope,
    host: &str,
    range: &UsageResolvedRange,
    filter: &UsageFilter,
    models: bool,
) -> Result<BTreeMap<String, GroupStats>, String> {
    if host.trim().is_empty()
        || host.contains('\0')
        || range.start_unix_seconds.is_some_and(|s| s.0 > range.end_unix_seconds.0)
    {
        return Err("invalid usage group scope or range".into());
    }
    // Snapshot prevents a concurrent prune from dropping or duplicating requests.
    let snapshot = connection.unchecked_transaction().map_err(|_| "usage snapshot unavailable")?;
    let key = serde_json::to_string(&(scope, host)).map_err(|_| "invalid usage scope")?;
    let mut statement = snapshot
        .prepare("SELECT payload_json FROM usage_records WHERE scope_key=?1")
        .map_err(|_| "usage storage unavailable")?;
    let rows = statement
        .query_map([key], |row| row.get::<_, String>(0))
        .map_err(|_| "usage storage unavailable")?;
    let mut groups = BTreeMap::new();
    for row in rows {
        let stored: StoredUsageRecord =
            serde_json::from_str(&row.map_err(|_| "usage storage unavailable")?)
                .map_err(|_| "invalid stored usage record")?;
        accumulate(&mut groups, &stored.record, 1, None, range, filter, models)?;
    }
    for rollup in
        load_daily_rollups(&snapshot, scope, host).map_err(|_| "usage rollups unavailable")?
    {
        let d = rollup.dimensions;
        // Legacy daily buckets cannot satisfy exact filters safely.
        let timestamp =
            d.started_at_unix_seconds.ok_or("legacy rollup lacks exact request time")?;
        let record = UsageRecord {
            id: String::new(),
            started_at_unix_seconds: UsageUnixSeconds(timestamp),
            provider: d.provider,
            sources: d.sources,
            models: d.models,
            status: d.status,
            billable: d.billable,
            tokens: rollup.tokens,
            cost: UsageCost {
                source: d.cost_source,
                amount_usd: (d.cost_source != UsageCostSource::Unknown)
                    .then_some(rollup.known_cost_usd),
            },
            performance: UsagePerformance {
                first_token_ms: None,
                latency_ms: None,
                output_tokens_per_second: None,
            },
            suspected_duplicate_of: None,
        };
        accumulate(
            &mut groups,
            &record,
            rollup.request_count,
            Some(rollup.unknown_cost_requests),
            range,
            filter,
            models,
        )?;
    }
    Ok(groups)
}

#[allow(clippy::too_many_arguments)]
fn accumulate(
    groups: &mut BTreeMap<String, GroupStats>,
    r: &UsageRecord,
    count: u64,
    unknown_costs: Option<u64>,
    range: &UsageResolvedRange,
    filter: &UsageFilter,
    models: bool,
) -> Result<(), String> {
    if !matches(r, range, filter) {
        return Ok(());
    }
    let key = if models {
        let model = [&r.models.pricing_model, &r.models.response_model, &r.models.request_model]
            .into_iter()
            .find_map(|v| v.as_ref().filter(|s| !s.is_empty()));
        serde_json::to_string(&model)
    } else {
        serde_json::to_string(&r.provider)
    }
    .map_err(|_| "invalid usage group")?;
    let stats = groups.entry(key).or_default();
    stats.counts.total_requests = add(stats.counts.total_requests, count)?;
    if r.billable {
        stats.counts.billable_requests = add(stats.counts.billable_requests, count)?;
    }
    let status = match r.status {
        UsageRequestStatus::Succeeded => &mut stats.succeeded,
        UsageRequestStatus::Failed => &mut stats.failed,
        UsageRequestStatus::Cancelled => &mut stats.cancelled,
        UsageRequestStatus::Unknown => &mut stats.unknown_status,
    };
    *status = add(*status, count)?;
    let known = stats.counts.total_requests - stats.unknown_status;
    stats.success_rate = (known > 0).then(|| stats.succeeded as f64 / known as f64);
    let unknown =
        unknown_costs.unwrap_or(if r.cost.source == UsageCostSource::Unknown { count } else { 0 });
    if unknown > count {
        return Err("invalid unknown cost count".into());
    }
    stats.counts.unknown_cost_requests = add(stats.counts.unknown_cost_requests, unknown)?;
    if r.cost.source != UsageCostSource::Unknown {
        let amount = Decimal::from_str(r.cost.amount_usd.as_deref().ok_or("missing usage cost")?)
            .map_err(|_| "invalid usage cost")?;
        if amount < Decimal::ZERO {
            return Err("negative usage cost".into());
        }
        let index =
            stats.costs.iter().position(|v| v.source == r.cost.source).unwrap_or_else(|| {
                stats.costs.push(CostStats {
                    source: r.cost.source,
                    requests: 0,
                    amount_usd: "0".into(),
                });
                stats.costs.len() - 1
            });
        let cost = &mut stats.costs[index];
        cost.requests = add(cost.requests, count - unknown)?;
        cost.amount_usd = Decimal::from_str(&cost.amount_usd)
            .map_err(|_| "invalid accumulated cost")?
            .checked_add(amount)
            .ok_or("usage cost overflow")?
            .normalize()
            .to_string();
    } else if r.cost.amount_usd.is_some() {
        return Err("unknown cost has an amount".into());
    }
    if let Some(tokens) = &r.tokens {
        let same_shape = |v: &TokenStats| {
            let a = &v.tokens;
            a.input_semantics == tokens.input_semantics
                && a.reasoning_semantics == tokens.reasoning_semantics
                && [
                    a.input,
                    a.output,
                    a.reasoning,
                    a.cache_read,
                    a.cache_creation,
                    a.cache_creation_1h,
                ]
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
        if let Some(bucket) = stats.tokens.iter_mut().find(|v| same_shape(v)) {
            bucket.requests = add(bucket.requests, count)?;
            for (sum, value) in [
                (&mut bucket.tokens.input, tokens.input),
                (&mut bucket.tokens.output, tokens.output),
                (&mut bucket.tokens.reasoning, tokens.reasoning),
                (&mut bucket.tokens.cache_read, tokens.cache_read),
                (&mut bucket.tokens.cache_creation, tokens.cache_creation),
                (&mut bucket.tokens.cache_creation_1h, tokens.cache_creation_1h),
            ] {
                if let (Some(a), Some(b)) = (*sum, value) {
                    *sum = Some(add(a, b)?);
                }
            }
        } else {
            stats.tokens.push(TokenStats { requests: count, tokens: tokens.clone() });
        }
    }
    for (index, measurement) in [
        &r.performance.first_token_ms,
        &r.performance.latency_ms,
        &r.performance.output_tokens_per_second,
    ]
    .into_iter()
    .enumerate()
    {
        if let Some(m) = measurement {
            let performance = match m.kind {
                UsageMeasurementKind::Measured => &mut stats.measured,
                UsageMeasurementKind::Estimated => &mut stats.estimated,
            };
            match index {
                0 => &mut performance.first_token_ms,
                1 => &mut performance.latency_ms,
                _ => &mut performance.output_tokens_per_second,
            }
            .add(m.value)?;
        }
    }
    Ok(())
}
