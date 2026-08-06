//! 子智能体后台任务的会话级生命周期协调器。

use anyhow::{Result, bail};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

/// 子任务生命周期状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ChildAgentTaskState {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

impl ChildAgentTaskState {
    pub(crate) fn is_terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Cancelled)
    }
}

/// 子任务的稳定状态快照。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct ChildAgentTaskSnapshot {
    pub(crate) parent_session_id: String,
    pub(crate) child_session_id: String,
    pub(crate) agent: String,
    pub(crate) task: String,
    pub(crate) state: ChildAgentTaskState,
    pub(crate) output: Option<String>,
    pub(crate) error: Option<String>,
    pub(crate) created_ms: u64,
    pub(crate) updated_ms: u64,
}

/// 等待任意任务或全部任务进入终态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WaitCondition {
    Any,
    All,
}

/// 一次事件驱动等待的结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChildAgentWaitOutcome {
    pub(crate) tasks: Vec<ChildAgentTaskSnapshot>,
    pub(crate) timed_out: bool,
}

#[derive(Debug)]
struct TaskEntry {
    snapshot: ChildAgentTaskSnapshot,
    cancellation: CancellationToken,
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
    blocking_changed: Condvar,
    max_terminal_per_parent: usize,
}

/// 可在多个 AgentTool 实例之间共享的子任务协调器。
#[derive(Debug, Clone)]
pub(crate) struct ChildAgentCoordinator {
    inner: Arc<CoordinatorInner>,
}

static GLOBAL_COORDINATOR: OnceLock<ChildAgentCoordinator> = OnceLock::new();

pub(crate) fn global_child_agent_coordinator() -> ChildAgentCoordinator {
    GLOBAL_COORDINATOR.get_or_init(|| ChildAgentCoordinator::new(128)).clone()
}

impl ChildAgentCoordinator {
    pub(crate) fn new(max_terminal_per_parent: usize) -> Self {
        let (changed, _) = watch::channel(0);
        Self {
            inner: Arc::new(CoordinatorInner {
                state: Mutex::new(CoordinatorState::default()),
                changed,
                blocking_changed: Condvar::new(),
                max_terminal_per_parent: max_terminal_per_parent.max(1),
            }),
        }
    }

    pub(crate) fn register(
        &self,
        parent_session_id: &str,
        child_session_id: &str,
        agent: &str,
        task: &str,
    ) -> Result<CancellationToken> {
        for (name, value) in [
            ("parent session ID", parent_session_id),
            ("child session ID", child_session_id),
            ("agent", agent),
            ("task", task),
        ] {
            if value.trim().is_empty() {
                bail!("child agent {name} must not be empty");
            }
        }

        let cancellation = CancellationToken::new();
        let now = current_time_ms();
        let version = {
            let mut state = self.lock_state();
            if state.tasks.contains_key(child_session_id) {
                bail!("child agent task '{child_session_id}' is already registered");
            }
            state.version = state.version.saturating_add(1);
            let sequence = state.version;
            state.tasks.insert(
                child_session_id.to_string(),
                TaskEntry {
                    snapshot: ChildAgentTaskSnapshot {
                        parent_session_id: parent_session_id.to_string(),
                        child_session_id: child_session_id.to_string(),
                        agent: agent.to_string(),
                        task: task.to_string(),
                        state: ChildAgentTaskState::Queued,
                        output: None,
                        error: None,
                        created_ms: now,
                        updated_ms: now,
                    },
                    cancellation: cancellation.clone(),
                    sequence,
                },
            );
            state.version
        };
        self.publish_change(version);
        Ok(cancellation)
    }

    pub(crate) fn mark_running(
        &self,
        parent_session_id: &str,
        child_session_id: &str,
    ) -> Result<bool> {
        self.transition(
            parent_session_id,
            child_session_id,
            ChildAgentTaskState::Running,
            None,
            None,
        )
    }

    pub(crate) fn mark_succeeded(
        &self,
        parent_session_id: &str,
        child_session_id: &str,
        output: impl Into<String>,
    ) -> Result<bool> {
        self.transition(
            parent_session_id,
            child_session_id,
            ChildAgentTaskState::Succeeded,
            Some(output.into()),
            None,
        )
    }

    pub(crate) fn mark_failed(
        &self,
        parent_session_id: &str,
        child_session_id: &str,
        error: impl Into<String>,
    ) -> Result<bool> {
        self.transition(
            parent_session_id,
            child_session_id,
            ChildAgentTaskState::Failed,
            None,
            Some(error.into()),
        )
    }

    pub(crate) fn cancel(&self, parent_session_id: &str, child_session_id: &str) -> Result<bool> {
        let (changed, cancellation, version) = {
            let mut state = self.lock_state();
            let entry = task_for_parent_mut(&mut state, parent_session_id, child_session_id)?;
            if entry.snapshot.state.is_terminal() {
                return Ok(false);
            }
            entry.snapshot.state = ChildAgentTaskState::Cancelled;
            entry.snapshot.error = Some("cancelled by parent agent".to_string());
            entry.snapshot.updated_ms = current_time_ms();
            let cancellation = entry.cancellation.clone();
            state.version = state.version.saturating_add(1);
            let sequence = state.version;
            state
                .tasks
                .get_mut(child_session_id)
                .expect("validated child task should remain present")
                .sequence = sequence;
            prune_terminal_tasks_for_ancestors(
                &mut state,
                parent_session_id,
                self.inner.max_terminal_per_parent,
            );
            (true, cancellation, state.version)
        };
        cancellation.cancel();
        self.publish_change(version);
        Ok(changed)
    }

    pub(crate) fn snapshots(
        &self,
        parent_session_id: &str,
        child_session_ids: &[String],
    ) -> Result<Vec<ChildAgentTaskSnapshot>> {
        snapshots_from_state(&self.lock_state(), parent_session_id, child_session_ids)
    }

    /// 查找当前会话父链中与候选任务相同的既有委派。
    ///
    /// 子智能体启动时，其自身会话 ID 对应协调器中的一条子任务记录。沿该记录
    /// 向上遍历即可在创建新 Session 前拒绝把祖先交下来的同一任务再次委派。
    pub(crate) fn matching_ancestor_task(
        &self,
        parent_session_id: &str,
        task: &str,
    ) -> Result<Option<ChildAgentTaskSnapshot>> {
        if parent_session_id.trim().is_empty() {
            bail!("child agent parent session ID must not be empty");
        }
        if task.trim().is_empty() {
            bail!("child agent task must not be empty");
        }

        let state = self.lock_state();
        let candidate = normalized_task(task);
        let mut current_session_id = parent_session_id.to_string();
        let mut visited = HashSet::new();
        while let Some(entry) = state.tasks.get(&current_session_id) {
            if !visited.insert(current_session_id.clone()) {
                bail!("child agent ancestor cycle detected at '{current_session_id}'");
            }
            if normalized_task(&entry.snapshot.task) == candidate {
                return Ok(Some(entry.snapshot.clone()));
            }
            current_session_id = entry.snapshot.parent_session_id.clone();
        }
        Ok(None)
    }

    #[allow(dead_code)] // 后续 plan-run 接线前先提供可复用的递归查询边界。
    pub(crate) fn descendant_snapshots(
        &self,
        parent_session_id: &str,
        child_session_ids: &[String],
    ) -> Result<Vec<ChildAgentTaskSnapshot>> {
        descendant_snapshots_from_state(&self.lock_state(), parent_session_id, child_session_ids)
    }

    pub(crate) async fn wait(
        &self,
        parent_session_id: &str,
        child_session_ids: &[String],
        condition: WaitCondition,
        timeout: Duration,
    ) -> Result<ChildAgentWaitOutcome> {
        let mut changed = self.inner.changed.subscribe();
        let deadline = Instant::now().checked_add(timeout).unwrap_or_else(Instant::now);

        loop {
            let tasks = self.snapshots(parent_session_id, child_session_ids)?;
            if wait_satisfied(&tasks, condition) {
                return Ok(ChildAgentWaitOutcome { tasks, timed_out: false });
            }

            let now = Instant::now();
            if now >= deadline {
                return Ok(ChildAgentWaitOutcome { tasks, timed_out: true });
            }
            let remaining = deadline.saturating_duration_since(now);
            if tokio::time::timeout(remaining, changed.changed()).await.is_err() {
                let tasks = self.snapshots(parent_session_id, child_session_ids)?;
                return Ok(ChildAgentWaitOutcome { tasks, timed_out: true });
            }
        }
    }

    #[allow(dead_code)] // 后续 plan-run 接线前先提供可复用的递归等待边界。
    pub(crate) async fn wait_for_descendants(
        &self,
        parent_session_id: &str,
        child_session_ids: &[String],
        timeout: Duration,
    ) -> Result<ChildAgentWaitOutcome> {
        let mut changed = self.inner.changed.subscribe();
        let deadline = Instant::now().checked_add(timeout).unwrap_or_else(Instant::now);

        loop {
            let tasks = self.descendant_snapshots(parent_session_id, child_session_ids)?;
            if tasks.iter().all(|task| task.state.is_terminal()) {
                tokio::task::yield_now().await;
                let stable_tasks =
                    self.descendant_snapshots(parent_session_id, child_session_ids)?;
                if stable_tasks == tasks {
                    return Ok(ChildAgentWaitOutcome { tasks: stable_tasks, timed_out: false });
                }
            }

            let now = Instant::now();
            if now >= deadline {
                let tasks = self.descendant_snapshots(parent_session_id, child_session_ids)?;
                return Ok(ChildAgentWaitOutcome { tasks, timed_out: true });
            }
            let remaining = deadline.saturating_duration_since(now);
            if tokio::time::timeout(remaining, changed.changed()).await.is_err() {
                let tasks = self.descendant_snapshots(parent_session_id, child_session_ids)?;
                return Ok(ChildAgentWaitOutcome { tasks, timed_out: true });
            }
        }
    }

    /// 阻塞等待指定子树进入稳定终态。
    ///
    /// 普通会话处理器运行在同步线程中，不能直接等待 Tokio `watch`。
    /// 条件变量与异步等待共享同一个 `CoordinatorState::version` 真源。
    pub(crate) fn wait_for_descendants_blocking(
        &self,
        parent_session_id: &str,
        child_session_ids: &[String],
        timeout: Duration,
    ) -> Result<ChildAgentWaitOutcome> {
        const QUIESCENCE_CONFIRMATION: Duration = Duration::from_millis(20);

        let deadline = Instant::now().checked_add(timeout).unwrap_or_else(Instant::now);
        let mut state = self.lock_state();
        loop {
            let tasks =
                descendant_snapshots_from_state(&state, parent_session_id, child_session_ids)?;
            if tasks.iter().all(|task| task.state.is_terminal()) {
                let version = state.version;
                let remaining = deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    return Ok(ChildAgentWaitOutcome { tasks, timed_out: false });
                }
                let confirmation = remaining.min(QUIESCENCE_CONFIRMATION);
                let (next_state, wait_result) = self
                    .inner
                    .blocking_changed
                    .wait_timeout(state, confirmation)
                    .unwrap_or_else(|error| error.into_inner());
                state = next_state;
                if wait_result.timed_out() && state.version == version {
                    let stable_tasks = descendant_snapshots_from_state(
                        &state,
                        parent_session_id,
                        child_session_ids,
                    )?;
                    if stable_tasks == tasks {
                        return Ok(ChildAgentWaitOutcome { tasks: stable_tasks, timed_out: false });
                    }
                }
                continue;
            }

            let now = Instant::now();
            if now >= deadline {
                return Ok(ChildAgentWaitOutcome { tasks, timed_out: true });
            }
            let remaining = deadline.saturating_duration_since(now);
            let (next_state, wait_result) = self
                .inner
                .blocking_changed
                .wait_timeout(state, remaining)
                .unwrap_or_else(|error| error.into_inner());
            state = next_state;
            if wait_result.timed_out() {
                let tasks =
                    descendant_snapshots_from_state(&state, parent_session_id, child_session_ids)?;
                return Ok(ChildAgentWaitOutcome { tasks, timed_out: true });
            }
        }
    }

    #[allow(dead_code)] // 后续 plan-run 接线前先提供可复用的递归取消边界。
    pub(crate) fn cancel_descendants(
        &self,
        parent_session_id: &str,
        child_session_ids: &[String],
    ) -> Result<usize> {
        let (cancellations, version) = {
            let mut state = self.lock_state();
            let tasks =
                descendant_snapshots_from_state(&state, parent_session_id, child_session_ids)?;
            let mut cancellations = Vec::new();
            let mut affected_parents = HashSet::new();

            for task in tasks {
                let child_session_id = task.child_session_id;
                let cancellation = {
                    let entry = state
                        .tasks
                        .get_mut(&child_session_id)
                        .expect("recursive snapshot task should remain present");
                    if entry.snapshot.state.is_terminal() {
                        None
                    } else {
                        entry.snapshot.state = ChildAgentTaskState::Cancelled;
                        entry.snapshot.error = Some("cancelled by ancestor agent".to_string());
                        entry.snapshot.updated_ms = current_time_ms();
                        affected_parents.insert(entry.snapshot.parent_session_id.clone());
                        Some(entry.cancellation.clone())
                    }
                };
                if let Some(cancellation) = cancellation {
                    state.version = state.version.saturating_add(1);
                    let sequence = state.version;
                    state
                        .tasks
                        .get_mut(&child_session_id)
                        .expect("cancelled descendant task should remain present")
                        .sequence = sequence;
                    cancellations.push(cancellation);
                }
            }

            if cancellations.is_empty() {
                return Ok(0);
            }
            for affected_parent in affected_parents {
                prune_terminal_tasks_for_ancestors(
                    &mut state,
                    &affected_parent,
                    self.inner.max_terminal_per_parent,
                );
            }
            (cancellations, state.version)
        };

        let cancelled_count = cancellations.len();
        for cancellation in cancellations {
            cancellation.cancel();
        }
        self.publish_change(version);
        Ok(cancelled_count)
    }

    fn transition(
        &self,
        parent_session_id: &str,
        child_session_id: &str,
        next: ChildAgentTaskState,
        output: Option<String>,
        error: Option<String>,
    ) -> Result<bool> {
        let version = {
            let mut state = self.lock_state();
            let entry = task_for_parent_mut(&mut state, parent_session_id, child_session_id)?;
            if entry.snapshot.state.is_terminal() || entry.snapshot.state == next {
                return Ok(false);
            }
            if next == ChildAgentTaskState::Queued {
                bail!("child agent task cannot transition back to queued");
            }
            entry.snapshot.state = next;
            entry.snapshot.output = output;
            entry.snapshot.error = error;
            entry.snapshot.updated_ms = current_time_ms();
            state.version = state.version.saturating_add(1);
            let sequence = state.version;
            state
                .tasks
                .get_mut(child_session_id)
                .expect("validated child task should remain present")
                .sequence = sequence;
            if next.is_terminal() {
                prune_terminal_tasks_for_ancestors(
                    &mut state,
                    parent_session_id,
                    self.inner.max_terminal_per_parent,
                );
            }
            state.version
        };
        self.publish_change(version);
        Ok(true)
    }

    fn publish_change(&self, version: u64) {
        self.inner.changed.send_replace(version);
        self.inner.blocking_changed.notify_all();
    }

    fn lock_state(&self) -> std::sync::MutexGuard<'_, CoordinatorState> {
        self.inner.state.lock().unwrap_or_else(|error| error.into_inner())
    }
}

fn task_for_parent_mut<'a>(
    state: &'a mut CoordinatorState,
    parent_session_id: &str,
    child_session_id: &str,
) -> Result<&'a mut TaskEntry> {
    let Some(entry) = state.tasks.get_mut(child_session_id) else {
        bail!("unknown child agent task '{child_session_id}'");
    };
    if entry.snapshot.parent_session_id != parent_session_id {
        bail!(
            "child agent task '{child_session_id}' does not belong to parent '{parent_session_id}'"
        );
    }
    Ok(entry)
}

fn snapshots_from_state(
    state: &CoordinatorState,
    parent_session_id: &str,
    child_session_ids: &[String],
) -> Result<Vec<ChildAgentTaskSnapshot>> {
    if child_session_ids.is_empty() {
        let mut entries = state
            .tasks
            .values()
            .filter(|entry| entry.snapshot.parent_session_id == parent_session_id)
            .collect::<Vec<_>>();
        entries.sort_by_key(|entry| entry.sequence);
        return Ok(entries.into_iter().map(|entry| entry.snapshot.clone()).collect());
    }

    child_session_ids
        .iter()
        .map(|child_session_id| {
            let Some(entry) = state.tasks.get(child_session_id) else {
                bail!("unknown child agent task '{child_session_id}'");
            };
            if entry.snapshot.parent_session_id != parent_session_id {
                bail!(
                    "child agent task '{child_session_id}' does not belong to parent '{parent_session_id}'"
                );
            }
            Ok(entry.snapshot.clone())
        })
        .collect()
}

fn descendant_snapshots_from_state(
    state: &CoordinatorState,
    parent_session_id: &str,
    child_session_ids: &[String],
) -> Result<Vec<ChildAgentTaskSnapshot>> {
    let roots = descendant_roots(state, parent_session_id, child_session_ids)?;
    let mut snapshots = Vec::new();
    let mut visiting = HashSet::new();
    let mut visited = HashSet::new();
    for child_session_id in roots {
        collect_descendants(state, &child_session_id, &mut visiting, &mut visited, &mut snapshots)?;
    }
    Ok(snapshots)
}

fn descendant_roots(
    state: &CoordinatorState,
    parent_session_id: &str,
    child_session_ids: &[String],
) -> Result<Vec<String>> {
    if child_session_ids.is_empty() {
        let mut entries = state
            .tasks
            .values()
            .filter(|entry| entry.snapshot.parent_session_id == parent_session_id)
            .collect::<Vec<_>>();
        entries.sort_by_key(|entry| entry.sequence);
        return Ok(entries
            .into_iter()
            .map(|entry| entry.snapshot.child_session_id.clone())
            .collect());
    }

    child_session_ids
        .iter()
        .map(|child_session_id| {
            let Some(entry) = state.tasks.get(child_session_id) else {
                bail!("unknown child agent task '{child_session_id}'");
            };
            if entry.snapshot.parent_session_id != parent_session_id {
                bail!(
                    "child agent task '{child_session_id}' does not belong to parent '{parent_session_id}'"
                );
            }
            Ok(child_session_id.clone())
        })
        .collect()
}

fn collect_descendants(
    state: &CoordinatorState,
    child_session_id: &str,
    visiting: &mut HashSet<String>,
    visited: &mut HashSet<String>,
    snapshots: &mut Vec<ChildAgentTaskSnapshot>,
) -> Result<()> {
    if visiting.contains(child_session_id) {
        bail!("child agent descendant cycle detected at '{child_session_id}'");
    }
    if visited.contains(child_session_id) {
        return Ok(());
    }

    let Some(entry) = state.tasks.get(child_session_id) else {
        bail!("unknown child agent task '{child_session_id}'");
    };
    visiting.insert(child_session_id.to_string());
    snapshots.push(entry.snapshot.clone());

    let mut children = state
        .tasks
        .values()
        .filter(|candidate| candidate.snapshot.parent_session_id == child_session_id)
        .collect::<Vec<_>>();
    children.sort_by_key(|candidate| candidate.sequence);
    for child in children {
        collect_descendants(state, &child.snapshot.child_session_id, visiting, visited, snapshots)?;
    }

    visiting.remove(child_session_id);
    visited.insert(child_session_id.to_string());
    Ok(())
}

fn wait_satisfied(tasks: &[ChildAgentTaskSnapshot], condition: WaitCondition) -> bool {
    if tasks.is_empty() {
        return false;
    }
    match condition {
        WaitCondition::Any => tasks.iter().any(|task| task.state.is_terminal()),
        WaitCondition::All => tasks.iter().all(|task| task.state.is_terminal()),
    }
}

fn normalized_task(task: &str) -> String {
    task.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

fn prune_terminal_tasks(
    state: &mut CoordinatorState,
    parent_session_id: &str,
    max_terminal_per_parent: usize,
) {
    let mut terminal = state
        .tasks
        .iter()
        .filter(|(_, entry)| {
            entry.snapshot.parent_session_id == parent_session_id
                && entry.snapshot.state.is_terminal()
        })
        .map(|(child_session_id, entry)| (child_session_id.clone(), entry.sequence))
        .collect::<Vec<_>>();
    terminal.sort_by_key(|(_, sequence)| *sequence);
    let removable = terminal
        .into_iter()
        .filter(|(child_session_id, _)| {
            task_subtree_is_terminal(state, child_session_id, &mut HashSet::new())
        })
        .collect::<Vec<_>>();
    let remove_count = removable.len().saturating_sub(max_terminal_per_parent);
    for (child_session_id, _) in removable.into_iter().take(remove_count) {
        remove_task_subtree(state, &child_session_id, &mut HashSet::new());
    }
}

fn prune_terminal_tasks_for_ancestors(
    state: &mut CoordinatorState,
    parent_session_id: &str,
    max_terminal_per_parent: usize,
) {
    let mut current_parent = Some(parent_session_id.to_string());
    let mut visited = HashSet::new();
    while let Some(parent_session_id) = current_parent {
        if !visited.insert(parent_session_id.clone()) {
            break;
        }
        current_parent = state
            .tasks
            .get(&parent_session_id)
            .map(|entry| entry.snapshot.parent_session_id.clone());
        prune_terminal_tasks(state, &parent_session_id, max_terminal_per_parent);
    }
}

fn task_subtree_is_terminal(
    state: &CoordinatorState,
    child_session_id: &str,
    visiting: &mut HashSet<String>,
) -> bool {
    if !visiting.insert(child_session_id.to_string()) {
        return false;
    }
    let Some(entry) = state.tasks.get(child_session_id) else {
        visiting.remove(child_session_id);
        return true;
    };
    if !entry.snapshot.state.is_terminal() {
        visiting.remove(child_session_id);
        return false;
    }
    let terminal = state
        .tasks
        .values()
        .filter(|candidate| candidate.snapshot.parent_session_id == child_session_id)
        .all(|child| task_subtree_is_terminal(state, &child.snapshot.child_session_id, visiting));
    visiting.remove(child_session_id);
    terminal
}

fn remove_task_subtree(
    state: &mut CoordinatorState,
    child_session_id: &str,
    visited: &mut HashSet<String>,
) {
    if !visited.insert(child_session_id.to_string()) {
        return;
    }
    let children = state
        .tasks
        .values()
        .filter(|candidate| candidate.snapshot.parent_session_id == child_session_id)
        .map(|candidate| candidate.snapshot.child_session_id.clone())
        .collect::<Vec<_>>();
    for child in children {
        remove_task_subtree(state, &child, visited);
    }
    state.tasks.remove(child_session_id);
}

fn current_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}
