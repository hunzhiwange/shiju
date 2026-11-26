//! Steps and runtime have independent lifetimes.
use super::lifecycle_bridge::LifecycleBridge;
use super::state_machine::AgentTaskStatus;
use vw_api_task::*;

pub(crate) fn workflow() -> TaskPoolWorkflowSnapshotDto {
    TaskPoolWorkflowSnapshotDto {
        id: "handoff".into(),
        label: "Handoff".into(),
        initial_status_key: "human".into(),
        statuses: vec![
            TaskPoolWorkflowStatusDto {
                key: "human".into(),
                label: "Human".into(),
                category: TaskPoolWorkflowStatusCategoryDto::Unstarted,
                enabled: true,
            },
            TaskPoolWorkflowStatusDto {
                key: "agent".into(),
                label: "Agent".into(),
                category: TaskPoolWorkflowStatusCategoryDto::Started,
                enabled: true,
            },
            TaskPoolWorkflowStatusDto {
                key: "done".into(),
                label: "Done".into(),
                category: TaskPoolWorkflowStatusCategoryDto::Done,
                enabled: true,
            },
        ],
        steps: vec![
            TaskPoolWorkflowStepDto {
                status_key: "human".into(),
                handoff: TaskPoolWorkflowStepHandoffDto::KeepAssignee,
            },
            TaskPoolWorkflowStepDto {
                status_key: "agent".into(),
                handoff: TaskPoolWorkflowStepHandoffDto::Agent {
                    agent_id: "main".into(),
                    instructions: "Review the evidence".into(),
                    completion_status_key: Some("done".into()),
                    return_status_key: Some("human".into()),
                },
            },
            TaskPoolWorkflowStepDto {
                status_key: "done".into(),
                handoff: TaskPoolWorkflowStepHandoffDto::KeepAssignee,
            },
        ],
    }
}
#[test]
fn lifecycle_bridge_dispatches_only_agent_steps_and_keeps_execution_key() {
    let workflow = workflow();
    workflow.validate_for_schedule().unwrap();
    let bridge = LifecycleBridge::new(&workflow);
    assert!(!bridge.dispatch_eligibility("human").unwrap().eligible);
    assert!(!bridge.dispatch_eligibility("done").unwrap().eligible);
    assert!(bridge.dispatch_eligibility("agent").unwrap().eligible);
    assert_eq!(bridge.execution_status_key("agent").unwrap(), "agent");
    assert!(bridge.execution_status_key("human").is_err());
    assert_eq!(bridge.resolve_outcome("agent", "retry").unwrap().status_key, "agent");
    assert!(bridge.resolve_outcome("agent", "success").is_err());
    assert_eq!(bridge.resolve_status("agent").unwrap().runtime_status, AgentTaskStatus::Queued);
}
#[test]
fn lifecycle_bridge_rejects_disabled_or_unconfigured_steps() {
    let mut workflow = workflow();
    workflow.statuses[1].enabled = false;
    let bridge = LifecycleBridge::new(&workflow);
    assert!(bridge.dispatch_eligibility("agent").is_err());
    assert!(bridge.dispatch_eligibility("missing").is_err());
}
