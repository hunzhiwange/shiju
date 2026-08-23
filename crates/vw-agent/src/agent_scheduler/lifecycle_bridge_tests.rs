//! 测试工作流状态角色到调度结果和运行状态的映射。
//! 缺失、禁用状态及禁用结果链中的环路必须明确拒绝。

use serde_json::json;
use vw_api_types::task::{
    TaskPoolWorkflowSnapshotDto, TaskPoolWorkflowStatusDto, TaskPoolWorkflowStatusRoleDto,
    TaskPoolWorkflowTransitionDto,
};

use super::lifecycle_bridge::LifecycleBridge;
use super::state_machine::AgentTaskStatus;

#[test]
fn lifecycle_bridge_resolves_outcome_and_runtime_status_by_role() {
    let workflow = workflow(vec![
        status(
            "campaign_queued",
            TaskPoolWorkflowStatusRoleDto::Queued,
            Some(transition("success", "campaign_review")),
            None,
            true,
        ),
        status("campaign_planning", TaskPoolWorkflowStatusRoleDto::Planning, None, None, true),
        status(
            "campaign_review",
            TaskPoolWorkflowStatusRoleDto::Review,
            None,
            Some("campaign_done"),
            false,
        ),
        status("campaign_done", TaskPoolWorkflowStatusRoleDto::Done, None, None, true),
    ]);
    let bridge = LifecycleBridge::new(&workflow);

    let queued = bridge.resolve_status("campaign_queued").expect("queued projection");
    assert_eq!(queued.status_key, "campaign_queued");
    assert_eq!(queued.status_role, TaskPoolWorkflowStatusRoleDto::Queued);
    assert_eq!(queued.runtime_status, AgentTaskStatus::Queued);
    assert!(bridge.dispatch_eligibility("campaign_queued").expect("dispatch eligibility").eligible);
    assert!(
        bridge
            .dispatch_eligibility("campaign_planning")
            .expect("planning dispatch eligibility")
            .eligible
    );

    let projected_metadata =
        bridge.projection_metadata("campaign_queued").expect("projection metadata");
    assert_eq!(projected_metadata["task_board.status_key"], json!("campaign_queued"));
    assert_eq!(projected_metadata["task_board.status_role"], json!("queued"));

    let advanced =
        bridge.resolve_outcome("campaign_queued", "success").expect("success target projection");
    assert_eq!(advanced.status_key, "campaign_done");
    assert_eq!(advanced.status_role, TaskPoolWorkflowStatusRoleDto::Done);
    assert_eq!(advanced.runtime_status, AgentTaskStatus::Completed);
}

#[test]
fn lifecycle_bridge_rejects_missing_or_disabled_status() {
    let workflow = workflow(vec![
        status("campaign_queued", TaskPoolWorkflowStatusRoleDto::Queued, None, None, true),
        status("campaign_disabled", TaskPoolWorkflowStatusRoleDto::Executing, None, None, false),
        status("campaign_custom", TaskPoolWorkflowStatusRoleDto::Custom, None, None, true),
    ]);
    let bridge = LifecycleBridge::new(&workflow);

    let missing = bridge.resolve_status("missing_status").expect_err("missing status fails");
    assert!(missing.to_string().contains("workflow status"));

    let disabled = bridge.resolve_status("campaign_disabled").expect_err("disabled status fails");
    assert!(disabled.to_string().contains("disabled"));

    let custom = bridge.resolve_status("campaign_custom").expect_err("custom role fails");
    assert!(custom.to_string().contains("role custom"));

    let no_transition =
        bridge.resolve_outcome("campaign_queued", "success").expect_err("missing outcome fails");
    assert!(no_transition.to_string().contains("outcome"));
}

#[test]
fn lifecycle_bridge_rejects_cycle_in_disabled_outcome_chain() {
    let workflow = workflow(vec![
        status(
            "queued",
            TaskPoolWorkflowStatusRoleDto::Queued,
            Some(transition(" SUCCESS ", "disabled_a")),
            None,
            true,
        ),
        status(
            "disabled_a",
            TaskPoolWorkflowStatusRoleDto::Executing,
            Some(transition("succeed", "disabled_b")),
            None,
            false,
        ),
        status(
            "disabled_b",
            TaskPoolWorkflowStatusRoleDto::Review,
            Some(transition("success", "disabled_a")),
            None,
            false,
        ),
    ]);

    let error = LifecycleBridge::new(&workflow)
        .resolve_outcome_target_status_key("queued", "succeed")
        .expect_err("disabled status cycle must fail");

    assert!(error.to_string().contains("disabled status chain contains a cycle"));
    assert!(error.to_string().contains("disabled_a"));
}

fn workflow(statuses: Vec<TaskPoolWorkflowStatusDto>) -> TaskPoolWorkflowSnapshotDto {
    TaskPoolWorkflowSnapshotDto {
        id: "campaign_workflow".to_string(),
        label: "Campaign Workflow".to_string(),
        statuses,
    }
}

fn status(
    key: &str,
    role: TaskPoolWorkflowStatusRoleDto,
    transition: Option<TaskPoolWorkflowTransitionDto>,
    next: Option<&str>,
    enabled: bool,
) -> TaskPoolWorkflowStatusDto {
    TaskPoolWorkflowStatusDto {
        key: key.to_string(),
        label: key.to_string(),
        role,
        enabled,
        terminal: false,
        next: next.map(str::to_string),
        transitions: transition.into_iter().collect(),
    }
}

fn transition(outcome: &str, target: &str) -> TaskPoolWorkflowTransitionDto {
    TaskPoolWorkflowTransitionDto {
        outcome: outcome.to_string(),
        label: None,
        target: target.to_string(),
    }
}
