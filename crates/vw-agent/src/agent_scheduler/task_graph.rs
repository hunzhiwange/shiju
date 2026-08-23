//! 在内存中校验有限任务依赖图，并根据前驱完成情况计算任务就绪状态。

use anyhow::{Result, bail};
use std::collections::{BTreeMap, BTreeSet};

use super::domain::TaskId;
use super::state_machine::{AgentTaskKind, AgentTaskStatus};

/// 以前驱成功完成为条件的有向依赖边。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentTaskDependency {
    pub predecessor: TaskId,
    pub successor: TaskId,
}

impl AgentTaskDependency {
    /// 创建从 `predecessor` 指向 `successor` 的依赖边。
    pub fn new(predecessor: TaskId, successor: TaskId) -> Self {
        Self { predecessor, successor }
    }
}

/// 无需访问存储即可计算图就绪状态的任务快照。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskGraphTaskSnapshot {
    pub id: TaskId,
    pub kind: AgentTaskKind,
    pub status: AgentTaskStatus,
}

impl TaskGraphTaskSnapshot {
    /// 创建参与依赖图计算的任务快照。
    pub fn new(id: TaskId, kind: AgentTaskKind, status: AgentTaskStatus) -> Self {
        Self { id, kind, status }
    }
}

/// 根据任务自身状态及其前驱状态得到的调度就绪结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskGraphReadiness {
    Ready,
    WaitingDependencies { task_ids: Vec<TaskId> },
    BlockedByDependency { task_ids: Vec<TaskId> },
    NotQueued,
}

/// 已验证的有限任务完成依赖图。
#[derive(Debug, Clone)]
pub struct TaskGraph {
    tasks: BTreeMap<String, TaskGraphTaskSnapshot>,
    dependencies: Vec<AgentTaskDependency>,
    predecessors: BTreeMap<String, Vec<TaskId>>,
}

impl TaskGraph {
    /// 构建依赖图，并拒绝非有限任务、重复节点、悬空边、自环和循环依赖。
    pub fn new(
        tasks: Vec<TaskGraphTaskSnapshot>,
        dependencies: Vec<AgentTaskDependency>,
    ) -> Result<Self> {
        let mut tasks_by_id = BTreeMap::new();
        for task in tasks {
            if task.kind != AgentTaskKind::Finite {
                bail!("task graph node {} must have finite task kind", task.id.as_str());
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

        Ok(Self { tasks: tasks_by_id, dependencies, predecessors })
    }

    /// 返回构图时保留的依赖边。
    pub fn dependencies(&self) -> &[AgentTaskDependency] {
        &self.dependencies
    }

    /// 按任务标识的稳定顺序遍历节点快照。
    pub fn tasks(&self) -> impl Iterator<Item = &TaskGraphTaskSnapshot> {
        self.tasks.values()
    }

    /// 计算单个任务的就绪状态；不在图中的标识返回错误。
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

        // 失败、取消或归档的前驱具有终止性，优先于仍可继续等待的前驱返回。
        if !blocking_task_ids.is_empty() {
            return Ok(TaskGraphReadiness::BlockedByDependency { task_ids: blocking_task_ids });
        }
        if !waiting_task_ids.is_empty() {
            return Ok(TaskGraphReadiness::WaitingDependencies { task_ids: waiting_task_ids });
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
    for successor in successors.get(task_id).expect("validated task must have a successor entry") {
        visit(successor, successors, states)?;
    }
    states.insert(task_id.to_owned(), VisitState::Visited);
    Ok(())
}
