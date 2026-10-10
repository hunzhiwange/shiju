//! 任务状态工作流元数据与注册层。
//!
//! 本模块在不改变持久化 `TaskStatus` 枚举的前提下，承载状态顺序、标签、key、
//! 终态标记、自动执行候选分类与默认下一状态等元数据，并以注册式提供工作流定义。
//! `TaskStatus` 的查询方法委托到默认工作流，保证 key、label、顺序与转移语义单点定义。

#[path = "workflow/registry.rs"]
pub(crate) mod registry;

use std::collections::HashSet;

use crate::app::task::models::TaskStatus;
pub(crate) use registry::TaskWorkflowRegistry;
pub use registry::{
    OwnedStatusDefinition, OwnedStatusTransition, OwnedTaskWorkflow, StatusOutcome, TaskWorkflow,
    all_task_workflows, is_builtin_task_workflow_id, task_workflow_by_id,
    update_runtime_task_lifecycle_cache,
};
#[cfg(test)]
pub(crate) use registry::{
    register_runtime_task_workflow, replace_runtime_task_lifecycle_workflows_for_tests,
    reset_runtime_task_lifecycle_cache, runtime_task_lifecycle_cache_test_lock,
};
use vw_gateway_client::vw_api_types::task::{
    TaskPoolWorkflowSnapshotDto, TaskPoolWorkflowStatusDto, TaskPoolWorkflowStatusRoleDto,
    TaskPoolWorkflowTransitionDto,
};

/// 默认任务工作流 id，表达现有开发生命周期语义。
pub const DEFAULT_TASK_WORKFLOW_ID: &str = "dev_lifecycle";

/// 状态的语义角色，表达引擎对该状态的行为约定，与具体枚举变体解耦。
///
/// 执行器、调度器、重试与超时逻辑后续将依赖角色而非具体 `TaskStatus` 变体，
/// 使用户自定义状态能在绑定已知角色后获得确定的引擎语义；`Custom` 预留给
/// 无引擎语义的纯展示状态，引擎不对其施加任何自动行为。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatusRole {
    /// 待规划的积压任务（任务池）。
    Backlog,
    /// 等待进入执行的排队任务。
    Queued,
    /// 正在拆分规划。
    Planning,
    /// 正在执行，占用并发执行槽位。
    Executing,
    /// 执行失败，可重试。
    Failed,
    /// 暂停，不参与自动调度。
    Paused,
    /// 执行产物就绪、等待进入审核。
    ReviewReady,
    /// 正在代码审核。
    Review,
    /// 正在合并代码。
    Merging,
    /// 已完成。
    Done,
    /// 已归档（终态）。
    Archived,
    /// 无引擎语义的纯展示自定义状态。
    Custom,
}

/// 单个任务状态的元数据定义。
///
/// 描述状态在工作流中的稳定属性：持久化枚举、序列化 key、展示标签、是否终态、
/// 是否参与自动执行候选分类、本地调度统计标记，以及默认线性流转的下一状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaskStatusDefinition {
    /// 对应的持久化状态枚举。
    pub status: TaskStatus,
    /// 状态的语义角色，引擎据此施加行为约定，与具体变体解耦。
    pub role: StatusRole,
    /// 稳定的序列化 key。
    pub key: &'static str,
    /// 面向用户的展示标签。
    pub label: &'static str,
    /// 是否为终态（没有默认下一状态）。
    pub is_terminal: bool,
    /// 是否参与自动执行候选分类。
    pub auto_execution_candidate: bool,
    /// 是否占用并发执行槽位（计入“运行中”统计）。
    pub occupies_execution_slot: bool,
    /// 是否为等待进入执行的排队任务（计入“池+待执行”容量统计）。
    pub is_scheduling_backlog: bool,
    /// 默认线性流转的下一状态，终态或分支末端为 `None`。
    pub next: Option<TaskStatus>,
}

/// 任务工作流定义，承载有序的状态元数据集合。
#[derive(Debug, Clone, Copy)]
pub struct TaskWorkflowDefinition {
    /// 工作流稳定 id。
    pub id: &'static str,
    /// 工作流展示名称。
    pub label: &'static str,
    /// 有序的状态定义集合，顺序与 `TaskStatus::all` 一致。
    pub statuses: &'static [TaskStatusDefinition],
}

impl TaskWorkflowDefinition {
    /// 按状态枚举查找其元数据定义。
    pub fn status_definition(&self, status: TaskStatus) -> Option<&TaskStatusDefinition> {
        self.statuses.iter().find(|definition| definition.status == status)
    }

    /// 返回给定状态的默认下一状态。
    pub fn next_status(&self, status: TaskStatus) -> Option<TaskStatus> {
        self.status_definition(status).and_then(|definition| definition.next)
    }

    /// 返回给定状态在本工作流中的语义角色。
    pub fn status_role(&self, status: TaskStatus) -> Option<StatusRole> {
        self.status_definition(status).map(|definition| definition.role)
    }

    /// 判断给定状态是否占用并发执行槽位，供本地调度运行中统计。
    pub fn occupies_execution_slot(&self, status: TaskStatus) -> bool {
        self.status_definition(status).is_some_and(|definition| definition.occupies_execution_slot)
    }

    /// 判断给定状态是否为等待进入执行的排队任务，供本地调度容量统计。
    pub fn is_scheduling_backlog(&self, status: TaskStatus) -> bool {
        self.status_definition(status).is_some_and(|definition| definition.is_scheduling_backlog)
    }

    /// 按语义角色推导给定状态是否占用并发执行槽位。
    ///
    /// 占槽角色为 `Executing` 与 `Planning`（拆分阶段已起进程占用并发）。
    /// 引擎据角色而非具体变体判定，使绑定这些角色的自定义状态获得相同语义。
    pub fn role_occupies_execution_slot(&self, status: TaskStatus) -> bool {
        matches!(self.status_role(status), Some(StatusRole::Executing | StatusRole::Planning))
    }

    /// 按语义角色推导给定状态是否为等待进入执行的调度积压。
    ///
    /// 积压角色为 `Backlog`、`Queued` 与 `Planning`（任务池、待执行、拆分中都计入
    /// “池+待执行”容量）。引擎据角色判定，与具体变体解耦。
    pub fn role_is_scheduling_backlog(&self, status: TaskStatus) -> bool {
        matches!(
            self.status_role(status),
            Some(StatusRole::Backlog | StatusRole::Queued | StatusRole::Planning)
        )
    }

    /// 按语义角色推导给定状态是否为失败重试候选。
    ///
    /// 重试角色为 `Failed`：仅失败态可被重新拉起执行。引擎据角色判定，
    /// 使绑定 `Failed` 角色的自定义状态同样进入重试流程。
    pub fn role_is_retry_candidate(&self, status: TaskStatus) -> bool {
        matches!(self.status_role(status), Some(StatusRole::Failed))
    }

    /// 按语义角色推导给定状态是否为运行超时监控对象。
    ///
    /// 超时角色为 `Executing`：仅执行中状态参与运行超时监控。引擎据角色判定，
    /// 使绑定 `Executing` 角色的自定义状态同样受超时约束。
    pub fn role_is_timeout_monitored(&self, status: TaskStatus) -> bool {
        matches!(self.status_role(status), Some(StatusRole::Executing))
    }
}

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

const DEV_LIFECYCLE_WORKFLOW: TaskWorkflowDefinition = TaskWorkflowDefinition {
    id: DEFAULT_TASK_WORKFLOW_ID,
    label: "开发生命周期",
    statuses: DEV_LIFECYCLE_STATUSES,
};

const CONTENT_PUBLISHING_STATUSES: &[TaskStatusDefinition] = &[
    TaskStatusDefinition {
        status: TaskStatus::Custom("content_idea"),
        role: StatusRole::Backlog,
        key: "content_idea",
        label: "选题池",
        is_terminal: false,
        auto_execution_candidate: true,
        occupies_execution_slot: false,
        is_scheduling_backlog: true,
        next: Some(TaskStatus::Custom("content_queued")),
    },
    TaskStatusDefinition {
        status: TaskStatus::Custom("content_queued"),
        role: StatusRole::Queued,
        key: "content_queued",
        label: "待写作",
        is_terminal: false,
        auto_execution_candidate: true,
        occupies_execution_slot: false,
        is_scheduling_backlog: true,
        next: Some(TaskStatus::Custom("content_outline")),
    },
    TaskStatusDefinition {
        status: TaskStatus::Custom("content_outline"),
        role: StatusRole::Planning,
        key: "content_outline",
        label: "资料与大纲",
        is_terminal: false,
        auto_execution_candidate: true,
        occupies_execution_slot: true,
        is_scheduling_backlog: true,
        next: Some(TaskStatus::Custom("content_writing")),
    },
    TaskStatusDefinition {
        status: TaskStatus::Custom("content_writing"),
        role: StatusRole::Executing,
        key: "content_writing",
        label: "写作中",
        is_terminal: false,
        auto_execution_candidate: true,
        occupies_execution_slot: true,
        is_scheduling_backlog: false,
        next: Some(TaskStatus::Custom("content_review_ready")),
    },
    TaskStatusDefinition {
        status: TaskStatus::Custom("content_revision"),
        role: StatusRole::Failed,
        key: "content_revision",
        label: "退回修改",
        is_terminal: false,
        auto_execution_candidate: true,
        occupies_execution_slot: false,
        is_scheduling_backlog: false,
        next: Some(TaskStatus::Custom("content_queued")),
    },
    TaskStatusDefinition {
        status: TaskStatus::Custom("content_paused"),
        role: StatusRole::Paused,
        key: "content_paused",
        label: "暂缓发布",
        is_terminal: true,
        auto_execution_candidate: false,
        occupies_execution_slot: false,
        is_scheduling_backlog: false,
        next: None,
    },
    TaskStatusDefinition {
        status: TaskStatus::Custom("content_review_ready"),
        role: StatusRole::ReviewReady,
        key: "content_review_ready",
        label: "待审稿",
        is_terminal: false,
        auto_execution_candidate: true,
        occupies_execution_slot: false,
        is_scheduling_backlog: false,
        next: Some(TaskStatus::Custom("content_reviewing")),
    },
    TaskStatusDefinition {
        status: TaskStatus::Custom("content_reviewing"),
        role: StatusRole::Review,
        key: "content_reviewing",
        label: "审稿中",
        is_terminal: false,
        auto_execution_candidate: true,
        occupies_execution_slot: false,
        is_scheduling_backlog: false,
        next: Some(TaskStatus::Custom("content_publishing")),
    },
    TaskStatusDefinition {
        status: TaskStatus::Custom("content_publishing"),
        role: StatusRole::Merging,
        key: "content_publishing",
        label: "多平台发布",
        is_terminal: false,
        auto_execution_candidate: true,
        occupies_execution_slot: false,
        is_scheduling_backlog: false,
        next: Some(TaskStatus::Custom("content_published")),
    },
    TaskStatusDefinition {
        status: TaskStatus::Custom("content_published"),
        role: StatusRole::Done,
        key: "content_published",
        label: "已发布",
        is_terminal: false,
        auto_execution_candidate: false,
        occupies_execution_slot: false,
        is_scheduling_backlog: false,
        next: Some(TaskStatus::Custom("content_archived")),
    },
    TaskStatusDefinition {
        status: TaskStatus::Custom("content_archived"),
        role: StatusRole::Archived,
        key: "content_archived",
        label: "归档复盘",
        is_terminal: true,
        auto_execution_candidate: false,
        occupies_execution_slot: false,
        is_scheduling_backlog: false,
        next: None,
    },
];

const CONTENT_PUBLISHING_WORKFLOW: TaskWorkflowDefinition = TaskWorkflowDefinition {
    id: "content_publishing",
    label: "内容创作与多平台发布",
    statuses: CONTENT_PUBLISHING_STATUSES,
};

const MARKETING_CAMPAIGN_STATUSES: &[TaskStatusDefinition] = &[
    TaskStatusDefinition {
        status: TaskStatus::Custom("campaign_ideas"),
        role: StatusRole::Backlog,
        key: "campaign_ideas",
        label: "活动想法",
        is_terminal: false,
        auto_execution_candidate: true,
        occupies_execution_slot: false,
        is_scheduling_backlog: true,
        next: Some(TaskStatus::Custom("campaign_queued")),
    },
    TaskStatusDefinition {
        status: TaskStatus::Custom("campaign_queued"),
        role: StatusRole::Queued,
        key: "campaign_queued",
        label: "待策划",
        is_terminal: false,
        auto_execution_candidate: true,
        occupies_execution_slot: false,
        is_scheduling_backlog: true,
        next: Some(TaskStatus::Custom("campaign_planning")),
    },
    TaskStatusDefinition {
        status: TaskStatus::Custom("campaign_planning"),
        role: StatusRole::Planning,
        key: "campaign_planning",
        label: "方案设计",
        is_terminal: false,
        auto_execution_candidate: true,
        occupies_execution_slot: true,
        is_scheduling_backlog: true,
        next: Some(TaskStatus::Custom("campaign_running")),
    },
    TaskStatusDefinition {
        status: TaskStatus::Custom("campaign_running"),
        role: StatusRole::Executing,
        key: "campaign_running",
        label: "执行投放",
        is_terminal: false,
        auto_execution_candidate: true,
        occupies_execution_slot: true,
        is_scheduling_backlog: false,
        next: Some(TaskStatus::Custom("campaign_review_ready")),
    },
    TaskStatusDefinition {
        status: TaskStatus::Custom("campaign_blocked"),
        role: StatusRole::Failed,
        key: "campaign_blocked",
        label: "投放异常",
        is_terminal: false,
        auto_execution_candidate: true,
        occupies_execution_slot: false,
        is_scheduling_backlog: false,
        next: Some(TaskStatus::Custom("campaign_queued")),
    },
    TaskStatusDefinition {
        status: TaskStatus::Custom("campaign_paused"),
        role: StatusRole::Paused,
        key: "campaign_paused",
        label: "暂停活动",
        is_terminal: true,
        auto_execution_candidate: false,
        occupies_execution_slot: false,
        is_scheduling_backlog: false,
        next: None,
    },
    TaskStatusDefinition {
        status: TaskStatus::Custom("campaign_review_ready"),
        role: StatusRole::ReviewReady,
        key: "campaign_review_ready",
        label: "待复盘",
        is_terminal: false,
        auto_execution_candidate: true,
        occupies_execution_slot: false,
        is_scheduling_backlog: false,
        next: Some(TaskStatus::Custom("campaign_reviewing")),
    },
    TaskStatusDefinition {
        status: TaskStatus::Custom("campaign_reviewing"),
        role: StatusRole::Review,
        key: "campaign_reviewing",
        label: "复盘中",
        is_terminal: false,
        auto_execution_candidate: true,
        occupies_execution_slot: false,
        is_scheduling_backlog: false,
        next: Some(TaskStatus::Custom("campaign_closing")),
    },
    TaskStatusDefinition {
        status: TaskStatus::Custom("campaign_closing"),
        role: StatusRole::Merging,
        key: "campaign_closing",
        label: "结案同步",
        is_terminal: false,
        auto_execution_candidate: true,
        occupies_execution_slot: false,
        is_scheduling_backlog: false,
        next: Some(TaskStatus::Custom("campaign_done")),
    },
    TaskStatusDefinition {
        status: TaskStatus::Custom("campaign_done"),
        role: StatusRole::Done,
        key: "campaign_done",
        label: "已结案",
        is_terminal: false,
        auto_execution_candidate: false,
        occupies_execution_slot: false,
        is_scheduling_backlog: false,
        next: Some(TaskStatus::Custom("campaign_archived")),
    },
    TaskStatusDefinition {
        status: TaskStatus::Custom("campaign_archived"),
        role: StatusRole::Archived,
        key: "campaign_archived",
        label: "资料归档",
        is_terminal: true,
        auto_execution_candidate: false,
        occupies_execution_slot: false,
        is_scheduling_backlog: false,
        next: None,
    },
];

const MARKETING_CAMPAIGN_WORKFLOW: TaskWorkflowDefinition = TaskWorkflowDefinition {
    id: "marketing_campaign",
    label: "营销活动执行",
    statuses: MARKETING_CAMPAIGN_STATUSES,
};

const CUSTOMER_SUPPORT_STATUSES: &[TaskStatusDefinition] = &[
    TaskStatusDefinition {
        status: TaskStatus::Custom("support_new"),
        role: StatusRole::Backlog,
        key: "support_new",
        label: "新工单",
        is_terminal: false,
        auto_execution_candidate: true,
        occupies_execution_slot: false,
        is_scheduling_backlog: true,
        next: Some(TaskStatus::Custom("support_queued")),
    },
    TaskStatusDefinition {
        status: TaskStatus::Custom("support_queued"),
        role: StatusRole::Queued,
        key: "support_queued",
        label: "待处理",
        is_terminal: false,
        auto_execution_candidate: true,
        occupies_execution_slot: false,
        is_scheduling_backlog: true,
        next: Some(TaskStatus::Custom("support_triage")),
    },
    TaskStatusDefinition {
        status: TaskStatus::Custom("support_triage"),
        role: StatusRole::Planning,
        key: "support_triage",
        label: "问题定位",
        is_terminal: false,
        auto_execution_candidate: true,
        occupies_execution_slot: true,
        is_scheduling_backlog: true,
        next: Some(TaskStatus::Custom("support_handling")),
    },
    TaskStatusDefinition {
        status: TaskStatus::Custom("support_handling"),
        role: StatusRole::Executing,
        key: "support_handling",
        label: "处理中",
        is_terminal: false,
        auto_execution_candidate: true,
        occupies_execution_slot: true,
        is_scheduling_backlog: false,
        next: Some(TaskStatus::Custom("support_followup_ready")),
    },
    TaskStatusDefinition {
        status: TaskStatus::Custom("support_blocked"),
        role: StatusRole::Failed,
        key: "support_blocked",
        label: "处理受阻",
        is_terminal: false,
        auto_execution_candidate: true,
        occupies_execution_slot: false,
        is_scheduling_backlog: false,
        next: Some(TaskStatus::Custom("support_queued")),
    },
    TaskStatusDefinition {
        status: TaskStatus::Custom("support_waiting"),
        role: StatusRole::Paused,
        key: "support_waiting",
        label: "等待客户",
        is_terminal: true,
        auto_execution_candidate: false,
        occupies_execution_slot: false,
        is_scheduling_backlog: false,
        next: None,
    },
    TaskStatusDefinition {
        status: TaskStatus::Custom("support_followup_ready"),
        role: StatusRole::ReviewReady,
        key: "support_followup_ready",
        label: "待回访",
        is_terminal: false,
        auto_execution_candidate: true,
        occupies_execution_slot: false,
        is_scheduling_backlog: false,
        next: Some(TaskStatus::Custom("support_following")),
    },
    TaskStatusDefinition {
        status: TaskStatus::Custom("support_following"),
        role: StatusRole::Review,
        key: "support_following",
        label: "回访中",
        is_terminal: false,
        auto_execution_candidate: true,
        occupies_execution_slot: false,
        is_scheduling_backlog: false,
        next: Some(TaskStatus::Custom("support_closing")),
    },
    TaskStatusDefinition {
        status: TaskStatus::Custom("support_closing"),
        role: StatusRole::Merging,
        key: "support_closing",
        label: "关闭确认",
        is_terminal: false,
        auto_execution_candidate: true,
        occupies_execution_slot: false,
        is_scheduling_backlog: false,
        next: Some(TaskStatus::Custom("support_resolved")),
    },
    TaskStatusDefinition {
        status: TaskStatus::Custom("support_resolved"),
        role: StatusRole::Done,
        key: "support_resolved",
        label: "已解决",
        is_terminal: false,
        auto_execution_candidate: false,
        occupies_execution_slot: false,
        is_scheduling_backlog: false,
        next: Some(TaskStatus::Custom("support_archived")),
    },
    TaskStatusDefinition {
        status: TaskStatus::Custom("support_archived"),
        role: StatusRole::Archived,
        key: "support_archived",
        label: "工单归档",
        is_terminal: true,
        auto_execution_candidate: false,
        occupies_execution_slot: false,
        is_scheduling_backlog: false,
        next: None,
    },
];

const CUSTOMER_SUPPORT_WORKFLOW: TaskWorkflowDefinition = TaskWorkflowDefinition {
    id: "customer_support",
    label: "客服工单处理",
    statuses: CUSTOMER_SUPPORT_STATUSES,
};

const RECRUITING_STATUSES: &[TaskStatusDefinition] = &[
    TaskStatusDefinition {
        status: TaskStatus::Custom("recruiting_pool"),
        role: StatusRole::Backlog,
        key: "recruiting_pool",
        label: "人才池",
        is_terminal: false,
        auto_execution_candidate: true,
        occupies_execution_slot: false,
        is_scheduling_backlog: true,
        next: Some(TaskStatus::Custom("recruiting_queued")),
    },
    TaskStatusDefinition {
        status: TaskStatus::Custom("recruiting_queued"),
        role: StatusRole::Queued,
        key: "recruiting_queued",
        label: "待筛选",
        is_terminal: false,
        auto_execution_candidate: true,
        occupies_execution_slot: false,
        is_scheduling_backlog: true,
        next: Some(TaskStatus::Custom("recruiting_screening")),
    },
    TaskStatusDefinition {
        status: TaskStatus::Custom("recruiting_screening"),
        role: StatusRole::Planning,
        key: "recruiting_screening",
        label: "简历初筛",
        is_terminal: false,
        auto_execution_candidate: true,
        occupies_execution_slot: true,
        is_scheduling_backlog: true,
        next: Some(TaskStatus::Custom("recruiting_interviewing")),
    },
    TaskStatusDefinition {
        status: TaskStatus::Custom("recruiting_interviewing"),
        role: StatusRole::Executing,
        key: "recruiting_interviewing",
        label: "面试中",
        is_terminal: false,
        auto_execution_candidate: true,
        occupies_execution_slot: true,
        is_scheduling_backlog: false,
        next: Some(TaskStatus::Custom("recruiting_review_ready")),
    },
    TaskStatusDefinition {
        status: TaskStatus::Custom("recruiting_blocked"),
        role: StatusRole::Failed,
        key: "recruiting_blocked",
        label: "流程异常",
        is_terminal: false,
        auto_execution_candidate: true,
        occupies_execution_slot: false,
        is_scheduling_backlog: false,
        next: Some(TaskStatus::Custom("recruiting_queued")),
    },
    TaskStatusDefinition {
        status: TaskStatus::Custom("recruiting_on_hold"),
        role: StatusRole::Paused,
        key: "recruiting_on_hold",
        label: "暂缓",
        is_terminal: true,
        auto_execution_candidate: false,
        occupies_execution_slot: false,
        is_scheduling_backlog: false,
        next: None,
    },
    TaskStatusDefinition {
        status: TaskStatus::Custom("recruiting_review_ready"),
        role: StatusRole::ReviewReady,
        key: "recruiting_review_ready",
        label: "待评审",
        is_terminal: false,
        auto_execution_candidate: true,
        occupies_execution_slot: false,
        is_scheduling_backlog: false,
        next: Some(TaskStatus::Custom("recruiting_reviewing")),
    },
    TaskStatusDefinition {
        status: TaskStatus::Custom("recruiting_reviewing"),
        role: StatusRole::Review,
        key: "recruiting_reviewing",
        label: "评审中",
        is_terminal: false,
        auto_execution_candidate: true,
        occupies_execution_slot: false,
        is_scheduling_backlog: false,
        next: Some(TaskStatus::Custom("recruiting_offer")),
    },
    TaskStatusDefinition {
        status: TaskStatus::Custom("recruiting_offer"),
        role: StatusRole::Merging,
        key: "recruiting_offer",
        label: "Offer 跟进",
        is_terminal: false,
        auto_execution_candidate: true,
        occupies_execution_slot: false,
        is_scheduling_backlog: false,
        next: Some(TaskStatus::Custom("recruiting_hired")),
    },
    TaskStatusDefinition {
        status: TaskStatus::Custom("recruiting_hired"),
        role: StatusRole::Done,
        key: "recruiting_hired",
        label: "已入职",
        is_terminal: false,
        auto_execution_candidate: false,
        occupies_execution_slot: false,
        is_scheduling_backlog: false,
        next: Some(TaskStatus::Custom("recruiting_archived")),
    },
    TaskStatusDefinition {
        status: TaskStatus::Custom("recruiting_archived"),
        role: StatusRole::Archived,
        key: "recruiting_archived",
        label: "候选人归档",
        is_terminal: true,
        auto_execution_candidate: false,
        occupies_execution_slot: false,
        is_scheduling_backlog: false,
        next: None,
    },
];

const RECRUITING_WORKFLOW: TaskWorkflowDefinition = TaskWorkflowDefinition {
    id: "recruiting_pipeline",
    label: "招聘流程",
    statuses: RECRUITING_STATUSES,
};

const BUILTIN_TASK_WORKFLOWS: &[&TaskWorkflowDefinition] = &[
    &DEV_LIFECYCLE_WORKFLOW,
    &CONTENT_PUBLISHING_WORKFLOW,
    &MARKETING_CAMPAIGN_WORKFLOW,
    &CUSTOMER_SUPPORT_WORKFLOW,
    &RECRUITING_WORKFLOW,
];

/// 返回默认任务工作流定义。
pub fn default_task_workflow() -> &'static TaskWorkflowDefinition {
    &DEV_LIFECYCLE_WORKFLOW
}

/// 返回所有内置任务工作流定义。
pub fn builtin_task_workflows() -> &'static [&'static TaskWorkflowDefinition] {
    BUILTIN_TASK_WORKFLOWS
}

/// 返回默认工作流中状态的有序集合，供 `TaskStatus::all` 委托。
pub fn default_task_workflow_statuses() -> &'static [TaskStatusDefinition] {
    DEV_LIFECYCLE_STATUSES
}

/// 将桌面任务工作流快照转换为 gateway 调度 API DTO。
pub fn task_workflow_snapshot_for_gateway(workflow: &TaskWorkflow) -> TaskPoolWorkflowSnapshotDto {
    TaskPoolWorkflowSnapshotDto {
        id: workflow.id().to_string(),
        label: workflow.label().to_string(),
        statuses: workflow
            .statuses()
            .iter()
            .map(|status| TaskPoolWorkflowStatusDto {
                key: status.key().to_string(),
                label: status.label().to_string(),
                role: status_role_for_gateway(status.role()),
                enabled: status.enabled,
                terminal: status.is_terminal,
                next: status.next.map(|target| workflow_status_key(workflow, target)),
                transitions: status
                    .transitions
                    .iter()
                    .map(|transition| TaskPoolWorkflowTransitionDto {
                        outcome: transition.outcome.key().to_string(),
                        label: transition.label.clone(),
                        target: workflow_status_key(workflow, transition.target),
                    })
                    .collect(),
            })
            .collect(),
    }
}

fn status_role_for_gateway(role: StatusRole) -> TaskPoolWorkflowStatusRoleDto {
    match role {
        StatusRole::Backlog => TaskPoolWorkflowStatusRoleDto::Backlog,
        StatusRole::Queued => TaskPoolWorkflowStatusRoleDto::Queued,
        StatusRole::Planning => TaskPoolWorkflowStatusRoleDto::Planning,
        StatusRole::Executing => TaskPoolWorkflowStatusRoleDto::Executing,
        StatusRole::Failed => TaskPoolWorkflowStatusRoleDto::Failed,
        StatusRole::Paused => TaskPoolWorkflowStatusRoleDto::Paused,
        StatusRole::ReviewReady => TaskPoolWorkflowStatusRoleDto::ReviewReady,
        StatusRole::Review => TaskPoolWorkflowStatusRoleDto::Review,
        StatusRole::Merging => TaskPoolWorkflowStatusRoleDto::Merging,
        StatusRole::Done => TaskPoolWorkflowStatusRoleDto::Done,
        StatusRole::Archived => TaskPoolWorkflowStatusRoleDto::Archived,
        StatusRole::Custom => TaskPoolWorkflowStatusRoleDto::Custom,
    }
}

fn workflow_status_key(workflow: &TaskWorkflow, status: TaskStatus) -> String {
    workflow
        .status_definition(status)
        .map(|definition| definition.key().to_string())
        .unwrap_or_else(|| status.to_string_key().to_string())
}

/// 状态转移意图，区分驱动状态变化的来源。
///
/// 手动拖拽 `ManualMove` 保留看板自由移动体验；其余自动意图被显式限制到工作流
/// 定义的专属边上，避免后台调度把任务流转到非法状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaskTransitionIntent {
    /// 看板手动拖拽，允许任意跨列移动。
    ManualMove,
    /// 自动提升：仅 `Pool -> Pending`。
    AutoPromote,
    /// 开始执行：`Pending`/`Planning`/`Failed -> Running`。
    StartExecution,
    /// 执行成功：`Running -> CodeComplete`。
    ExecutionSucceeded,
    /// 执行失败：`Running -> Failed`。
    ExecutionFailed,
    /// 进入代码审核：`CodeComplete -> CodeReview`。
    EnterReview,
    /// 跳过代码审核直接合并：`CodeComplete -> PrSubmitted`。
    SkipReview,
    /// 审核通过：`CodeReview -> PrSubmitted`。
    ReviewApproved,
    /// 审核驳回：`CodeReview -> Failed`。
    ReviewBlocked,
    /// 合并成功：`PrSubmitted -> Completed`。
    MergeSucceeded,
    /// 合并失败：`PrSubmitted -> Failed`。
    MergeFailed,
    /// 归档：`Completed -> Archived`。
    Archive,
    /// 模拟执行步骤：沿默认线性边推进，用于测试与演示。
    Simulation,
}

impl TaskTransitionIntent {
    fn outcome(self) -> Option<StatusOutcome> {
        match self {
            Self::ManualMove => None,
            Self::ExecutionFailed | Self::ReviewBlocked | Self::MergeFailed => {
                Some(StatusOutcome::failed())
            }
            Self::SkipReview => Some(StatusOutcome::manual()),
            Self::AutoPromote
            | Self::StartExecution
            | Self::ExecutionSucceeded
            | Self::EnterReview
            | Self::ReviewApproved
            | Self::MergeSucceeded
            | Self::Archive
            | Self::Simulation => Some(StatusOutcome::succeed()),
        }
    }
}

/// 单条状态转移边定义，绑定来源状态、目标状态与触发意图。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaskStatusTransition {
    /// 来源状态。
    pub from: TaskStatus,
    /// 目标状态。
    pub to: TaskStatus,
    /// 允许该边的转移意图。
    pub intent: TaskTransitionIntent,
}

/// 默认工作流的显式自动转移边集合。
///
/// 手动 `ManualMove` 不在此表内，由状态机单独放行任意跨列移动；
/// `Simulation` 不枚举具体边，运行时按默认线性 `next` 推进。
const DEV_LIFECYCLE_TRANSITIONS: &[TaskStatusTransition] = &[
    TaskStatusTransition {
        from: TaskStatus::Pool,
        to: TaskStatus::Pending,
        intent: TaskTransitionIntent::AutoPromote,
    },
    TaskStatusTransition {
        from: TaskStatus::Pending,
        to: TaskStatus::Running,
        intent: TaskTransitionIntent::StartExecution,
    },
    TaskStatusTransition {
        from: TaskStatus::Planning,
        to: TaskStatus::Running,
        intent: TaskTransitionIntent::StartExecution,
    },
    TaskStatusTransition {
        from: TaskStatus::Failed,
        to: TaskStatus::Running,
        intent: TaskTransitionIntent::StartExecution,
    },
    TaskStatusTransition {
        from: TaskStatus::Running,
        to: TaskStatus::CodeComplete,
        intent: TaskTransitionIntent::ExecutionSucceeded,
    },
    TaskStatusTransition {
        from: TaskStatus::Running,
        to: TaskStatus::Failed,
        intent: TaskTransitionIntent::ExecutionFailed,
    },
    TaskStatusTransition {
        from: TaskStatus::CodeComplete,
        to: TaskStatus::CodeReview,
        intent: TaskTransitionIntent::EnterReview,
    },
    TaskStatusTransition {
        from: TaskStatus::CodeComplete,
        to: TaskStatus::PrSubmitted,
        intent: TaskTransitionIntent::SkipReview,
    },
    TaskStatusTransition {
        from: TaskStatus::CodeReview,
        to: TaskStatus::PrSubmitted,
        intent: TaskTransitionIntent::ReviewApproved,
    },
    TaskStatusTransition {
        from: TaskStatus::CodeReview,
        to: TaskStatus::Failed,
        intent: TaskTransitionIntent::ReviewBlocked,
    },
    TaskStatusTransition {
        from: TaskStatus::PrSubmitted,
        to: TaskStatus::Completed,
        intent: TaskTransitionIntent::MergeSucceeded,
    },
    TaskStatusTransition {
        from: TaskStatus::PrSubmitted,
        to: TaskStatus::Failed,
        intent: TaskTransitionIntent::MergeFailed,
    },
    TaskStatusTransition {
        from: TaskStatus::Completed,
        to: TaskStatus::Archived,
        intent: TaskTransitionIntent::Archive,
    },
];

/// 状态机转移校验失败，携带足够定位非法流转的上下文。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskTransitionError {
    /// 工作流 id，标明被违反的规则集合归属。
    pub workflow_id: String,
    /// 来源状态。
    pub from: TaskStatus,
    /// 目标状态。
    pub to: TaskStatus,
    /// 触发意图。
    pub intent: TaskTransitionIntent,
}

impl std::fmt::Display for TaskTransitionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "工作流 {} 拒绝转移: {:?} -> {:?} (意图 {:?})",
            self.workflow_id, self.from, self.to, self.intent
        )
    }
}

impl std::error::Error for TaskTransitionError {}

/// 纯领域状态机：基于工作流定义校验状态转移意图，不接入存储、看板或调度。
///
/// ```
/// use vw_desktop::app::task::models::TaskStatus;
/// use vw_desktop::app::task::workflow::{TaskStateMachine, TaskTransitionIntent};
///
/// let machine = TaskStateMachine::default_lifecycle();
///
/// // 手动拖拽保留看板自由移动。
/// assert!(machine.can_transition(
///     TaskStatus::Pool,
///     TaskStatus::Completed,
///     TaskTransitionIntent::ManualMove,
/// ));
///
/// // 自动提升只允许 Pool -> Pending。
/// assert!(machine.can_transition(
///     TaskStatus::Pool,
///     TaskStatus::Pending,
///     TaskTransitionIntent::AutoPromote,
/// ));
/// assert!(!machine.can_transition(
///     TaskStatus::Pending,
///     TaskStatus::Planning,
///     TaskTransitionIntent::AutoPromote,
/// ));
///
/// // 非法自动转移返回携带上下文的错误。
/// let err = machine
///     .validate_transition(
///         TaskStatus::Running,
///         TaskStatus::Completed,
///         TaskTransitionIntent::MergeSucceeded,
///     )
///     .unwrap_err();
/// assert_eq!(err.from, TaskStatus::Running);
/// assert_eq!(err.intent, TaskTransitionIntent::MergeSucceeded);
/// ```
#[derive(Debug, Clone)]
pub struct TaskStateMachine {
    workflow: TaskWorkflow,
}

impl TaskStateMachine {
    /// 用给定工作流定义构造状态机。
    pub fn new(workflow: impl Into<TaskWorkflow>) -> Self {
        Self { workflow: workflow.into() }
    }

    /// 用默认工作流构造状态机。
    pub fn default_lifecycle() -> Self {
        Self::new(default_task_workflow())
    }

    /// 返回底层工作流定义。
    pub fn workflow(&self) -> &TaskWorkflow {
        &self.workflow
    }

    /// 判断给定意图下从 `from` 到 `to` 的转移是否允许。
    ///
    /// `ManualMove` 始终允许工作流内部任意两个已知状态间移动，保留看板自由移动；
    /// `Simulation` 沿默认线性 `next` 推进；其余自动意图必须命中
    /// 工作流的显式转移边。
    pub fn can_transition(
        &self,
        from: TaskStatus,
        to: TaskStatus,
        intent: TaskTransitionIntent,
    ) -> bool {
        match intent {
            TaskTransitionIntent::ManualMove => {
                self.workflow.status_definition(from).is_some()
                    && self.workflow.status_definition(to).is_some()
            }
            TaskTransitionIntent::Simulation => self.workflow.next_status(from) == Some(to),
            _ if self.workflow.id() == DEFAULT_TASK_WORKFLOW_ID => {
                DEV_LIFECYCLE_TRANSITIONS.iter().any(|edge| {
                    edge.from == from
                        && self.workflow.next_enabled_status(edge.to) == Some(to)
                        && edge.intent == intent
                })
            }
            _ => {
                intent
                    .outcome()
                    .and_then(|outcome| self.workflow.next_status_for_outcome(from, outcome))
                    == Some(to)
            }
        }
    }

    /// 校验转移，允许时返回 `Ok`，否则返回携带上下文的 `TaskTransitionError`。
    pub fn validate_transition(
        &self,
        from: TaskStatus,
        to: TaskStatus,
        intent: TaskTransitionIntent,
    ) -> Result<(), TaskTransitionError> {
        if self.can_transition(from, to, intent) {
            Ok(())
        } else {
            Err(TaskTransitionError { workflow_id: self.workflow.id.clone(), from, to, intent })
        }
    }

    /// 返回给定意图在 `from` 状态下的唯一目标状态。
    ///
    /// 对自动意图，从显式转移边解析唯一目标；对 `Simulation` 返回默认
    /// 线性下一状态；`ManualMove` 没有确定目标，返回 `None`。
    pub fn next_for_intent(
        &self,
        from: TaskStatus,
        intent: TaskTransitionIntent,
    ) -> Option<TaskStatus> {
        match intent {
            TaskTransitionIntent::ManualMove => None,
            TaskTransitionIntent::Simulation => self.workflow.next_status(from),
            _ if self.workflow.id() == DEFAULT_TASK_WORKFLOW_ID => DEV_LIFECYCLE_TRANSITIONS
                .iter()
                .find(|edge| edge.from == from && edge.intent == intent)
                .and_then(|edge| self.workflow.next_enabled_status(edge.to)),
            _ => intent
                .outcome()
                .and_then(|outcome| self.workflow.next_status_for_outcome(from, outcome)),
        }
    }

    /// 判断 `to` 是否为从 `from` 出发、沿工作流前进方向可达的真实状态变化。
    ///
    /// 不接受 `ManualMove` 的任意跨列移动，也不接受状态倒流。执行器结果可能在一次回调内
    /// 连续推进多步（如 `Running -> CodeComplete -> CodeReview`），交接记录的首尾状态并非
    /// 单步边，因此按默认线性 `next` 链做前向可达性判断：能从 `from` 顺序走到 `to` 即认可。
    /// 用于在不知道具体来源意图时校验“该状态变化是否被状态机前向认可”，例如员工交接消息。
    pub fn is_known_transition(&self, from: TaskStatus, to: TaskStatus) -> bool {
        if from == to {
            return false;
        }
        let mut visited = HashSet::new();
        let mut stack = vec![from];
        while let Some(current) = stack.pop() {
            if !visited.insert(current) {
                continue;
            }
            for transition in self.workflow.status_transitions(current) {
                let Some(target) = self.workflow.next_enabled_status(transition.target) else {
                    continue;
                };
                if target == to {
                    return true;
                }
                stack.push(target);
            }
        }
        false
    }
}

#[cfg(test)]
#[path = "workflow/registry_tests.rs"]
mod registry_tests;

#[cfg(test)]
#[path = "workflow_tests.rs"]
mod workflow_tests;
