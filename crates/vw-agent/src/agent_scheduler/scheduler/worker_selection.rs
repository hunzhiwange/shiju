use std::collections::HashMap;

use crate::agent_scheduler::domain::{AgentCapabilityKind, AgentWorkerStatus, TeamId, WorkerName};
use crate::agent_scheduler::state_machine::{AgentTaskKind, AgentTaskStatus};
use crate::agent_scheduler::store::tasks::StoredAgentTask;
use crate::agent_scheduler::store::workers::StoredAgentWorker;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkerSelectionRequest {
    pub required_skills: Vec<String>,
    pub team_id: Option<TeamId>,
    pub preferred_runtime: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerAvailability {
    Idle,
    Busy,
    Stopped,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerCandidateScore {
    pub worker_id: WorkerName,
    pub availability: WorkerAvailability,
    pub finite_active_tasks: usize,
    pub infinite_active_tasks: usize,
    pub active_tasks: usize,
    pub runtime_matched: bool,
    pub can_dispatch_now: bool,
    pub requires_wakeup: bool,
}

pub fn select_worker_candidates(
    request: &WorkerSelectionRequest,
    workers: &[StoredAgentWorker],
    active_tasks: &[StoredAgentTask],
) -> Vec<WorkerCandidateScore> {
    let required_skills = normalized_required_skills(&request.required_skills);
    let loads = worker_loads(active_tasks);
    let mut candidates = workers
        .iter()
        .filter(|worker| matches_team(worker, request.team_id.as_ref()))
        .filter(|worker| matches_required_skills(worker, &required_skills))
        .filter_map(|worker| score_worker(request, worker, loads.get(worker.id.as_str()).copied()))
        .collect::<Vec<_>>();

    candidates.sort_by(|left, right| {
        left.availability
            .sort_rank()
            .cmp(&right.availability.sort_rank())
            .then_with(|| left.finite_active_tasks.cmp(&right.finite_active_tasks))
            .then_with(|| left.infinite_active_tasks.cmp(&right.infinite_active_tasks))
            .then_with(|| left.active_tasks.cmp(&right.active_tasks))
            .then_with(|| runtime_rank(left).cmp(&runtime_rank(right)))
            .then_with(|| left.worker_id.as_str().cmp(right.worker_id.as_str()))
    });
    candidates
}

fn score_worker(
    request: &WorkerSelectionRequest,
    worker: &StoredAgentWorker,
    load: Option<WorkerLoad>,
) -> Option<WorkerCandidateScore> {
    let load = load.unwrap_or_default();
    let availability = worker_availability(worker, load);
    if availability == WorkerAvailability::Unavailable {
        return None;
    }

    Some(WorkerCandidateScore {
        worker_id: worker.id.clone(),
        availability,
        finite_active_tasks: load.finite_active_tasks,
        infinite_active_tasks: load.infinite_active_tasks,
        active_tasks: load.active_tasks,
        runtime_matched: runtime_matched(worker, request.preferred_runtime.as_deref()),
        can_dispatch_now: matches!(
            availability,
            WorkerAvailability::Idle | WorkerAvailability::Busy
        ),
        requires_wakeup: availability == WorkerAvailability::Stopped,
    })
}

fn worker_availability(worker: &StoredAgentWorker, load: WorkerLoad) -> WorkerAvailability {
    if is_unavailable(worker) {
        return WorkerAvailability::Unavailable;
    }
    if is_stopped(worker) {
        return WorkerAvailability::Stopped;
    }
    if worker.status == AgentWorkerStatus::Busy || load.finite_active_tasks > 0 {
        return WorkerAvailability::Busy;
    }
    if matches!(worker.status, AgentWorkerStatus::Idle | AgentWorkerStatus::Running) {
        return WorkerAvailability::Idle;
    }
    WorkerAvailability::Unavailable
}

fn is_unavailable(worker: &StoredAgentWorker) -> bool {
    if matches!(worker.status, AgentWorkerStatus::Failed | AgentWorkerStatus::Unavailable) {
        return true;
    }

    matches!(worker.container_state.as_deref(), Some("failed" | "not_found" | "unavailable"))
}

fn is_stopped(worker: &StoredAgentWorker) -> bool {
    matches!(worker.status, AgentWorkerStatus::Sleeping | AgentWorkerStatus::Stopped)
        || matches!(worker.container_state.as_deref(), Some("stopped" | "exited"))
}

fn normalized_required_skills(required_skills: &[String]) -> Vec<&str> {
    required_skills.iter().map(|skill| skill.trim()).filter(|skill| !skill.is_empty()).collect()
}

fn matches_team(worker: &StoredAgentWorker, team_id: Option<&TeamId>) -> bool {
    team_id.is_none_or(|team_id| worker.team_id.as_ref() == Some(team_id))
}

fn matches_required_skills(worker: &StoredAgentWorker, required_skills: &[&str]) -> bool {
    required_skills.iter().all(|required_skill| {
        worker.capabilities.iter().any(|capability| {
            capability.kind == AgentCapabilityKind::Skill && capability.name == *required_skill
        })
    })
}

fn runtime_matched(worker: &StoredAgentWorker, preferred_runtime: Option<&str>) -> bool {
    preferred_runtime
        .is_none_or(|preferred_runtime| worker.runtime.as_deref() == Some(preferred_runtime.trim()))
}

fn runtime_rank(candidate: &WorkerCandidateScore) -> u8 {
    u8::from(!candidate.runtime_matched)
}

impl WorkerAvailability {
    fn sort_rank(self) -> u8 {
        match self {
            Self::Idle => 0,
            Self::Busy => 1,
            Self::Stopped => 2,
            Self::Unavailable => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct WorkerLoad {
    finite_active_tasks: usize,
    infinite_active_tasks: usize,
    active_tasks: usize,
}

fn worker_loads(active_tasks: &[StoredAgentTask]) -> HashMap<&str, WorkerLoad> {
    let mut loads = HashMap::new();
    for task in active_tasks.iter().filter(|task| is_load_bearing_status(task.status)) {
        let Some(worker_id) = &task.assigned_to else {
            continue;
        };
        let load = loads.entry(worker_id.as_str()).or_insert_with(WorkerLoad::default);
        match task.kind {
            AgentTaskKind::Finite => load.finite_active_tasks += 1,
            AgentTaskKind::Infinite => load.infinite_active_tasks += 1,
            AgentTaskKind::TeamDag => {}
        }
        load.active_tasks += 1;
    }
    loads
}

fn is_load_bearing_status(status: AgentTaskStatus) -> bool {
    !matches!(
        status,
        AgentTaskStatus::Completed | AgentTaskStatus::Cancelled | AgentTaskStatus::Archived
    )
}
