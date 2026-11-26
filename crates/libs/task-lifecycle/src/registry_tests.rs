//! 注册和替换必须完整保留步骤，并原子拒绝无效配置。
use crate::TaskLifecycleRegistry;
use vw_api_task_lifecycle::*;

fn custom_workflow(id: &str) -> TaskLifecycleWorkflowDto {
    TaskLifecycleWorkflowDto {
        id: id.into(),
        label: "Support Review".into(),
        built_in: false,
        initial_status_key: "queued".into(),
        statuses: [
            ("queued", TaskLifecycleStatusCategoryDto::Unstarted),
            ("running", TaskLifecycleStatusCategoryDto::Started),
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
                status_key: "queued".into(),
                handoff: TaskLifecycleStepHandoffDto::Member { member_id: "alice".into() },
            },
            TaskLifecycleStepDto {
                status_key: "running".into(),
                handoff: TaskLifecycleStepHandoffDto::Agent {
                    agent_id: "worker".into(),
                    instructions: "Resolve the request".into(),
                    completion_status_key: None,
                    return_status_key: Some("queued".into()),
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
fn task_lifecycle_registry_round_trips_step_rules() {
    let workflow = custom_workflow("support");
    let payload = serde_json::to_string(&workflow).unwrap();
    let registry =
        TaskLifecycleRegistry::from_custom_workflows(vec![serde_json::from_str(&payload).unwrap()])
            .unwrap();
    assert_eq!(registry.workflow_by_id("support"), Some(workflow.clone()));
    assert_eq!(
        registry.workflow_by_id("support").unwrap().completion_status_key("running"),
        Some("done")
    );
    let mut explicit = workflow;
    let TaskLifecycleStepHandoffDto::Agent { completion_status_key, .. } =
        &mut explicit.steps[1].handoff
    else {
        unreachable!()
    };
    *completion_status_key = Some("closed".into());
    explicit.validate().unwrap();
    assert_eq!(explicit.completion_status_key("running"), Some("closed"));
}

#[test]
fn task_lifecycle_registry_rejects_unknown_disabled_and_self_targets() {
    for target in ["missing", "running", " "] {
        let mut invalid = custom_workflow("invalid");
        let TaskLifecycleStepHandoffDto::Agent { completion_status_key, .. } =
            &mut invalid.steps[1].handoff
        else {
            unreachable!()
        };
        *completion_status_key = Some(target.into());
        assert!(TaskLifecycleRegistry::new().register(invalid).is_err(), "{target}");
    }
    let mut invalid = custom_workflow("disabled_target");
    invalid.statuses[2].enabled = false;
    assert!(
        TaskLifecycleRegistry::new()
            .register(invalid)
            .unwrap_err()
            .to_string()
            .contains("target is disabled")
    );
    let mut invalid = custom_workflow("missing_default");
    invalid.steps.truncate(2);
    assert!(
        TaskLifecycleRegistry::new()
            .register(invalid)
            .unwrap_err()
            .to_string()
            .contains("requires a completion target")
    );
}

#[test]
fn task_lifecycle_registry_rejects_invalid_assignees_and_instructions() {
    for id in ["", " ", " alice", "alice\n"] {
        let mut invalid = custom_workflow("member");
        invalid.steps[0].handoff = TaskLifecycleStepHandoffDto::Member { member_id: id.into() };
        assert!(TaskLifecycleRegistry::new().register(invalid).is_err());
        let mut invalid = custom_workflow("agent");
        let TaskLifecycleStepHandoffDto::Agent { agent_id, .. } = &mut invalid.steps[1].handoff
        else {
            unreachable!()
        };
        *agent_id = id.into();
        assert!(TaskLifecycleRegistry::new().register(invalid).is_err());
    }
    let mut invalid = custom_workflow("instructions");
    let TaskLifecycleStepHandoffDto::Agent { instructions, .. } = &mut invalid.steps[1].handoff
    else {
        unreachable!()
    };
    *instructions = " ".into();
    assert!(
        TaskLifecycleRegistry::new()
            .register(invalid)
            .unwrap_err()
            .to_string()
            .contains("requires instructions")
    );
}

#[test]
fn task_lifecycle_registry_enforces_terminal_and_initial_rules() {
    for index in [2, 3] {
        let mut invalid = custom_workflow("terminal_handoff");
        invalid.steps[index].handoff =
            TaskLifecycleStepHandoffDto::Member { member_id: "alice".into() };
        assert!(
            TaskLifecycleRegistry::new()
                .register(invalid)
                .unwrap_err()
                .to_string()
                .contains("terminal workflow status cannot hand off")
        );
        let mut invalid = custom_workflow("terminal_return");
        let target = invalid.statuses[index].key.clone();
        let TaskLifecycleStepHandoffDto::Agent { return_status_key, .. } =
            &mut invalid.steps[1].handoff
        else {
            unreachable!()
        };
        *return_status_key = Some(target);
        assert!(
            TaskLifecycleRegistry::new()
                .register(invalid)
                .unwrap_err()
                .to_string()
                .contains("cannot return to a terminal status")
        );
    }
    let mut invalid = custom_workflow("initial");
    invalid.statuses[0].enabled = false;
    assert!(TaskLifecycleRegistry::new().register(invalid).is_err());
}

#[test]
fn task_lifecycle_registry_replaces_atomically_and_preserves_builtin_ids() {
    let mut registry =
        TaskLifecycleRegistry::from_custom_workflows(vec![custom_workflow("first")]).unwrap();
    let original = registry.all_workflows();
    let mut invalid = custom_workflow("invalid");
    invalid.steps[0].status_key = "missing".into();
    assert!(registry.replace_custom_workflows(vec![custom_workflow("second"), invalid]).is_err());
    assert_eq!(registry.all_workflows(), original);
    assert!(
        registry
            .replace_custom_workflows(vec![custom_workflow("same"), custom_workflow("same")])
            .is_err()
    );
    registry.register(custom_workflow("dev_lifecycle")).unwrap();
    assert!(registry.workflow_by_id("dev_lifecycle").unwrap().built_in);
    assert!(registry.register(custom_workflow("first")).is_err());
    for id in ["", " surrounded ", "Uppercase"] {
        assert!(TaskLifecycleRegistry::new().register(custom_workflow(id)).is_err());
    }
}
