//! 进程内性能指标的收集、计时与快照查询。

use std::collections::HashMap;
use std::future::Future;
use std::sync::{LazyLock, Mutex, MutexGuard};
use std::time::Instant;

use crate::types::{PerfMetricSummary, PerfMetricsSnapshot};

#[derive(Debug, Clone, Default)]
struct TimingBucket {
    count: i64,
    total_ms: f64,
    max_ms: f64,
}

static COUNTERS: LazyLock<Mutex<HashMap<String, i64>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static GAUGES: LazyLock<Mutex<HashMap<String, f64>>> = LazyLock::new(|| Mutex::new(HashMap::new()));
static TIMINGS: LazyLock<Mutex<HashMap<String, TimingBucket>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[cfg(test)]
pub(crate) static TEST_PERF_METRICS_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn duration_ms(started_at: Instant) -> f64 {
    started_at.elapsed().as_secs_f64() * 1_000.0
}

fn round_metric(value: f64) -> f64 {
    (value * 1_000.0).round() / 1_000.0
}

pub fn increment_perf_counter(name: &str, delta: i64) {
    let mut counters = lock(&COUNTERS);
    let next_value = counters.get(name).copied().unwrap_or_default() + delta;
    counters.insert(name.to_string(), next_value);
}

pub fn set_perf_gauge(name: &str, value: f64) {
    lock(&GAUGES).insert(name.to_string(), value);
}

pub fn record_perf_duration(name: &str, duration_ms_value: f64) {
    let mut timings = lock(&TIMINGS);
    let bucket = timings.entry(name.to_string()).or_default();
    bucket.count += 1;
    bucket.total_ms += duration_ms_value;
    bucket.max_ms = bucket.max_ms.max(duration_ms_value);
}

pub async fn measure_perf<T, F, Fut>(name: &str, run: F) -> T
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = T>,
{
    let started_at = Instant::now();
    let result = run().await;
    record_perf_duration(name, duration_ms(started_at));
    result
}

pub fn start_perf_timer(name: impl Into<String>) -> impl FnOnce() -> f64 {
    let started_at = Instant::now();
    let name = name.into();
    move || {
        let elapsed_ms = duration_ms(started_at);
        record_perf_duration(&name, elapsed_ms);
        elapsed_ms
    }
}

pub fn get_perf_metrics_snapshot() -> PerfMetricsSnapshot {
    let counters = lock(&COUNTERS).clone();
    let gauges =
        lock(&GAUGES).iter().map(|(name, value)| (name.clone(), round_metric(*value))).collect();
    let timings = lock(&TIMINGS)
        .iter()
        .map(|(name, bucket)| {
            (
                name.clone(),
                PerfMetricSummary {
                    count: bucket.count,
                    total_ms: round_metric(bucket.total_ms),
                    max_ms: round_metric(bucket.max_ms),
                },
            )
        })
        .collect();

    PerfMetricsSnapshot { counters, gauges, timings }
}

pub fn reset_perf_metrics() {
    lock(&COUNTERS).clear();
    lock(&GAUGES).clear();
    lock(&TIMINGS).clear();
}

pub fn format_perf_metric(name: &str, duration_ms_value: f64) -> String {
    format!("{name}={}ms", round_metric(duration_ms_value))
}

#[cfg(test)]
#[path = "perf_metrics_tests.rs"]
mod perf_metrics_tests;
