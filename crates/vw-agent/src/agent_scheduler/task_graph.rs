use anyhow::{Result, bail};
use std::collections::{BTreeMap, BTreeSet};

use super::domain::TaskId;
use super::state_machine::{AgentTaskKind, AgentTaskStatus};

/// A successful-completion dependency from `predecessor` to `successor`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentTaskDependency {
    pub predecessor: TaskId,
    pub successor: TaskId,
}

impl AgentTaskDependency {
    pub fn new(predecessor: TaskId, successor: TaskId) -> Self {
        Self {
            predecessor,
            successor,
        }
    }
}

/// The task data needed to derive graph readiness without accessing storage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskGraphTaskSnapshot {
    pub id: TaskId,
    pub kind: AgentTaskKind,
    pub status: AgentTaskStatus,
}

impl TaskGraphTaskSnapshot {
    pub fn new(id: TaskId, kind: AgentTaskKind, status: AgentTaskStatus) -> Self {
        Self { id, kind, status }
    }
}

/// The scheduling result derived from a task's current status and predecessors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskGraphReadiness {
    Ready,
    WaitingDependencies { task_ids: Vec<TaskId> },
    BlockedByDependency { task_ids: Vec<TaskId> },
    NotQueued,
}

/// A validated, in-memory graph of finite task completion dependencies.
#[derive(Debug, Clone)]
pub struct TaskGraph {
    tasks: BTreeMap<String, TaskGraphTaskSnapshot>,
    dependencies: Vec<AgentTaskDependency>,
    predecessors: BTreeMap<String, Vec<TaskId>>,
}

impl TaskGraph {
    pub fn new(
        tasks: Vec<TaskGraphTaskSnapshot>,
        dependencies: Vec<AgentTaskDependency>,
    ) -> Result<Self> {
        let mut tasks_by_id = BTreeMap::new();
        for task in tasks {
            if task.kind != AgentTaskKind::Finite {
                bail!(
                    "task graph node {} must have finite task kind",
                    task.id.as_str()
                );
            }

            let task_id = task.id.as_str().to_owned();
            if tasks_by_id.insert(task_id.clone(), task).is_some() {
                bail!("task graph contains duplicate node {task_id}");
            }
        }

        let mut predecessors = tasks_by_id
            .keys()
            .map(|task_id| (task_id.clone(), Vec::new()))
            .collect::<BTreeMap<_, _>>();
        let mut successors = tasks_by_id
            .keys()
            .map(|task_id| (task_id.clone(), Vec::new()))
            .collect::<BTreeMap<_, _>>();
        let mut edge_keys = BTreeSet::new();

        for dependency in &dependencies {
            let predecessor = dependency.predecessor.as_str();
            let successor = dependency.successor.as_str();

            if predecessor == successor {
                bail!("task graph rejects self dependency for task {predecessor}");
            }
            if !tasks_by_id.contains_key(predecessor) {
                bail!("task graph dependency predecessor {predecessor} does not exist");
            }
            if !tasks_by_id.contains_key(successor) {
                bail!("task graph dependency successor {successor} does not exist");
            }
            if !edge_keys.insert((predecessor.to_owned(), successor.to_owned())) {
                bail!("task graph contains duplicate dependency {predecessor} -> {successor}");
            }

            predecessors
                .get_mut(successor)
                .expect("validated successor must have a predecessor entry")
                .push(dependency.predecessor.clone());
            successors
                .get_mut(predecessor)
                .expect("validated predecessor must have a successor entry")
                .push(successor.to_owned());
        }

        for task_ids in predecessors.values_mut() {
            task_ids.sort_by(|left, right| left.as_str().cmp(right.as_str()));
        }
        for task_ids in successors.values_mut() {
            task_ids.sort();
        }
        ensure_acyclic(&successors)?;

        Ok(Self {
            tasks: tasks_by_id,
            dependencies,
            predecessors,
        })
    }

    pub fn dependencies(&self) -> &[AgentTaskDependency] {
        &self.dependencies
    }

    pub fn readiness_for(&self, task_id: &TaskId) -> Result<TaskGraphReadiness> {
        let task = self.task(task_id)?;
        if task.status != AgentTaskStatus::Queued {
            return Ok(TaskGraphReadiness::NotQueued);
        }

        let predecessors = self
            .predecessors
            .get(task_id.as_str())
            .expect("validated task must have a predecessor entry");
        let mut waiting_task_ids = Vec::new();
        let mut blocking_task_ids = Vec::new();

        for predecessor_id in predecessors {
            let predecessor = self.task(predecessor_id)?;
            match predecessor.status {
                AgentTaskStatus::Completed => {}
                AgentTaskStatus::Failed
                | AgentTaskStatus::Cancelled
                | AgentTaskStatus::Archived => blocking_task_ids.push(predecessor_id.clone()),
                _ => waiting_task_ids.push(predecessor_id.clone()),
            }
        }

        if !blocking_task_ids.is_empty() {
            return Ok(TaskGraphReadiness::BlockedByDependency {
                task_ids: blocking_task_ids,
            });
        }
        if !waiting_task_ids.is_empty() {
            return Ok(TaskGraphReadiness::WaitingDependencies {
                task_ids: waiting_task_ids,
            });
        }

        Ok(TaskGraphReadiness::Ready)
    }

    fn task(&self, task_id: &TaskId) -> Result<&TaskGraphTaskSnapshot> {
        self.tasks
            .get(task_id.as_str())
            .ok_or_else(|| anyhow::anyhow!("task graph does not contain task {}", task_id.as_str()))
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum VisitState {
    Visiting,
    Visited,
}

fn ensure_acyclic(successors: &BTreeMap<String, Vec<String>>) -> Result<()> {
    let mut states = BTreeMap::new();
    for task_id in successors.keys() {
        visit(task_id, successors, &mut states)?;
    }
    Ok(())
}

fn visit(
    task_id: &str,
    successors: &BTreeMap<String, Vec<String>>,
    states: &mut BTreeMap<String, VisitState>,
) -> Result<()> {
    match states.get(task_id) {
        Some(VisitState::Visiting) => bail!("task graph contains a cycle involving task {task_id}"),
        Some(VisitState::Visited) => return Ok(()),
        None => {}
    }

    states.insert(task_id.to_owned(), VisitState::Visiting);
    for successor in successors
        .get(task_id)
        .expect("validated task must have a successor entry")
    {
        visit(successor, successors, states)?;
    }
    states.insert(task_id.to_owned(), VisitState::Visited);
    Ok(())
}
