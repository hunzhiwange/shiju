//! ACP 进程内性能指标与捕获落盘。

pub mod perf_metrics;
pub mod perf_metrics_capture;

pub use perf_metrics::{
    format_perf_metric, get_perf_metrics_snapshot, increment_perf_counter, measure_perf,
    record_perf_duration, reset_perf_metrics, set_perf_gauge, start_perf_timer,
};
pub use perf_metrics_capture::{
    CaptureReason, PERF_METRICS_FILE_ENV, PerfMetricsCaptureOptions,
    checkpoint_perf_metrics_capture, current_perf_metrics_capture_file_from_env,
    flush_perf_metrics_capture, install_perf_metrics_capture, perf_metrics_capture_file_from_env,
};
