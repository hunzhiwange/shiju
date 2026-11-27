//! 从当前项目任务快照派生的只读工作视图，不参与存储和调度。
use chrono::{DateTime, Datelike, Days, Local, NaiveDate};
use desktop_task_model::{
    StatusRole, TASK_AGENT_MAIN, Task, TaskWorkflow, normalize_task_acp_agent_input,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use vw_api_agent_scheduler::AgentSchedulerProjectProgress;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TaskBoardViewMode {
    #[default]
    Kanban,
    List,
    Table,
    Swimlane,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkToolbarMenu {
    Display,
    Actions,
    SidebarViews,
    ViewActions,
    Filter,
    AdvancedFilter,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkFilterField {
    Status,
    AgentRunning,
    ReadState,
    Agent,
    Priority,
    Date,
    Scope(WorkScopeField),
    Label,
    Property(String),
    Advanced,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkFilterJoin {
    #[default]
    All,
    Any,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkFilterGroup {
    pub join: WorkFilterJoin,
    pub children: Vec<WorkFilterNode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkFilterNode {
    Condition(WorkFilterCondition),
    Group(WorkFilterGroup),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkFilterCondition {
    pub field: WorkFilterField,
    pub excluded: bool,
    pub rule: WorkViewFilter,
}

impl WorkFilterGroup {
    pub fn condition_count(&self) -> usize {
        self.children.iter().map(|node| match node {
            WorkFilterNode::Condition(condition) => (condition.rule != WorkViewFilter::default()) as usize,
            WorkFilterNode::Group(group) => group.condition_count(),
        }).sum()
    }

    pub fn trim_empty(&mut self) {
        self.children.retain_mut(|node| match node {
            WorkFilterNode::Condition(condition) => condition.rule != WorkViewFilter::default(),
            WorkFilterNode::Group(group) => {
                group.trim_empty();
                !group.children.is_empty()
            }
        });
    }

    fn matches(&self, task: &Task, workflow: &TaskWorkflow, context: WorkFilterContext<'_>) -> Option<bool> {
        let mut result = None;
        for node in &self.children {
            let matched = match node {
                WorkFilterNode::Condition(condition) if condition.rule != WorkViewFilter::default() => {
                    Some(condition.rule.matches(task, workflow, context) != condition.excluded)
                }
                WorkFilterNode::Condition(_) => None,
                WorkFilterNode::Group(group) => group.matches(task, workflow, context),
            };
            if let Some(matched) = matched {
                result = Some(matched);
                if matched == (self.join == WorkFilterJoin::Any) {
                    break;
                }
            }
        }
        result
    }
}

#[derive(Debug, Clone)]
pub struct WorkAdvancedFilterDraft {
    pub simple: WorkViewFilter,
    pub group: WorkFilterGroup,
    pub editing: Option<Vec<usize>>,
    pub menu: Option<Vec<usize>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkScopeField {
    Assignee,
    Creator,
    Relation,
    Project,
    ProjectStatus,
    DueDate,
}

impl WorkScopeField {
    pub fn label(self) -> &'static str {
        match self {
            Self::Assignee => "负责人",
            Self::Creator => "创建者",
            Self::Relation => "与我的关系",
            Self::Project => "项目",
            Self::ProjectStatus => "项目状态",
            Self::DueDate => "截止日期",
        }
    }
}

pub const WORK_RELATIONS: [(&str, &str); 4] = [
    ("assigned_to_me", "分配给我的"),
    ("created_by_me", "我创建的"),
    ("my_agent_running", "我的智能体在做"),
    ("subscribed", "我订阅的"),
];

pub const WORK_DUE_DATES: [(&str, &str); 5] = [
    ("overdue", "已逾期"),
    ("today", "今天截止"),
    ("this_week", "本周截止"),
    ("next_7_days", "7 天内截止"),
    ("none", "无截止日期"),
];

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkScopeSelection {
    pub values: BTreeSet<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub excluded: bool,
}

#[derive(Clone, Copy, Default)]
pub struct WorkFilterContext<'a> {
    pub unread: bool,
    pub project_id: Option<&'a str>,
    pub project_progress: Option<AgentSchedulerProjectProgress>,
    pub viewer_id: Option<&'a str>,
    pub configured_agent: bool,
    pub subscribed: bool,
}

impl WorkScopeSelection {
    fn matches(
        &self,
        field: WorkScopeField,
        task: &Task,
        workflow: &TaskWorkflow,
        context: WorkFilterContext<'_>,
    ) -> bool {
        if self.values.is_empty() {
            return true;
        }
        let matched = match field {
            WorkScopeField::Assignee => self.values.contains(task.assignee.trim()),
            WorkScopeField::Creator => {
                task.created_by.as_deref().is_some_and(|creator| self.values.contains(creator))
            }
            WorkScopeField::Project => self.values.contains(context.project_id.unwrap_or("")),
            WorkScopeField::ProjectStatus => context.project_progress
                .is_some_and(|progress| self.values.contains(progress.key())),
            WorkScopeField::DueDate => self.matches_due_date(task, Local::now().date_naive()),
            WorkScopeField::Relation => self.values.iter().any(|relation| match relation.as_str() {
                "assigned_to_me" => context.viewer_id.is_some_and(|id| id == task.assignee.trim()),
                "created_by_me" => context.viewer_id.is_some_and(|id| Some(id) == task.created_by.as_deref()),
                "my_agent_running" => {
                    context.configured_agent
                        && workflow.status_role(task.status) == Some(StatusRole::Executing)
                }
                "subscribed" => context.subscribed,
                _ => false,
            }),
        };
        matched != self.excluded
    }

    fn matches_due_date(&self, task: &Task, today: NaiveDate) -> bool {
        let Some(due) = task.due_date.as_deref().and_then(|date| date.parse::<NaiveDate>().ok()) else {
            return self.values.contains("none");
        };
        let offset = due.signed_duration_since(today).num_days();
        let weekday = i64::from(today.weekday().num_days_from_monday());
        self.values.iter().any(|bucket| match bucket.as_str() {
            "overdue" => offset < 0,
            "today" => offset == 0,
            "this_week" => (-weekday..7 - weekday).contains(&offset),
            "next_7_days" => (0..7).contains(&offset),
            _ => false,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkPriorityLevel {
    None,
    Urgent,
    High,
    Medium,
    Low,
}

impl WorkPriorityLevel {
    pub const ALL: [Self; 5] = [Self::None, Self::Urgent, Self::High, Self::Medium, Self::Low];

    pub fn label(self) -> &'static str {
        match self {
            Self::None => "无优先级",
            Self::Urgent => "紧急",
            Self::High => "高",
            Self::Medium => "中",
            Self::Low => "低",
        }
    }

    pub fn for_priority(priority: u32) -> Self {
        match priority {
            // 999 是默认任务优先级，0 也表示无优先级。
            0 | 999 => Self::None,
            1..=100 => Self::Urgent,
            101..=250 => Self::High,
            251..=500 => Self::Medium,
            _ => Self::Low,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkDateField {
    #[default]
    Created,
    Updated,
}

impl WorkDateField {
    pub fn label(self) -> &'static str {
        match self {
            Self::Created => "创建时间",
            Self::Updated => "更新时间",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkDateRange {
    Today,
    LastThreeDays,
    LastSevenDays,
    Custom { start: NaiveDate, end: NaiveDate },
}

impl WorkDateRange {
    pub const PRESETS: [Self; 3] = [Self::Today, Self::LastThreeDays, Self::LastSevenDays];

    pub fn label(self) -> &'static str {
        match self {
            Self::Today => "今天",
            Self::LastThreeDays => "最近 3 天",
            Self::LastSevenDays => "最近 7 天",
            Self::Custom { .. } => "自定义日期或范围",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkDateFilter {
    pub field: WorkDateField,
    pub range: WorkDateRange,
}

impl WorkDateFilter {
    fn matches(self, task: &Task) -> bool {
        let timestamp = match self.field {
            WorkDateField::Created => task.created_at_ms,
            WorkDateField::Updated => task.updated_at_ms,
        };
        let date = i64::try_from(timestamp)
            .ok()
            .and_then(DateTime::from_timestamp_millis)
            .map(|value| value.with_timezone(&Local).date_naive());
        let (start, end) = match self.range {
            WorkDateRange::Custom { start, end } => (start, end),
            range => {
                let today = Local::now().date_naive();
                let days_back = match range {
                    WorkDateRange::LastThreeDays => 2,
                    WorkDateRange::LastSevenDays => 6,
                    _ => 0,
                };
                (today - Days::new(days_back), today)
            }
        };
        date.is_some_and(|date| (start..=end).contains(&date))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct WorkDateCalendar {
    pub month: NaiveDate,
    pub start: Option<NaiveDate>,
    pub end: Option<NaiveDate>,
}

impl WorkFilterField {
    pub fn has_search(&self) -> bool {
        matches!(
            self,
            Self::Status
                | Self::Agent
                | Self::Label
                | Self::Scope(WorkScopeField::Assignee | WorkScopeField::Creator | WorkScopeField::Project)
        )
    }
}

pub const SIDEBAR_TASK_PAGE_SIZE: usize = 5;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SidebarWorkView {
    Builtin(BuiltinWorkView),
    Saved(String),
}

#[derive(Debug, Clone)]
pub struct WorkSidebarMenu {
    pub view: SidebarWorkView,
    pub task_id: Option<String>,
    pub position: iced::Point,
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

pub fn task_matches_search(task: &Task, query: &str) -> bool {
    let query = query.trim();
    if query.is_empty() {
        return true;
    }
    let query = query.to_lowercase();
    [&task.id, &task.description, &task.prompt]
        .into_iter()
        .any(|value| value.to_lowercase().contains(&query))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuiltinWorkView {
    MyTasks,
    AgentExecuting,
    Subscribed,
    AllTasks,
}

impl BuiltinWorkView {
    pub fn label(self) -> &'static str {
        match self {
            Self::MyTasks => "我的工作",
            Self::AgentExecuting => "运行中",
            Self::Subscribed => "已订阅",
            Self::AllTasks => "全部任务",
        }
    }

    pub fn matches(
        self,
        task: &Task,
        workflow: &TaskWorkflow,
        personal_task_ids: &[String],
        subscribed: bool,
    ) -> bool {
        if task.archived || task.deleted {
            return false;
        }
        match self {
            Self::MyTasks => personal_task_ids.contains(&task.id),
            Self::AgentExecuting => {
                workflow.status_role(task.status) == Some(StatusRole::Executing)
            }
            Self::Subscribed => subscribed,
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
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub status_excluded: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_running: Option<bool>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub unread_only: bool,
    pub priority_min: Option<u32>,
    pub priority_max: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub priority_levels: Vec<WorkPriorityLevel>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub priority_excluded: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<WorkDateFilter>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub scopes: BTreeMap<WorkScopeField, WorkScopeSelection>,
    #[serde(default)]
    pub agents: Vec<ExecutionAgent>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub label_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub labels_excluded: bool,
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub properties: vw_api_agent_scheduler::AgentSchedulerTaskPropertyValues,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub unset_properties: BTreeSet<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub advanced: Option<WorkFilterGroup>,
}

impl WorkViewFilter {
    pub fn matches(&self, task: &Task, workflow: &TaskWorkflow, context: WorkFilterContext<'_>) -> bool {
        !task.archived
            && !task.deleted
            && (self.status_keys.is_empty()
                || (workflow.status_definition(task.status)
                    .is_some_and(|definition| self.status_keys.contains(&definition.key))
                    != self.status_excluded))
            && self.agent_running.is_none_or(|running| {
                (workflow.status_role(task.status) == Some(StatusRole::Executing)) == running
            })
            && (!self.unread_only || context.unread)
            && self.priority_min.is_none_or(|min| task.priority >= min)
            && self.priority_max.is_none_or(|max| task.priority <= max)
            && (self.priority_levels.is_empty()
                || (self.priority_levels.contains(&WorkPriorityLevel::for_priority(task.priority))
                    != self.priority_excluded))
            && self.date.is_none_or(|date| date.matches(task))
            && self.scopes.iter().all(|(&field, selection)| {
                selection.matches(field, task, workflow, context)
            })
            && (self.agents.is_empty() || self.agents.iter().any(|agent| agent.matches(task)))
            && (self.label_ids.is_empty()
                || (self.label_ids.iter().any(|id| task.label_ids.contains(id)) != self.labels_excluded))
            && self.unset_properties.iter().all(|id| {
                self.properties.contains_key(id) || !task.properties.contains_key(id)
            })
            && self.properties.iter().all(|(id, expected)| {
                use vw_api_agent_scheduler::AgentSchedulerTaskPropertyValue::{MultiMember, MultiSelect};
                match (task.properties.get(id), expected) {
                    (Some(MultiSelect(actual)), MultiSelect(expected))
                    | (Some(MultiMember(actual)), MultiMember(expected)) => {
                        expected.iter().any(|value| actual.contains(value))
                    }
                    (None, _) => self.unset_properties.contains(id),
                    (actual, expected) => actual == Some(expected),
                }
            })
            && self.advanced.as_ref().and_then(|group| group.matches(task, workflow, context)).unwrap_or(true)
    }

    pub fn remove(&mut self, field: &WorkFilterField) {
        match field {
            WorkFilterField::Status => {
                self.status_keys.clear();
                self.status_excluded = false;
            }
            WorkFilterField::AgentRunning => self.agent_running = None,
            WorkFilterField::ReadState => self.unread_only = false,
            WorkFilterField::Agent => self.agents.clear(),
            WorkFilterField::Priority => {
                self.priority_min = None;
                self.priority_max = None;
                self.priority_levels.clear();
                self.priority_excluded = false;
            }
            WorkFilterField::Date => self.date = None,
            WorkFilterField::Scope(field) => {
                self.scopes.remove(field);
            }
            WorkFilterField::Label => {
                self.label_ids.clear();
                self.labels_excluded = false;
            }
            WorkFilterField::Property(id) => {
                self.properties.remove(id);
                self.unset_properties.remove(id);
            }
            WorkFilterField::Advanced => self.advanced = None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedWorkView {
    pub id: String,
    pub name: String,
    pub filter: WorkViewFilter,
}

impl SavedWorkView {
    pub fn matches(&self, task: &Task, workflow: &TaskWorkflow, context: WorkFilterContext<'_>) -> bool {
        self.filter.matches(task, workflow, context)
    }
}
