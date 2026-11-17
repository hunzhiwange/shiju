use super::*;
use gateway_usage_read::{CostStats, PerformanceStats};
use vw_api_usage::{UsageCostSource, UsageRequestCounts};

fn stats(requests: u64, amount: Option<&str>) -> GroupStats {
    GroupStats {
        counts: UsageRequestCounts {
            total_requests: requests,
            billable_requests: 0,
            unknown_cost_requests: 0,
        },
        succeeded: requests,
        failed: 0,
        cancelled: 0,
        unknown_status: 0,
        success_rate: Some(1.0),
        tokens: vec![],
        costs: amount
            .map(|amount| {
                vec![CostStats {
                    source: UsageCostSource::Unknown,
                    requests,
                    amount_usd: amount.into(),
                }]
            })
            .unwrap_or_default(),
        measured: PerformanceStats::default(),
        estimated: PerformanceStats::default(),
    }
}
#[test]
fn usage_replica_05_numeric_sort_and_unknown_cost() {
    let a = stats(9, Some("9.50"));
    let b = stats(20, Some("100.01"));
    assert!(compare(&a, &b, Sort::default()).is_gt());
    for descending in [true, false] {
        let sort = Sort { column: SortColumn::Cost, descending };
        assert_eq!(compare(&a, &b, sort).is_gt(), descending);
        assert!(compare(&stats(1, None), &a, sort).is_gt());
    }
}
#[test]
fn usage_replica_05_weighted_performance_prefers_measured() {
    let mut s = stats(4, None);
    s.measured.speed_output_tokens = 1000;
    s.measured.speed_duration_ms = 2000.0;
    s.measured.speed_samples = 2;
    s.measured.output_tokens_per_second.mean = Some(999.0);
    s.estimated.speed_output_tokens = 2000;
    s.estimated.speed_duration_ms = 1000.0;
    s.estimated.speed_samples = 1;
    assert_eq!(super::super::metrics::generation_speed(&s).unwrap().value, 500.0);
    s.measured.speed_samples = 0;
    assert!(
        super::super::metrics::format_performance(
            super::super::metrics::generation_speed(&s),
            "tok/s"
        )
        .starts_with('≈')
    );
    s.estimated.speed_duration_ms = f64::NAN;
    assert!(super::super::metrics::generation_speed(&s).is_none());
    s.measured.first_token_median_ms = Some(300.0);
    s.measured.first_token_samples = 3;
    s.estimated.first_token_median_ms = Some(2000.0);
    s.estimated.first_token_samples = 2;
    assert_eq!(super::super::metrics::first_token(&s).unwrap().value, 300.0);
}
#[test]
fn usage_replica_05_sort_toggle_keeps_results() {
    let mut state = State { result: Some(Ok((vec![], vec![]))), ..Default::default() };
    let _ = update(&mut state, None, true, true, 0, Event::Sort(SortColumn::Cost));
    assert!(state.sort.descending);
    let _ = update(&mut state, None, true, true, 0, Event::Sort(SortColumn::Cost));
    assert!(!state.sort.descending);
    assert!(state.result.is_some());
}

#[test]
fn usage_replica_05_numeric_cost_and_request_order_keep_unknown_last() {
    let low = stats(20, Some("9.99"));
    let high = stats(2, Some("100.0"));
    let unknown = stats(30, None);
    assert!(compare(&low, &high, Sort::default()).is_lt());
    let descending = Sort { column: SortColumn::Cost, descending: true };
    assert!(compare(&low, &high, descending).is_gt());
    assert!(compare(&unknown, &high, descending).is_gt());
    let ascending = Sort { descending: false, ..descending };
    assert!(compare(&low, &high, ascending).is_lt());
    assert!(compare(&unknown, &high, ascending).is_gt());
}
