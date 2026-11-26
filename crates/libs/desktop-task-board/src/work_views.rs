//! 从当前项目任务快照派生的只读工作视图，不参与存储和调度。
use desktop_task_model::{
    StatusRole, TASK_AGENT_MAIN, Task, TaskWorkflow, normalize_task_acp_agent_input,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TaskBoardViewMode {
    #[default]
    Kanban,
    List,
    Table,
    Swimlane,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WorkSort {
    #[default]
    Created,
    Updated,
    Priority,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LaneGrouping {
    Assignee,
    Agent,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum WorkDisplayGroup {
    Status(desktop_task_model::TaskStatus),
    Lane(LaneGrouping, String),
}

#[derive(Debug, Clone, Copy)]
pub struct WorkDisplay {
    pub sort: WorkSort,
    pub ascending: bool,
    pub table_group_status: bool,
    pub lane_grouping: LaneGrouping,
    pub hide_empty_statuses: bool,
}

impl Default for WorkDisplay {
    fn default() -> Self {
        Self {
            sort: WorkSort::Created,
            ascending: false,
            table_group_status: false,
            lane_grouping: LaneGrouping::Assignee,
            hide_empty_statuses: false,
        }
    }
}

pub(crate) fn task_matches_search(task: &Task, query: &str) -> bool {
    let query = query.trim();
    if query.is_empty() {
        return true;
    }
    let query = query.to_lowercase();
    [&task.id, &task.description, &task.prompt]
        .into_iter()
        .any(|value| value.to_lowercase().contains(&query))
}

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
            && self.properties.iter().all(|(id, expected)| {
                use vw_api_agent_scheduler::AgentSchedulerTaskPropertyValue::{MultiMember, MultiSelect};
                match (task.properties.get(id), expected) {
                    (Some(MultiSelect(actual)), MultiSelect(expected))
                    | (Some(MultiMember(actual)), MultiMember(expected)) => {
                        expected.iter().any(|value| actual.contains(value))
                    }
                    (actual, expected) => actual == Some(expected),
                }
            })
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
