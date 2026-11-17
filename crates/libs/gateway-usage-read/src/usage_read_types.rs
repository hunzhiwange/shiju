//! Wire responses for the gateway usage ledger read API.
use serde::{Deserialize, Serialize};
use vw_api_usage::*;

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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MetricSamples {
    pub samples: u64,
    pub mean: Option<f64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PerformanceStats {
    /// Replica sample rules: successful relay TTFT median, native estimated TTFT
    /// median, and weighted generation speed. Legacy means remain independent.
    #[serde(default)]
    pub first_token_median_ms: Option<f64>,
    #[serde(default)]
    pub first_token_samples: u64,
    #[serde(default)]
    pub speed_output_tokens: u64,
    #[serde(default)]
    pub speed_duration_ms: f64,
    #[serde(default)]
    pub speed_samples: u64,

    pub first_token_ms: MetricSamples,
    pub latency_ms: MetricSamples,
    pub output_tokens_per_second: MetricSamples,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStats {
    #[serde(default)]
    pub app: Option<String>,
    pub provider: UsageProvider,
    pub stats: GroupStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelStats {
    /// Pricing model, falling back to response then request model; None is unknown.
    pub model: Option<String>,
    pub stats: GroupStats,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UsageSeriesInterval {
    Hour,
    Day,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageSeriesBucket {
    /// Hour labels include the UTC offset to distinguish repeated wall-clock hours.
    pub label: String,
    /// Bounds are clipped to the requested half-open range, including partial days.
    pub start_unix_seconds: UsageUnixSeconds,
    pub end_unix_seconds: UsageUnixSeconds,
    /// Unknown costs remain counted separately from the known USD subtotal.
    pub summary: UsageSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageSeries {
    pub range: UsageResolvedRange,
    pub interval: UsageSeriesInterval,
    pub buckets: Vec<UsageSeriesBucket>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageSessionSummary {
    pub app: String,
    pub native_session_id: String,
    pub summary: UsageSummary,
}

/// Normalized counters. None means unavailable or ambiguous, never a fabricated zero.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UsageTokenTotals {
    pub fresh_input: Option<u64>,
    pub output: Option<u64>,
    pub cache_read: Option<u64>,
    pub cache_creation: Option<u64>,
    pub real_tokens: Option<u64>,
}

impl UsageTokenTotals {
    pub fn from_groups<'a>(groups: impl IntoIterator<Item = &'a UsageTokens>) -> Self {
        let groups: Vec<_> = groups.into_iter().collect();
        if groups.is_empty() { return Self::default(); }
        let sum = |get: fn(&UsageTokens) -> Option<u64>| groups.iter()
            .try_fold(0_u64, |sum, group| sum.checked_add(get(group)?));
        let fresh = |t: &UsageTokens| match t.input_semantics {
            UsageInputSemantics::Fresh => t.input,
            UsageInputSemantics::Total => t.input?
                .checked_sub(t.cache_read?)?.checked_sub(t.cache_creation?),
            UsageInputSemantics::Unknown => None,
        };
        Self {
            fresh_input: sum(fresh),
            output: sum(|t| t.output),
            cache_read: sum(|t| t.cache_read),
            cache_creation: sum(|t| t.cache_creation),
            // Included reasoning and the 1h creation subset must never be counted twice.
            real_tokens: sum(|t| {
                let input = match t.input_semantics {
                    UsageInputSemantics::Fresh => t.input?.checked_add(t.cache_read?)?
                        .checked_add(t.cache_creation?)?,
                    UsageInputSemantics::Total => t.input?,
                    UsageInputSemantics::Unknown => return None,
                };
                let output = match t.reasoning_semantics {
                    UsageReasoningSemantics::IncludedInOutput => t.output?,
                    UsageReasoningSemantics::SeparateFromOutput => t.output?.checked_add(t.reasoning?)?,
                    UsageReasoningSemantics::Unknown if t.reasoning == Some(0) => t.output?,
                    UsageReasoningSemantics::Unknown => return None,
                };
                input.checked_add(output)
            }),
        }
    }

    pub fn cache_hit_rate(&self) -> Option<f64> {
        let denominator = self.fresh_input?.checked_add(self.cache_read?)?
            .checked_add(self.cache_creation?)?;
        Some(if denominator == 0 { 0.0 } else { self.cache_read? as f64 / denominator as f64 })
    }
}

impl UsageSummary {
    pub fn token_totals(&self) -> UsageTokenTotals {
        if self.requests_without_tokens > 0 { return UsageTokenTotals::default(); }
        UsageTokenTotals::from_groups(&self.token_groups)
    }

    /// Replica uses all matched requests as the denominator, including unknown statuses.
    pub fn replica_success_rate(&self) -> f64 {
        if self.counts.total_requests == 0 { 0.0 }
        else { self.succeeded_requests as f64 / self.counts.total_requests as f64 }
    }
}

#[cfg(test)]
#[path = "usage_replica_01_tests.rs"]
mod usage_replica_01_tests;
