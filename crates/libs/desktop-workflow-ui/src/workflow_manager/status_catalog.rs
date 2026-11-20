use desktop_task_model::{
    OwnedStatusDefinition, StatusOutcome, StatusRole, TaskWorkflow, builtin_task_workflows,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum StatusGroup {
    Unstarted,
    Started,
    Done,
    Closed,
}

impl StatusGroup {
    const ORDER: [Self; 4] = [Self::Unstarted, Self::Started, Self::Done, Self::Closed];

    fn for_role(role: StatusRole) -> Self {
        match role {
            StatusRole::Backlog | StatusRole::Queued => Self::Unstarted,
            StatusRole::Planning
            | StatusRole::Executing
            | StatusRole::Failed
            | StatusRole::Paused
            | StatusRole::ReviewReady
            | StatusRole::Review
            | StatusRole::Merging
            | StatusRole::Custom => Self::Started,
            StatusRole::Done => Self::Done,
            StatusRole::Archived => Self::Closed,
        }
    }

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Unstarted => "未开始",
            Self::Started => "已开始",
            Self::Done => "已完成",
            Self::Closed => "已关闭",
        }
    }
}

pub(super) struct StatusCatalogRow<'a> {
    pub(super) workflow_id: &'a str,
    pub(super) status: &'a OwnedStatusDefinition,
    pub(super) is_builtin: bool,
    pub(super) behavior: StatusBehavior,
}

pub(super) struct StatusBehavior {
    pub(super) summary: &'static str,
    pub(super) help: &'static str,
}

pub(super) fn status_catalog(workflow: &TaskWorkflow) -> [(StatusGroup, Vec<StatusCatalogRow<'_>>); 4] {
    let preset = builtin_task_workflows().iter().find(|preset| preset.id == workflow.id);
    StatusGroup::ORDER.map(|group| {
        let rows = workflow
            .statuses
            .iter()
            .filter(|status| StatusGroup::for_role(status.role) == group)
            .map(|status| StatusCatalogRow {
                workflow_id: &workflow.id,
                status,
                is_builtin: preset.is_some_and(|preset| {
                    preset.statuses.iter().any(|preset_status| preset_status.key == status.key)
                }),
                behavior: status_behavior(workflow, status),
            })
            .collect();
        (group, rows)
    })
}

pub(super) fn status_behavior(workflow: &TaskWorkflow, status: &OwnedStatusDefinition) -> StatusBehavior {
    if !status.enabled {
        return StatusBehavior {
            summary: "已停用",
            help: "停用的状态仍可查看和编辑。流转时，如有可用的后续状态，任务会跳过这一步。",
        };
    }

    let can_auto_start = status.auto_execution_candidate && !status.is_terminal;
    let (summary, help) = match status.role {
        StatusRole::Backlog => ("等待排队", "任务先留在池中，进入排队后才会等待执行名额。"),
        StatusRole::Queued if can_auto_start => (
            "等待执行名额",
            "允许调度后，这一步会等待可用执行名额。",
        ),
        StatusRole::Queued => ("等待启动", "可以从任务看板发起执行。"),
        StatusRole::Planning if can_auto_start => (
            "等待规划",
            "允许调度后，有可用执行名额时可以开始规划。",
        ),
        StatusRole::Planning => ("准备执行计划", "这一步用于拆分任务和准备执行计划。"),
        StatusRole::Executing => ("正在执行", "这一步记录任务的执行进度。"),
        StatusRole::Failed if has_retry_target(workflow, status) => (
            "失败后按项目设置重试",
            "项目允许重试且未达到次数上限时，任务会进入配置的重试状态。",
        ),
        StatusRole::Failed => ("等待处理失败", "请检查失败原因并决定后续操作。"),
        StatusRole::Paused => ("等待恢复", "恢复前不会自动继续。"),
        StatusRole::ReviewReady => ("等待审核", "执行结果已可供审核。"),
        StatusRole::Review => ("审核结果", "审核完成后，可按已配置的结果流转。"),
        StatusRole::Merging => ("处理收尾", "这一步记录合并、发布等收尾工作。"),
        StatusRole::Done => ("任务已完成", "任务已完成。配置了后续状态时，可以继续流转。"),
        StatusRole::Archived => ("归档后停止调度", "归档后的任务可以查看，调度器会停止派发。"),
        StatusRole::Custom => ("按配置流转", "进入这个状态后，可按已配置的结果流转。"),
    };
    StatusBehavior { summary, help }
}

fn has_retry_target(workflow: &TaskWorkflow, status: &OwnedStatusDefinition) -> bool {
    let retry = StatusOutcome::retry();
    let target = if status.transition_target(retry).is_some() {
        workflow.next_status_for_outcome(status.status, StatusOutcome::retry())
    } else {
        // Older saved definitions used the default successor for retries.
        status.next.and_then(|target| {
            if workflow.status_enabled(target) {
                Some(target)
            } else {
                workflow.next_status(target)
            }
        })
    };
    target
        .and_then(|target| workflow.status_role(target))
        .is_some_and(|role| matches!(role, StatusRole::Queued | StatusRole::Planning))
}
