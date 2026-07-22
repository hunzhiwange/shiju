//! 员工模式阶段能力注册表。

use crate::app::task::workflow::StatusRole;

pub const ADVANCE_STATUS_CAPABILITY_ID: &str = "advance_status";
pub const EXECUTE_TASK_CAPABILITY_ID: &str = "execute_task";
pub const CODE_REVIEW_CAPABILITY_ID: &str = "code_review";
pub const GIT_MERGE_CAPABILITY_ID: &str = "git_merge";
pub const SAVED_WORKFLOW_CAPABILITY_ID: &str = "saved_workflow";
pub const MANUAL_CAPABILITY_ID: &str = "manual";

const ALL_STATUS_ROLES: &[StatusRole] = &[
    StatusRole::Backlog,
    StatusRole::Queued,
    StatusRole::Planning,
    StatusRole::Executing,
    StatusRole::Failed,
    StatusRole::Paused,
    StatusRole::ReviewReady,
    StatusRole::Review,
    StatusRole::Merging,
    StatusRole::Done,
    StatusRole::Archived,
    StatusRole::Custom,
];

const ADVANCE_STATUS_SUPPORTED_ROLES: &[StatusRole] = &[
    StatusRole::Backlog,
    StatusRole::Failed,
    StatusRole::ReviewReady,
    StatusRole::Review,
    StatusRole::Merging,
];
const EXECUTE_TASK_SUPPORTED_ROLES: &[StatusRole] = &[StatusRole::Queued, StatusRole::Planning];
const CODE_REVIEW_SUPPORTED_ROLES: &[StatusRole] = &[StatusRole::ReviewReady, StatusRole::Review];
const GIT_MERGE_SUPPORTED_ROLES: &[StatusRole] = &[StatusRole::Merging];

const STAGE_CAPABILITIES: &[StageCapability] = &[
    StageCapability {
        id: StageCapabilityId::AdvanceStatus,
        stable_id: ADVANCE_STATUS_CAPABILITY_ID,
        label: "推进状态",
        supported_roles: ADVANCE_STATUS_SUPPORTED_ROLES,
        requires_workflow_uuid: false,
        input_contract: "任务状态、目标阶段、操作者成员",
        result_semantics: "任务进入目标阶段",
        permission_hint: "只能推进到当前工作流允许的下一阶段，不读取任务外上下文。",
    },
    StageCapability {
        id: StageCapabilityId::ExecuteTask,
        stable_id: EXECUTE_TASK_CAPABILITY_ID,
        label: "执行任务",
        supported_roles: EXECUTE_TASK_SUPPORTED_ROLES,
        requires_workflow_uuid: false,
        input_contract: "任务摘要、阶段 key、成员、优先级",
        result_semantics: "产出执行记录，并按工作流推进到后续阶段",
        permission_hint: "需要任务执行后端权限，只传递安全任务摘要。",
    },
    StageCapability {
        id: StageCapabilityId::CodeReview,
        stable_id: CODE_REVIEW_CAPABILITY_ID,
        label: "代码审核",
        supported_roles: CODE_REVIEW_SUPPORTED_ROLES,
        requires_workflow_uuid: false,
        input_contract: "任务摘要、变更范围、审核成员",
        result_semantics: "产出审核结论，并可推进到合并或返工阶段",
        permission_hint: "需要读取工作区差异，不能记录密钥或原始令牌。",
    },
    StageCapability {
        id: StageCapabilityId::GitMerge,
        stable_id: GIT_MERGE_CAPABILITY_ID,
        label: "Git 合并",
        supported_roles: GIT_MERGE_SUPPORTED_ROLES,
        requires_workflow_uuid: false,
        input_contract: "任务摘要、目标分支、合并成员",
        result_semantics: "产出合并结果，并在成功后推进完成阶段",
        permission_hint: "需要最小 Git 写权限，失败时保留明确错误。",
    },
    StageCapability {
        id: StageCapabilityId::SavedWorkflow,
        stable_id: SAVED_WORKFLOW_CAPABILITY_ID,
        label: "已保存 Workflow",
        supported_roles: ALL_STATUS_ROLES,
        requires_workflow_uuid: true,
        input_contract: "Workflow UUID、任务摘要、阶段 key、成员、优先级",
        result_semantics: "产出 Workflow 运行结果，由配置决定后续阶段",
        permission_hint: "只传递安全任务摘要和阶段上下文，不暴露完整敏感载荷。",
    },
    StageCapability {
        id: StageCapabilityId::Manual,
        stable_id: MANUAL_CAPABILITY_ID,
        label: "人工处理",
        supported_roles: ALL_STATUS_ROLES,
        requires_workflow_uuid: false,
        input_contract: "任务摘要、阶段 key、成员",
        result_semantics: "记录人工接管，不自动执行阶段动作",
        permission_hint: "不请求额外系统权限，等待人工明确操作。",
    },
];

static STAGE_CAPABILITY_REGISTRY: StageCapabilityRegistry =
    StageCapabilityRegistry { capabilities: STAGE_CAPABILITIES };

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StageCapabilityId {
    AdvanceStatus,
    ExecuteTask,
    CodeReview,
    GitMerge,
    SavedWorkflow,
    Manual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StageCapability {
    pub id: StageCapabilityId,
    pub stable_id: &'static str,
    pub label: &'static str,
    pub supported_roles: &'static [StatusRole],
    pub requires_workflow_uuid: bool,
    pub input_contract: &'static str,
    pub result_semantics: &'static str,
    pub permission_hint: &'static str,
}

#[derive(Debug)]
pub struct StageCapabilityRegistry {
    capabilities: &'static [StageCapability],
}

impl StageCapabilityRegistry {
    pub fn all(&self) -> &'static [StageCapability] {
        self.capabilities
    }

    pub fn len(&self) -> usize {
        self.capabilities.len()
    }

    pub fn get(&self, id: StageCapabilityId) -> Option<&'static StageCapability> {
        self.capabilities.iter().find(|capability| capability.id == id)
    }

    pub fn supports_role(&self, id: StageCapabilityId, role: StatusRole) -> bool {
        self.get(id).is_some_and(|capability| capability.supported_roles.contains(&role))
    }
}

pub fn stage_capability_registry() -> &'static StageCapabilityRegistry {
    &STAGE_CAPABILITY_REGISTRY
}
