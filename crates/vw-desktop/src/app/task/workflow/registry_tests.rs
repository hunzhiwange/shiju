use super::{
    DEFAULT_TASK_WORKFLOW_ID, OwnedStatusDefinition, OwnedStatusTransition, OwnedTaskWorkflow,
    StatusOutcome, StatusRole, TaskWorkflowRegistry, all_task_workflows,
    is_builtin_task_workflow_id, reset_runtime_task_lifecycle_cache,
    runtime_task_lifecycle_cache_test_lock, task_workflow_by_id,
    update_runtime_task_lifecycle_cache,
};
use crate::app::task::models::TaskStatus;
use crate::app::task::{TaskStateMachine, TaskTransitionIntent};
use vw_gateway_client::vw_api_types::task_lifecycle::{
    TaskLifecycleStatusDto, TaskLifecycleStatusRoleDto, TaskLifecycleStatusTransitionDto,
    TaskLifecycleWorkflowDto,
};

fn custom_status(key: &'static str) -> TaskStatus {
    TaskStatus::parse_key(key).expect("custom status key should parse")
}

fn owned_status(
    key: &'static str,
    label: &'static str,
    role: StatusRole,
    next: Option<TaskStatus>,
) -> OwnedStatusDefinition {
    OwnedStatusDefinition {
        status: custom_status(key),
        key: key.to_string(),
        label: label.to_string(),
        role,
        enabled: true,
        is_terminal: next.is_none(),
        auto_execution_candidate: !matches!(role, StatusRole::Done | StatusRole::Archived),
        occupies_execution_slot: matches!(role, StatusRole::Executing | StatusRole::Planning),
        is_scheduling_backlog: matches!(
            role,
            StatusRole::Backlog | StatusRole::Queued | StatusRole::Planning
        ),
        next,
        transitions: Vec::new(),
    }
}

fn custom_workflow() -> OwnedTaskWorkflow {
    let triage = custom_status("triage");
    let in_progress = custom_status("custom_running");
    let done = custom_status("custom_done");

    OwnedTaskWorkflow {
        id: "custom_ops".to_string(),
        label: "Custom Ops".to_string(),
        statuses: vec![
            OwnedStatusDefinition {
                status: triage,
                key: "triage".to_string(),
                label: "Triage".to_string(),
                role: StatusRole::Queued,
                enabled: true,
                is_terminal: false,
                auto_execution_candidate: true,
                occupies_execution_slot: false,
                is_scheduling_backlog: true,
                next: Some(in_progress),
                transitions: Vec::new(),
            },
            OwnedStatusDefinition {
                status: in_progress,
                key: "custom_running".to_string(),
                label: "Custom Running".to_string(),
                role: StatusRole::Executing,
                enabled: true,
                is_terminal: false,
                auto_execution_candidate: true,
                occupies_execution_slot: true,
                is_scheduling_backlog: false,
                next: Some(done),
                transitions: Vec::new(),
            },
            OwnedStatusDefinition {
                status: done,
                key: "custom_done".to_string(),
                label: "Custom Done".to_string(),
                role: StatusRole::Done,
                enabled: true,
                is_terminal: true,
                auto_execution_candidate: false,
                occupies_execution_slot: false,
                is_scheduling_backlog: false,
                next: None,
                transitions: Vec::new(),
            },
        ],
    }
}

#[test]
fn custom_workflow_registers_and_resolves() {
    let workflow = custom_workflow();
    let custom_running = custom_status("custom_running");
    let mut registry = TaskWorkflowRegistry::new();

    registry.register(workflow).expect("custom workflow should register");

    let resolved = registry.workflow_by_id("custom_ops").expect("custom workflow should resolve");
    assert_eq!(resolved.id(), "custom_ops");
    assert_eq!(resolved.label(), "Custom Ops");
    assert_eq!(
        resolved.status_definition(custom_running).map(|definition| definition.role()),
        Some(StatusRole::Executing)
    );
    assert_eq!(resolved.next_status(custom_running), Some(custom_status("custom_done")));
    assert!(
        registry.all_workflows().iter().any(|workflow| workflow.id() == DEFAULT_TASK_WORKFLOW_ID)
    );
    assert!(registry.all_workflows().iter().any(|workflow| workflow.id() == "custom_ops"));
}

#[test]
fn custom_workflow_routes_intents_by_outcome() {
    let start = custom_status("branch_start");
    let done = custom_status("branch_done");
    let failed = custom_status("branch_failed");
    let mut start_definition =
        owned_status("branch_start", "Branch Start", StatusRole::Executing, Some(done));
    start_definition.transitions = vec![
        OwnedStatusTransition { outcome: StatusOutcome::succeed(), label: None, target: done },
        OwnedStatusTransition { outcome: StatusOutcome::failed(), label: None, target: failed },
    ];
    let workflow = OwnedTaskWorkflow {
        id: "branch_ops".to_string(),
        label: "Branch Ops".to_string(),
        statuses: vec![
            start_definition,
            owned_status("branch_done", "Branch Done", StatusRole::Done, None),
            owned_status("branch_failed", "Branch Failed", StatusRole::Failed, None),
        ],
    };
    let mut registry = TaskWorkflowRegistry::new();

    registry.register(workflow).expect("branch workflow should register");

    let resolved = registry.workflow_by_id("branch_ops").expect("branch workflow should resolve");
    assert_eq!(resolved.next_status_for_outcome(start, StatusOutcome::succeed()), Some(done));
    assert_eq!(resolved.next_status_for_outcome(start, StatusOutcome::failed()), Some(failed));

    let machine = TaskStateMachine::new(resolved);
    assert!(machine.can_transition(start, done, TaskTransitionIntent::ExecutionSucceeded));
    assert!(machine.can_transition(start, failed, TaskTransitionIntent::ExecutionFailed));
    assert_eq!(machine.next_for_intent(start, TaskTransitionIntent::ExecutionFailed), Some(failed));
    assert!(machine.is_known_transition(start, failed));
}

#[test]
fn disabled_status_is_skipped_by_success_chain() {
    let queued = custom_status("triage");
    let disabled = custom_status("custom_running");
    let done = custom_status("custom_done");
    let mut disabled_status =
        owned_status("custom_running", "Disabled Running", StatusRole::Executing, Some(done));
    disabled_status.enabled = false;
    let workflow = OwnedTaskWorkflow {
        id: "disabled_step_ops".to_string(),
        label: "Disabled Step Ops".to_string(),
        statuses: vec![
            owned_status("triage", "Triage", StatusRole::Queued, Some(disabled)),
            disabled_status,
            owned_status("custom_done", "Done", StatusRole::Done, None),
        ],
    };
    let mut registry = TaskWorkflowRegistry::new();

    registry.register(workflow).expect("workflow should register");

    let resolved = registry.workflow_by_id("disabled_step_ops").expect("workflow should resolve");
    assert_eq!(resolved.next_status(queued), Some(done));
    assert!(!resolved.role_occupies_execution_slot(disabled));
    assert!(!resolved.role_is_timeout_monitored(disabled));
}

#[test]
fn status_outcome_json_uses_string_keys_and_legacy_aliases() {
    let old_success: StatusOutcome =
        serde_json::from_str("\"success\"").expect("legacy success should deserialize");
    let old_failure: StatusOutcome =
        serde_json::from_str("\"failure\"").expect("legacy failure should deserialize");
    let old_other: StatusOutcome =
        serde_json::from_str("\"other\"").expect("legacy other should deserialize");

    assert_eq!(old_success.key(), "succeed");
    assert_eq!(old_failure.key(), "failed");
    assert_eq!(old_other.key(), "manual");
    assert_eq!(
        serde_json::to_string(&StatusOutcome::new("platform_rejected"))
            .expect("custom outcome should serialize"),
        "\"platform_rejected\""
    );
}

#[test]
fn status_outcome_labels_and_transition_labels_cover_builtin_and_custom_results() {
    let cases = [
        (StatusOutcome::succeed(), "成功后"),
        (StatusOutcome::failed(), "失败后"),
        (StatusOutcome::manual(), "人工/例外后"),
        (StatusOutcome::retry(), "重试后"),
        (StatusOutcome::new("platform_rejected"), "platform_rejected 后"),
    ];

    for (outcome, expected) in cases {
        assert_eq!(outcome.label(), expected);
    }
    assert!(StatusOutcome::failed().is_failed());
    assert!(StatusOutcome::retry().is_retry());

    let fallback = OwnedStatusTransition {
        outcome: StatusOutcome::failed(),
        label: Some("  ".to_string()),
        target: TaskStatus::Failed,
    };
    let explicit = OwnedStatusTransition {
        outcome: StatusOutcome::manual(),
        label: Some("  人工审核  ".to_string()),
        target: TaskStatus::CodeReview,
    };
    assert_eq!(fallback.display_label(), "失败后");
    assert_eq!(explicit.display_label(), "人工审核");
}

#[test]
fn invalid_custom_workflow_is_rejected() {
    let mut registry = TaskWorkflowRegistry::new();
    let mut duplicate = custom_workflow();
    duplicate.statuses.push(owned_status(
        "custom_running",
        "Duplicate Running",
        StatusRole::Executing,
        None,
    ));
    assert!(registry.register(duplicate).is_err());

    let mut cyclic = custom_workflow();
    cyclic.statuses[2].is_terminal = false;
    cyclic.statuses[2].next = Some(custom_status("triage"));
    assert!(registry.register(cyclic).is_err());

    let mut bad_builtin = custom_workflow();
    bad_builtin.statuses[0].status = TaskStatus::Running;
    bad_builtin.statuses[0].key = "not_running".to_string();
    assert!(registry.register(bad_builtin).is_err());
}

#[test]
fn runtime_lifecycle_cache_accepts_valid_dto_and_rejects_invalid_cache_shapes() {
    let _guard = runtime_task_lifecycle_cache_test_lock();
    reset_runtime_task_lifecycle_cache().expect("cache should reset before test");

    let workflow = lifecycle_workflow_dto();
    update_runtime_task_lifecycle_cache(vec![workflow.clone()])
        .expect("valid lifecycle workflow should replace cache");

    let resolved = task_workflow_by_id(" dto_ops ").expect("workflow id should be trimmed");
    let start = custom_status("dto_start");
    let done = custom_status("dto_done");
    let failed = custom_status("dto_failed");
    assert_eq!(resolved.id(), "dto_ops");
    assert_eq!(resolved.status_role(start), Some(StatusRole::Executing));
    assert!(resolved.occupies_execution_slot(start));
    assert!(resolved.role_is_timeout_monitored(start));
    assert_eq!(resolved.next_status(start), Some(done));
    assert_eq!(resolved.next_status_for_outcome(start, StatusOutcome::failed()), Some(failed));
    let failed_transition = resolved
        .status_transitions(start)
        .into_iter()
        .find(|transition| transition.outcome.is_failed())
        .expect("failed transition should be preserved");
    assert_eq!(failed_transition.display_label(), "失败");
    assert_eq!(
        all_task_workflows().iter().map(|workflow| workflow.id()).collect::<Vec<_>>(),
        vec!["dto_ops"]
    );
    assert!(!is_builtin_task_workflow_id("dto_ops"));
    assert!(is_builtin_task_workflow_id(DEFAULT_TASK_WORKFLOW_ID));

    assert!(update_runtime_task_lifecycle_cache(Vec::new()).is_err());

    let mut duplicate = workflow.clone();
    duplicate.label = "Duplicate".to_string();
    assert!(update_runtime_task_lifecycle_cache(vec![workflow.clone(), duplicate]).is_err());

    let mut whitespace_id = workflow.clone();
    whitespace_id.id = " dto_ops".to_string();
    assert!(update_runtime_task_lifecycle_cache(vec![whitespace_id]).is_err());

    let mut bad_transition = workflow;
    bad_transition.statuses[0].transitions[0].target = String::new();
    assert!(update_runtime_task_lifecycle_cache(vec![bad_transition]).is_err());

    reset_runtime_task_lifecycle_cache().expect("cache should reset after test");
}

#[test]
fn replace_custom_workflows_rejects_specific_definition_errors() {
    let mut registry = TaskWorkflowRegistry::new();
    registry
        .replace_custom_workflows(vec![custom_workflow()])
        .expect("valid workflow should replace registry");
    assert!(registry.workflow_by_id("custom_ops").is_some());

    let mut duplicate_id = custom_workflow();
    duplicate_id.label = "Duplicate".to_string();
    assert!(registry.replace_custom_workflows(vec![custom_workflow(), duplicate_id]).is_err());

    let mut blank_id = custom_workflow();
    blank_id.id = "  ".to_string();
    assert!(registry.replace_custom_workflows(vec![blank_id]).is_err());

    let mut blank_label = custom_workflow();
    blank_label.label = "  ".to_string();
    assert!(registry.replace_custom_workflows(vec![blank_label]).is_err());

    let mut no_statuses = custom_workflow();
    no_statuses.statuses.clear();
    assert!(registry.replace_custom_workflows(vec![no_statuses]).is_err());

    let mut bad_status_key = custom_workflow();
    bad_status_key.statuses[0].key = "Bad Key".to_string();
    assert!(registry.replace_custom_workflows(vec![bad_status_key]).is_err());

    let mut blank_status_label = custom_workflow();
    blank_status_label.statuses[0].label = " ".to_string();
    assert!(registry.replace_custom_workflows(vec![blank_status_label]).is_err());

    let mut unknown_next = custom_workflow();
    unknown_next.statuses[0].next = Some(custom_status("missing_target"));
    assert!(registry.replace_custom_workflows(vec![unknown_next]).is_err());

    let mut duplicate_outcome = custom_workflow();
    duplicate_outcome.statuses[0].transitions = vec![
        OwnedStatusTransition {
            outcome: StatusOutcome::failed(),
            label: None,
            target: custom_status("custom_done"),
        },
        OwnedStatusTransition {
            outcome: StatusOutcome::failed(),
            label: None,
            target: custom_status("custom_done"),
        },
    ];
    assert!(registry.replace_custom_workflows(vec![duplicate_outcome]).is_err());

    let mut mismatched_success = custom_workflow();
    mismatched_success.statuses[0].transitions = vec![OwnedStatusTransition {
        outcome: StatusOutcome::succeed(),
        label: None,
        target: custom_status("custom_done"),
    }];
    assert!(registry.replace_custom_workflows(vec![mismatched_success]).is_err());
}

#[test]
fn workflow_status_helpers_cover_disabled_cycles_and_role_flags() {
    let start = custom_status("loop_start");
    let disabled_a = custom_status("loop_a");
    let disabled_b = custom_status("loop_b");
    let mut workflow = custom_workflow();
    workflow.id = "disabled_loop".to_string();
    workflow.statuses = vec![
        owned_status("loop_start", "Loop Start", StatusRole::Queued, Some(disabled_a)),
        owned_status("loop_a", "Loop A", StatusRole::Planning, Some(disabled_b)),
        owned_status("loop_b", "Loop B", StatusRole::Failed, Some(disabled_a)),
    ];
    workflow.statuses[1].enabled = false;
    workflow.statuses[2].enabled = false;
    workflow.statuses[2].transitions = vec![OwnedStatusTransition {
        outcome: StatusOutcome::retry(),
        label: None,
        target: disabled_a,
    }];

    let resolved = crate::app::task::workflow::TaskWorkflow::from(workflow);
    assert_eq!(resolved.next_status(start), None);
    assert!(resolved.status_definition(start).expect("status").has_outgoing_transitions());
    assert!(resolved.role_is_scheduling_backlog(start));
    assert!(!resolved.role_is_retry_candidate(disabled_b));
    assert!(!resolved.role_occupies_execution_slot(disabled_a));
    assert_eq!(resolved.status_transitions(custom_status("unknown")), Vec::new());

    let mut fallback = owned_status("fallback", "Fallback", StatusRole::Queued, Some(start));
    assert_eq!(fallback.transition_target(StatusOutcome::succeed()), Some(start));
    assert_eq!(fallback.outgoing_transitions()[0].outcome.key(), "succeed");
    fallback.next = None;
    assert!(!fallback.has_outgoing_transitions());

    assert_eq!(StatusOutcome::new(" success ").key(), "succeed");
    assert!(StatusOutcome::succeed().is_succeed());
}

fn lifecycle_workflow_dto() -> TaskLifecycleWorkflowDto {
    TaskLifecycleWorkflowDto {
        id: "dto_ops".to_string(),
        label: "DTO Ops".to_string(),
        built_in: false,
        statuses: vec![
            TaskLifecycleStatusDto {
                key: "dto_start".to_string(),
                label: "DTO Start".to_string(),
                role: TaskLifecycleStatusRoleDto::Executing,
                enabled: true,
                terminal: false,
                auto_execution_candidate: true,
                occupies_execution_slot: true,
                is_scheduling_backlog: false,
                next: Some("dto_done".to_string()),
                transitions: vec![TaskLifecycleStatusTransitionDto {
                    outcome: "failed".to_string(),
                    label: Some("失败".to_string()),
                    target: "dto_failed".to_string(),
                }],
            },
            TaskLifecycleStatusDto {
                key: "dto_done".to_string(),
                label: "DTO Done".to_string(),
                role: TaskLifecycleStatusRoleDto::Done,
                enabled: true,
                terminal: true,
                auto_execution_candidate: false,
                occupies_execution_slot: false,
                is_scheduling_backlog: false,
                next: None,
                transitions: Vec::new(),
            },
            TaskLifecycleStatusDto {
                key: "dto_failed".to_string(),
                label: "DTO Failed".to_string(),
                role: TaskLifecycleStatusRoleDto::Failed,
                enabled: true,
                terminal: true,
                auto_execution_candidate: false,
                occupies_execution_slot: false,
                is_scheduling_backlog: false,
                next: None,
                transitions: Vec::new(),
            },
        ],
    }
}
