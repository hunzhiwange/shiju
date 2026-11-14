//! Wire responses for the gateway usage ledger read API.
use serde::{Deserialize, Serialize};
use vw_api_types::usage::*;

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
