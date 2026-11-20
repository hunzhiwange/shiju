//! 从当前项目任务快照派生的只读工作视图，不参与存储和调度。
use desktop_task_model::{
    StatusRole, TASK_AGENT_MAIN, Task, TaskWorkflow, normalize_task_acp_agent_input,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuiltinWorkView {
    MyTasks,
    AgentExecuting,
    AllTasks,
}

impl BuiltinWorkView {
    pub fn matches(
        self,
        task: &Task,
        workflow: &TaskWorkflow,
        personal_task_ids: &[String],
    ) -> bool {
        if task.archived || task.deleted {
            return false;
        }
        match self {
            Self::MyTasks => personal_task_ids.contains(&task.id),
            Self::AgentExecuting => {
                workflow.status_role(task.status) == Some(StatusRole::Executing)
            }
            Self::AllTasks => true,
        }
    }
}

/// 使用任务已有的配置标识；同名内置智能体与 ACP 执行器互不匹配。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ExecutionAgent {
    Builtin { id: String },
    Acp { id: String },
}

impl ExecutionAgent {
    fn matches(&self, task: &Task) -> bool {
        let acp = task.acp_agent.as_deref().and_then(normalize_task_acp_agent_input);
        match self {
            Self::Builtin { id } => {
                acp.is_none()
                    && !id.is_empty()
                    && task.agent.as_deref().unwrap_or(TASK_AGENT_MAIN) == id
            }
            Self::Acp { id } => !id.is_empty() && acp.as_deref() == Some(id.as_str()),
        }
    }
}

/// 空选项列表表示不限制；同字段选项取并集，不同字段取交集。
/// 状态 key 不转换为默认状态：工作流移除 key 后，规则保留且无匹配。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkViewFilter {
    #[serde(default)]
    pub status_keys: Vec<String>,
    pub priority_min: Option<u32>,
    pub priority_max: Option<u32>,
    #[serde(default)]
    pub agents: Vec<ExecutionAgent>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub label_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub properties: vw_api_agent_scheduler::AgentSchedulerTaskPropertyValues,
}

impl WorkViewFilter {
    pub fn matches(&self, task: &Task, workflow: &TaskWorkflow) -> bool {
        !task.archived
            && !task.deleted
            && (self.status_keys.is_empty()
                || workflow
                    .status_definition(task.status)
                    .is_some_and(|definition| self.status_keys.contains(&definition.key)))
            && self.priority_min.is_none_or(|min| task.priority >= min)
            && self.priority_max.is_none_or(|max| task.priority <= max)
            && (self.agents.is_empty() || self.agents.iter().any(|agent| agent.matches(task)))
            && (self.label_ids.is_empty()
                || self.label_ids.iter().any(|id| task.label_ids.contains(id)))
            && self.properties.iter().all(|(id, value)| task.properties.get(id) == Some(value))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedWorkView {
    pub id: String,
    pub name: String,
    pub filter: WorkViewFilter,
}

impl SavedWorkView {
    pub fn matches(&self, task: &Task, workflow: &TaskWorkflow) -> bool {
        self.filter.matches(task, workflow)
    }
}

use std::collections::{BTreeMap, BTreeSet};
use vw_api_agent_scheduler::{
    AgentSchedulerDispatchBlockerDto, AgentSchedulerTaskBoardEventDto,
    AgentSchedulerTaskPlanPlacementDto as Placement,
};

#[derive(Debug)]
pub struct WorkPlanTask {
    pub placement: Option<Placement>,
    pub status: desktop_task_model::TaskStatus,
    pub status_label: Option<String>,
    pub blocker: Option<AgentSchedulerDispatchBlockerDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorkPlanGroupKey {
    Ordinary,
    Outer(u32),
    Root(u32, u32),
    Child(String, u32),
}

#[derive(Debug, Default)]
pub struct WorkPlanView {
    pub project: Option<String>,
    pub open: bool,
    pub search: String,
    pub tasks: BTreeMap<String, WorkPlanTask>,
    pub collapsed: BTreeSet<WorkPlanGroupKey>,
    pub batch_draft: Option<BatchDraft>,
    pub pending_batches: BTreeMap<String, u64>,
    pub child_draft: Option<ChildTaskDraft>,
    pub history: Option<RunHistory>,
    pub publishing: BTreeSet<String>,
}

impl WorkPlanView {
    pub fn placement(&self, task_id: &str) -> Option<&Placement> {
        self.tasks.get(task_id)?.placement.as_ref()
    }

    pub fn roots<'a>(
        &self,
        tasks: &'a [Task],
        workflow: &TaskWorkflow,
        visible: impl Fn(&Task) -> bool,
    ) -> BTreeMap<Option<(u32, u32)>, WorkPlanGroup<'a>> {
        let mut groups = BTreeMap::<_, WorkPlanGroup>::new();
        for task in tasks.iter().filter(|task| !task.deleted) {
            let key = match self.placement(&task.id) {
                Some(Placement::Root { outer_batch, inner_batch }) => {
                    Some((*outer_batch, *inner_batch))
                }
                Some(Placement::Child { .. }) => continue,
                None => None,
            };
            groups.entry(key).or_default().add(
                task,
                self.tasks.get(&task.id),
                workflow,
                visible(task),
            );
        }
        groups
    }

    pub fn children<'a>(
        &self,
        tasks: &'a [Task],
        workflow: &TaskWorkflow,
        parent: &str,
    ) -> BTreeMap<u32, WorkPlanGroup<'a>> {
        let mut groups = BTreeMap::<_, WorkPlanGroup>::new();
        for task in tasks.iter().filter(|task| !task.deleted) {
            if let Some(Placement::Child { parent_task_id, batch }) = self.placement(&task.id)
                && parent_task_id == parent
            {
                groups.entry(*batch).or_default().add(
                    task,
                    self.tasks.get(&task.id),
                    workflow,
                    true,
                );
            }
        }
        groups
    }

    pub fn close_detail(&mut self) {
        self.batch_draft = None;
        self.child_draft = None;
        self.history = None;
    }
}

#[derive(Debug, Default)]
pub struct WorkPlanGroup<'a> {
    pub rows: Vec<&'a Task>,
    pub progress: WorkPlanProgress,
}

#[derive(Debug, Default)]
pub struct WorkPlanProgress {
    pub total: usize,
    pub completed: usize,
    pub executing: usize,
    pub waiting: usize,
}

impl<'a> WorkPlanGroup<'a> {
    fn add(
        &mut self,
        task: &'a Task,
        snapshot: Option<&WorkPlanTask>,
        workflow: &TaskWorkflow,
        visible: bool,
    ) {
        let status = snapshot.map_or(task.status, |snapshot| snapshot.status);
        let role = workflow.status_role(status);
        self.progress.total += 1;
        self.progress.completed += usize::from(role == Some(StatusRole::Done) && !task.archived);
        self.progress.executing += usize::from(role == Some(StatusRole::Executing));
        self.progress.waiting +=
            usize::from(snapshot.is_some_and(|snapshot| snapshot.blocker.is_some()));
        if visible {
            self.rows.push(task);
        }
    }
}

#[derive(Debug)]
pub struct BatchDraft {
    pub task_id: String,
    pub original: Option<Placement>,
    pub outer: String,
    pub inner: String,
    pub pending: Option<u64>,
    pub feedback: Option<Result<(), String>>,
}

pub fn positive_batch(value: &str) -> Option<u32> {
    value.trim().parse::<u32>().ok().filter(|value| *value > 0)
}

#[derive(Debug)]
pub struct ChildTaskDraft {
    pub project: String,
    pub parent_id: String,
    pub prompt: iced::widget::text_editor::Content,
    pub batch: String,
    pub pending: bool,
    pub error: Option<String>,
    pub task: Task,
}

impl ChildTaskDraft {
    pub fn new(project: String, parent: &Task, status: desktop_task_model::TaskStatus) -> Self {
        let mut task = Task::new(parent.priority);
        task.status = status;
        task.model.clone_from(&parent.model);
        task.agent.clone_from(&parent.agent);
        task.acp_agent.clone_from(&parent.acp_agent);
        task.reasoning_effort = parent.reasoning_effort;
        task.task_split_mode = desktop_task_model::TaskSplitMode::Disabled;
        Self {
            project,
            parent_id: parent.id.clone(),
            prompt: iced::widget::text_editor::Content::new(),
            batch: "1".into(),
            pending: false,
            error: None,
            task,
        }
    }

    pub fn build(&self) -> Option<(Task, Placement)> {
        let batch = positive_batch(&self.batch)?;
        let prompt = self.prompt.text();
        let prompt = prompt.trim();
        if prompt.is_empty() {
            return None;
        }
        let mut task = self.task.clone();
        task.prompt = prompt.to_owned();
        task.description = prompt.lines().next()?.chars().take(120).collect();
        Some((task, Placement::Child { parent_task_id: self.parent_id.clone(), batch }))
    }
}

#[derive(Debug)]
pub struct RunHistory {
    pub task_id: String,
    pub request: u64,
    pub include_children: bool,
    pub result: Option<Result<Vec<AgentSchedulerTaskBoardEventDto>, String>>,
}

/// An active run keeps ownership of its completion. Resplitting is for tasks that can be started again.
pub fn can_resplit_legacy_work_plan(task: &desktop_task_model::Task, workflow: &desktop_task_model::TaskWorkflow) -> bool {
    use desktop_task_model::workflow::StatusRole;
    !task.subtasks.is_empty() && !task.archived && !task.deleted
        && matches!(
            workflow.status_role(task.status),
            Some(StatusRole::Backlog | StatusRole::Queued | StatusRole::Failed | StatusRole::Paused)
        )
}
