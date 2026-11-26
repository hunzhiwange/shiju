//! 新工作流配置经桌面投影及调度快照后保持交接规则。
use super::*;
use vw_api_task_lifecycle::*;

fn lifecycle_workflow_dto() -> TaskLifecycleWorkflowDto {
    TaskLifecycleWorkflowDto {
        id: "custom_ops".into(),
        label: "Custom Ops".into(),
        built_in: false,
        initial_status_key: "triage".into(),
        statuses: [
            ("triage", TaskLifecycleStatusCategoryDto::Unstarted),
            ("working", TaskLifecycleStatusCategoryDto::Started),
            ("done", TaskLifecycleStatusCategoryDto::Done),
            ("closed", TaskLifecycleStatusCategoryDto::Closed),
        ]
        .into_iter()
        .map(|(key, category)| TaskLifecycleStatusDto {
            key: key.into(),
            label: key.into(),
            category,
            enabled: true,
        })
        .collect(),
        steps: vec![
            TaskLifecycleStepDto {
                status_key: "triage".into(),
                handoff: TaskLifecycleStepHandoffDto::Member { member_id: "alice".into() },
            },
            TaskLifecycleStepDto {
                status_key: "working".into(),
                handoff: TaskLifecycleStepHandoffDto::Agent {
                    agent_id: "worker".into(),
                    instructions: "Resolve the request".into(),
                    completion_status_key: None,
                    return_status_key: Some("triage".into()),
                },
            },
            TaskLifecycleStepDto {
                status_key: "done".into(),
                handoff: TaskLifecycleStepHandoffDto::KeepAssignee,
            },
            TaskLifecycleStepDto {
                status_key: "closed".into(),
                handoff: TaskLifecycleStepHandoffDto::KeepAssignee,
            },
        ],
    }
}

#[test]
fn lifecycle_projection_preserves_step_rules_and_categories() {
    let dto = lifecycle_workflow_dto();
    let workflow = registry::task_workflow_from_lifecycle_dto(&dto).unwrap();
    assert_eq!(workflow.steps, dto.steps);
    assert_eq!(lifecycle_dto_from_task_workflow(workflow.clone(), false), dto);
    let snapshot = task_workflow_snapshot_for_gateway(&workflow);
    assert_eq!(snapshot.steps, dto.steps);
    assert_eq!(snapshot.completion_status_key("working"), Some("done"));
    assert_eq!(task_workflow_from_snapshot(&snapshot), Some(workflow));
}

#[test]
fn lifecycle_cache_replaces_atomically_and_keeps_handoff_configuration() {
    let _guard = runtime_task_lifecycle_cache_test_lock();
    reset_runtime_task_lifecycle_cache().unwrap();
    let mut workflows = builtin_task_workflows()
        .iter()
        .map(|workflow| lifecycle_dto_from_task_workflow(TaskWorkflow::from(*workflow), true))
        .collect::<Vec<_>>();
    let dto = lifecycle_workflow_dto();
    workflows.push(dto.clone());
    update_runtime_task_lifecycle_cache(workflows.clone()).unwrap();
    let restored = task_workflow_by_id("custom_ops").unwrap();
    assert_eq!(lifecycle_dto_from_task_workflow(restored, false), dto);
    let last = workflows.last_mut().unwrap();
    last.steps[0].handoff = TaskLifecycleStepHandoffDto::Member { member_id: " ".into() };
    assert!(update_runtime_task_lifecycle_cache(workflows).is_err());
    assert_eq!(task_workflow_by_id("custom_ops").unwrap().steps, dto.steps);
    reset_runtime_task_lifecycle_cache().unwrap();
}

#[test]
fn local_registry_rejects_unknown_step_targets() {
    let dto = lifecycle_workflow_dto();
    let workflow = registry::task_workflow_from_lifecycle_dto(&dto).unwrap();
    let mut owned = OwnedTaskWorkflow {
        id: workflow.id,
        label: workflow.label,
        statuses: workflow.statuses,
        initial_status_key: workflow.initial_status_key,
        steps: workflow.steps,
    };
    let mut registry = TaskWorkflowRegistry::new();
    registry.register(owned.clone()).unwrap();
    owned.id = "invalid".into();
    let TaskLifecycleStepHandoffDto::Agent { completion_status_key, .. } =
        &mut owned.steps[1].handoff
    else {
        unreachable!()
    };
    *completion_status_key = Some("missing".into());
    assert!(registry.register(owned).is_err());
    assert_eq!(registry.all_workflows().len(), builtin_task_workflows().len() + 1);
}
