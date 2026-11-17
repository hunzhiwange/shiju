//! Shared replica display contract. Scope/query freshness remains owned by center.
use gateway_usage_read::{GroupStats, PerformanceStats, UsageSummary, UsageTokenTotals};
use vw_api_usage::UsageMeasurementKind;

#[derive(Clone, Debug)]
pub struct SummaryMetrics {
    pub requests: u64,
    /// Known subtotal is kept even if some requests have unknown prices.
    pub known_cost_usd: String,
    pub unknown_cost_requests: u64,
    pub tokens: UsageTokenTotals,
    pub success_rate: f64,
}

impl From<&UsageSummary> for SummaryMetrics {
    fn from(summary: &UsageSummary) -> Self {
        Self {
            requests: summary.counts.total_requests,
            known_cost_usd: summary.known_cost_usd.clone(),
            unknown_cost_requests: summary.counts.unknown_cost_requests,
            tokens: summary.token_totals(),
            success_rate: summary.replica_success_rate(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PerformanceValue {
    pub value: f64,
    pub samples: u64,
    pub kind: UsageMeasurementKind,
}

fn preferred(
    measured: &PerformanceStats,
    estimated: &PerformanceStats,
    get: impl Fn(&PerformanceStats) -> Option<(f64, u64)>,
) -> Option<PerformanceValue> {
    [(measured, UsageMeasurementKind::Measured), (estimated, UsageMeasurementKind::Estimated)]
        .into_iter().find_map(|(stats, kind)| {
            let (value, samples) = get(stats)?;
            (samples > 0 && value.is_finite() && value >= 0.0)
                .then_some(PerformanceValue { value, samples, kind })
        })
}

/// Never fall back to the legacy mean: old servers without medians display —.
pub fn first_token(stats: &GroupStats) -> Option<PerformanceValue> {
    preferred(&stats.measured, &stats.estimated, |s| {
        Some((s.first_token_median_ms?, s.first_token_samples))
    })
}

/// Ratio of sums, not the mean of individual request speeds.
pub fn generation_speed(stats: &GroupStats) -> Option<PerformanceValue> {
    preferred(&stats.measured, &stats.estimated, |s| {
        (s.speed_duration_ms.is_finite() && s.speed_duration_ms > 0.0)
            .then(|| (s.speed_output_tokens as f64 * 1000.0 / s.speed_duration_ms, s.speed_samples))
    })
}

pub fn format_performance(value: Option<PerformanceValue>, unit: &str) -> String {
    value.map_or_else(|| "—".into(), |v| format!("{}{:.2} {unit}",
        if v.kind == UsageMeasurementKind::Estimated { "≈" } else { "" }, v.value))
}

pub fn format_hit_rate(rate: Option<f64>) -> String {
    rate.filter(|r| r.is_finite()).map_or_else(|| "—".into(), |rate| {
        let percent = rate.clamp(0.0, 1.0) * 100.0;
        if percent >= 99.95 { format!("{percent:.0}%") } else { format!("{percent:.1}%") }
    })
}
