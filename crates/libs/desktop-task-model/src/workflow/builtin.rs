//! 内置工作流目录与稳定状态定义。
use super::{StatusRole, TaskStatusDefinition, DEFAULT_TASK_WORKFLOW_ID, TaskWorkflowDefinition};
use crate::models::TaskStatus;

mod content_publishing;
use content_publishing::CONTENT_PUBLISHING_WORKFLOW;
mod marketing_campaign;
use marketing_campaign::MARKETING_CAMPAIGN_WORKFLOW;
mod customer_support;
use customer_support::CUSTOMER_SUPPORT_WORKFLOW;
mod recruiting;
use recruiting::RECRUITING_WORKFLOW;

/// 默认工作流的有序状态定义。
///
/// 顺序、key、label 与 `TaskStatus` 现有契约完全一致；`next` 保留现有
/// `Pool -> Pending -> Planning -> Running -> CodeComplete -> CodeReview ->
/// PrSubmitted -> Completed -> Archived` 主路径，以及 `Failed -> Pending` 重试分支与
/// `Paused` 末端。自动执行候选排除终态、暂停态。
const DEV_LIFECYCLE_STATUSES: &[TaskStatusDefinition] = &[
    TaskStatusDefinition {
        status: TaskStatus::Pool,
        role: StatusRole::Backlog,
        key: "pool",
        label: "任务池",
        is_terminal: false,
        auto_execution_candidate: true,
        occupies_execution_slot: false,
        is_scheduling_backlog: true,
        next: Some(TaskStatus::Pending),
    },
    TaskStatusDefinition {
        status: TaskStatus::Pending,
        role: StatusRole::Queued,
        key: "pending",
        label: "待执行",
        is_terminal: false,
        auto_execution_candidate: true,
        occupies_execution_slot: false,
        is_scheduling_backlog: true,
        next: Some(TaskStatus::Planning),
    },
    TaskStatusDefinition {
        status: TaskStatus::Planning,
        role: StatusRole::Planning,
        key: "planning",
        label: "任务拆分",
        is_terminal: false,
        auto_execution_candidate: true,
        occupies_execution_slot: true,
        is_scheduling_backlog: true,
        next: Some(TaskStatus::Running),
    },
    TaskStatusDefinition {
        status: TaskStatus::Running,
        role: StatusRole::Executing,
        key: "running",
        label: "执行中",
        is_terminal: false,
        auto_execution_candidate: true,
        occupies_execution_slot: true,
        is_scheduling_backlog: false,
        next: Some(TaskStatus::CodeComplete),
    },
    TaskStatusDefinition {
        status: TaskStatus::Failed,
        role: StatusRole::Failed,
        key: "failed",
        label: "执行失败",
        is_terminal: false,
        auto_execution_candidate: true,
        occupies_execution_slot: false,
        is_scheduling_backlog: false,
        next: Some(TaskStatus::Pending),
    },
    TaskStatusDefinition {
        status: TaskStatus::Paused,
        role: StatusRole::Paused,
        key: "paused",
        label: "暂停中",
        is_terminal: true,
        auto_execution_candidate: false,
        occupies_execution_slot: false,
        is_scheduling_backlog: false,
        next: None,
    },
    TaskStatusDefinition {
        status: TaskStatus::CodeComplete,
        role: StatusRole::ReviewReady,
        key: "code_complete",
        label: "代码完成",
        is_terminal: false,
        auto_execution_candidate: true,
        occupies_execution_slot: false,
        is_scheduling_backlog: false,
        next: Some(TaskStatus::CodeReview),
    },
    TaskStatusDefinition {
        status: TaskStatus::CodeReview,
        role: StatusRole::Review,
        key: "code_review",
        label: "代码审核",
        is_terminal: false,
        auto_execution_candidate: true,
        occupies_execution_slot: false,
        is_scheduling_backlog: false,
        next: Some(TaskStatus::PrSubmitted),
    },
    TaskStatusDefinition {
        status: TaskStatus::PrSubmitted,
        role: StatusRole::Merging,
        key: "pr_submitted",
        label: "合并代码",
        is_terminal: false,
        auto_execution_candidate: true,
        occupies_execution_slot: false,
        is_scheduling_backlog: false,
        next: Some(TaskStatus::Completed),
    },
    TaskStatusDefinition {
        status: TaskStatus::Completed,
        role: StatusRole::Done,
        key: "completed",
        label: "任务完成",
        is_terminal: false,
        auto_execution_candidate: false,
        occupies_execution_slot: false,
        is_scheduling_backlog: false,
        next: Some(TaskStatus::Archived),
    },
    TaskStatusDefinition {
        status: TaskStatus::Archived,
        role: StatusRole::Archived,
        key: "archived",
        label: "任务归档",
        is_terminal: true,
        auto_execution_candidate: false,
        occupies_execution_slot: false,
        is_scheduling_backlog: false,
        next: None,
    },
];

pub(super) const DEV_LIFECYCLE_WORKFLOW: TaskWorkflowDefinition = TaskWorkflowDefinition {
    id: "dev_lifecycle",
    label: "开发生命周期",
    statuses: DEV_LIFECYCLE_STATUSES,
};

const GENERIC_TASK_STATUSES: &[TaskStatusDefinition] = &[
    DEV_LIFECYCLE_STATUSES[0],
    TaskStatusDefinition { next: Some(TaskStatus::Running), ..DEV_LIFECYCLE_STATUSES[1] },
    DEV_LIFECYCLE_STATUSES[2],
    TaskStatusDefinition { next: Some(TaskStatus::Completed), ..DEV_LIFECYCLE_STATUSES[3] },
    DEV_LIFECYCLE_STATUSES[4],
    DEV_LIFECYCLE_STATUSES[5],
    TaskStatusDefinition { is_terminal: true, next: None, ..DEV_LIFECYCLE_STATUSES[9] },
    DEV_LIFECYCLE_STATUSES[10],
];
const GENERIC_TASK_WORKFLOW: TaskWorkflowDefinition = TaskWorkflowDefinition {
    id: DEFAULT_TASK_WORKFLOW_ID,
    label: "通用任务",
    statuses: GENERIC_TASK_STATUSES,
};

const BUILTIN_TASK_WORKFLOWS: &[&TaskWorkflowDefinition] = &[
    &GENERIC_TASK_WORKFLOW,
    &DEV_LIFECYCLE_WORKFLOW,
    &CONTENT_PUBLISHING_WORKFLOW,
    &MARKETING_CAMPAIGN_WORKFLOW,
    &CUSTOMER_SUPPORT_WORKFLOW,
    &RECRUITING_WORKFLOW,
];

/// 返回默认任务工作流定义。
pub fn default_task_workflow() -> &'static TaskWorkflowDefinition {
    &GENERIC_TASK_WORKFLOW
}

/// 返回所有内置任务工作流定义。
pub fn builtin_task_workflows() -> &'static [&'static TaskWorkflowDefinition] {
    BUILTIN_TASK_WORKFLOWS
}

/// 返回内置枚举的完整稳定目录，供 `TaskStatus::all` 委托；不随项目默认流程缩减。
pub fn default_task_workflow_statuses() -> &'static [TaskStatusDefinition] {
    DEV_LIFECYCLE_STATUSES
}
