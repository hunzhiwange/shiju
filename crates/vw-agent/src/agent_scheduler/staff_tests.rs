//! 测试职员配置到团队、阶段和任务分配的调度转换。
//! 配置夹具覆盖缺省值和非法引用，确保生成的团队计划不包含悬空成员或阶段。

use super::*;
use crate::agent_scheduler::domain::{AgentTaskKind, TaskId, WorkerName};
use vw_api_types::agent_scheduler::{
    AgentSchedulerCommandKindDto, AgentSchedulerStageActionInputsDto,
    AgentSchedulerStageActionOutcomeDto, AgentTaskStatusDto,
};
use vw_api_types::task::TaskPoolWorkflowTransitionDto;
use vw_config_types::staff::{
    StaffCapabilityConfig, StaffCapabilityIoSummaryConfig, StaffCapabilityKindConfig, StaffConfig,
    StaffMemberConfig, StaffMemberTaskAssignmentConfig, StaffStageActionConfig,
    StaffStageAssignmentConfig, StaffStageWorkflowBindingConfig, StaffWorkflowStageConfig,
};

#[test]
fn staff_status_role_aliases_map_to_workflow_roles() {
    let cases = [
        (" backlog ", TaskPoolWorkflowStatusRoleDto::Backlog),
        ("pending", TaskPoolWorkflowStatusRoleDto::Queued),
        ("assigned", TaskPoolWorkflowStatusRoleDto::Planning),
        ("active", TaskPoolWorkflowStatusRoleDto::Executing),
        ("blocked", TaskPoolWorkflowStatusRoleDto::Failed),
        ("paused", TaskPoolWorkflowStatusRoleDto::Paused),
        ("submitted", TaskPoolWorkflowStatusRoleDto::ReviewReady),
        ("code_review", TaskPoolWorkflowStatusRoleDto::Review),
        ("pr_submitted", TaskPoolWorkflowStatusRoleDto::Merging),
        ("done", TaskPoolWorkflowStatusRoleDto::Done),
        ("archived", TaskPoolWorkflowStatusRoleDto::Archived),
    ];

    for (status_key, expected) in cases {
        assert_eq!(status_role_for_key(status_key), Some(expected));
    }
    assert_eq!(status_role_for_key("custom-status"), None);
}

#[test]
fn staff_agent_status_mapping_identifies_only_active_execution_states() {
    let running = [
        AgentTaskStatus::Assigned,
        AgentTaskStatus::Dispatched,
        AgentTaskStatus::Acknowledged,
        AgentTaskStatus::Active,
        AgentTaskStatus::Running,
    ];
    for status in running {
        assert!(agent_task_status_is_running(&agent_task_status_to_dto(status)));
    }

    let stopped = [
        AgentTaskStatus::Draft,
        AgentTaskStatus::Queued,
        AgentTaskStatus::Submitted,
        AgentTaskStatus::Revision,
        AgentTaskStatus::Blocked,
        AgentTaskStatus::Paused,
        AgentTaskStatus::Completed,
        AgentTaskStatus::Failed,
        AgentTaskStatus::Cancelled,
        AgentTaskStatus::Archived,
    ];
    for status in stopped {
        assert!(!agent_task_status_is_running(&agent_task_status_to_dto(status)));
    }
}

#[test]
fn staff_metadata_helpers_reject_blank_invalid_and_overflow_values() {
    let metadata = serde_json::json!({
        "task_board.status_role": "review_ready",
        "name": "  reviewer  ",
        "blank": "   ",
        "capacity": u32::MAX,
        "overflow": u64::from(u32::MAX) + 1,
        "order": -7
    });

    assert_eq!(
        task_board_status_role_from_metadata(&metadata),
        Some(TaskPoolWorkflowStatusRoleDto::ReviewReady)
    );
    assert_eq!(string_field(&metadata, "name").as_deref(), Some("reviewer"));
    assert_eq!(string_field(&metadata, "blank"), None);
    assert_eq!(u32_field(&metadata, "capacity"), Some(u32::MAX));
    assert_eq!(u32_field(&metadata, "overflow"), None);
    assert_eq!(i64_field(&metadata, "order"), Some(-7));
    assert_eq!(non_empty("  value  ", "field"), Ok("value"));
    assert_eq!(non_empty("  ", "field"), Err("field must not be empty".to_string()));
}

#[test]
fn staff_stage_action_outcome_errors_are_normalized_for_users() {
    assert_eq!(stage_action_outcome_key(AgentSchedulerStageActionOutcomeDto::Success), "success");
    assert_eq!(stage_action_outcome_key(AgentSchedulerStageActionOutcomeDto::Failure), "failure");
    assert_eq!(stage_action_outcome_key(AgentSchedulerStageActionOutcomeDto::Retry), "retry");
    assert_eq!(stage_action_outcome_key(AgentSchedulerStageActionOutcomeDto::Blocked), "blocked");
    assert_eq!(
        stage_action_outcome_error(
            "review",
            "retry",
            anyhow::anyhow!("status review has no transition for retry")
        ),
        "stage action outcome retry has no workflow transition from review"
    );
    assert_eq!(
        stage_action_outcome_error(
            "missing",
            "success",
            anyhow::anyhow!("status missing not found in workflow")
        ),
        "unknown current workflow status for stage action outcome: missing"
    );
    assert_eq!(
        stage_action_outcome_error("review", "success", anyhow::anyhow!("storage unavailable")),
        "storage unavailable"
    );
}

#[test]
fn agent_scheduler_capability_catalog_merges_builtin_and_custom() {
    let mut config = StaffConfig::default();
    config.project_state.capabilities = vec![
        StaffCapabilityConfig {
            id: "release_workflow".to_string(),
            label: "Release Workflow".to_string(),
            capability_type: StaffCapabilityKindConfig::SavedWorkflow,
            built_in: false,
            input_summary: StaffCapabilityIoSummaryConfig {
                description: "Workflow UUID and release channel".to_string(),
                fields: vec!["workflow_uuid".to_string(), "release_channel".to_string()],
            },
            result_summary: StaffCapabilityIoSummaryConfig {
                description: "Release automation result".to_string(),
                fields: vec!["summary".to_string()],
            },
        },
        StaffCapabilityConfig {
            id: CODE_REVIEW_CAPABILITY_ID.to_string(),
            label: "Overridden Review".to_string(),
            capability_type: StaffCapabilityKindConfig::SavedWorkflow,
            built_in: false,
            input_summary: StaffCapabilityIoSummaryConfig::default(),
            result_summary: StaffCapabilityIoSummaryConfig::default(),
        },
    ];

    let catalog = stage_capabilities_for_config(&config);

    let ids =
        catalog.iter().map(|capability| capability.capability_id.as_str()).collect::<Vec<_>>();
    assert!(ids.contains(&CODE_REVIEW_CAPABILITY_ID));
    assert!(ids.contains(&"release_workflow"));
    assert_eq!(
        catalog
            .iter()
            .filter(|capability| capability.capability_id == CODE_REVIEW_CAPABILITY_ID)
            .count(),
        1
    );

    let code_review = catalog
        .iter()
        .find(|capability| capability.capability_id == CODE_REVIEW_CAPABILITY_ID)
        .expect("built-in code review capability");
    assert_eq!(code_review.label, "代码审核");
    assert!(code_review.built_in);
    let advance_status = catalog
        .iter()
        .find(|capability| capability.capability_id == ADVANCE_STATUS_CAPABILITY_ID)
        .expect("built-in advance status capability");
    assert!(advance_status.supported_roles.contains(&TaskPoolWorkflowStatusRoleDto::Review));
    assert!(advance_status.supported_roles.contains(&TaskPoolWorkflowStatusRoleDto::Merging));

    let custom = catalog
        .iter()
        .find(|capability| capability.capability_id == "release_workflow")
        .expect("custom release workflow capability");
    assert!(!custom.built_in);
    assert_eq!(custom.label, "Release Workflow");
    assert!(custom.requires_workflow_uuid);
    assert!(custom.supported_roles.contains(&TaskPoolWorkflowStatusRoleDto::Queued));
}

#[test]
fn agent_scheduler_stage_action_validation_allows_catalog_capability_inputs() {
    let mut config = StaffConfig::default();
    config.project_state.capabilities = vec![StaffCapabilityConfig {
        id: "release_workflow".to_string(),
        label: "Release Workflow".to_string(),
        capability_type: StaffCapabilityKindConfig::SavedWorkflow,
        built_in: false,
        input_summary: StaffCapabilityIoSummaryConfig::default(),
        result_summary: StaffCapabilityIoSummaryConfig::default(),
    }];
    let catalog = stage_capabilities_for_config(&config);
    let inputs = AgentSchedulerStageActionInputsDto {
        workflow_uuid: Some("018ff4b0-1d0d-7a2c-a30c-7bcf9ee1f099".to_string()),
        parameters: [("release_channel".to_string(), serde_json::json!("stable"))]
            .into_iter()
            .collect(),
    };
    validate_stage_action_payload(
        "review",
        CODE_REVIEW_CAPABILITY_ID,
        None,
        &AgentSchedulerStageActionInputsDto::default(),
        Some(TaskPoolWorkflowStatusRoleDto::Review),
        &catalog,
    )
    .expect("catalog capability should accept supported status role");
    validate_stage_action_payload(
        "planning",
        "release_workflow",
        None,
        &inputs,
        Some(TaskPoolWorkflowStatusRoleDto::Planning),
        &catalog,
    )
    .expect("custom saved_workflow capability should accept input workflow uuid");

    let error = validate_stage_action_payload(
        "planning",
        "missing_capability",
        None,
        &AgentSchedulerStageActionInputsDto::default(),
        None,
        &catalog,
    )
    .expect_err("unknown capability still fails");
    assert!(error.contains("missing_capability"));
}

#[test]
fn agent_scheduler_stage_action_validation_rejects_unsupported_role() {
    let catalog = stage_capabilities();

    let error = validate_stage_action_payload(
        "pending",
        CODE_REVIEW_CAPABILITY_ID,
        None,
        &AgentSchedulerStageActionInputsDto::default(),
        Some(TaskPoolWorkflowStatusRoleDto::Queued),
        &catalog,
    )
    .expect_err("code_review should not be accepted for queued stages");

    assert!(error.contains(CODE_REVIEW_CAPABILITY_ID));
    assert!(error.contains("role"));
    assert!(error.contains("queued"));
}

#[test]
fn staff_stage_action_config_builds_runtime_action_for_stage_status() {
    let mut config = StaffConfig::default();
    config.workflow_stages = vec![
        StaffWorkflowStageConfig {
            id: "intake_stage".to_string(),
            name: "Intake".to_string(),
            task_status: "queued".to_string(),
            role: Some("orchestrator".to_string()),
            enabled: true,
            display_only: false,
        },
        StaffWorkflowStageConfig {
            id: "review_stage".to_string(),
            name: "Review".to_string(),
            task_status: "code_complete".to_string(),
            role: Some("reviewer".to_string()),
            enabled: true,
            display_only: false,
        },
    ];
    config.project_state.stage_actions = vec![StaffStageActionConfig {
        stage_id: "intake_stage".to_string(),
        capability_id: EXECUTE_TASK_CAPABILITY_ID.to_string(),
        workflow_uuid: None,
        inputs: Default::default(),
    }];
    let workflow = workflow_snapshot_from_staff_config(&config);

    let runtime = runtime_config(&config, &workflow, &[]).expect("runtime config");
    let action = runtime
        .actions
        .iter()
        .find(|action| action.stage_id == "intake_stage")
        .expect("stage action");

    assert_eq!(action.stage_id, "intake_stage");
    assert_eq!(action.status_key, "queued");
    assert_eq!(action.capability_id, EXECUTE_TASK_CAPABILITY_ID);
}

#[test]
fn staff_stage_action_outcome_skips_disabled_status() {
    let mut disabled = business_status(
        "campaign_review",
        "Review",
        TaskPoolWorkflowStatusRoleDto::Review,
        Some("campaign_done"),
    );
    disabled.enabled = false;
    let workflow = TaskPoolWorkflowSnapshotDto {
        id: "marketing_campaign".to_string(),
        label: "Marketing Campaign".to_string(),
        statuses: vec![
            business_status(
                "campaign_queued",
                "Queued",
                TaskPoolWorkflowStatusRoleDto::Queued,
                Some("campaign_review"),
            ),
            disabled,
            business_status("campaign_done", "Done", TaskPoolWorkflowStatusRoleDto::Done, None),
        ],
    };

    let target = resolve_stage_action_outcome_target_status_key(
        &workflow,
        "campaign_queued",
        AgentSchedulerStageActionOutcomeDto::Success,
    )
    .expect("success target should resolve");

    assert_eq!(target, "campaign_done");
}

#[test]
fn staff_stage_action_retry_outcome_uses_explicit_transition() {
    let mut failed =
        business_status("campaign_failed", "Failed", TaskPoolWorkflowStatusRoleDto::Failed, None);
    failed.transitions = vec![TaskPoolWorkflowTransitionDto {
        outcome: "retry".to_string(),
        label: Some("Retry".to_string()),
        target: "campaign_queued".to_string(),
    }];
    let workflow = TaskPoolWorkflowSnapshotDto {
        id: "marketing_campaign".to_string(),
        label: "Marketing Campaign".to_string(),
        statuses: vec![
            failed,
            business_status(
                "campaign_queued",
                "Queued",
                TaskPoolWorkflowStatusRoleDto::Queued,
                None,
            ),
        ],
    };

    let target = resolve_stage_action_outcome_target_status_key(
        &workflow,
        "campaign_failed",
        AgentSchedulerStageActionOutcomeDto::Retry,
    )
    .expect("retry target should resolve");

    assert_eq!(target, "campaign_queued");
}

#[test]
fn staff_workflow_snapshot_adds_retry_transition_for_failed_stage() {
    let mut config = StaffConfig::default();
    config.workflow_stages = vec![
        business_stage("pending", "Pending", "orchestrator"),
        business_stage("running", "Running", "executor"),
        business_stage("failed", "Failed", "executor"),
        business_stage("pending_retry", "Pending Retry", "orchestrator"),
    ];

    let workflow = workflow_snapshot_from_staff_config(&config);
    let failed = workflow
        .statuses
        .iter()
        .find(|status| status.key == "failed")
        .expect("failed stage exists");

    assert_eq!(failed.next.as_deref(), Some("pending_retry"));
    assert_eq!(failed.transitions.len(), 1);
    assert_eq!(failed.transitions[0].outcome, "retry");
    assert_eq!(failed.transitions[0].target, "pending_retry");
}

#[test]
fn staff_runtime_restores_project_workflow_stage_actions_after_restart() {
    let mut config = StaffConfig::default();
    config.project_state.stage_actions = vec![StaffStageActionConfig {
        stage_id: "campaign_planning".to_string(),
        capability_id: SAVED_WORKFLOW_CAPABILITY_ID.to_string(),
        workflow_uuid: Some("018ff4b0-1d0d-7a2c-a30c-7bcf9ee1f041".to_string()),
        inputs: Default::default(),
    }];
    let workflow = TaskPoolWorkflowSnapshotDto {
        id: "marketing_campaign".to_string(),
        label: "Marketing Campaign".to_string(),
        statuses: vec![
            business_status(
                "campaign_queued",
                "Queued",
                TaskPoolWorkflowStatusRoleDto::Queued,
                Some("campaign_planning"),
            ),
            business_status(
                "campaign_planning",
                "Planning",
                TaskPoolWorkflowStatusRoleDto::Planning,
                Some("campaign_done"),
            ),
            business_status("campaign_done", "Done", TaskPoolWorkflowStatusRoleDto::Done, None),
        ],
    };

    let runtime = runtime_config(&config, &workflow, &[]).expect("runtime config");
    let action = runtime
        .actions
        .iter()
        .find(|action| action.stage_id == "campaign_planning")
        .expect("saved workflow action");

    assert_eq!(action.status_key, "campaign_planning");
    assert_eq!(action.capability_id, SAVED_WORKFLOW_CAPABILITY_ID);
    assert_eq!(action.workflow_uuid.as_deref(), Some("018ff4b0-1d0d-7a2c-a30c-7bcf9ee1f041"));
    assert!(runtime.stages.iter().any(|stage| {
        stage.stage_id == "campaign_planning" && stage.role.as_deref() == Some("planner")
    }));
}

#[test]
fn staff_auto_schedule_ignores_disabled_workflow_stage_state() {
    let mut config = StaffConfig::default();
    config.project_state.stage_actions = vec![StaffStageActionConfig {
        stage_id: "planning".to_string(),
        capability_id: EXECUTE_TASK_CAPABILITY_ID.to_string(),
        workflow_uuid: None,
        inputs: Default::default(),
    }];
    config.project_state.stage_assignments = vec![StaffStageAssignmentConfig {
        stage_id: "planning".to_string(),
        primary_member_id: Some("staff-planner".to_string()),
        backup_member_ids: Vec::new(),
        manager_member_ids: Vec::new(),
        autonomous: true,
    }];
    let mut workflow = workflow_snapshot_from_staff_config(&config);
    let planning = workflow
        .statuses
        .iter_mut()
        .find(|status| status.key == "planning")
        .expect("planning status");
    planning.enabled = false;
    let task = staff_task("task-planning", "planning", AgentTaskStatusDto::Queued);

    let projection =
        build_staff_projection(&[task.clone()], &config, &workflow, &[]).expect("projection");
    let plan = build_staff_auto_schedule_plan(
        &[task],
        &config,
        &projection,
        &workflow,
        &[],
        TimestampMs(1_783_000_000_000),
    )
    .expect("auto schedule plan");

    assert!(projection.lanes.iter().any(|lane| lane.status_key == "planning" && lane.display_only));
    assert!(plan.commands.is_empty());
    assert!(plan.command_contexts.is_empty());
    assert!(plan.skipped.is_empty());
}

#[test]
fn staff_runtime_defaults_business_builtin_stage_actions() {
    let mut config = StaffConfig::default();
    config.workflow_stages = vec![
        business_stage("campaign_ideas", "Ideas", "orchestrator"),
        business_stage("campaign_queued", "Queued", "orchestrator"),
        business_stage("campaign_planning", "Planning", "planner"),
        business_stage("campaign_running", "Running", "executor"),
        business_stage("campaign_review_ready", "Review Ready", "reviewer"),
        business_stage("campaign_reviewing", "Reviewing", "reviewer"),
        business_stage("campaign_closing", "Closing", "merger"),
        StaffWorkflowStageConfig {
            id: "campaign_done".to_string(),
            name: "Done".to_string(),
            task_status: "campaign_done".to_string(),
            role: None,
            enabled: true,
            display_only: true,
        },
    ];
    let workflow = TaskPoolWorkflowSnapshotDto {
        id: "marketing_campaign".to_string(),
        label: "Marketing Campaign".to_string(),
        statuses: vec![
            business_status(
                "campaign_ideas",
                "Ideas",
                TaskPoolWorkflowStatusRoleDto::Backlog,
                Some("campaign_queued"),
            ),
            business_status(
                "campaign_queued",
                "Queued",
                TaskPoolWorkflowStatusRoleDto::Queued,
                Some("campaign_planning"),
            ),
            business_status(
                "campaign_planning",
                "Planning",
                TaskPoolWorkflowStatusRoleDto::Planning,
                Some("campaign_running"),
            ),
            business_status(
                "campaign_running",
                "Running",
                TaskPoolWorkflowStatusRoleDto::Executing,
                Some("campaign_review_ready"),
            ),
            business_status(
                "campaign_review_ready",
                "Review Ready",
                TaskPoolWorkflowStatusRoleDto::ReviewReady,
                Some("campaign_reviewing"),
            ),
            business_status(
                "campaign_reviewing",
                "Reviewing",
                TaskPoolWorkflowStatusRoleDto::Review,
                Some("campaign_closing"),
            ),
            business_status(
                "campaign_closing",
                "Closing",
                TaskPoolWorkflowStatusRoleDto::Merging,
                Some("campaign_done"),
            ),
            business_status("campaign_done", "Done", TaskPoolWorkflowStatusRoleDto::Done, None),
        ],
    };

    let runtime = runtime_config(&config, &workflow, &[]).expect("runtime config");

    assert_eq!(
        runtime_action_capability(&runtime, "campaign_ideas"),
        Some(ADVANCE_STATUS_CAPABILITY_ID)
    );
    assert_eq!(
        runtime_action_capability(&runtime, "campaign_queued"),
        Some(EXECUTE_TASK_CAPABILITY_ID)
    );
    assert_eq!(
        runtime_action_capability(&runtime, "campaign_planning"),
        Some(EXECUTE_TASK_CAPABILITY_ID)
    );
    assert_eq!(runtime_action_capability(&runtime, "campaign_running"), None);
    assert_eq!(
        runtime_action_capability(&runtime, "campaign_review_ready"),
        Some(ADVANCE_STATUS_CAPABILITY_ID)
    );
    assert_eq!(runtime_action_capability(&runtime, "campaign_reviewing"), None);
    assert_eq!(runtime_action_capability(&runtime, "campaign_closing"), None);
}

#[test]
fn staff_workflow_orchestration_defaults_builtin_autonomous_assignments() {
    let workflow = TaskPoolWorkflowSnapshotDto {
        id: "content_publishing".to_string(),
        label: "Content Publishing".to_string(),
        statuses: vec![
            business_status(
                "content_idea",
                "Idea",
                TaskPoolWorkflowStatusRoleDto::Backlog,
                Some("content_queued"),
            ),
            business_status(
                "content_queued",
                "Queued",
                TaskPoolWorkflowStatusRoleDto::Queued,
                Some("content_outline"),
            ),
            business_status(
                "content_outline",
                "Outline",
                TaskPoolWorkflowStatusRoleDto::Planning,
                Some("content_writing"),
            ),
            business_status(
                "content_writing",
                "Writing",
                TaskPoolWorkflowStatusRoleDto::Executing,
                Some("content_review_ready"),
            ),
            business_status(
                "content_review_ready",
                "Review Ready",
                TaskPoolWorkflowStatusRoleDto::ReviewReady,
                Some("content_reviewing"),
            ),
            business_status(
                "content_reviewing",
                "Reviewing",
                TaskPoolWorkflowStatusRoleDto::Review,
                Some("content_publishing"),
            ),
            business_status(
                "content_publishing",
                "Publishing",
                TaskPoolWorkflowStatusRoleDto::Merging,
                Some("content_published"),
            ),
            business_status(
                "content_published",
                "Published",
                TaskPoolWorkflowStatusRoleDto::Done,
                None,
            ),
        ],
    };

    let defaulted = staff_config_for_workflow_orchestration(
        &StaffConfig::default(),
        "content_publishing",
        &workflow,
    );
    assert_eq!(
        defaulted
            .project_state
            .stage_assignments
            .iter()
            .map(|assignment| {
                (assignment.stage_id.as_str(), assignment.primary_member_id.as_deref())
            })
            .collect::<Vec<_>>(),
        vec![
            ("content_idea", Some("staff-orchestrator")),
            ("content_queued", Some("staff-orchestrator")),
            ("content_outline", Some("staff-planner")),
            ("content_review_ready", Some("staff-reviewer")),
            ("content_reviewing", Some("staff-reviewer")),
            ("content_publishing", Some("staff-merger")),
        ]
    );
    assert!(
        defaulted.project_state.stage_assignments.iter().all(|assignment| assignment.autonomous)
    );

    let task = staff_task("task-content", "content_idea", AgentTaskStatusDto::Draft);
    let projection = build_staff_projection(&[task.clone()], &defaulted, &workflow, &[])
        .expect("content publishing projection");
    let plan = build_staff_auto_schedule_plan(
        &[task],
        &defaulted,
        &projection,
        &workflow,
        &[],
        TimestampMs(1_783_000_000_000),
    )
    .expect("content publishing auto schedule plan");
    assert_eq!(
        plan.commands,
        vec![StaffCommandDto::AssignTask {
            command_id: "staff-auto-assign-task-content-staff-orchestrator-1783000000000"
                .to_string(),
            task_id: "task-content".to_string(),
            member_id: "staff-orchestrator".to_string(),
        }]
    );

    let mut explicitly_disabled = StaffConfig::default();
    explicitly_disabled
        .workflow_orchestrations
        .insert("content_publishing".to_string(), Default::default());
    let explicitly_disabled = staff_config_for_workflow_orchestration(
        &explicitly_disabled,
        "content_publishing",
        &workflow,
    );
    assert!(explicitly_disabled.project_state.stage_assignments.is_empty());
}

fn business_stage(id: &str, name: &str, role: &str) -> StaffWorkflowStageConfig {
    StaffWorkflowStageConfig {
        id: id.to_string(),
        name: name.to_string(),
        task_status: id.to_string(),
        role: Some(role.to_string()),
        enabled: true,
        display_only: false,
    }
}

fn business_status(
    key: &str,
    label: &str,
    role: TaskPoolWorkflowStatusRoleDto,
    next: Option<&str>,
) -> TaskPoolWorkflowStatusDto {
    TaskPoolWorkflowStatusDto {
        key: key.to_string(),
        label: label.to_string(),
        role,
        enabled: true,
        terminal: next.is_none(),
        next: next.map(ToString::to_string),
        transitions: Vec::new(),
    }
}

fn runtime_action_capability<'a>(
    runtime: &'a StaffRuntimeConfig,
    status_key: &str,
) -> Option<&'a str> {
    runtime
        .actions
        .iter()
        .find(|action| action.status_key == status_key)
        .map(|action| action.capability_id.as_str())
}

#[test]
fn staff_stage_capabilities_expose_stable_ids() {
    let ids = stage_capabilities()
        .into_iter()
        .map(|capability| capability.capability_id)
        .collect::<Vec<_>>();

    assert_eq!(
        ids,
        vec![
            ADVANCE_STATUS_CAPABILITY_ID,
            EXECUTE_TASK_CAPABILITY_ID,
            CODE_REVIEW_CAPABILITY_ID,
            GIT_MERGE_CAPABILITY_ID,
            SAVED_WORKFLOW_CAPABILITY_ID,
            MANUAL_CAPABILITY_ID,
        ]
    );
}

#[test]
fn staff_command_policy_rejects_wrong_role_transition() {
    let config = StaffConfig::default();
    let workflow = workflow_snapshot_from_staff_config(&config);
    let task = staff_task("task-1", "running", AgentTaskStatusDto::Running);
    let command = StaffCommandDto::StartTask {
        command_id: "cmd-1".to_string(),
        task_id: "task-1".to_string(),
        member_id: "staff-reviewer".to_string(),
    };

    let error = validate_staff_command(&command, &[task], &config, &workflow)
        .expect_err("reviewer cannot start executor stage");

    assert!(error.contains("角色不匹配"));
    assert!(error.contains("staff-reviewer"));
}

#[test]
fn staff_legacy_workflow_binding_does_not_create_stage_action() {
    let mut config = StaffConfig::default();
    config.project_state.stage_workflow_bindings = vec![StaffStageWorkflowBindingConfig {
        stage_id: "code_review".to_string(),
        workflow_uuid: "018ff4b0-1d0d-7a2c-a30c-7bcf9ee1f020".to_string(),
    }];
    let workflow = workflow_snapshot_from_staff_config(&config);
    let task = staff_task("task-review", "code_review", AgentTaskStatusDto::Submitted);
    let command = StaffCommandDto::StartTask {
        command_id: "cmd-review".to_string(),
        task_id: "task-review".to_string(),
        member_id: "staff-reviewer".to_string(),
    };

    let action = validate_staff_command(&command, &[task], &config, &workflow)
        .expect("default code review action should remain available");

    assert_eq!(action, StaffCommandActionDto::RunCodeReview { task_id: "task-review".to_string() });
}

#[test]
fn staff_auto_schedule_assigns_autonomous_stage_member() {
    let mut config = StaffConfig::default();
    config.project_state.stage_assignments = vec![StaffStageAssignmentConfig {
        stage_id: "pending".to_string(),
        primary_member_id: Some("staff-orchestrator".to_string()),
        backup_member_ids: Vec::new(),
        manager_member_ids: Vec::new(),
        autonomous: true,
    }];
    let workflow = workflow_snapshot_from_staff_config(&config);
    let task = staff_task("task-pending", "pending", AgentTaskStatusDto::Queued);
    let projection =
        build_staff_projection(&[task.clone()], &config, &workflow, &[]).expect("projection");

    let plan = build_staff_auto_schedule_plan(
        &[task],
        &config,
        &projection,
        &workflow,
        &[],
        TimestampMs(1_783_000_000_000),
    )
    .expect("auto schedule plan");

    assert_eq!(
        plan.commands,
        vec![StaffCommandDto::AssignTask {
            command_id: "staff-auto-assign-task-pending-staff-orchestrator-1783000000000"
                .to_string(),
            task_id: "task-pending".to_string(),
            member_id: "staff-orchestrator".to_string(),
        }]
    );
    assert!(plan.skipped.is_empty());
}

#[test]
fn staff_auto_schedule_reassigns_stale_member_after_stage_change() {
    let mut config = StaffConfig::default();
    config.project_state.member_task_assignments = vec![StaffMemberTaskAssignmentConfig {
        member_id: "staff-executor".to_string(),
        task_ids: vec!["task-code-complete".to_string()],
    }];
    config.project_state.stage_assignments = vec![StaffStageAssignmentConfig {
        stage_id: "code_complete".to_string(),
        primary_member_id: Some("staff-reviewer".to_string()),
        backup_member_ids: Vec::new(),
        manager_member_ids: Vec::new(),
        autonomous: true,
    }];
    let workflow = workflow_snapshot_from_staff_config(&config);
    let task = staff_task("task-code-complete", "code_complete", AgentTaskStatusDto::Submitted);
    let projection =
        build_staff_projection(&[task.clone()], &config, &workflow, &[]).expect("projection");

    let plan = build_staff_auto_schedule_plan(
        &[task],
        &config,
        &projection,
        &workflow,
        &[],
        TimestampMs(1_783_000_000_000),
    )
    .expect("auto schedule plan");

    assert_eq!(
        plan.commands,
        vec![StaffCommandDto::AssignTask {
            command_id: "staff-auto-assign-task-code-complete-staff-reviewer-1783000000000"
                .to_string(),
            task_id: "task-code-complete".to_string(),
            member_id: "staff-reviewer".to_string(),
        }]
    );
    assert!(plan.skipped.is_empty());
}

#[test]
fn staff_read_model_extracts_task_board_metadata_and_running_ids() {
    let config = StaffConfig::default();
    let workflow = workflow_snapshot_from_staff_config(&config);
    let task = StoredAgentTask {
        id: TaskId::new("task-meta").expect("task id"),
        title: "Meta task".to_string(),
        kind: AgentTaskKind::Finite,
        status: AgentTaskStatus::Running,
        spec_uri: "memory://spec".to_string(),
        meta_uri: "memory://meta".to_string(),
        assigned_to: Some(WorkerName::new("Planner").expect("worker")),
        team_id: None,
        room_id: None,
        project_room_id: None,
        delegated_to_team: None,
        schedule: None,
        timezone: None,
        next_scheduled_at: None,
        last_executed_at: None,
        dag: Vec::new(),
        metadata: serde_json::json!({
            "task_board.status": "planning",
            "task_board.status_label": " Planning Review ",
            "task_board.status_role": "planning",
            "priority": 9,
            "order_no": -4,
            "last_error": " last failure ",
            "pause_reason": " paused by user ",
            "task_board.staff.member_id": " staff-planner ",
            "acp_agent": " planner-agent "
        }),
        created_at_ms: TimestampMs(10),
        updated_at_ms: TimestampMs(20),
    };

    let read_model = staff_task_read_model_from_task(task, &workflow);

    assert_eq!(read_model.task_id, "task-meta");
    assert_eq!(read_model.title, "Meta task");
    assert_eq!(read_model.status_key, "running");
    assert_eq!(read_model.status_label.as_deref(), Some("Planning Review"));
    assert_eq!(read_model.status_role, Some(TaskPoolWorkflowStatusRoleDto::Planning));
    assert_eq!(read_model.agent_status, Some(AgentTaskStatusDto::Running));
    assert_eq!(read_model.priority, 9);
    assert_eq!(read_model.order_no, -4);
    assert_eq!(read_model.updated_at_ms, TimestampMs(20));
    assert_eq!(read_model.last_error.as_deref(), Some("last failure"));
    assert_eq!(read_model.pause_reason.as_deref(), Some("paused by user"));
    assert_eq!(read_model.staff_member_id.as_deref(), Some("staff-planner"));
    assert_eq!(read_model.assignee.as_deref(), Some("Planner"));
    assert_eq!(read_model.acp_agent.as_deref(), Some("planner-agent"));

    let stopped = staff_task("task-stopped", "pending", AgentTaskStatusDto::Queued);
    assert_eq!(running_staff_task_ids(&[read_model, stopped]), vec!["task-meta".to_string()]);
}

#[test]
fn staff_auto_schedule_command_classifiers_identify_assignments_and_continuations() {
    let assign = StaffCommandDto::AssignTask {
        command_id: "assign".to_string(),
        task_id: "task".to_string(),
        member_id: "staff-orchestrator".to_string(),
    };
    let start = StaffCommandDto::StartTask {
        command_id: "start".to_string(),
        task_id: "task".to_string(),
        member_id: "staff-orchestrator".to_string(),
    };
    let resume = StaffCommandDto::ResumeTask {
        command_id: "resume".to_string(),
        task_id: "task".to_string(),
        member_id: "staff-orchestrator".to_string(),
    };
    let logs = StaffCommandDto::OpenTaskLogs {
        command_id: "logs".to_string(),
        task_id: "task".to_string(),
    };

    assert!(staff_auto_schedule_command_is_assignment(&assign));
    assert!(!staff_auto_schedule_command_is_assignment(&start));
    assert!(staff_auto_schedule_command_is_continuation(&start));
    assert!(staff_auto_schedule_command_is_continuation(&resume));
    assert!(!staff_auto_schedule_command_is_continuation(&logs));
}

#[test]
fn staff_command_validation_reports_pause_resume_and_stage_configuration_errors() {
    let config = StaffConfig::default();
    let workflow = workflow_snapshot_from_staff_config(&config);
    let pending = staff_task("task-pending", "pending", AgentTaskStatusDto::Queued);
    let paused = staff_task("task-paused", "paused", AgentTaskStatusDto::Paused);

    let pause_error = validate_staff_command(
        &StaffCommandDto::PauseTask {
            command_id: "pause".to_string(),
            task_id: "task-pending".to_string(),
            member_id: "staff-orchestrator".to_string(),
        },
        &[pending],
        &config,
        &workflow,
    )
    .expect_err("pending task cannot pause");
    assert!(pause_error.contains("不能暂停"));

    let workflow_without_resume_target = TaskPoolWorkflowSnapshotDto {
        id: "paused_only".to_string(),
        label: "Paused Only".to_string(),
        statuses: vec![business_status(
            "paused_only",
            "Paused Only",
            TaskPoolWorkflowStatusRoleDto::Paused,
            None,
        )],
    };
    let mut paused_only_config = StaffConfig::default();
    paused_only_config.workflow_stages = vec![StaffWorkflowStageConfig::new(
        "paused_only",
        "Paused Only",
        "paused_only",
        "orchestrator",
    )];
    let resume_error = validate_staff_command(
        &StaffCommandDto::ResumeTask {
            command_id: "resume".to_string(),
            task_id: "task-paused".to_string(),
            member_id: "staff-orchestrator".to_string(),
        },
        &[StaffTaskReadModel { status_key: "paused_only".to_string(), ..paused }],
        &paused_only_config,
        &workflow_without_resume_target,
    )
    .expect_err("paused task without target cannot resume");
    assert!(resume_error.contains("不能继续"));

    let mut disabled_stage_config = StaffConfig::default();
    disabled_stage_config
        .workflow_stages
        .iter_mut()
        .find(|stage| stage.id == "pending")
        .expect("pending stage")
        .enabled = false;
    let disabled_stage_error = validate_staff_command(
        &StaffCommandDto::StartTask {
            command_id: "disabled-stage".to_string(),
            task_id: "task-pending".to_string(),
            member_id: "staff-orchestrator".to_string(),
        },
        &[staff_task("task-pending", "pending", AgentTaskStatusDto::Queued)],
        &disabled_stage_config,
        &workflow_snapshot_from_staff_config(&disabled_stage_config),
    )
    .expect_err("disabled stage");
    assert!(disabled_stage_error.contains("阶段已停用"));

    let mut display_only_config = StaffConfig::default();
    display_only_config
        .workflow_stages
        .iter_mut()
        .find(|stage| stage.id == "pending")
        .expect("pending stage")
        .role = None;
    let no_role_error = validate_staff_command(
        &StaffCommandDto::StartTask {
            command_id: "no-role".to_string(),
            task_id: "task-pending".to_string(),
            member_id: "staff-orchestrator".to_string(),
        },
        &[staff_task("task-pending", "pending", AgentTaskStatusDto::Queued)],
        &display_only_config,
        &workflow_snapshot_from_staff_config(&display_only_config),
    )
    .expect_err("stage without role");
    assert!(no_role_error.contains("requires a role: pending"));
}

#[test]
fn staff_projection_routes_hinted_blocked_display_and_fallback_tasks() {
    let mut config = StaffConfig::default();
    config
        .members
        .iter_mut()
        .find(|member| member.id == "staff-orchestrator")
        .expect("orchestrator")
        .capacity = 4;
    config
        .members
        .iter_mut()
        .find(|member| member.id == "staff-reviewer")
        .expect("reviewer")
        .acp_agent = Some("review-agent".to_string());
    config.project_state.stage_assignments = vec![StaffStageAssignmentConfig {
        stage_id: "pending".to_string(),
        primary_member_id: Some("staff-orchestrator".to_string()),
        backup_member_ids: Vec::new(),
        manager_member_ids: vec!["staff-planner".to_string()],
        autonomous: false,
    }];
    config.project_state.member_task_assignments = vec![StaffMemberTaskAssignmentConfig {
        member_id: "staff-orchestrator".to_string(),
        task_ids: vec!["legacy-running".to_string()],
    }];
    let workflow = workflow_snapshot_from_staff_config(&config);
    let mut pending = staff_task("task-pending", "pending", AgentTaskStatusDto::Queued);
    pending.status_label = None;
    let mut failed = staff_task("task-failed", "failed", AgentTaskStatusDto::Failed);
    failed.staff_member_id = Some("staff-orchestrator".to_string());
    failed.last_error = Some(" build failed ".to_string());
    let mut paused = staff_task("task-paused", "paused", AgentTaskStatusDto::Paused);
    paused.pause_reason = Some("waiting for input".to_string());
    let mut planning = staff_task("task-planning", "planning", AgentTaskStatusDto::Queued);
    planning.assignee = Some("Planner".to_string());
    let mut review = staff_task("task-review", "code_complete", AgentTaskStatusDto::Submitted);
    review.acp_agent = Some("review-agent".to_string());
    let completed = staff_task("task-done", "completed", AgentTaskStatusDto::Completed);
    let unknown = staff_task("task-unknown", "mystery", AgentTaskStatusDto::Draft);
    let tasks = vec![pending, failed, paused, planning, review, completed, unknown];

    let projection = build_staff_projection(
        &tasks,
        &config,
        &workflow,
        &["legacy-running".to_string(), "task-planning".to_string()],
    )
    .expect("projection");

    let pending = projection_assignment(&projection, "task-pending");
    assert_eq!(pending.member_id.as_deref(), Some("staff-orchestrator"));
    assert_eq!(pending.status_label, "Pending");
    assert!(pending.actions.can_start);
    assert!(pending.actions.can_assign);
    let failed = projection_assignment(&projection, "task-failed");
    assert_eq!(failed.member_id.as_deref(), Some("staff-orchestrator"));
    assert!(failed.blocked);
    assert_eq!(failed.block_reason.as_deref(), Some("task failed: build failed"));
    assert!(failed.actions.can_start);
    assert!(!failed.actions.can_assign);
    let paused = projection_assignment(&projection, "task-paused");
    assert!(paused.blocked);
    assert!(paused.block_reason.as_deref().unwrap_or_default().contains("original staff member"));
    assert!(!paused.actions.can_filter_member);
    let planning = projection_assignment(&projection, "task-planning");
    assert_eq!(planning.member_id.as_deref(), Some("staff-planner"));
    assert!(planning.actions.can_start);
    let review = projection_assignment(&projection, "task-review");
    assert_eq!(review.member_id.as_deref(), Some("staff-reviewer"));
    assert!(review.actions.can_start);
    let completed = projection_assignment(&projection, "task-done");
    assert!(completed.display_only);
    assert!(!completed.actions.can_assign);
    let unknown = projection_assignment(&projection, "task-unknown");
    assert_eq!(unknown.member_id.as_deref(), Some("staff-orchestrator"));
    assert_eq!(unknown.status_key, "mystery");
    assert_eq!(unknown.role.as_deref(), Some("orchestrator"));

    let orchestrator_load = projection
        .member_loads
        .iter()
        .find(|load| load.member_id == "staff-orchestrator")
        .expect("orchestrator load");
    assert!(orchestrator_load.task_ids.contains(&"legacy-running".to_string()));
    assert!(orchestrator_load.running_task_ids.contains(&"legacy-running".to_string()));
    assert!(orchestrator_load.blocked_task_ids.contains(&"task-failed".to_string()));
    assert!(!orchestrator_load.over_capacity);
    assert_eq!(projection.blocked_task_count, 2);
    assert!(projection.unassigned.iter().any(|assignment| assignment.task_id == "task-paused"));
}

#[test]
fn staff_projection_reports_unavailable_stage_and_role_reasons() {
    let mut config = StaffConfig {
        members: vec![
            StaffMemberConfig {
                capacity: 1,
                ..StaffMemberConfig::new(
                    "disabled-orchestrator",
                    "Disabled",
                    "orchestrator",
                    "member",
                )
            },
            StaffMemberConfig {
                capacity: 1,
                ..StaffMemberConfig::new(
                    "disabled-backup",
                    "Disabled Backup",
                    "orchestrator",
                    "member",
                )
            },
            StaffMemberConfig::new("busy-planner", "Busy Planner", "planner", "manager"),
        ],
        ..StaffConfig::default()
    };
    config.members[0].enabled = false;
    config.members[1].enabled = false;
    config.project_state.stage_assignments = vec![
        StaffStageAssignmentConfig {
            stage_id: "pending".to_string(),
            primary_member_id: Some("disabled-orchestrator".to_string()),
            backup_member_ids: vec!["disabled-backup".to_string()],
            manager_member_ids: Vec::new(),
            autonomous: false,
        },
        StaffStageAssignmentConfig {
            stage_id: "planning".to_string(),
            primary_member_id: None,
            backup_member_ids: Vec::new(),
            manager_member_ids: Vec::new(),
            autonomous: false,
        },
    ];
    config.project_state.member_task_assignments = vec![StaffMemberTaskAssignmentConfig {
        member_id: "busy-planner".to_string(),
        task_ids: vec!["already-planning".to_string()],
    }];
    let workflow = workflow_snapshot_from_staff_config(&config);
    let pending = staff_task("task-pending", "pending", AgentTaskStatusDto::Queued);
    let planning = staff_task("task-planning", "planning", AgentTaskStatusDto::Queued);
    let running = staff_task("task-running", "running", AgentTaskStatusDto::Running);

    let projection = build_staff_projection(&[pending, planning, running], &config, &workflow, &[])
        .expect("projection with unavailable members");

    let pending = projection_assignment(&projection, "task-pending");
    let pending_reason = pending.block_reason.as_deref().unwrap_or_default();
    assert!(pending.blocked);
    assert!(pending_reason.contains("configured member disabled-orchestrator is disabled"));
    assert!(pending_reason.contains("configured member disabled-backup is disabled"));
    assert!(pending_reason.contains("no enabled orchestrator staff member is available"));
    let planning = projection_assignment(&projection, "task-planning");
    assert!(
        planning
            .block_reason
            .as_deref()
            .unwrap_or_default()
            .contains("no enabled planner staff member has available capacity")
    );
    let running = projection_assignment(&projection, "task-running");
    assert!(
        running
            .block_reason
            .as_deref()
            .unwrap_or_default()
            .contains("no executor staff member is configured")
    );
    assert_eq!(projection.unassigned.len(), 3);
}

#[test]
fn staff_command_validation_covers_assignment_status_log_and_filter_actions() {
    let mut config = StaffConfig::default();
    config.project_state.member_task_assignments = vec![StaffMemberTaskAssignmentConfig {
        member_id: "staff-orchestrator".to_string(),
        task_ids: vec!["existing-task".to_string()],
    }];
    let workflow = workflow_snapshot_from_staff_config(&config);
    let pending = staff_task("task-pending", "pending", AgentTaskStatusDto::Queued);
    let running = staff_task("task-running", "running", AgentTaskStatusDto::Running);
    let paused = staff_task("task-paused", "paused", AgentTaskStatusDto::Paused);
    let completed = staff_task("task-completed", "completed", AgentTaskStatusDto::Completed);
    let tasks = vec![pending.clone(), running.clone(), paused.clone(), completed.clone()];

    assert_eq!(
        validate_staff_command(
            &StaffCommandDto::AssignTask {
                command_id: "assign".to_string(),
                task_id: "task-pending".to_string(),
                member_id: "staff-orchestrator".to_string(),
            },
            &tasks,
            &StaffConfig::default(),
            &workflow,
        )
        .expect("assignment"),
        StaffCommandActionDto::AssignTask {
            task_id: "task-pending".to_string(),
            member_id: "staff-orchestrator".to_string()
        }
    );
    assert_eq!(
        validate_staff_command(
            &StaffCommandDto::TakeOverTask {
                command_id: "take-over".to_string(),
                task_id: "task-pending".to_string(),
                member_id: "staff-orchestrator".to_string(),
            },
            &[pending],
            &StaffConfig::default(),
            &workflow,
        )
        .expect("take over"),
        StaffCommandActionDto::AssignTask {
            task_id: "task-pending".to_string(),
            member_id: "staff-orchestrator".to_string()
        }
    );
    assert_eq!(
        validate_staff_command(
            &StaffCommandDto::PauseTask {
                command_id: "pause".to_string(),
                task_id: "task-running".to_string(),
                member_id: "staff-executor".to_string(),
            },
            &tasks,
            &StaffConfig::default(),
            &workflow,
        )
        .expect("pause"),
        StaffCommandActionDto::ChangeTaskStatus {
            task_id: "task-running".to_string(),
            to_status_key: "paused".to_string(),
            to_status: Some(AgentTaskStatusDto::Paused),
        }
    );
    assert_eq!(
        validate_staff_command(
            &StaffCommandDto::ResumeTask {
                command_id: "resume".to_string(),
                task_id: "task-paused".to_string(),
                member_id: "staff-orchestrator".to_string(),
            },
            &tasks,
            &StaffConfig::default(),
            &workflow,
        )
        .expect("resume"),
        StaffCommandActionDto::ChangeTaskStatus {
            task_id: "task-paused".to_string(),
            to_status_key: "code_complete".to_string(),
            to_status: Some(AgentTaskStatusDto::Submitted),
        }
    );
    assert_eq!(
        validate_staff_command(
            &StaffCommandDto::OpenTaskLogs {
                command_id: "logs".to_string(),
                task_id: "task-running".to_string(),
            },
            &tasks,
            &StaffConfig::default(),
            &workflow,
        )
        .expect("logs"),
        StaffCommandActionDto::ViewTaskLogs { task_id: "task-running".to_string() }
    );
    assert_eq!(
        validate_staff_command(
            &StaffCommandDto::FilterByMember {
                command_id: "filter".to_string(),
                member_id: "staff-reviewer".to_string(),
            },
            &tasks,
            &StaffConfig::default(),
            &workflow,
        )
        .expect("filter"),
        StaffCommandActionDto::FilterByMember { member_id: "staff-reviewer".to_string() }
    );

    let capacity_error = validate_staff_command(
        &StaffCommandDto::AssignTask {
            command_id: "capacity".to_string(),
            task_id: "task-pending".to_string(),
            member_id: "staff-orchestrator".to_string(),
        },
        &[staff_task("task-pending", "pending", AgentTaskStatusDto::Queued)],
        &config,
        &workflow,
    )
    .expect_err("capacity full");
    assert!(capacity_error.contains("容量已满"));

    let ended_error = validate_staff_command(
        &StaffCommandDto::AssignTask {
            command_id: "ended".to_string(),
            task_id: "task-completed".to_string(),
            member_id: "staff-orchestrator".to_string(),
        },
        &[completed],
        &StaffConfig::default(),
        &workflow,
    )
    .expect_err("ended task");
    assert!(ended_error.contains("已结束"));

    let mut disabled_config = StaffConfig::default();
    disabled_config
        .members
        .iter_mut()
        .find(|member| member.id == "staff-reviewer")
        .expect("reviewer")
        .enabled = false;
    let disabled_error = validate_staff_command(
        &StaffCommandDto::FilterByMember {
            command_id: "disabled".to_string(),
            member_id: "staff-reviewer".to_string(),
        },
        &[],
        &disabled_config,
        &workflow_snapshot_from_staff_config(&disabled_config),
    )
    .expect_err("disabled member");
    assert!(disabled_error.contains("已停用"));
}

#[test]
fn staff_start_command_uses_stage_actions_and_reports_unsupported_states() {
    let config = StaffConfig::default();
    let workflow = workflow_snapshot_from_staff_config(&config);
    assert_eq!(
        validate_staff_command(
            &StaffCommandDto::StartTask {
                command_id: "advance".to_string(),
                task_id: "task-pool".to_string(),
                member_id: "staff-orchestrator".to_string(),
            },
            &[staff_task("task-pool", "pool", AgentTaskStatusDto::Draft)],
            &config,
            &workflow,
        )
        .expect("advance from pool"),
        StaffCommandActionDto::ChangeTaskStatus {
            task_id: "task-pool".to_string(),
            to_status_key: "pending".to_string(),
            to_status: Some(AgentTaskStatusDto::Queued),
        }
    );
    assert_eq!(
        validate_staff_command(
            &StaffCommandDto::StartTask {
                command_id: "execute".to_string(),
                task_id: "task-pending".to_string(),
                member_id: "staff-orchestrator".to_string(),
            },
            &[staff_task("task-pending", "pending", AgentTaskStatusDto::Queued)],
            &config,
            &workflow,
        )
        .expect("execute pending"),
        StaffCommandActionDto::ExecuteTask { task_id: "task-pending".to_string() }
    );
    assert_eq!(
        validate_staff_command(
            &StaffCommandDto::StartTask {
                command_id: "merge".to_string(),
                task_id: "task-merge".to_string(),
                member_id: "staff-merger".to_string(),
            },
            &[staff_task("task-merge", "pr_submitted", AgentTaskStatusDto::Submitted)],
            &config,
            &workflow,
        )
        .expect("merge"),
        StaffCommandActionDto::RunMerge { task_id: "task-merge".to_string() }
    );

    let mut saved_workflow_config = StaffConfig::default();
    saved_workflow_config.project_state.stage_actions = vec![StaffStageActionConfig {
        stage_id: "planning".to_string(),
        capability_id: SAVED_WORKFLOW_CAPABILITY_ID.to_string(),
        workflow_uuid: Some("018ff4b0-1d0d-7a2c-a30c-7bcf9ee1f099".to_string()),
        inputs: Default::default(),
    }];
    assert_eq!(
        validate_staff_command(
            &StaffCommandDto::StartTask {
                command_id: "saved".to_string(),
                task_id: "task-plan".to_string(),
                member_id: "staff-planner".to_string(),
            },
            &[staff_task("task-plan", "planning", AgentTaskStatusDto::Queued)],
            &saved_workflow_config,
            &workflow_snapshot_from_staff_config(&saved_workflow_config),
        )
        .expect("saved workflow"),
        StaffCommandActionDto::RunStageWorkflow {
            task_id: "task-plan".to_string(),
            stage_id: "planning".to_string(),
            status_key: "planning".to_string(),
            workflow_uuid: "018ff4b0-1d0d-7a2c-a30c-7bcf9ee1f099".to_string(),
            member_id: "staff-planner".to_string(),
        }
    );

    let mut manual_config = StaffConfig::default();
    manual_config.project_state.stage_actions = vec![StaffStageActionConfig {
        stage_id: "planning".to_string(),
        capability_id: MANUAL_CAPABILITY_ID.to_string(),
        workflow_uuid: None,
        inputs: Default::default(),
    }];
    let manual_error = validate_staff_command(
        &StaffCommandDto::StartTask {
            command_id: "manual".to_string(),
            task_id: "task-manual".to_string(),
            member_id: "staff-planner".to_string(),
        },
        &[staff_task("task-manual", "planning", AgentTaskStatusDto::Queued)],
        &manual_config,
        &workflow_snapshot_from_staff_config(&manual_config),
    )
    .expect_err("manual action cannot start");
    assert!(manual_error.contains("人工处理"));

    let running_error = validate_staff_command(
        &StaffCommandDto::StartTask {
            command_id: "running".to_string(),
            task_id: "task-running".to_string(),
            member_id: "staff-executor".to_string(),
        },
        &[staff_task("task-running", "running", AgentTaskStatusDto::Running)],
        &config,
        &workflow,
    )
    .expect_err("running task");
    assert!(running_error.contains("已在执行中"));
}

#[test]
fn staff_scheduler_command_conversion_only_emits_neutral_lifecycle_commands() {
    let source = StaffCommandDto::StartTask {
        command_id: "cmd-source".to_string(),
        task_id: "task-source".to_string(),
        member_id: "staff-reviewer".to_string(),
    };
    assert!(scheduler_command_for_action(
        &StaffCommandActionDto::AssignTask {
            task_id: "task-assign".to_string(),
            member_id: "staff-orchestrator".to_string(),
        },
        &source,
        Some("because".to_string()),
    ).is_none());

    let moved = scheduler_command_for_action(
        &StaffCommandActionDto::ChangeTaskStatus {
            task_id: "task-move".to_string(),
            to_status_key: "paused".to_string(),
            to_status: Some(AgentTaskStatusDto::Paused),
        },
        &source,
        None,
    )
    .expect("move request");
    assert_eq!(moved.command, AgentSchedulerCommandKindDto::Move);
    let move_payload = moved.payload.move_task.expect("move payload");
    assert_eq!(move_payload.to_status_key, "paused");
    assert_eq!(move_payload.to_status, Some(AgentTaskStatusDto::Paused));

    let execute = scheduler_command_for_action(
        &StaffCommandActionDto::ExecuteTask { task_id: "task-start".to_string() },
        &source,
        None,
    )
    .expect("start request");
    assert_eq!(execute.command, AgentSchedulerCommandKindDto::Start);

    assert!(scheduler_command_for_action(
        &StaffCommandActionDto::RunCodeReview { task_id: "task-review".to_string() },
        &source,
        None,
    ).is_none());

    let merge = scheduler_command_for_action(
        &StaffCommandActionDto::RunMerge { task_id: "task-merge".to_string() },
        &source,
        None,
    )
    .expect("merge request");
    assert_eq!(merge.command, AgentSchedulerCommandKindDto::Merge);

    assert!(scheduler_command_for_action(
        &StaffCommandActionDto::RunStageWorkflow {
            task_id: "task-workflow".to_string(),
            stage_id: "planning".to_string(),
            status_key: "planning".to_string(),
            workflow_uuid: "018ff4b0-1d0d-7a2c-a30c-7bcf9ee1f099".to_string(),
            member_id: "staff-planner".to_string(),
        },
        &source,
        None,
    ).is_none());

    assert!(
        scheduler_command_for_action(
            &StaffCommandActionDto::ViewTaskLogs { task_id: "task".to_string() },
            &source,
            None,
        )
        .is_none()
    );
    assert!(
        scheduler_command_for_action(
            &StaffCommandActionDto::FilterByMember { member_id: "staff-planner".to_string() },
            &source,
            None,
        )
        .is_none()
    );
}

#[test]
fn staff_auto_schedule_continues_assigned_tasks_and_records_orchestration_contexts() {
    let mut config = StaffConfig::default();
    config.project_state.member_task_assignments = vec![
        StaffMemberTaskAssignmentConfig {
            member_id: "staff-orchestrator".to_string(),
            task_ids: vec![
                "task-pending".to_string(),
                "task-paused".to_string(),
                "task-running-list".to_string(),
                "task-ended".to_string(),
            ],
        },
        StaffMemberTaskAssignmentConfig {
            member_id: "staff-planner".to_string(),
            task_ids: vec!["task-busy".to_string()],
        },
    ];
    config.project_state.stage_assignments = vec![
        StaffStageAssignmentConfig {
            stage_id: "pending".to_string(),
            primary_member_id: Some("staff-orchestrator".to_string()),
            backup_member_ids: Vec::new(),
            manager_member_ids: vec!["staff-planner".to_string()],
            autonomous: true,
        },
        StaffStageAssignmentConfig {
            stage_id: "paused".to_string(),
            primary_member_id: Some("staff-orchestrator".to_string()),
            backup_member_ids: Vec::new(),
            manager_member_ids: Vec::new(),
            autonomous: true,
        },
        StaffStageAssignmentConfig {
            stage_id: "planning".to_string(),
            primary_member_id: Some("staff-planner".to_string()),
            backup_member_ids: Vec::new(),
            manager_member_ids: Vec::new(),
            autonomous: true,
        },
    ];
    let workflow = workflow_snapshot_from_staff_config(&config);
    let tasks = vec![
        staff_task("task-pending", "pending", AgentTaskStatusDto::Queued),
        staff_task("task-paused", "paused", AgentTaskStatusDto::Paused),
        staff_task("task-running-list", "pending", AgentTaskStatusDto::Queued),
        staff_task("task-ended", "pending", AgentTaskStatusDto::Completed),
        staff_task("task-planning", "planning", AgentTaskStatusDto::Queued),
    ];
    let projection =
        build_staff_projection(&tasks, &config, &workflow, &["task-running-list".to_string()])
            .expect("projection");

    let plan = build_staff_auto_schedule_plan(
        &tasks,
        &config,
        &projection,
        &workflow,
        &["task-running-list".to_string()],
        TimestampMs(1_783_000_000_000),
    )
    .expect("auto schedule plan");

    assert_eq!(
        plan.commands,
        vec![
            StaffCommandDto::StartTask {
                command_id: "staff-auto-start-task-pending-staff-orchestrator-1783000000000"
                    .to_string(),
                task_id: "task-pending".to_string(),
                member_id: "staff-orchestrator".to_string(),
            },
            StaffCommandDto::ResumeTask {
                command_id: "staff-auto-resume-task-paused-staff-orchestrator-1783000000000"
                    .to_string(),
                task_id: "task-paused".to_string(),
                member_id: "staff-orchestrator".to_string(),
            },
        ]
    );
    assert_eq!(plan.command_contexts.len(), 2);
    let start_context = &plan.command_contexts[0].orchestration;
    assert_eq!(start_context.stage_id, "pending");
    assert_eq!(start_context.member_level_label, "主智能体");
    assert_eq!(start_context.manager_member_names, vec!["Planner".to_string()]);
    assert!(start_context.target.contains("开始 task-pending"));
    let resume_context = &plan.command_contexts[1].orchestration;
    assert!(resume_context.target.contains("继续 task-paused"));
    assert!(plan.skipped.iter().any(|skip| skip.task_id == "task-running-list"
        && skip.reason == "task is already in the running list"));
    assert!(
        plan.skipped.iter().any(|skip| skip.task_id == "task-ended" && skip.reason == "task ended")
    );
    assert!(plan.skipped.iter().any(|skip| {
        skip.task_id == "task-planning" && skip.reason == "member staff-planner capacity is full"
    }));
}

#[test]
fn staff_stage_action_payload_validation_rejects_conflicting_missing_and_unexpected_inputs() {
    let catalog = stage_capabilities();
    let mut saved_inputs = AgentSchedulerStageActionInputsDto {
        workflow_uuid: Some("input-workflow".to_string()),
        ..AgentSchedulerStageActionInputsDto::default()
    };
    let conflict = validate_stage_action_payload(
        "planning",
        SAVED_WORKFLOW_CAPABILITY_ID,
        Some("payload-workflow"),
        &saved_inputs,
        Some(TaskPoolWorkflowStatusRoleDto::Planning),
        &catalog,
    )
    .expect_err("conflicting workflow uuid");
    assert!(conflict.contains("conflicts"));

    saved_inputs.workflow_uuid = None;
    let missing = validate_stage_action_payload(
        "planning",
        SAVED_WORKFLOW_CAPABILITY_ID,
        None,
        &saved_inputs,
        Some(TaskPoolWorkflowStatusRoleDto::Planning),
        &catalog,
    )
    .expect_err("missing workflow uuid");
    assert!(missing.contains("cannot be empty"));

    let unexpected = validate_stage_action_payload(
        "planning",
        EXECUTE_TASK_CAPABILITY_ID,
        Some("unexpected-workflow"),
        &AgentSchedulerStageActionInputsDto::default(),
        Some(TaskPoolWorkflowStatusRoleDto::Planning),
        &catalog,
    )
    .expect_err("unexpected workflow uuid");
    assert!(unexpected.contains("must be empty"));

    let mut blank_parameter_inputs = AgentSchedulerStageActionInputsDto::default();
    blank_parameter_inputs.parameters.insert("   ".to_string(), serde_json::json!("value"));
    let blank_parameter = validate_stage_action_payload(
        "planning",
        EXECUTE_TASK_CAPABILITY_ID,
        None,
        &blank_parameter_inputs,
        Some(TaskPoolWorkflowStatusRoleDto::Planning),
        &catalog,
    )
    .expect_err("blank parameter");
    assert!(blank_parameter.contains("input parameter"));

    let missing_role = validate_stage_action_payload(
        "planning",
        EXECUTE_TASK_CAPABILITY_ID,
        None,
        &AgentSchedulerStageActionInputsDto::default(),
        None,
        &catalog,
    )
    .expect_err("missing role");
    assert!(missing_role.contains("requires current status role"));
}

#[test]
fn staff_config_for_runtime_workflow_prunes_stale_project_state_and_clears_legacy_bindings() {
    let mut config = StaffConfig::default();
    config.project_state.stage_assignments = vec![
        StaffStageAssignmentConfig {
            stage_id: "queued_custom".to_string(),
            primary_member_id: Some("staff-orchestrator".to_string()),
            backup_member_ids: Vec::new(),
            manager_member_ids: Vec::new(),
            autonomous: true,
        },
        StaffStageAssignmentConfig {
            stage_id: "stale_stage".to_string(),
            primary_member_id: Some("staff-planner".to_string()),
            backup_member_ids: Vec::new(),
            manager_member_ids: Vec::new(),
            autonomous: true,
        },
    ];
    config.project_state.stage_actions = vec![
        StaffStageActionConfig {
            stage_id: "queued_custom".to_string(),
            capability_id: EXECUTE_TASK_CAPABILITY_ID.to_string(),
            workflow_uuid: None,
            inputs: Default::default(),
        },
        StaffStageActionConfig {
            stage_id: "stale_stage".to_string(),
            capability_id: EXECUTE_TASK_CAPABILITY_ID.to_string(),
            workflow_uuid: None,
            inputs: Default::default(),
        },
    ];
    config.project_state.stage_workflow_bindings = vec![StaffStageWorkflowBindingConfig {
        stage_id: "queued_custom".to_string(),
        workflow_uuid: "legacy-workflow".to_string(),
    }];
    let workflow = TaskPoolWorkflowSnapshotDto {
        id: "custom_runtime".to_string(),
        label: "Custom Runtime".to_string(),
        statuses: vec![
            business_status(
                "queued_custom",
                "Queued Custom",
                TaskPoolWorkflowStatusRoleDto::Queued,
                Some("done_custom"),
            ),
            business_status(
                "done_custom",
                "Done Custom",
                TaskPoolWorkflowStatusRoleDto::Done,
                None,
            ),
        ],
    };

    let runtime_config = staff_config_for_workflow(&config, &workflow);

    assert_eq!(
        runtime_config
            .workflow_stages
            .iter()
            .map(|stage| {
                (stage.id.as_str(), stage.role.as_deref(), stage.enabled, stage.display_only)
            })
            .collect::<Vec<_>>(),
        vec![
            ("queued_custom", Some("orchestrator"), true, false),
            ("done_custom", None, true, true),
        ]
    );
    assert_eq!(runtime_config.project_state.stage_assignments.len(), 1);
    assert_eq!(runtime_config.project_state.stage_assignments[0].stage_id, "queued_custom");
    assert_eq!(runtime_config.project_state.stage_actions.len(), 1);
    assert_eq!(runtime_config.project_state.stage_actions[0].stage_id, "queued_custom");
    assert!(runtime_config.project_state.stage_workflow_bindings.is_empty());
}

fn staff_task(
    task_id: &str,
    status_key: &str,
    agent_status: AgentTaskStatusDto,
) -> StaffTaskReadModel {
    StaffTaskReadModel {
        task_id: task_id.to_string(),
        title: format!("Task {task_id}"),
        status_key: status_key.to_string(),
        status_label: Some(status_key.to_string()),
        status_role: None,
        agent_status: Some(agent_status),
        priority: 1,
        order_no: 0,
        updated_at_ms: TimestampMs(1),
        last_error: None,
        pause_reason: None,
        staff_member_id: None,
        assignee: None,
        acp_agent: None,
    }
}

fn projection_assignment<'a>(
    projection: &'a StaffProjectionDto,
    task_id: &str,
) -> &'a StaffAssignmentDto {
    projection
        .lanes
        .iter()
        .flat_map(|lane| lane.assignments.iter())
        .find(|assignment| assignment.task_id == task_id)
        .expect("projection assignment")
}
