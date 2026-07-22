//! 性能指标捕获文件的安装、检查点与落盘。

use std::collections::HashMap;
use std::fs::{OpenOptions, create_dir_all};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex, MutexGuard};

use serde::Serialize;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::get_perf_metrics_snapshot;
use crate::reset_perf_metrics;
use crate::types::PerfMetricsSnapshot;

pub const PERF_METRICS_FILE_ENV: &str = "VWACP_PERF_METRICS_FILE";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CaptureReason {
    Checkpoint,
    Exit,
    Signal,
}

#[derive(Debug, Clone, Default)]
pub struct PerfMetricsCaptureOptions {
    pub argv: Option<Vec<String>>,
    pub role: Option<String>,
    pub file_path: Option<PathBuf>,
}

#[derive(Debug, Clone)]
struct PerfMetricsCaptureState {
    installed: bool,
    flushed: bool,
    capture_file_path: Option<PathBuf>,
    capture_role: String,
    capture_argv: Vec<String>,
    capture_sequence: u64,
}

impl Default for PerfMetricsCaptureState {
    fn default() -> Self {
        Self {
            installed: false,
            flushed: false,
            capture_file_path: None,
            capture_role: "cli".to_string(),
            capture_argv: Vec::new(),
            capture_sequence: 0,
        }
    }
}

#[derive(Debug, Serialize)]
struct PerfMetricsCapturePayload {
    timestamp: String,
    pid: u32,
    ppid: u32,
    role: String,
    argv: Vec<String>,
    cwd: String,
    sequence: u64,
    reason: CaptureReason,
    metrics: PerfMetricsSnapshot,
}

static PERF_METRICS_CAPTURE_STATE: LazyLock<Mutex<PerfMetricsCaptureState>> =
    LazyLock::new(|| Mutex::new(PerfMetricsCaptureState::default()));

fn lock_state() -> MutexGuard<'static, PerfMetricsCaptureState> {
    PERF_METRICS_CAPTURE_STATE.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn current_timestamp() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

#[cfg(unix)]
fn current_parent_pid() -> u32 {
    unsafe { libc::getppid() as u32 }
}

#[cfg(not(unix))]
fn current_parent_pid() -> u32 {
    0
}

fn current_working_directory() -> String {
    std::env::current_dir().map(|path| path.display().to_string()).unwrap_or_default()
}

fn should_capture(state: &PerfMetricsCaptureState) -> bool {
    state.capture_file_path.as_ref().is_some_and(|path| !path.as_os_str().is_empty())
}

fn build_payload(
    state: &PerfMetricsCaptureState,
    reason: CaptureReason,
) -> PerfMetricsCapturePayload {
    PerfMetricsCapturePayload {
        timestamp: current_timestamp(),
        pid: std::process::id(),
        ppid: current_parent_pid(),
        role: state.capture_role.clone(),
        argv: state.capture_argv.clone(),
        cwd: current_working_directory(),
        sequence: state.capture_sequence,
        reason,
        metrics: get_perf_metrics_snapshot(),
    }
}

fn snapshot_has_data(snapshot: &PerfMetricsSnapshot) -> bool {
    !snapshot.counters.is_empty() || !snapshot.gauges.is_empty() || !snapshot.timings.is_empty()
}

fn write_perf_metrics_capture(reason: CaptureReason, reset_after_write: bool) -> bool {
    let (capture_file_path, payload) = {
        let state = lock_state();
        let Some(capture_file_path) =
            state.capture_file_path.clone().filter(|path| !path.as_os_str().is_empty())
        else {
            return false;
        };

        let payload = build_payload(&state, reason);
        if !snapshot_has_data(&payload.metrics) {
            return false;
        }

        (capture_file_path, payload)
    };

    let Some(parent_dir) = capture_file_path.parent() else {
        return false;
    };

    if create_dir_all(parent_dir).is_err() {
        return false;
    }

    let mut file = match OpenOptions::new().create(true).append(true).open(&capture_file_path) {
        Ok(file) => file,
        Err(_) => return false,
    };

    let line = match serde_json::to_string(&payload) {
        Ok(line) => line,
        Err(_) => return false,
    };

    if writeln!(file, "{line}").is_err() {
        return false;
    }

    {
        let mut state = lock_state();
        state.capture_sequence += 1;
    }

    if reset_after_write {
        reset_perf_metrics();
    }

    true
}

pub fn checkpoint_perf_metrics_capture() {
    {
        let mut state = lock_state();
        state.flushed = false;
    }
    let _ = write_perf_metrics_capture(CaptureReason::Checkpoint, true);
}

pub fn flush_perf_metrics_capture(reason: CaptureReason) {
    {
        let mut state = lock_state();
        if state.flushed || !should_capture(&state) {
            return;
        }
        state.flushed = true;
    }

    let _ = write_perf_metrics_capture(reason, false);
}

#[cfg(unix)]
extern "C" fn flush_perf_metrics_capture_at_exit() {
    flush_perf_metrics_capture(CaptureReason::Exit);
}

pub fn install_perf_metrics_capture(options: PerfMetricsCaptureOptions) {
    let capture_file_path = options.file_path.or_else(current_perf_metrics_capture_file_from_env);

    {
        let mut state = lock_state();
        state.capture_file_path = capture_file_path;
        if !should_capture(&state) {
            return;
        }

        reset_perf_metrics();
        state.capture_role = options.role.unwrap_or_else(|| state.capture_role.clone());
        state.capture_argv = options.argv.unwrap_or_default();
        state.capture_sequence = 0;
        state.flushed = false;

        if state.installed {
            return;
        }
        state.installed = true;
    }

    #[cfg(unix)]
    unsafe {
        libc::atexit(flush_perf_metrics_capture_at_exit);
    }
}

pub fn perf_metrics_capture_file_from_env(env: &HashMap<String, String>) -> Option<PathBuf> {
    let value = env.get(PERF_METRICS_FILE_ENV)?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(PathBuf::from(trimmed))
}

pub fn current_perf_metrics_capture_file_from_env() -> Option<PathBuf> {
    let value = std::env::var(PERF_METRICS_FILE_ENV).ok()?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(PathBuf::from(trimmed))
}

#[cfg(test)]
#[path = "perf_metrics_capture_tests.rs"]
mod perf_metrics_capture_tests;
