//! 可见任务会话的进程内运行状态协调器。

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::watch;

/// 单次任务会话等待允许的最长时间。
pub(crate) const MAX_TASK_SESSION_WAIT: Duration = Duration::from_secs(60);

/// 可见任务会话的运行状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TaskSessionState {
    Queued,
    Running,
    Succeeded,
    Failed,
}

impl TaskSessionState {
    pub(crate) fn is_terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed)
    }
}

/// 可见任务会话的稳定状态快照。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TaskSessionSnapshot {
    pub(crate) parent_session_id: String,
    pub(crate) task_session_id: String,
    pub(crate) state: TaskSessionState,
    pub(crate) error: Option<String>,
    pub(crate) created_ms: u64,
    pub(crate) updated_ms: u64,
}

/// 等待任一或全部目标进入终态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TaskSessionWaitCondition {
    Any,
    All,
}

/// 一次有界等待的结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TaskSessionWaitOutcome {
    pub(crate) tasks: Vec<TaskSessionSnapshot>,
    pub(crate) timed_out: bool,
}

#[derive(Debug)]
struct TaskEntry {
    snapshot: TaskSessionSnapshot,
    sequence: u64,
}

#[derive(Debug, Default)]
struct CoordinatorState {
    tasks: HashMap<String, TaskEntry>,
    version: u64,
}

#[derive(Debug)]
struct CoordinatorInner {
    state: Mutex<CoordinatorState>,
    changed: watch::Sender<u64>,
}

/// 可在多个 SessionTool 实例之间共享的可见任务会话协调器。
#[derive(Debug, Clone)]
pub(crate) struct TaskSessionCoordinator {
    inner: Arc<CoordinatorInner>,
}

impl TaskSessionCoordinator {
    pub(crate) fn new() -> Self {
        let (changed, _) = watch::channel(0);
        Self {
            inner: Arc::new(CoordinatorInner {
                state: Mutex::new(CoordinatorState::default()),
                changed,
            }),
        }
    }

    pub(crate) fn register(&self, parent_session_id: &str, task_session_id: &str) -> Result<()> {
        validate_id("parent session", parent_session_id)?;
        validate_id("task session", task_session_id)?;

        let version = {
            let mut state = self.lock_state();
            if state.tasks.contains_key(task_session_id) {
                bail!("task session '{task_session_id}' is already registered");
            }
            state.version = state.version.saturating_add(1);
            let sequence = state.version;
            let now = current_time_ms();
            state.tasks.insert(
                task_session_id.to_string(),
                TaskEntry {
                    snapshot: TaskSessionSnapshot {
                        parent_session_id: parent_session_id.to_string(),
                        task_session_id: task_session_id.to_string(),
                        state: TaskSessionState::Queued,
                        error: None,
                        created_ms: now,
                        updated_ms: now,
                    },
                    sequence,
                },
            );
            state.version
        };
        self.publish_change(version);
        Ok(())
    }

    /// 恢复缺失条目；仍在本进程运行的任务不会被旧磁盘状态覆盖。
    pub(crate) fn restore(&self, snapshot: TaskSessionSnapshot) {
        let mut state = self.lock_state();
        if state.tasks.contains_key(&snapshot.task_session_id) {
            return;
        }
        state.version = state.version.saturating_add(1);
        let sequence = state.version;
        state.tasks.insert(snapshot.task_session_id.clone(), TaskEntry { snapshot, sequence });
        self.publish_change(state.version);
    }

    pub(crate) fn mark_running(
        &self,
        parent_session_id: &str,
        task_session_id: &str,
    ) -> Result<bool> {
        self.transition(parent_session_id, task_session_id, TaskSessionState::Running, None)
    }

    /// 为已结束的任务开启新一轮；排队或运行中的任务会被明确拒绝。
    pub(crate) fn begin_turn(&self, parent_session_id: &str, task_session_id: &str) -> Result<()> {
        let version = {
            let mut state = self.lock_state();
            let entry = task_for_parent_mut(&mut state, parent_session_id, task_session_id)?;
            if !entry.snapshot.state.is_terminal() {
                bail!("task session '{task_session_id}' is busy");
            }
            entry.snapshot.state = TaskSessionState::Running;
            entry.snapshot.error = None;
            entry.snapshot.updated_ms = current_time_ms();
            state.version = state.version.saturating_add(1);
            state.version
        };
        self.publish_change(version);
        Ok(())
    }

    pub(crate) fn mark_succeeded(
        &self,
        parent_session_id: &str,
        task_session_id: &str,
    ) -> Result<bool> {
        self.transition(parent_session_id, task_session_id, TaskSessionState::Succeeded, None)
    }

    pub(crate) fn mark_failed(
        &self,
        parent_session_id: &str,
        task_session_id: &str,
        error: impl Into<String>,
    ) -> Result<bool> {
        self.transition(
            parent_session_id,
            task_session_id,
            TaskSessionState::Failed,
            Some(error.into()),
        )
    }

    pub(crate) fn snapshots(
        &self,
        parent_session_id: &str,
        task_session_ids: &[String],
    ) -> Result<Vec<TaskSessionSnapshot>> {
        snapshots_from_state(&self.lock_state(), parent_session_id, task_session_ids)
    }

    pub(crate) async fn wait(
        &self,
        parent_session_id: &str,
        task_session_ids: &[String],
        condition: TaskSessionWaitCondition,
        timeout: Duration,
    ) -> Result<TaskSessionWaitOutcome> {
        let mut changed = self.inner.changed.subscribe();
        let timeout = timeout.min(MAX_TASK_SESSION_WAIT);
        let deadline = Instant::now().checked_add(timeout).unwrap_or_else(Instant::now);

        loop {
            let tasks = self.snapshots(parent_session_id, task_session_ids)?;
            if wait_satisfied(&tasks, condition) {
                return Ok(TaskSessionWaitOutcome { tasks, timed_out: false });
            }

            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero()
                || tokio::time::timeout(remaining, changed.changed()).await.is_err()
            {
                let tasks = self.snapshots(parent_session_id, task_session_ids)?;
                return Ok(TaskSessionWaitOutcome { tasks, timed_out: true });
            }
        }
    }

    fn transition(
        &self,
        parent_session_id: &str,
        task_session_id: &str,
        next: TaskSessionState,
        error: Option<String>,
    ) -> Result<bool> {
        let version = {
            let mut state = self.lock_state();
            let entry = task_for_parent_mut(&mut state, parent_session_id, task_session_id)?;
            if entry.snapshot.state.is_terminal() || entry.snapshot.state == next {
                return Ok(false);
            }
            entry.snapshot.state = next;
            entry.snapshot.error = error;
            entry.snapshot.updated_ms = current_time_ms();
            state.version = state.version.saturating_add(1);
            state.version
        };
        self.publish_change(version);
        Ok(true)
    }

    fn publish_change(&self, version: u64) {
        self.inner.changed.send_replace(version);
    }

    fn lock_state(&self) -> std::sync::MutexGuard<'_, CoordinatorState> {
        self.inner.state.lock().unwrap_or_else(|error| error.into_inner())
    }
}

impl Default for TaskSessionCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

/// 返回供各轮 SessionTool 共享的进程内协调器。
pub(crate) fn global_task_session_coordinator() -> TaskSessionCoordinator {
    static COORDINATOR: OnceLock<TaskSessionCoordinator> = OnceLock::new();
    COORDINATOR.get_or_init(TaskSessionCoordinator::new).clone()
}

fn task_for_parent_mut<'a>(
    state: &'a mut CoordinatorState,
    parent_session_id: &str,
    task_session_id: &str,
) -> Result<&'a mut TaskEntry> {
    let Some(entry) = state.tasks.get_mut(task_session_id) else {
        bail!("unknown task session '{task_session_id}'");
    };
    if entry.snapshot.parent_session_id != parent_session_id {
        bail!("task session '{task_session_id}' does not belong to parent '{parent_session_id}'");
    }
    Ok(entry)
}

fn snapshots_from_state(
    state: &CoordinatorState,
    parent_session_id: &str,
    task_session_ids: &[String],
) -> Result<Vec<TaskSessionSnapshot>> {
    if task_session_ids.is_empty() {
        let mut entries = state
            .tasks
            .values()
            .filter(|entry| entry.snapshot.parent_session_id == parent_session_id)
            .collect::<Vec<_>>();
        entries.sort_by_key(|entry| entry.sequence);
        return Ok(entries.into_iter().map(|entry| entry.snapshot.clone()).collect());
    }

    let mut seen = HashSet::new();
    task_session_ids
        .iter()
        .filter(|task_session_id| seen.insert((*task_session_id).clone()))
        .map(|task_session_id| {
            let Some(entry) = state.tasks.get(task_session_id) else {
                bail!("unknown task session '{task_session_id}'");
            };
            if entry.snapshot.parent_session_id != parent_session_id {
                bail!(
                    "task session '{task_session_id}' does not belong to parent '{parent_session_id}'"
                );
            }
            Ok(entry.snapshot.clone())
        })
        .collect()
}

fn wait_satisfied(tasks: &[TaskSessionSnapshot], condition: TaskSessionWaitCondition) -> bool {
    if tasks.is_empty() {
        return false;
    }
    match condition {
        TaskSessionWaitCondition::Any => tasks.iter().any(|task| task.state.is_terminal()),
        TaskSessionWaitCondition::All => tasks.iter().all(|task| task.state.is_terminal()),
    }
}

fn validate_id(name: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        bail!("{name} ID must not be empty");
    }
    Ok(())
}

fn current_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}
