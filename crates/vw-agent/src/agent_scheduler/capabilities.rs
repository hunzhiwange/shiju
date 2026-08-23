//! 将内建阶段能力及其支持的工作流状态映射为员工配置可消费的能力描述。

use std::collections::HashSet;

use vw_api_types::staff::{
    StaffCapabilityIoSummaryDto, StaffCapabilityKindDto, StaffStageCapabilityDto,
};
use vw_api_types::task::TaskPoolWorkflowStatusRoleDto;
use vw_config_types::staff::{
    StaffCapabilityConfig, StaffCapabilityIoSummaryConfig, StaffCapabilityKindConfig, StaffConfig,
};

pub const ADVANCE_STATUS_CAPABILITY_ID: &str = "advance_status";
pub const EXECUTE_TASK_CAPABILITY_ID: &str = "execute_task";
pub const CODE_REVIEW_CAPABILITY_ID: &str = "code_review";
pub const GIT_MERGE_CAPABILITY_ID: &str = "git_merge";
pub const SAVED_WORKFLOW_CAPABILITY_ID: &str = "saved_workflow";
pub const MANUAL_CAPABILITY_ID: &str = "manual";

const ALL_STATUS_ROLES: &[TaskPoolWorkflowStatusRoleDto] = &[
    TaskPoolWorkflowStatusRoleDto::Backlog,
    TaskPoolWorkflowStatusRoleDto::Queued,
    TaskPoolWorkflowStatusRoleDto::Planning,
    TaskPoolWorkflowStatusRoleDto::Executing,
    TaskPoolWorkflowStatusRoleDto::Failed,
    TaskPoolWorkflowStatusRoleDto::Paused,
    TaskPoolWorkflowStatusRoleDto::ReviewReady,
    TaskPoolWorkflowStatusRoleDto::Review,
    TaskPoolWorkflowStatusRoleDto::Merging,
    TaskPoolWorkflowStatusRoleDto::Done,
    TaskPoolWorkflowStatusRoleDto::Archived,
    TaskPoolWorkflowStatusRoleDto::Custom,
];
const ADVANCE_STATUS_SUPPORTED_ROLES: &[TaskPoolWorkflowStatusRoleDto] = &[
    TaskPoolWorkflowStatusRoleDto::Backlog,
    TaskPoolWorkflowStatusRoleDto::Failed,
    TaskPoolWorkflowStatusRoleDto::ReviewReady,
    TaskPoolWorkflowStatusRoleDto::Review,
    TaskPoolWorkflowStatusRoleDto::Merging,
];
const EXECUTE_TASK_SUPPORTED_ROLES: &[TaskPoolWorkflowStatusRoleDto] =
    &[TaskPoolWorkflowStatusRoleDto::Queued, TaskPoolWorkflowStatusRoleDto::Planning];
const CODE_REVIEW_SUPPORTED_ROLES: &[TaskPoolWorkflowStatusRoleDto] =
    &[TaskPoolWorkflowStatusRoleDto::ReviewReady, TaskPoolWorkflowStatusRoleDto::Review];
const GIT_MERGE_SUPPORTED_ROLES: &[TaskPoolWorkflowStatusRoleDto] =
    &[TaskPoolWorkflowStatusRoleDto::Merging];

const BUILT_IN_CAPABILITIES: &[BuiltInStageCapability] = &[
    BuiltInStageCapability {
        stable_id: ADVANCE_STATUS_CAPABILITY_ID,
        label: "推进状态",
        supported_roles: ADVANCE_STATUS_SUPPORTED_ROLES,
        requires_workflow_uuid: false,
        input_contract: "任务状态、目标阶段、操作者成员",
        result_semantics: "任务进入目标阶段",
        permission_hint: "只能推进到当前工作流允许的下一阶段，不读取任务外上下文。",
    },
    BuiltInStageCapability {
        stable_id: EXECUTE_TASK_CAPABILITY_ID,
        label: "执行任务",
        supported_roles: EXECUTE_TASK_SUPPORTED_ROLES,
        requires_workflow_uuid: false,
        input_contract: "任务摘要、阶段 key、成员、优先级",
        result_semantics: "产出执行记录，并按工作流推进到后续阶段",
        permission_hint: "需要任务执行后端权限，只传递安全任务摘要。",
    },
    BuiltInStageCapability {
        stable_id: CODE_REVIEW_CAPABILITY_ID,
        label: "代码审核",
        supported_roles: CODE_REVIEW_SUPPORTED_ROLES,
        requires_workflow_uuid: false,
        input_contract: "任务摘要、变更范围、审核成员",
        result_semantics: "产出审核结论，并可推进到合并或返工阶段",
        permission_hint: "需要读取工作区差异，不能记录密钥或原始令牌。",
    },
    BuiltInStageCapability {
        stable_id: GIT_MERGE_CAPABILITY_ID,
        label: "Git 合并",
        supported_roles: GIT_MERGE_SUPPORTED_ROLES,
        requires_workflow_uuid: false,
        input_contract: "任务摘要、目标分支、合并成员",
        result_semantics: "产出合并结果，并在成功后推进完成阶段",
        permission_hint: "需要最小 Git 写权限，失败时保留明确错误。",
    },
    BuiltInStageCapability {
        stable_id: SAVED_WORKFLOW_CAPABILITY_ID,
        label: "已保存 Workflow",
        supported_roles: ALL_STATUS_ROLES,
        requires_workflow_uuid: true,
        input_contract: "Workflow UUID、任务摘要、阶段 key、成员、优先级",
        result_semantics: "产出 Workflow 运行结果，由配置决定后续阶段",
        permission_hint: "只传递安全任务摘要和阶段上下文，不暴露完整敏感载荷。",
    },
    BuiltInStageCapability {
        stable_id: MANUAL_CAPABILITY_ID,
        label: "人工处理",
        supported_roles: ALL_STATUS_ROLES,
        requires_workflow_uuid: false,
        input_contract: "任务摘要、阶段 key、成员",
        result_semantics: "记录人工接管，不自动执行阶段动作",
        permission_hint: "不请求额外系统权限，等待人工明确操作。",
    },
];

#[derive(Debug, Clone, Copy)]
struct BuiltInStageCapability {
    stable_id: &'static str,
    label: &'static str,
    supported_roles: &'static [TaskPoolWorkflowStatusRoleDto],
    requires_workflow_uuid: bool,
    input_contract: &'static str,
    result_semantics: &'static str,
    permission_hint: &'static str,
}

/// 返回不依赖项目配置的内建阶段能力目录。
pub fn builtin_stage_capabilities() -> Vec<StaffStageCapabilityDto> {
    BUILT_IN_CAPABILITIES.iter().map(builtin_stage_capability_to_dto).collect()
}

/// 合并内建能力与配置能力，并按能力标识去重。
pub fn stage_capabilities_for_config(config: &StaffConfig) -> Vec<StaffStageCapabilityDto> {
    let mut catalog = builtin_stage_capabilities();
    let built_in_ids =
        catalog.iter().map(|capability| capability.capability_id.clone()).collect::<HashSet<_>>();

    for capability in &config.project_state.capabilities {
        let capability_id = capability.id.trim();
        if capability_id.is_empty() || built_in_ids.contains(capability_id) || capability.built_in {
            continue;
        }
        catalog.push(custom_stage_capability_to_dto(capability));
    }
    catalog
}

/// 按稳定标识查找能力；不存在时返回 `None`。
pub fn capability_by_id<'a>(
    catalog: &'a [StaffStageCapabilityDto],
    capability_id: &str,
) -> Option<&'a StaffStageCapabilityDto> {
    let capability_id = capability_id.trim();
    catalog.iter().find(|capability| capability.capability_id == capability_id)
}

fn builtin_stage_capability_to_dto(capability: &BuiltInStageCapability) -> StaffStageCapabilityDto {
    let capability_type = if capability.stable_id == SAVED_WORKFLOW_CAPABILITY_ID {
        StaffCapabilityKindDto::SavedWorkflow
    } else {
        StaffCapabilityKindDto::BuiltIn
    };
    StaffStageCapabilityDto {
        capability_id: capability.stable_id.to_string(),
        label: capability.label.to_string(),
        capability_type,
        built_in: true,
        supported_roles: capability.supported_roles.to_vec(),
        requires_workflow_uuid: capability.requires_workflow_uuid,
        input_summary: StaffCapabilityIoSummaryDto {
            description: capability.input_contract.to_string(),
            fields: Vec::new(),
        },
        result_summary: StaffCapabilityIoSummaryDto {
            description: capability.result_semantics.to_string(),
            fields: Vec::new(),
        },
        input_contract: capability.input_contract.to_string(),
        result_semantics: capability.result_semantics.to_string(),
        permission_hint: capability.permission_hint.to_string(),
    }
}

fn custom_stage_capability_to_dto(capability: &StaffCapabilityConfig) -> StaffStageCapabilityDto {
    let capability_type = capability_kind_to_dto(capability.capability_type);
    let input_contract =
        summary_description_or(&capability.input_summary, "自定义能力输入由目录配置定义");
    let result_semantics =
        summary_description_or(&capability.result_summary, "自定义能力结果由目录配置定义");
    StaffStageCapabilityDto {
        capability_id: capability.id.trim().to_string(),
        label: capability.label.trim().to_string(),
        capability_type,
        built_in: false,
        supported_roles: ALL_STATUS_ROLES.to_vec(),
        requires_workflow_uuid: capability.capability_type
            == StaffCapabilityKindConfig::SavedWorkflow,
        input_summary: capability_io_summary_to_dto(&capability.input_summary),
        result_summary: capability_io_summary_to_dto(&capability.result_summary),
        input_contract,
        result_semantics,
        permission_hint: custom_permission_hint(capability.capability_type).to_string(),
    }
}

fn capability_kind_to_dto(kind: StaffCapabilityKindConfig) -> StaffCapabilityKindDto {
    match kind {
        StaffCapabilityKindConfig::BuiltIn => StaffCapabilityKindDto::BuiltIn,
        StaffCapabilityKindConfig::SavedWorkflow => StaffCapabilityKindDto::SavedWorkflow,
    }
}

fn capability_io_summary_to_dto(
    summary: &StaffCapabilityIoSummaryConfig,
) -> StaffCapabilityIoSummaryDto {
    StaffCapabilityIoSummaryDto {
        description: summary.description.trim().to_string(),
        fields: summary.fields.iter().map(|field| field.trim().to_string()).collect(),
    }
}

fn summary_description_or(summary: &StaffCapabilityIoSummaryConfig, fallback: &str) -> String {
    let description = summary.description.trim();
    if description.is_empty() { fallback.to_string() } else { description.to_string() }
}

fn custom_permission_hint(kind: StaffCapabilityKindConfig) -> &'static str {
    match kind {
        StaffCapabilityKindConfig::BuiltIn => "使用自定义内置能力配置声明的权限范围。",
        StaffCapabilityKindConfig::SavedWorkflow => "使用已保存 Workflow 配置声明的权限范围。",
    }
}

#[cfg(test)]
#[path = "capabilities_tests.rs"]
mod tests;
