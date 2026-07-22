use crate::app::task::workflow::StatusRole;

use super::{StageCapabilityId, stage_capability_registry};

#[test]
fn stage_capability_registry_lists_builtin_capabilities() {
    let registry = stage_capability_registry();

    let cases = [
        (
            StageCapabilityId::AdvanceStatus,
            "advance_status",
            "推进状态",
            false,
            "任务状态、目标阶段、操作者成员",
            "任务进入目标阶段",
            "只能推进到当前工作流允许的下一阶段，不读取任务外上下文。",
        ),
        (
            StageCapabilityId::ExecuteTask,
            "execute_task",
            "执行任务",
            false,
            "任务摘要、阶段 key、成员、优先级",
            "产出执行记录，并按工作流推进到后续阶段",
            "需要任务执行后端权限，只传递安全任务摘要。",
        ),
        (
            StageCapabilityId::CodeReview,
            "code_review",
            "代码审核",
            false,
            "任务摘要、变更范围、审核成员",
            "产出审核结论，并可推进到合并或返工阶段",
            "需要读取工作区差异，不能记录密钥或原始令牌。",
        ),
        (
            StageCapabilityId::GitMerge,
            "git_merge",
            "Git 合并",
            false,
            "任务摘要、目标分支、合并成员",
            "产出合并结果，并在成功后推进完成阶段",
            "需要最小 Git 写权限，失败时保留明确错误。",
        ),
        (
            StageCapabilityId::SavedWorkflow,
            "saved_workflow",
            "已保存 Workflow",
            true,
            "Workflow UUID、任务摘要、阶段 key、成员、优先级",
            "产出 Workflow 运行结果，由配置决定后续阶段",
            "只传递安全任务摘要和阶段上下文，不暴露完整敏感载荷。",
        ),
        (
            StageCapabilityId::Manual,
            "manual",
            "人工处理",
            false,
            "任务摘要、阶段 key、成员",
            "记录人工接管，不自动执行阶段动作",
            "不请求额外系统权限，等待人工明确操作。",
        ),
    ];

    assert_eq!(registry.len(), cases.len());

    for (
        id,
        stable_id,
        label,
        requires_workflow_uuid,
        input_contract,
        result_semantics,
        permission_hint,
    ) in cases
    {
        let capability = registry
            .get(id)
            .unwrap_or_else(|| panic!("{stable_id} capability should be registered"));

        assert_eq!(capability.id, id);
        assert_eq!(capability.stable_id, stable_id);
        assert_eq!(capability.label, label);
        assert_eq!(capability.requires_workflow_uuid, requires_workflow_uuid);
        assert_eq!(capability.input_contract, input_contract);
        assert_eq!(capability.result_semantics, result_semantics);
        assert_eq!(capability.permission_hint, permission_hint);
    }
}

#[test]
fn stage_capability_registry_validates_supported_roles() {
    let registry = stage_capability_registry();

    assert!(registry.supports_role(StageCapabilityId::AdvanceStatus, StatusRole::Backlog));
    assert!(registry.supports_role(StageCapabilityId::AdvanceStatus, StatusRole::Failed));
    assert!(registry.supports_role(StageCapabilityId::AdvanceStatus, StatusRole::ReviewReady));
    assert!(registry.supports_role(StageCapabilityId::AdvanceStatus, StatusRole::Review));
    assert!(registry.supports_role(StageCapabilityId::AdvanceStatus, StatusRole::Merging));
    assert!(registry.supports_role(StageCapabilityId::ExecuteTask, StatusRole::Queued));
    assert!(registry.supports_role(StageCapabilityId::ExecuteTask, StatusRole::Planning));
    assert!(registry.supports_role(StageCapabilityId::CodeReview, StatusRole::ReviewReady));
    assert!(registry.supports_role(StageCapabilityId::CodeReview, StatusRole::Review));
    assert!(registry.supports_role(StageCapabilityId::GitMerge, StatusRole::Merging));
    assert!(registry.supports_role(StageCapabilityId::SavedWorkflow, StatusRole::Custom));
    assert!(registry.supports_role(StageCapabilityId::Manual, StatusRole::Archived));

    assert!(!registry.supports_role(StageCapabilityId::ExecuteTask, StatusRole::Review));
    assert!(!registry.supports_role(StageCapabilityId::CodeReview, StatusRole::Merging));
    assert!(!registry.supports_role(StageCapabilityId::GitMerge, StatusRole::Executing));
    assert!(!registry.supports_role(StageCapabilityId::AdvanceStatus, StatusRole::Done));
}
