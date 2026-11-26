use desktop_i18n::I18n;
use desktop_task_model::{OwnedStatusDefinition, StatusRole, TaskWorkflow, builtin_task_workflows};

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

    pub(super) fn label(self, i18n: I18n) -> &'static str {
        match self {
            Self::Unstarted => i18n.__("未开始"),
            Self::Started => i18n.__("已开始"),
            Self::Done => i18n.__("已完成"),
            // The unqualified translation refers to a disabled setting.
            Self::Closed => i18n.__("已关闭（工作流状态）").trim_end_matches("（工作流状态）"),
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

pub(super) fn status_behavior(
    _workflow: &TaskWorkflow,
    status: &OwnedStatusDefinition,
) -> StatusBehavior {
    if !status.enabled {
        StatusBehavior { summary: "已停用", help: "停用的状态仍可查看和编辑。" }
    } else {
        StatusBehavior {
            summary: "按步骤配置交接", help: "接手规则在工作流步骤中配置。"
        }
    }
}
