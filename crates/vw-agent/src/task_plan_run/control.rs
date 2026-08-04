//! 后台 plan-run 的租户隔离控制注册表。

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex, OnceLock};

use tokio_util::sync::CancellationToken;
use vw_api_types::id::TenantScope;
use vw_api_types::task::{
    TaskPlanRunStatus, TaskPlanRunStatusResponse, TaskPlanRunStreamEventDto,
};

use crate::session::child_agent::coordinator::ChildAgentCoordinator;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct RunKey {
    tenant: TenantScope,
    task_id: String,
}

#[derive(Debug)]
struct RunEntry {
    status: TaskPlanRunStatus,
    current_session_id: Option<String>,
    session_ids: HashSet<String>,
    root_cancellation: CancellationToken,
    terminal_event: Option<TaskPlanRunStreamEventDto>,
}

impl RunEntry {
    fn snapshot(&self, task_id: &str) -> TaskPlanRunStatusResponse {
        TaskPlanRunStatusResponse {
            task_id: task_id.to_string(),
            status: self.status,
            current_session_id: self.current_session_id.clone(),
            terminal_event: self.terminal_event.clone(),
        }
    }
}

#[derive(Debug, Default)]
struct RegistryState {
    runs: HashMap<RunKey, RunEntry>,
    terminal_order: VecDeque<RunKey>,
}

/// 一次 stop 请求的内部结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlanRunStopOutcome {
    pub(crate) status: TaskPlanRunStatusResponse,
    pub(crate) cancelled_descendants: usize,
    pub(crate) already_terminal: bool,
}

/// 可共享的 plan-run 控制注册表。
#[derive(Debug, Clone)]
pub(crate) struct PlanRunControlRegistry {
    inner: Arc<Mutex<RegistryState>>,
    max_terminal_entries: usize,
}

static GLOBAL_PLAN_RUN_CONTROL_REGISTRY: OnceLock<PlanRunControlRegistry> = OnceLock::new();

pub(crate) fn global_plan_run_control_registry() -> PlanRunControlRegistry {
    GLOBAL_PLAN_RUN_CONTROL_REGISTRY
        .get_or_init(|| PlanRunControlRegistry::new(128))
        .clone()
}

impl PlanRunControlRegistry {
    pub(crate) fn new(max_terminal_entries: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(RegistryState::default())),
            max_terminal_entries: max_terminal_entries.max(1),
        }
    }

    pub(crate) fn register(
        &self,
        tenant: TenantScope,
        task_id: &str,
    ) -> Result<CancellationToken, String> {
        if task_id.trim().is_empty() {
            return Err("task-plan-run task ID must not be empty".to_string());
        }
        let key = RunKey { tenant, task_id: task_id.to_string() };
        let cancellation = CancellationToken::new();
        let mut state = self.lock_state();
        if state.runs.contains_key(&key) {
            return Err(format!("task-plan-run '{task_id}' is already registered"));
        }
        state.runs.insert(
            key,
            RunEntry {
                status: TaskPlanRunStatus::Running,
                current_session_id: None,
                session_ids: HashSet::new(),
                root_cancellation: cancellation.clone(),
                terminal_event: None,
            },
        );
        Ok(cancellation)
    }

    pub(crate) fn status(
        &self,
        tenant: &TenantScope,
        task_id: &str,
    ) -> Option<TaskPlanRunStatusResponse> {
        let key = RunKey { tenant: tenant.clone(), task_id: task_id.to_string() };
        self.lock_state().runs.get(&key).map(|entry| entry.snapshot(task_id))
    }

    pub(crate) fn observe_event(
        &self,
        tenant: &TenantScope,
        task_id: &str,
        event: &TaskPlanRunStreamEventDto,
    ) -> Result<bool, String> {
        let key = RunKey { tenant: tenant.clone(), task_id: task_id.to_string() };
        let mut state = self.lock_state();
        let Some(entry) = state.runs.get_mut(&key) else {
            return Err(format!("unknown task-plan-run '{task_id}'"));
        };

        if let Some(session_id) = event_session_id(event) {
            entry.current_session_id = Some(session_id.to_string());
            entry.session_ids.insert(session_id.to_string());
        }

        if entry.status.is_terminal() {
            if entry.status == TaskPlanRunStatus::Cancelled
                && matches!(event, TaskPlanRunStreamEventDto::Cancelled { .. })
                && entry.terminal_event.is_none()
            {
                entry.terminal_event = Some(event.clone());
                return Ok(true);
            }
            return Ok(false);
        }

        let next_status = event_status(event);
        entry.status = next_status;
        if next_status.is_terminal() {
            entry.terminal_event = Some(event.clone());
            state.terminal_order.push_back(key);
            self.prune_terminal_entries(&mut state);
        }
        Ok(true)
    }

    pub(crate) fn stop(
        &self,
        tenant: &TenantScope,
        task_id: &str,
        coordinator: &ChildAgentCoordinator,
    ) -> Result<Option<PlanRunStopOutcome>, String> {
        let key = RunKey { tenant: tenant.clone(), task_id: task_id.to_string() };
        let (root_cancellation, session_ids, status, already_terminal) = {
            let mut state = self.lock_state();
            let (root_cancellation, session_ids, snapshot, already_terminal) = {
                let Some(entry) = state.runs.get_mut(&key) else {
                    return Ok(None);
                };
                let already_terminal = entry.status.is_terminal();
                if !already_terminal {
                    entry.status = TaskPlanRunStatus::Cancelled;
                }
                (
                    entry.root_cancellation.clone(),
                    entry.session_ids.iter().cloned().collect::<Vec<_>>(),
                    entry.snapshot(task_id),
                    already_terminal,
                )
            };
            if !already_terminal {
                state.terminal_order.push_back(key.clone());
                self.prune_terminal_entries(&mut state);
            }
            (root_cancellation, session_ids, snapshot, already_terminal)
        };

        if already_terminal {
            return Ok(Some(PlanRunStopOutcome {
                status,
                cancelled_descendants: 0,
                already_terminal: true,
            }));
        }

        root_cancellation.cancel();
        let mut cancelled_descendants = 0usize;
        for session_id in session_ids {
            cancelled_descendants = cancelled_descendants.saturating_add(
                coordinator
                    .cancel_descendants(&session_id, &[])
                    .map_err(|error| {
                        format!(
                            "cancel descendants of task-plan-run session '{session_id}' failed: {error}"
                        )
                    })?,
            );
        }

        Ok(Some(PlanRunStopOutcome {
            status,
            cancelled_descendants,
            already_terminal: false,
        }))
    }

    fn prune_terminal_entries(&self, state: &mut RegistryState) {
        while state.terminal_order.len() > self.max_terminal_entries {
            let Some(oldest) = state.terminal_order.pop_front() else {
                break;
            };
            if state
                .runs
                .get(&oldest)
                .is_some_and(|entry| entry.status.is_terminal())
            {
                state.runs.remove(&oldest);
            }
        }
    }

    fn lock_state(&self) -> std::sync::MutexGuard<'_, RegistryState> {
        self.inner.lock().unwrap_or_else(|error| error.into_inner())
    }
}

fn event_status(event: &TaskPlanRunStreamEventDto) -> TaskPlanRunStatus {
    match event {
        TaskPlanRunStreamEventDto::SessionWaiting { .. } => TaskPlanRunStatus::Waiting,
        TaskPlanRunStreamEventDto::Finished { .. } => TaskPlanRunStatus::Completed,
        TaskPlanRunStreamEventDto::Blocked { .. } => TaskPlanRunStatus::Blocked,
        TaskPlanRunStreamEventDto::Error { .. } => TaskPlanRunStatus::Failed,
        TaskPlanRunStreamEventDto::Cancelled { .. } => TaskPlanRunStatus::Cancelled,
        _ => TaskPlanRunStatus::Running,
    }
}

fn event_session_id(event: &TaskPlanRunStreamEventDto) -> Option<&str> {
    match event {
        TaskPlanRunStreamEventDto::PlanningStarted { session_id, .. }
        | TaskPlanRunStreamEventDto::SessionDelta { session_id, .. }
        | TaskPlanRunStreamEventDto::SessionWaiting { session_id, .. }
        | TaskPlanRunStreamEventDto::PlanCreated { session_id, .. }
        | TaskPlanRunStreamEventDto::GoalTurnStarted { session_id, .. }
        | TaskPlanRunStreamEventDto::GoalEvaluated { session_id, .. }
        | TaskPlanRunStreamEventDto::SubtaskStarted { session_id, .. }
        | TaskPlanRunStreamEventDto::SubtaskFinished { session_id, .. }
        | TaskPlanRunStreamEventDto::Blocked { session_id, .. } => Some(session_id),
        TaskPlanRunStreamEventDto::Finished { session_id, .. }
        | TaskPlanRunStreamEventDto::Cancelled { session_id, .. } => session_id.as_deref(),
        TaskPlanRunStreamEventDto::BatchStarted { .. }
        | TaskPlanRunStreamEventDto::BatchFinished { .. }
        | TaskPlanRunStreamEventDto::Error { .. } => None,
    }
}
