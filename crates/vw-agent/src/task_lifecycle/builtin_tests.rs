//! 验证内置任务生命周期定义及状态转换契约。
//! 目录固定内置工作流及其终止、重试和角色形状，防止配置兼容性漂移。

use super::*;

#[test]
fn built_in_workflows_expose_stable_unique_catalog() {
    let workflows = built_in_workflows();
    let ids = workflows.iter().map(|workflow| workflow.id.as_str()).collect::<Vec<_>>();

    assert_eq!(
        ids,
        vec![
            DEFAULT_TASK_LIFECYCLE_WORKFLOW_ID,
            "content_publishing",
            "marketing_campaign",
            "customer_support",
            "recruiting_pipeline",
        ]
    );
    assert!(workflows.iter().all(|workflow| workflow.built_in));
    assert!(workflows.iter().all(|workflow| !workflow.statuses.is_empty()));
}

#[test]
fn built_in_workflow_lookup_matches_catalog_and_rejects_unknown_id() {
    for workflow in built_in_workflows() {
        assert_eq!(built_in_workflow_by_id(&workflow.id), Some(workflow.clone()));
        assert!(is_built_in_workflow_id(&workflow.id));
    }

    assert_eq!(built_in_workflow_by_id("unknown"), None);
    assert!(!is_built_in_workflow_id("unknown"));
    assert!(!is_built_in_workflow_id(""));
}

#[test]
// 状态标志、终态性质和下一跳共同构成内置工作流契约，三者必须相互一致。
fn built_in_statuses_preserve_spec_flags_and_next_targets() {
    for workflow in built_in_workflows() {
        for status in &workflow.statuses {
            assert!(status.enabled);
            if let Some(next) = &status.next {
                assert!(
                    workflow.statuses.iter().any(|candidate| candidate.key == *next),
                    "{} status {} points to missing target {}",
                    workflow.id,
                    status.key,
                    next
                );
            }
        }
    }

    let workflow = built_in_workflow_by_id(DEFAULT_TASK_LIFECYCLE_WORKFLOW_ID).unwrap();
    let pool = workflow.statuses.iter().find(|status| status.key == "pool").unwrap();
    assert!(pool.is_scheduling_backlog);
    assert!(!pool.occupies_execution_slot);
    let running = workflow.statuses.iter().find(|status| status.key == "running").unwrap();
    assert!(running.auto_execution_candidate);
    assert!(running.occupies_execution_slot);
}

#[test]
fn failed_statuses_with_next_target_expose_retry_transition() {
    let failed_statuses = built_in_workflows()
        .into_iter()
        .flat_map(|workflow| workflow.statuses)
        .filter(|status| status.role == TaskLifecycleStatusRoleDto::Failed)
        .collect::<Vec<_>>();

    assert!(!failed_statuses.is_empty());
    for status in failed_statuses {
        let target = status.next.as_deref().expect("built-in failed status should be retryable");
        assert_eq!(status.transitions.len(), 1);
        assert_eq!(status.transitions[0].outcome, "retry");
        assert_eq!(status.transitions[0].label.as_deref(), Some("重试"));
        assert_eq!(status.transitions[0].target, target);
    }
}

#[test]
fn built_in_workflows_preserve_domain_specific_terminal_and_role_shape() {
    let expected = [
        (DEFAULT_TASK_LIFECYCLE_WORKFLOW_ID, "开发生命周期", "archived", "failed", "pending"),
        (
            "content_publishing",
            "内容创作与多平台发布",
            "content_archived",
            "content_revision",
            "content_queued",
        ),
        (
            "marketing_campaign",
            "营销活动执行",
            "campaign_archived",
            "campaign_blocked",
            "campaign_queued",
        ),
        (
            "customer_support",
            "客服工单处理",
            "support_archived",
            "support_blocked",
            "support_queued",
        ),
        (
            "recruiting_pipeline",
            "招聘流程",
            "recruiting_archived",
            "recruiting_blocked",
            "recruiting_queued",
        ),
    ];

    for (workflow_id, label, terminal_key, failed_key, retry_target) in expected {
        let workflow = built_in_workflow_by_id(workflow_id).expect("built-in workflow");
        assert_eq!(workflow.label, label);
        assert_eq!(workflow.statuses.len(), 11);

        let terminal = workflow
            .statuses
            .iter()
            .find(|status| status.key == terminal_key)
            .expect("terminal status");
        assert!(terminal.terminal);
        assert_eq!(terminal.role, TaskLifecycleStatusRoleDto::Archived);
        assert!(terminal.next.is_none());
        assert!(terminal.transitions.is_empty());

        let failed = workflow
            .statuses
            .iter()
            .find(|status| status.key == failed_key)
            .expect("failed status");
        assert!(!failed.terminal);
        assert_eq!(failed.role, TaskLifecycleStatusRoleDto::Failed);
        assert_eq!(failed.next.as_deref(), Some(retry_target));
        assert_eq!(failed.transitions.len(), 1);
        assert_eq!(failed.transitions[0].target, retry_target);

        assert!(workflow.statuses.iter().filter(|status| status.occupies_execution_slot).all(
            |status| matches!(
                status.role,
                TaskLifecycleStatusRoleDto::Planning | TaskLifecycleStatusRoleDto::Executing
            )
        ));
    }
}
