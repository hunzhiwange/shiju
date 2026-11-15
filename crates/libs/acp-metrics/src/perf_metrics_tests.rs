//! 验证性能计数器和计时器的累计、舍入、复位与锁恢复。
//!
//! 串行锁隔离全局计数器状态，并显式验证锁中毒后的恢复路径。

use super::*;

use std::sync::MutexGuard;

fn lock_test_metrics() -> MutexGuard<'static, ()> {
    TEST_PERF_METRICS_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

// 下列行为用例覆盖累计、舍入、负增量、复位和计时器单次记录。
#[test]
fn perf_metrics_snapshot_rounds_values_and_accumulates() {
    let _guard = lock_test_metrics();
    reset_perf_metrics();

    increment_perf_counter("requests", 2);
    increment_perf_counter("requests", 3);
    set_perf_gauge("depth", 1.23456);
    record_perf_duration("run", 1.1111);
    record_perf_duration("run", 2.2222);

    let snapshot = get_perf_metrics_snapshot();
    assert_eq!(snapshot.counters["requests"], 5);
    assert_eq!(snapshot.gauges["depth"], 1.235);
    assert_eq!(snapshot.timings["run"].count, 2);
    assert_eq!(snapshot.timings["run"].total_ms, 3.333);
    assert_eq!(snapshot.timings["run"].max_ms, 2.222);
}

#[test]
fn reset_perf_metrics_clears_all_metric_groups() {
    let _guard = lock_test_metrics();
    reset_perf_metrics();

    increment_perf_counter("requests", 1);
    set_perf_gauge("depth", 1.0);
    record_perf_duration("run", 1.0);
    reset_perf_metrics();

    let snapshot = get_perf_metrics_snapshot();
    assert!(snapshot.counters.is_empty());
    assert!(snapshot.gauges.is_empty());
    assert!(snapshot.timings.is_empty());
}

#[test]
fn increment_perf_counter_allows_negative_delta() {
    let _guard = lock_test_metrics();
    reset_perf_metrics();

    increment_perf_counter("requests", 2);
    increment_perf_counter("requests", -5);

    let snapshot = get_perf_metrics_snapshot();
    assert_eq!(snapshot.counters["requests"], -3);
}

#[test]
fn lock_recovers_poisoned_mutex_guard() {
    let mutex = Mutex::new(41);
    let result = std::panic::catch_unwind(|| {
        let _guard = mutex.lock().expect("mutex should start unpoisoned");
        panic!("poison test mutex");
    });

    assert!(result.is_err());
    assert_eq!(*lock(&mutex), 41);
}

#[test]
fn format_perf_metric_uses_same_rounding_rule() {
    assert_eq!(format_perf_metric("load", 1.23456), "load=1.235ms");
}

#[test]
fn measure_perf_returns_result_and_records_timing() {
    let _guard = lock_test_metrics();
    reset_perf_metrics();

    let runtime =
        tokio::runtime::Builder::new_current_thread().build().expect("test runtime should build");
    let result = runtime.block_on(measure_perf("async-run", || async { "done" }));

    let snapshot = get_perf_metrics_snapshot();
    let timing = &snapshot.timings["async-run"];
    assert_eq!(result, "done");
    assert_eq!(timing.count, 1);
    assert!(timing.total_ms >= 0.0);
    assert_eq!(timing.max_ms, timing.total_ms);
}

#[test]
fn start_perf_timer_records_elapsed_duration_once() {
    let _guard = lock_test_metrics();
    reset_perf_metrics();

    let stop_timer = start_perf_timer("manual-run");
    let elapsed_ms = stop_timer();

    let snapshot = get_perf_metrics_snapshot();
    let timing = &snapshot.timings["manual-run"];
    assert!(elapsed_ms >= 0.0);
    assert_eq!(timing.count, 1);
    assert_eq!(timing.total_ms, timing.max_ms);
}
