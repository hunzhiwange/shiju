//! Deferred behavioral checks; this task does not execute tests.
use super::{axis, thresholds};
use crate::center::hero::Metric;
use gateway_usage_read::UsageSummary;
use vw_api_usage::{UsageInputSemantics, UsageReasoningSemantics, UsageRequestCounts, UsageTokens};

fn summary() -> UsageSummary {
    UsageSummary {
        counts: UsageRequestCounts { total_requests: 12, billable_requests: 10, unknown_cost_requests: 2 },
        succeeded_requests: 10, failed_requests: 2, cancelled_requests: 0, unknown_status_requests: 0,
        known_cost_usd: "2.5".into(), requests_without_tokens: 0, first_usage_date: None,
        token_groups: vec![UsageTokens {
            input: Some(1000), output: Some(200), cache_read: Some(200), cache_creation: Some(100),
            reasoning: Some(20), cache_creation_1h: Some(50),
            input_semantics: UsageInputSemantics::Total,
            reasoning_semantics: UsageReasoningSemantics::IncludedInOutput,
        }],
    }
}
#[test]
fn usage_replica_03_switch_metric_same_summary() {
    let s = summary();
    assert_eq!(Metric::Cost.value(&s), Some(2.5));
    assert_eq!(Metric::Requests.value(&s), Some(12.0));
    assert_eq!(Metric::Tokens.value(&s), Some(1200.0));
    assert_eq!(Metric::HitRate.value(&s), Some(0.2));
}
#[test]
fn usage_replica_03_missing_zero_and_unknown_cost() {
    let mut s = summary();
    s.requests_without_tokens = 1;
    assert_eq!(Metric::Tokens.value(&s), None);
    assert_eq!(Metric::HitRate.value(&s), None);
    assert_eq!(Metric::Cost.value(&s), Some(2.5)); // known subtotal remains visible
    s.counts.total_requests = 0;
    assert_eq!(Metric::Requests.value(&s), Some(0.0));
    assert_eq!(Metric::HitRate.value(&s), None);
}
#[test]
fn usage_replica_03_axis_extremes_and_quartiles() {
    assert_eq!(axis(&[], Metric::Tokens), (0.0,1.0));
    assert_eq!(axis(&[None], Metric::HitRate), (0.0,1.0));
    assert_eq!(axis(&[Some(0.98),Some(0.99)], Metric::HitRate), (0.95,1.0));
    assert_eq!(axis(&[Some(u64::MAX as f64)], Metric::Tokens), (0.0,u64::MAX as f64));
    assert_eq!(thresholds(&[Some(0.0),Some(1.0),Some(2.0),Some(3.0),Some(1e9)],Metric::Tokens),[2.0,3.0,1e9]);
    assert_eq!(thresholds(&[Some(0.0)],Metric::HitRate),[0.0;3]);
}
