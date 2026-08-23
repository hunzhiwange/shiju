//! 验证任务生命周期注册表的注册、查询和重复处理。

use crate::task_lifecycle::TaskLifecycleRegistry;
use vw_api_types::task_lifecycle::{
    TaskLifecycleStatusDto, TaskLifecycleStatusRoleDto, TaskLifecycleStatusTransitionDto,
    TaskLifecycleWorkflowDto,
};

#[test]
fn task_lifecycle_registry_validates_custom_workflows() {
    let mut registry = TaskLifecycleRegistry::new();

    let built_ins = registry.all_workflows();
    assert_eq!(built_ins.len(), 5);
    assert!(built_ins.iter().all(|workflow| workflow.built_in));
    assert_eq!(
        built_ins.iter().map(|workflow| workflow.id.as_str()).collect::<Vec<_>>(),
        vec![
            "dev_lifecycle",
            "content_publishing",
            "marketing_campaign",
            "customer_support",
            "recruiting_pipeline",
        ]
    );
    let dev_lifecycle = built_ins
        .iter()
        .find(|workflow| workflow.id == "dev_lifecycle")
        .expect("dev lifecycle exists");
    let failed = dev_lifecycle
        .statuses
        .iter()
        .find(|status| status.key == "failed")
        .expect("dev lifecycle failed status exists");
    assert_eq!(failed.transitions.len(), 1);
    assert_eq!(failed.transitions[0].outcome, "retry");
    assert_eq!(failed.transitions[0].target, "pending");

    registry.register(custom_workflow("support_review")).expect("valid custom workflow");
    let workflows = registry.all_workflows();
    assert_eq!(workflows.len(), 6);
    assert_eq!(
        registry.workflow_by_id("support_review").expect("custom workflow is registered").label,
        "Support Review"
    );

    assert_validation_error(
        registry.register(custom_workflow("support_review")),
        "lifecycle workflow id already exists: support_review",
    );

    let mut invalid = custom_workflow("duplicate_status_key");
    invalid.statuses[1].key = "queued".to_string();
    assert_validation_error(
        TaskLifecycleRegistry::new().register(invalid),
        "workflow status key must be unique: queued",
    );

    let mut invalid = custom_workflow("bad_target");
    invalid.statuses[0].next = Some("missing".to_string());
    assert_validation_error(
        TaskLifecycleRegistry::new().register(invalid),
        "unknown workflow next target: missing",
    );

    let mut invalid = custom_workflow("bad_transition_target");
    invalid.statuses[1].transitions[0].target = "missing".to_string();
    assert_validation_error(
        TaskLifecycleRegistry::new().register(invalid),
        "unknown workflow transition target: missing",
    );

    let mut invalid = custom_workflow("cycle");
    invalid.statuses[2].next = Some("queued".to_string());
    invalid.statuses[2].terminal = false;
    assert_validation_error(
        TaskLifecycleRegistry::new().register(invalid),
        "lifecycle workflow cycle next chain: queued",
    );

    let mut invalid = custom_workflow("bad_terminal");
    invalid.statuses[2].terminal = false;
    assert_validation_error(
        TaskLifecycleRegistry::new().register(invalid),
        "lifecycle workflow status done terminal flag must match outgoing transitions",
    );

    let mut override_workflow = custom_workflow("dev_lifecycle");
    override_workflow.label = "Override".to_string();
    let mut override_registry = TaskLifecycleRegistry::new();
    override_registry
        .register(override_workflow)
        .expect("built-in workflow override should register");
    let overridden = override_registry
        .workflow_by_id("dev_lifecycle")
        .expect("built-in override should resolve");
    assert_eq!(overridden.label, "Override");
    assert!(overridden.built_in);
}

#[test]
fn task_lifecycle_registry_replaces_custom_workflows_atomically() {
    let mut registry = TaskLifecycleRegistry::from_custom_workflows(vec![custom_workflow("first")])
        .expect("initial custom workflow");

    let mut built_in_override = custom_workflow("dev_lifecycle");
    built_in_override.built_in = false;
    registry
        .replace_custom_workflows(vec![custom_workflow("second"), built_in_override])
        .expect("replacement workflows");

    assert_eq!(
        registry.custom_workflows().iter().map(|workflow| workflow.id.as_str()).collect::<Vec<_>>(),
        vec!["second", "dev_lifecycle"]
    );
    assert!(registry.custom_workflows()[1].built_in);

    let error = registry
        .replace_custom_workflows(vec![custom_workflow("duplicate"), custom_workflow("duplicate")])
        .expect_err("duplicate replacement ids should be rejected");
    assert_eq!(error.to_string(), "lifecycle workflow id duplicated: duplicate");
    assert_eq!(
        registry.custom_workflows().iter().map(|workflow| workflow.id.as_str()).collect::<Vec<_>>(),
        vec!["second", "dev_lifecycle"]
    );
}

#[test]
fn task_lifecycle_registry_rejects_invalid_ids_and_empty_metadata() {
    for (workflow, expected) in [
        (custom_workflow(""), "custom lifecycle workflow id must not be empty"),
        (
            custom_workflow(" surrounded "),
            "custom lifecycle workflow id must not contain surrounding whitespace:  surrounded ",
        ),
        (custom_workflow("Uppercase"), "custom lifecycle workflow id invalid: Uppercase"),
    ] {
        assert_validation_error(TaskLifecycleRegistry::new().register(workflow), expected);
    }

    let mut empty_label = custom_workflow("empty_label");
    empty_label.label = "  ".to_string();
    assert_validation_error(
        TaskLifecycleRegistry::new().register(empty_label),
        "custom lifecycle workflow label must not be empty: empty_label",
    );

    let mut empty_statuses = custom_workflow("empty_statuses");
    empty_statuses.statuses.clear();
    assert_validation_error(
        TaskLifecycleRegistry::new().register(empty_statuses),
        "custom lifecycle workflow must contain at least one status: empty_statuses",
    );
}

#[test]
fn task_lifecycle_registry_rejects_invalid_status_transition_contracts() {
    let mut invalid_key = custom_workflow("invalid_status_key");
    invalid_key.statuses[0].key = "Queued!".to_string();
    invalid_key.statuses[1].transitions[0].target = "Queued!".to_string();
    assert_validation_error(
        TaskLifecycleRegistry::new().register(invalid_key),
        "lifecycle workflow status key invalid: Queued!",
    );

    let mut empty_label = custom_workflow("empty_status_label");
    empty_label.statuses[1].label = " ".to_string();
    assert_validation_error(
        TaskLifecycleRegistry::new().register(empty_label),
        "lifecycle workflow status label must not be empty: empty_status_label:running",
    );

    let mut invalid_outcome = custom_workflow("invalid_outcome");
    invalid_outcome.statuses[1].transitions[0].outcome = "Failed!".to_string();
    assert_validation_error(
        TaskLifecycleRegistry::new().register(invalid_outcome),
        "lifecycle workflow transition outcome invalid: running:Failed!",
    );

    let mut duplicate_outcome = custom_workflow("duplicate_outcome");
    duplicate_outcome.statuses[1].transitions.push(transition("failed", "queued"));
    assert_validation_error(
        TaskLifecycleRegistry::new().register(duplicate_outcome),
        "lifecycle workflow status running transition outcome duplicated: failed",
    );

    let mut mismatched_success = custom_workflow("mismatched_success");
    mismatched_success.statuses[1].transitions.push(transition("succeed", "queued"));
    assert_validation_error(
        TaskLifecycleRegistry::new().register(mismatched_success),
        "lifecycle workflow status running next must match succeed transition",
    );
}

fn custom_workflow(id: &str) -> TaskLifecycleWorkflowDto {
    TaskLifecycleWorkflowDto {
        id: id.to_string(),
        label: "Support Review".to_string(),
        built_in: false,
        statuses: vec![
            status(
                "queued",
                "Queued",
                TaskLifecycleStatusRoleDto::Queued,
                false,
                true,
                false,
                true,
                Some("running"),
                Vec::new(),
            ),
            status(
                "running",
                "Running",
                TaskLifecycleStatusRoleDto::Executing,
                false,
                true,
                true,
                false,
                Some("done"),
                vec![transition("failed", "queued")],
            ),
            status(
                "done",
                "Done",
                TaskLifecycleStatusRoleDto::Done,
                true,
                false,
                false,
                false,
                None,
                Vec::new(),
            ),
        ],
    }
}

fn status(
    key: &str,
    label: &str,
    role: TaskLifecycleStatusRoleDto,
    terminal: bool,
    auto_execution_candidate: bool,
    occupies_execution_slot: bool,
    is_scheduling_backlog: bool,
    next: Option<&str>,
    transitions: Vec<TaskLifecycleStatusTransitionDto>,
) -> TaskLifecycleStatusDto {
    TaskLifecycleStatusDto {
        key: key.to_string(),
        label: label.to_string(),
        role,
        enabled: true,
        terminal,
        auto_execution_candidate,
        occupies_execution_slot,
        is_scheduling_backlog,
        next: next.map(str::to_string),
        transitions,
    }
}

fn transition(outcome: &str, target: &str) -> TaskLifecycleStatusTransitionDto {
    TaskLifecycleStatusTransitionDto {
        outcome: outcome.to_string(),
        label: None,
        target: target.to_string(),
    }
}

fn assert_validation_error(result: Result<(), anyhow::Error>, expected: &str) {
    let err = result.expect_err("workflow should be rejected");
    assert_eq!(err.to_string(), expected);
}
