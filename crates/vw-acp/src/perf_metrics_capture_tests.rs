//! 验证性能指标捕获配置、快照载荷、检查点写入与状态重置。
//!
//! 串行锁与临时捕获文件隔离全局指标状态，避免用例间相互污染。

use super::*;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

static ENV_LOCK: Mutex<()> = Mutex::new(());

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(label: &str) -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("vw-acp-perf-capture-{label}-{unique}"));
        fs::create_dir_all(&path).expect("create temp dir");
        Self { path }
    }

    fn join(&self, path: impl AsRef<Path>) -> PathBuf {
        self.path.join(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

struct EnvVarGuard {
    _lock: MutexGuard<'static, ()>,
    saved: Option<String>,
}

impl EnvVarGuard {
    fn new() -> Self {
        let lock = ENV_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let saved = std::env::var(PERF_METRICS_FILE_ENV).ok();
        unsafe { std::env::remove_var(PERF_METRICS_FILE_ENV) };
        Self { _lock: lock, saved }
    }

    fn set(&self, value: &str) {
        unsafe { std::env::set_var(PERF_METRICS_FILE_ENV, value) };
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        match &self.saved {
            Some(value) => unsafe { std::env::set_var(PERF_METRICS_FILE_ENV, value) },
            None => unsafe { std::env::remove_var(PERF_METRICS_FILE_ENV) },
        }
    }
}

fn lock_test_state() -> MutexGuard<'static, ()> {
    crate::perf_metrics::TEST_PERF_METRICS_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn reset_capture_state(installed: bool) {
    reset_perf_metrics();
    let mut state = lock_state();
    *state = PerfMetricsCaptureState { installed, ..PerfMetricsCaptureState::default() };
}

fn install_state(file_path: PathBuf) {
    let mut state = lock_state();
    *state = PerfMetricsCaptureState {
        installed: true,
        flushed: false,
        capture_file_path: Some(file_path),
        capture_role: "worker".to_string(),
        capture_argv: vec!["acp".to_string(), "run".to_string()],
        capture_sequence: 0,
    };
}

fn read_capture_lines(file_path: &Path) -> Vec<Value> {
    fs::read_to_string(file_path)
        .expect("read capture file")
        .lines()
        .map(|line| serde_json::from_str(line).expect("capture line should be json"))
        .collect()
}

// 下列行为用例覆盖累计、舍入、检查点写入和失败后的复位。
#[test]
fn perf_metrics_capture_file_from_env_trims_and_rejects_empty_values() {
    let mut env = HashMap::new();
    assert_eq!(perf_metrics_capture_file_from_env(&env), None);

    env.insert(PERF_METRICS_FILE_ENV.to_string(), "  /tmp/vwacp-metrics.ndjson  ".to_string());

    assert_eq!(
        perf_metrics_capture_file_from_env(&env).as_deref(),
        Some(std::path::Path::new("/tmp/vwacp-metrics.ndjson"))
    );

    env.insert(PERF_METRICS_FILE_ENV.to_string(), "   ".to_string());
    assert_eq!(perf_metrics_capture_file_from_env(&env), None);
}

#[test]
fn current_perf_metrics_capture_file_from_env_reads_trimmed_process_env() {
    let _guard = EnvVarGuard::new();
    assert_eq!(current_perf_metrics_capture_file_from_env(), None);

    let temp = TempDir::new("env");
    let file_path = temp.join("metrics.ndjson");
    let value = format!("  {}  ", file_path.display());
    _guard.set(&value);

    assert_eq!(current_perf_metrics_capture_file_from_env(), Some(file_path));

    _guard.set("   ");
    assert_eq!(current_perf_metrics_capture_file_from_env(), None);
}

#[test]
fn capture_reason_serializes_to_lowercase_labels() {
    assert_eq!(
        serde_json::to_string(&CaptureReason::Checkpoint).expect("checkpoint should serialize"),
        "\"checkpoint\""
    );
    assert_eq!(
        serde_json::to_string(&CaptureReason::Exit).expect("exit should serialize"),
        "\"exit\""
    );
    assert_eq!(
        serde_json::to_string(&CaptureReason::Signal).expect("signal should serialize"),
        "\"signal\""
    );
}

#[test]
fn capture_state_default_is_not_captureable() {
    let state = PerfMetricsCaptureState::default();

    assert!(!state.installed);
    assert!(!state.flushed);
    assert!(!should_capture(&state));
    assert_eq!(state.capture_role, "cli");

    let state = PerfMetricsCaptureState {
        capture_file_path: Some(PathBuf::new()),
        ..PerfMetricsCaptureState::default()
    };
    assert!(!should_capture(&state));
}

#[test]
fn snapshot_has_data_detects_each_metric_group() {
    let mut snapshot = PerfMetricsSnapshot::default();
    assert!(!snapshot_has_data(&snapshot));

    snapshot.counters.insert("requests".to_string(), 1);
    assert!(snapshot_has_data(&snapshot));
    snapshot.counters.clear();

    snapshot.gauges.insert("queue_depth".to_string(), 1.0);
    assert!(snapshot_has_data(&snapshot));
    snapshot.gauges.clear();

    snapshot.timings.insert(
        "prompt".to_string(),
        crate::types::PerfMetricSummary { count: 1, total_ms: 1.0, max_ms: 1.0 },
    );
    assert!(snapshot_has_data(&snapshot));
}

#[test]
fn build_payload_copies_process_and_metric_context() {
    let _guard = lock_test_state();
    reset_capture_state(true);
    crate::increment_perf_counter("requests", 2);
    crate::set_perf_gauge("queue_depth", 1.25);
    crate::record_perf_duration("prompt", 3.5);

    let state = PerfMetricsCaptureState {
        capture_role: "queue-owner".to_string(),
        capture_argv: vec!["acp".to_string(), "queue-owner".to_string()],
        capture_sequence: 7,
        ..PerfMetricsCaptureState::default()
    };
    let payload = build_payload(&state, CaptureReason::Signal);

    assert!(!payload.timestamp.is_empty());
    assert_eq!(payload.pid, std::process::id());
    #[cfg(unix)]
    assert!(payload.ppid > 0);
    #[cfg(not(unix))]
    assert_eq!(payload.ppid, 0);
    assert_eq!(payload.role, "queue-owner");
    assert_eq!(payload.argv, ["acp", "queue-owner"]);
    assert!(!payload.cwd.is_empty());
    assert_eq!(payload.sequence, 7);
    assert_eq!(payload.reason, CaptureReason::Signal);
    assert_eq!(payload.metrics.counters["requests"], 2);
    assert_eq!(payload.metrics.gauges["queue_depth"], 1.25);
    assert_eq!(payload.metrics.timings["prompt"].count, 1);

    reset_capture_state(true);
}

#[test]
fn write_perf_metrics_capture_appends_payload_and_resets_after_checkpoint() {
    let _guard = lock_test_state();
    let temp = TempDir::new("write");
    let file_path = temp.join("nested/metrics.ndjson");
    reset_capture_state(true);
    install_state(file_path.clone());
    crate::increment_perf_counter("requests", 3);

    assert!(write_perf_metrics_capture(CaptureReason::Checkpoint, true));

    let lines = read_capture_lines(&file_path);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0]["role"], "worker");
    assert_eq!(lines[0]["argv"], serde_json::json!(["acp", "run"]));
    assert_eq!(lines[0]["sequence"], 0);
    assert_eq!(lines[0]["reason"], "checkpoint");
    assert_eq!(lines[0]["metrics"]["counters"]["requests"], 3);
    assert!(get_perf_metrics_snapshot().counters.is_empty());
    assert_eq!(lock_state().capture_sequence, 1);

    reset_capture_state(true);
}

#[test]
fn write_perf_metrics_capture_rejects_uncapturable_empty_and_unwritable_paths() {
    let _guard = lock_test_state();
    let temp = TempDir::new("write-errors");
    reset_capture_state(true);

    assert!(!write_perf_metrics_capture(CaptureReason::Checkpoint, true));

    crate::increment_perf_counter("requests", 1);
    install_state(PathBuf::new());
    assert!(!write_perf_metrics_capture(CaptureReason::Checkpoint, true));

    reset_perf_metrics();
    install_state(temp.join("empty.ndjson"));
    assert!(!write_perf_metrics_capture(CaptureReason::Checkpoint, true));

    crate::increment_perf_counter("requests", 1);
    install_state(PathBuf::from("/"));
    assert!(!write_perf_metrics_capture(CaptureReason::Checkpoint, true));

    crate::increment_perf_counter("requests", 1);
    let file_parent = temp.join("file-parent");
    fs::write(&file_parent, "not a directory").expect("write blocking parent");
    install_state(file_parent.join("metrics.ndjson"));
    assert!(!write_perf_metrics_capture(CaptureReason::Checkpoint, true));

    crate::increment_perf_counter("requests", 1);
    let directory_target = temp.join("directory-target");
    fs::create_dir_all(&directory_target).expect("create directory target");
    install_state(directory_target);
    assert!(!write_perf_metrics_capture(CaptureReason::Checkpoint, true));

    reset_capture_state(true);
}

#[test]
fn checkpoint_clears_flush_guard_and_records_next_sequence() {
    let _guard = lock_test_state();
    let temp = TempDir::new("checkpoint");
    let file_path = temp.join("metrics.ndjson");
    reset_capture_state(true);
    install_state(file_path.clone());

    crate::increment_perf_counter("first", 1);
    flush_perf_metrics_capture(CaptureReason::Signal);
    crate::increment_perf_counter("second", 2);
    flush_perf_metrics_capture(CaptureReason::Exit);
    checkpoint_perf_metrics_capture();

    let lines = read_capture_lines(&file_path);
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0]["reason"], "signal");
    assert_eq!(lines[0]["sequence"], 0);
    assert_eq!(lines[1]["reason"], "checkpoint");
    assert_eq!(lines[1]["sequence"], 1);
    assert_eq!(lines[1]["metrics"]["counters"]["second"], 2);
    assert!(!lock_state().flushed);

    reset_capture_state(true);
}

#[cfg(unix)]
#[test]
fn flush_perf_metrics_capture_at_exit_records_exit_once() {
    let _guard = lock_test_state();
    let temp = TempDir::new("atexit");
    let file_path = temp.join("metrics.ndjson");
    reset_capture_state(true);
    install_state(file_path.clone());

    crate::increment_perf_counter("requests", 1);
    flush_perf_metrics_capture_at_exit();
    crate::increment_perf_counter("requests", 1);
    flush_perf_metrics_capture_at_exit();

    let lines = read_capture_lines(&file_path);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0]["reason"], "exit");
    assert_eq!(lines[0]["metrics"]["counters"]["requests"], 1);

    reset_capture_state(true);
}

#[test]
fn flush_perf_metrics_capture_is_idempotent_and_skips_disabled_capture() {
    let _guard = lock_test_state();
    let temp = TempDir::new("flush");
    let file_path = temp.join("metrics.ndjson");
    reset_capture_state(true);

    crate::increment_perf_counter("ignored", 1);
    flush_perf_metrics_capture(CaptureReason::Signal);
    assert!(!file_path.exists());

    install_state(file_path.clone());
    crate::increment_perf_counter("requests", 1);
    flush_perf_metrics_capture(CaptureReason::Exit);
    crate::increment_perf_counter("requests", 1);
    flush_perf_metrics_capture(CaptureReason::Signal);

    let lines = read_capture_lines(&file_path);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0]["reason"], "exit");
    assert_eq!(lines[0]["metrics"]["counters"]["requests"], 1);

    reset_capture_state(true);
}

#[test]
fn flush_without_metrics_consumes_guard_until_checkpoint() {
    let _guard = lock_test_state();
    let temp = TempDir::new("empty-flush");
    let file_path = temp.join("metrics.ndjson");
    reset_capture_state(true);
    install_state(file_path.clone());

    flush_perf_metrics_capture(CaptureReason::Signal);
    assert!(lock_state().flushed);
    assert!(!file_path.exists());

    crate::increment_perf_counter("after-empty-flush", 1);
    flush_perf_metrics_capture(CaptureReason::Exit);
    assert!(!file_path.exists());

    checkpoint_perf_metrics_capture();
    let lines = read_capture_lines(&file_path);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0]["reason"], "checkpoint");
    assert_eq!(lines[0]["metrics"]["counters"]["after-empty-flush"], 1);

    reset_capture_state(true);
}

#[test]
fn install_perf_metrics_capture_uses_options_env_and_preserves_existing_registration() {
    let _guard = lock_test_state();
    let env_guard = EnvVarGuard::new();
    let temp = TempDir::new("install");
    let option_file = temp.join("options.ndjson");
    reset_capture_state(false);
    crate::increment_perf_counter("stale", 1);

    install_perf_metrics_capture(PerfMetricsCaptureOptions {
        argv: Some(vec!["acp".to_string(), "run".to_string()]),
        role: Some("runner".to_string()),
        file_path: Some(option_file.clone()),
    });

    let snapshot = get_perf_metrics_snapshot();
    let state = lock_state().clone();
    assert!(snapshot.counters.is_empty());
    assert!(state.installed);
    assert!(!state.flushed);
    assert_eq!(state.capture_file_path, Some(option_file));
    assert_eq!(state.capture_role, "runner");
    assert_eq!(state.capture_argv, ["acp", "run"]);
    assert_eq!(state.capture_sequence, 0);
    drop(state);

    let env_file = temp.join("env.ndjson");
    env_guard.set(&env_file.display().to_string());
    install_perf_metrics_capture(PerfMetricsCaptureOptions::default());

    let state = lock_state().clone();
    assert!(state.installed);
    assert_eq!(state.capture_file_path, Some(env_file));
    assert_eq!(state.capture_role, "runner");
    assert!(state.capture_argv.is_empty());

    reset_capture_state(true);
}

#[test]
fn install_perf_metrics_capture_without_file_keeps_metrics_and_disables_capture() {
    let _guard = lock_test_state();
    let _env_guard = EnvVarGuard::new();
    reset_capture_state(false);
    crate::increment_perf_counter("pending", 4);

    install_perf_metrics_capture(PerfMetricsCaptureOptions {
        argv: Some(vec!["ignored".to_string()]),
        role: Some("ignored-role".to_string()),
        file_path: None,
    });

    let state = lock_state().clone();
    assert!(!state.installed);
    assert_eq!(state.capture_file_path, None);
    assert_eq!(state.capture_role, "cli");
    assert!(state.capture_argv.is_empty());
    assert_eq!(get_perf_metrics_snapshot().counters["pending"], 4);

    reset_capture_state(false);
}

#[test]
fn install_perf_metrics_capture_with_empty_path_keeps_metrics_and_disables_capture() {
    let _guard = lock_test_state();
    reset_capture_state(false);
    crate::increment_perf_counter("pending", 2);

    install_perf_metrics_capture(PerfMetricsCaptureOptions {
        argv: Some(vec!["ignored".to_string()]),
        role: Some("ignored-role".to_string()),
        file_path: Some(PathBuf::new()),
    });

    let state = lock_state().clone();
    assert!(!state.installed);
    assert_eq!(state.capture_file_path, Some(PathBuf::new()));
    assert_eq!(state.capture_role, "cli");
    assert!(state.capture_argv.is_empty());
    assert_eq!(get_perf_metrics_snapshot().counters["pending"], 2);

    reset_capture_state(false);
}

#[test]
fn lock_state_recovers_after_mutex_poison() {
    let _guard = lock_test_state();
    reset_capture_state(false);

    let result = std::panic::catch_unwind(|| {
        let _state =
            PERF_METRICS_CAPTURE_STATE.lock().expect("capture mutex should start unlocked");
        panic!("poison capture mutex");
    });

    assert!(result.is_err());
    let state = lock_state();
    assert!(!state.installed);
    assert!(!state.flushed);
}
