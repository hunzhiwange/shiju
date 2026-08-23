//! 员工模型测试，覆盖默认角色工作流、配置归一化和渠道绑定投影。
//! 自定义阶段能力必须完整且角色合法，持久化的旧生命周期映射不得污染当前模型。

#[test]
fn default_staff_config_contains_required_roles() {
    let config = super::TaskBoardStaffConfig::default();
    let roles = config.members.iter().map(|member| member.role).collect::<Vec<_>>();
    let levels = config.members.iter().map(|member| member.level).collect::<Vec<_>>();

    assert!(roles.contains(&super::TaskBoardStaffRole::Orchestrator));
    assert!(roles.contains(&super::TaskBoardStaffRole::Planner));
    assert!(roles.contains(&super::TaskBoardStaffRole::Executor));
    assert!(roles.contains(&super::TaskBoardStaffRole::Reviewer));
    assert!(roles.contains(&super::TaskBoardStaffRole::Merger));
    assert!(levels.contains(&super::TaskBoardStaffLevel::Principal));
    assert!(levels.contains(&super::TaskBoardStaffLevel::Manager));
    assert!(levels.contains(&super::TaskBoardStaffLevel::Executor));
    assert!(config.stage_assignments.is_empty());
}

#[test]
fn default_staff_workflow_maps_task_statuses_to_roles() {
    let config = super::TaskBoardStaffConfig::default();

    assert_eq!(
        config.role_for_status(crate::app::task::TaskStatus::Pool),
        Some(super::TaskBoardStaffRole::Orchestrator)
    );
    assert_eq!(
        config.role_for_status(crate::app::task::TaskStatus::Planning),
        Some(super::TaskBoardStaffRole::Planner)
    );
    assert_eq!(
        config.role_for_status(crate::app::task::TaskStatus::Running),
        Some(super::TaskBoardStaffRole::Executor)
    );
    assert_eq!(
        config.role_for_status(crate::app::task::TaskStatus::CodeReview),
        Some(super::TaskBoardStaffRole::Reviewer)
    );
    assert_eq!(
        config.role_for_status(crate::app::task::TaskStatus::PrSubmitted),
        Some(super::TaskBoardStaffRole::Merger)
    );
    assert_eq!(config.role_for_status(crate::app::task::TaskStatus::Completed), None);
    assert_eq!(config.workflow.len(), crate::app::task::TaskStatus::all().len());
}

#[test]
fn staff_config_derives_stage_roles_from_marketing_workflow() {
    let stages = super::models::workflow_stages_for_workflow_id("marketing_campaign")
        .expect("marketing workflow stages");

    assert_workflow_stage_summary(
        &stages,
        &[
            ("campaign_ideas", Some(super::TaskBoardStaffRole::Orchestrator), false),
            ("campaign_queued", Some(super::TaskBoardStaffRole::Orchestrator), false),
            ("campaign_planning", Some(super::TaskBoardStaffRole::Planner), false),
            ("campaign_running", Some(super::TaskBoardStaffRole::Executor), false),
            ("campaign_blocked", Some(super::TaskBoardStaffRole::Orchestrator), false),
            ("campaign_paused", Some(super::TaskBoardStaffRole::Orchestrator), false),
            ("campaign_review_ready", Some(super::TaskBoardStaffRole::Reviewer), false),
            ("campaign_reviewing", Some(super::TaskBoardStaffRole::Reviewer), false),
            ("campaign_closing", Some(super::TaskBoardStaffRole::Merger), false),
            ("campaign_done", None, true),
            ("campaign_archived", None, true),
        ],
    );

    let normalized = super::TaskBoardStaffConfig {
        workflow: Vec::new(),
        ..super::TaskBoardStaffConfig::default()
    }
    .normalize_for_workflow_id("marketing_campaign")
    .expect("normalize marketing workflow");
    assert_eq!(normalized.workflow, stages);
}

#[test]
fn staff_config_derives_stage_roles_from_recruiting_workflow() {
    let stages = super::models::workflow_stages_for_workflow_id("recruiting_pipeline")
        .expect("recruiting workflow stages");

    assert_workflow_stage_summary(
        &stages,
        &[
            ("recruiting_pool", Some(super::TaskBoardStaffRole::Orchestrator), false),
            ("recruiting_queued", Some(super::TaskBoardStaffRole::Orchestrator), false),
            ("recruiting_screening", Some(super::TaskBoardStaffRole::Planner), false),
            ("recruiting_interviewing", Some(super::TaskBoardStaffRole::Executor), false),
            ("recruiting_blocked", Some(super::TaskBoardStaffRole::Orchestrator), false),
            ("recruiting_on_hold", Some(super::TaskBoardStaffRole::Orchestrator), false),
            ("recruiting_review_ready", Some(super::TaskBoardStaffRole::Reviewer), false),
            ("recruiting_reviewing", Some(super::TaskBoardStaffRole::Reviewer), false),
            ("recruiting_offer", Some(super::TaskBoardStaffRole::Merger), false),
            ("recruiting_hired", None, true),
            ("recruiting_archived", None, true),
        ],
    );

    let normalized = super::TaskBoardStaffConfig {
        workflow: Vec::new(),
        ..super::TaskBoardStaffConfig::default()
    }
    .normalize_for_workflow_id("recruiting_pipeline")
    .expect("normalize recruiting workflow");
    assert_eq!(normalized.workflow, stages);
}

#[test]
fn staff_config_projection_ignores_persisted_dev_lifecycle_stage_roles() {
    let mut config = vw_config_types::staff::StaffConfig::default();
    let planning = config
        .workflow_stages
        .iter_mut()
        .find(|stage| stage.id == "planning")
        .expect("planning stage");
    planning.role = None;
    planning.enabled = false;
    planning.display_only = true;

    let projected =
        super::task_board_staff_config_from_staff_config_for_workflow_id(config, "dev_lifecycle")
            .expect("project dev lifecycle staff config");

    let planning = projected
        .workflow
        .iter()
        .find(|stage| stage.status == crate::app::task::TaskStatus::Planning)
        .expect("projected planning stage");
    assert_eq!(planning.role, Some(super::TaskBoardStaffRole::Planner));
    assert!(!planning.display_only);
}

#[test]
fn staff_config_projection_defaults_content_publishing_stage_capabilities() {
    let projected = super::task_board_staff_config_from_staff_config_for_workflow_id(
        vw_config_types::staff::StaffConfig::default(),
        "content_publishing",
    )
    .expect("project content publishing staff config");

    assert_eq!(projected.stage_assignments.len(), 6);
    assert_eq!(projected.stage_actions.len(), 7);
    let content_idea =
        crate::app::task::TaskStatus::parse_key("content_idea").expect("content idea status");
    let content_publishing = crate::app::task::TaskStatus::parse_key("content_publishing")
        .expect("content publishing status");
    assert!(projected.stage_assignments.iter().any(|assignment| {
        assignment.status == content_idea
            && assignment.primary_member_id.as_deref() == Some("staff-orchestrator")
            && assignment.autonomous
    }));
    assert!(projected.stage_actions.iter().any(|action| {
        action.status == content_publishing
            && action.capability_id == super::stage_capabilities::ADVANCE_STATUS_CAPABILITY_ID
    }));

    let mut explicitly_empty = vw_config_types::staff::StaffConfig::default();
    explicitly_empty
        .workflow_orchestrations
        .insert("content_publishing".to_string(), Default::default());
    let explicitly_empty = super::task_board_staff_config_from_staff_config_for_workflow_id(
        explicitly_empty,
        "content_publishing",
    )
    .expect("explicit empty content publishing staff config");
    assert!(explicitly_empty.stage_assignments.is_empty());
    assert!(explicitly_empty.stage_actions.is_empty());
}

#[test]
fn staff_capability_catalog_request_keeps_only_custom_draft_items() {
    let catalog = super::task_board_staff_capabilities_from_gateway_response(
        vw_gateway_client::vw_api_types::staff::StaffProjectCapabilityCatalogResponse {
            capabilities: vec![
                vw_gateway_client::vw_api_types::staff::StaffStageCapabilityDto {
                    capability_id: "manual".to_string(),
                    label: "人工处理".to_string(),
                    capability_type: vw_gateway_client::vw_api_types::staff::StaffCapabilityKindDto::BuiltIn,
                    built_in: true,
                    supported_roles: Vec::new(),
                    requires_workflow_uuid: false,
                    input_summary: vw_gateway_client::vw_api_types::staff::StaffCapabilityIoSummaryDto::default(),
                    result_summary: vw_gateway_client::vw_api_types::staff::StaffCapabilityIoSummaryDto::default(),
                    input_contract: "task".to_string(),
                    result_semantics: "manual".to_string(),
                    permission_hint: "none".to_string(),
                },
                vw_gateway_client::vw_api_types::staff::StaffStageCapabilityDto {
                    capability_id: "release_workflow".to_string(),
                    label: "Release Workflow".to_string(),
                    capability_type: vw_gateway_client::vw_api_types::staff::StaffCapabilityKindDto::SavedWorkflow,
                    built_in: false,
                    supported_roles: Vec::new(),
                    requires_workflow_uuid: true,
                    input_summary: vw_gateway_client::vw_api_types::staff::StaffCapabilityIoSummaryDto {
                        description: "Workflow UUID and release channel".to_string(),
                        fields: vec!["workflow_uuid".to_string(), "release_channel".to_string()],
                    },
                    result_summary: vw_gateway_client::vw_api_types::staff::StaffCapabilityIoSummaryDto {
                        description: "Release result".to_string(),
                        fields: vec!["summary".to_string()],
                    },
                    input_contract: "workflow".to_string(),
                    result_semantics: "release".to_string(),
                    permission_hint: "scoped".to_string(),
                },
            ],
        },
    );

    let request = super::task_board_staff_capability_catalog_request(&catalog);

    assert_eq!(request.capabilities.len(), 1);
    assert_eq!(request.capabilities[0].id, "release_workflow");
    assert_eq!(
        request.capabilities[0].input_summary.fields,
        vec!["workflow_uuid".to_string(), "release_channel".to_string()]
    );
    assert_eq!(request.capabilities[0].result_summary.fields, vec!["summary".to_string()]);
}

#[test]
fn staff_config_normalize_rejects_invalid_member_identity_and_capacity() {
    let mut empty_id = super::TaskBoardStaffConfig::default();
    empty_id.members[0].id = "  ".to_string();
    assert!(empty_id.normalize().expect_err("empty id").contains("id cannot be empty"));

    let mut duplicate_id = super::TaskBoardStaffConfig::default();
    duplicate_id.members[1].id = duplicate_id.members[0].id.clone();
    assert!(
        duplicate_id
            .normalize()
            .expect_err("duplicate id")
            .contains("duplicate task board staff member id")
    );

    let mut zero_capacity = super::TaskBoardStaffConfig::default();
    zero_capacity.members[0].capacity = 0;
    assert!(zero_capacity.normalize().expect_err("zero capacity").contains("capacity cannot be 0"));
}

#[test]
fn staff_config_normalize_rejects_incomplete_workflow() {
    let mut config = super::TaskBoardStaffConfig::default();
    config.workflow.retain(|stage| stage.status != crate::app::task::TaskStatus::Running);

    assert!(
        config
            .normalize()
            .expect_err("missing running stage")
            .contains("missing task board workflow status: running")
    );
}

fn assert_workflow_stage_summary(
    stages: &[super::TaskBoardWorkflowStage],
    expected: &[(&str, Option<super::TaskBoardStaffRole>, bool)],
) {
    let actual = stages
        .iter()
        .map(|stage| (stage.status.to_string_key(), stage.role, stage.display_only))
        .collect::<Vec<_>>();
    assert_eq!(actual, expected);
}

#[test]
fn staff_member_new_assigns_level_from_role() {
    let orchestrator =
        super::TaskBoardStaffMember::new("lead", "Lead", super::TaskBoardStaffRole::Orchestrator);
    let planner =
        super::TaskBoardStaffMember::new("planner", "Planner", super::TaskBoardStaffRole::Planner);
    let executor = super::TaskBoardStaffMember::new(
        "executor",
        "Executor",
        super::TaskBoardStaffRole::Executor,
    );
    let observer = super::TaskBoardStaffMember::new(
        "observer",
        "Observer",
        super::TaskBoardStaffRole::Observer,
    );

    assert_eq!(orchestrator.level, super::TaskBoardStaffLevel::Principal);
    assert_eq!(planner.level, super::TaskBoardStaffLevel::Manager);
    assert_eq!(executor.level, super::TaskBoardStaffLevel::Executor);
    assert_eq!(observer.level, super::TaskBoardStaffLevel::Member);
}

#[test]
fn staff_channel_binding_task_board_staff_config_preserves_channel_bindings() {
    let mut config = vw_config_types::staff::StaffConfig::default();
    let planner = config
        .members
        .iter_mut()
        .find(|member| member.id == "staff-planner")
        .expect("default planner");
    planner.channel_bindings = vec![
        vw_config_types::staff::StaffChannelBindingConfig {
            channel_type: "matrix".to_string(),
            instance_id: "planner".to_string(),
        },
        vw_config_types::staff::StaffChannelBindingConfig {
            channel_type: "feishu".to_string(),
            instance_id: "planner-app".to_string(),
        },
    ];
    let expected_bindings = planner.channel_bindings.clone();

    let task_board =
        super::task_board_staff_config_from_staff_config(config.clone()).expect("project staff");
    let projected_planner = task_board
        .members
        .iter()
        .find(|member| member.id == "staff-planner")
        .expect("projected planner");
    assert_eq!(projected_planner.channel_bindings, expected_bindings);

    let mut round_tripped = config.clone();
    round_tripped
        .members
        .iter_mut()
        .find(|member| member.id == "staff-planner")
        .expect("round-tripped planner")
        .channel_bindings
        .clear();
    super::apply_task_board_staff_config_to_staff_config(&mut round_tripped, &task_board)
        .expect("apply task board staff config");
    assert_eq!(
        round_tripped
            .members
            .iter()
            .find(|member| member.id == "staff-planner")
            .expect("round-tripped planner")
            .channel_bindings,
        expected_bindings
    );
}

#[test]
fn staff_config_round_trips_message_targets_and_workflow_bindings() {
    let mut config = vw_config_types::staff::StaffConfig::default();
    let planner = config
        .members
        .iter_mut()
        .find(|member| member.id == "staff-planner")
        .expect("default planner");
    planner.ipc_agent_id = Some(" ipc-planner ".to_string());
    config.workflow_orchestrations.insert(
        "dev_lifecycle".to_string(),
        vw_config_types::staff::StaffWorkflowOrchestrationConfig {
            stage_actions: vec![vw_config_types::staff::StaffStageActionConfig {
                stage_id: "planning".to_string(),
                capability_id: super::stage_capabilities::SAVED_WORKFLOW_CAPABILITY_ID.to_string(),
                workflow_uuid: Some("018ff4b0-1d0d-7a2c-a30c-7bcf9ee1f001".to_string()),
                inputs: vw_config_types::staff::StaffStageActionInputsConfig {
                    workflow_uuid: Some("018ff4b0-1d0d-7a2c-a30c-7bcf9ee1f001".to_string()),
                    parameters: std::collections::BTreeMap::new(),
                },
            }],
            ..vw_config_types::staff::StaffWorkflowOrchestrationConfig::default()
        },
    );

    let mut task_board =
        super::task_board_staff_config_from_staff_config(config.clone()).expect("project staff");
    let projected_planner = task_board
        .members
        .iter()
        .find(|member| member.id == "staff-planner")
        .expect("projected planner");
    assert_eq!(projected_planner.ipc_agent_id.as_deref(), Some("ipc-planner"));
    assert_eq!(
        task_board.stage_actions,
        vec![super::models::TaskBoardStageAction {
            status: crate::app::task::TaskStatus::Planning,
            capability_id: super::stage_capabilities::SAVED_WORKFLOW_CAPABILITY_ID.to_string(),
            workflow_uuid: Some("018ff4b0-1d0d-7a2c-a30c-7bcf9ee1f001".to_string()),
            input_parameters: std::collections::BTreeMap::new(),
        }]
    );

    task_board
        .members
        .iter_mut()
        .find(|member| member.id == "staff-planner")
        .expect("projected planner")
        .ipc_agent_id = Some(" ipc-planner-next ".to_string());
    task_board.stage_actions = vec![super::models::TaskBoardStageAction {
        status: crate::app::task::TaskStatus::CodeReview,
        capability_id: super::stage_capabilities::SAVED_WORKFLOW_CAPABILITY_ID.to_string(),
        workflow_uuid: Some("018ff4b0-1d0d-7a2c-a30c-7bcf9ee1f002".to_string()),
        input_parameters: std::collections::BTreeMap::new(),
    }];

    super::apply_task_board_staff_config_to_staff_config(&mut config, &task_board)
        .expect("apply task board staff config");
    let planner = config
        .members
        .iter()
        .find(|member| member.id == "staff-planner")
        .expect("round-tripped planner");
    assert_eq!(planner.ipc_agent_id.as_deref(), Some("ipc-planner-next"));
    assert!(config.project_state.stage_workflow_bindings.is_empty());
    let orchestration =
        config.workflow_orchestrations.get("dev_lifecycle").expect("workflow orchestration");
    assert!(orchestration.stage_workflow_bindings.is_empty());
    assert_eq!(
        orchestration.stage_actions,
        vec![vw_config_types::staff::StaffStageActionConfig {
            stage_id: "code_review".to_string(),
            capability_id: super::stage_capabilities::SAVED_WORKFLOW_CAPABILITY_ID.to_string(),
            workflow_uuid: Some("018ff4b0-1d0d-7a2c-a30c-7bcf9ee1f002".to_string()),
            inputs: vw_config_types::staff::StaffStageActionInputsConfig {
                workflow_uuid: Some("018ff4b0-1d0d-7a2c-a30c-7bcf9ee1f002".to_string()),
                parameters: std::collections::BTreeMap::new(),
            },
        }]
    );
}

#[test]
fn staff_config_ignores_legacy_project_workflow_bindings_and_keeps_defaults() {
    let mut config = vw_config_types::staff::StaffConfig::default();
    config.project_state.stage_workflow_bindings =
        vec![vw_config_types::staff::StaffStageWorkflowBindingConfig {
            stage_id: "planning".to_string(),
            workflow_uuid: "018ff4b0-1d0d-7a2c-a30c-7bcf9ee1f001".to_string(),
        }];

    let task_board =
        super::task_board_staff_config_from_staff_config(config.clone()).expect("project staff");
    assert!(!task_board.stage_actions.is_empty());
    assert!(task_board.stage_actions.iter().all(|action| {
        action.capability_id != super::stage_capabilities::SAVED_WORKFLOW_CAPABILITY_ID
    }));
    assert!(task_board.stage_workflow_bindings.is_empty());
}

#[test]
fn staff_config_apply_preserves_project_capability_catalog() {
    let mut config = vw_config_types::staff::StaffConfig::default();
    let capability = vw_config_types::staff::StaffCapabilityConfig {
        id: "release_workflow".to_string(),
        label: "Release Workflow".to_string(),
        capability_type: vw_config_types::staff::StaffCapabilityKindConfig::SavedWorkflow,
        built_in: false,
        input_summary: vw_config_types::staff::StaffCapabilityIoSummaryConfig {
            description: "Workflow UUID and release channel".to_string(),
            fields: vec!["workflow_uuid".to_string(), "release_channel".to_string()],
        },
        result_summary: vw_config_types::staff::StaffCapabilityIoSummaryConfig {
            description: "Release result".to_string(),
            fields: vec!["summary".to_string()],
        },
    };
    config.workflow_orchestrations.insert(
        "dev_lifecycle".to_string(),
        vw_config_types::staff::StaffWorkflowOrchestrationConfig {
            capabilities: vec![capability],
            ..vw_config_types::staff::StaffWorkflowOrchestrationConfig::default()
        },
    );
    let expected_capabilities =
        config.workflow_orchestrations["dev_lifecycle"].capabilities.clone();

    let mut task_board =
        super::task_board_staff_config_from_staff_config(config.clone()).expect("project staff");
    task_board.stage_actions = vec![super::models::TaskBoardStageAction {
        status: crate::app::task::TaskStatus::CodeReview,
        capability_id: "release_workflow".to_string(),
        workflow_uuid: Some("018ff4b0-1d0d-7a2c-a30c-7bcf9ee1f003".to_string()),
        input_parameters: std::collections::BTreeMap::new(),
    }];

    super::apply_task_board_staff_config_to_staff_config(&mut config, &task_board)
        .expect("apply task board staff config");

    let orchestration =
        config.workflow_orchestrations.get("dev_lifecycle").expect("workflow orchestration");
    assert_eq!(orchestration.capabilities, expected_capabilities);
    assert!(
        orchestration.stage_actions.iter().any(|action| action.capability_id == "release_workflow")
    );
}

#[test]
fn staff_config_normalize_cleans_stage_assignment_member_ids() {
    let mut config = super::TaskBoardStaffConfig::default();
    config.stage_assignments = vec![super::TaskBoardStageAssignment {
        status: crate::app::task::TaskStatus::CodeReview,
        primary_member_id: Some(" staff-reviewer ".to_string()),
        backup_member_ids: vec![" staff-merger ".to_string(), String::new(), "  ".to_string()],
        manager_member_ids: vec![" staff-orchestrator ".to_string(), String::new()],
        autonomous: true,
    }];

    let normalized = config.normalize().expect("normalize stage assignment");
    let assignment = normalized.stage_assignments.first().expect("stage assignment");

    assert_eq!(assignment.primary_member_id.as_deref(), Some("staff-reviewer"));
    assert_eq!(assignment.backup_member_ids, vec!["staff-merger".to_string()]);
    assert_eq!(assignment.manager_member_ids, vec!["staff-orchestrator".to_string()]);
    assert!(assignment.autonomous);
}

#[test]
fn staff_config_normalize_rejects_invalid_stage_assignments() {
    let mut unknown = super::TaskBoardStaffConfig::default();
    unknown.stage_assignments = vec![super::TaskBoardStageAssignment {
        status: crate::app::task::TaskStatus::CodeReview,
        primary_member_id: Some("missing".to_string()),
        backup_member_ids: Vec::new(),
        manager_member_ids: Vec::new(),
        autonomous: false,
    }];
    assert!(
        unknown
            .normalize()
            .expect_err("unknown member")
            .contains("unknown task board stage assignment member id: missing")
    );

    let mut duplicate_member = super::TaskBoardStaffConfig::default();
    duplicate_member.stage_assignments = vec![super::TaskBoardStageAssignment {
        status: crate::app::task::TaskStatus::CodeReview,
        primary_member_id: Some("staff-reviewer".to_string()),
        backup_member_ids: vec!["staff-reviewer".to_string()],
        manager_member_ids: Vec::new(),
        autonomous: false,
    }];
    assert!(
        duplicate_member
            .normalize()
            .expect_err("duplicate member")
            .contains("duplicate task board stage assignment member id: staff-reviewer")
    );

    let mut duplicate_status = super::TaskBoardStaffConfig::default();
    duplicate_status.stage_assignments = vec![
        super::TaskBoardStageAssignment {
            status: crate::app::task::TaskStatus::CodeReview,
            primary_member_id: Some("staff-reviewer".to_string()),
            backup_member_ids: Vec::new(),
            manager_member_ids: Vec::new(),
            autonomous: false,
        },
        super::TaskBoardStageAssignment {
            status: crate::app::task::TaskStatus::CodeReview,
            primary_member_id: Some("staff-merger".to_string()),
            backup_member_ids: Vec::new(),
            manager_member_ids: Vec::new(),
            autonomous: false,
        },
    ];
    assert!(
        duplicate_status
            .normalize()
            .expect_err("duplicate status")
            .contains("duplicate task board stage assignment status: code_review")
    );
}

#[test]
fn staff_role_catalog_exposes_stable_keys_and_levels() {
    let expected = [
        (
            super::TaskBoardStaffRole::Orchestrator,
            "orchestrator",
            super::TaskBoardStaffLevel::Principal,
        ),
        (super::TaskBoardStaffRole::Planner, "planner", super::TaskBoardStaffLevel::Manager),
        (super::TaskBoardStaffRole::Executor, "executor", super::TaskBoardStaffLevel::Executor),
        (super::TaskBoardStaffRole::Reviewer, "reviewer", super::TaskBoardStaffLevel::Manager),
        (super::TaskBoardStaffRole::Merger, "merger", super::TaskBoardStaffLevel::Manager),
        (super::TaskBoardStaffRole::Observer, "observer", super::TaskBoardStaffLevel::Member),
    ];

    assert_eq!(super::TaskBoardStaffRole::all().len(), expected.len());
    for (role, key, level) in expected {
        assert_eq!(role.key(), key);
        assert_eq!(super::TaskBoardStaffLevel::for_role(role), level);
    }
}

#[test]
fn staff_member_deserialize_defaults_level_status_enabled_capacity_and_task_ids() {
    let member: super::TaskBoardStaffMember = serde_json::from_value(serde_json::json!({
        "id": "reviewer",
        "name": "Reviewer",
        "role": "reviewer",
        "delegate_agent": "main",
        "current_task_ids": ["task-1", "task-2"]
    }))
    .expect("member should deserialize");

    assert_eq!(member.level, super::TaskBoardStaffLevel::Manager);
    assert_eq!(member.status, super::TaskBoardStaffStatus::Online);
    assert!(member.enabled);
    assert_eq!(member.capacity, 1);
    assert_eq!(member.current_task_ids, vec!["task-1", "task-2"]);

    let explicit: super::TaskBoardStaffMember = serde_json::from_value(serde_json::json!({
        "id": "observer",
        "name": "Observer",
        "role": "observer",
        "level": "principal",
        "status": "blocked",
        "enabled": false,
        "capacity": 4
    }))
    .expect("explicit member should deserialize");
    assert_eq!(explicit.level, super::TaskBoardStaffLevel::Principal);
    assert_eq!(explicit.status, super::TaskBoardStaffStatus::Blocked);
    assert!(!explicit.enabled);
    assert_eq!(explicit.capacity, 4);
}

#[test]
fn capability_request_trims_custom_saved_workflow_fields_and_skips_built_ins() {
    let built_in = super::TaskBoardStaffCapability {
        id: "manual".to_string(),
        label: "Manual".to_string(),
        capability_type: super::TaskBoardStaffCapabilityKind::BuiltIn,
        built_in: true,
        supported_roles: Vec::new(),
        requires_workflow_uuid: false,
        input_summary: Default::default(),
        result_summary: Default::default(),
        input_contract: String::new(),
        result_semantics: String::new(),
        permission_hint: String::new(),
    };
    let mut custom = super::TaskBoardStaffCapability::custom_saved_workflow(
        " release ".to_string(),
        " Release ".to_string(),
    );
    custom.input_summary.description = " Inputs ".to_string();
    custom.input_summary.fields =
        vec![" workflow_uuid ".to_string(), String::new(), " channel ".to_string()];
    custom.result_summary.description = " Results ".to_string();
    custom.result_summary.fields = vec![" summary ".to_string(), "  ".to_string()];

    assert!(!built_in.editable());
    assert!(custom.editable());

    let request = super::task_board_staff_capability_catalog_request(&[built_in, custom]);

    assert_eq!(request.capabilities.len(), 1);
    assert_eq!(request.capabilities[0].id, "release");
    assert_eq!(request.capabilities[0].label, "Release");
    assert_eq!(request.capabilities[0].input_summary.description, "Inputs");
    assert_eq!(
        request.capabilities[0].input_summary.fields,
        vec!["workflow_uuid".to_string(), "channel".to_string()]
    );
    assert_eq!(request.capabilities[0].result_summary.description, "Results");
    assert_eq!(request.capabilities[0].result_summary.fields, vec!["summary".to_string()]);
}

#[test]
fn staff_config_normalize_cleans_member_bindings_targets_stage_workflows_and_action_inputs() {
    let mut config = super::TaskBoardStaffConfig::default();
    let planner =
        config.members.iter_mut().find(|member| member.id == "staff-planner").expect("planner");
    planner.name = " Planner ".to_string();
    planner.delegate_agent = Some(" main ".to_string());
    planner.acp_agent = Some("  ".to_string());
    planner.ipc_agent_id = Some(" ipc-planner ".to_string());
    planner.current_task_ids =
        vec![" task-2 ".to_string(), "task-1".to_string(), "task-2".to_string(), String::new()];
    planner.channel_bindings = vec![vw_config_types::staff::StaffChannelBindingConfig {
        channel_type: " MATRIX ".to_string(),
        instance_id: " planner ".to_string(),
    }];
    config.stage_workflow_bindings = vec![super::TaskBoardStageWorkflowBinding {
        status: crate::app::task::TaskStatus::Planning,
        workflow_uuid: " workflow-uuid ".to_string(),
    }];
    config.stage_actions = vec![super::TaskBoardStageAction {
        status: crate::app::task::TaskStatus::Planning,
        capability_id: super::stage_capabilities::SAVED_WORKFLOW_CAPABILITY_ID.to_string(),
        workflow_uuid: Some(" workflow-uuid ".to_string()),
        input_parameters: std::collections::BTreeMap::from([
            (" topic ".to_string(), serde_json::json!(" rust ")),
            (" ".to_string(), serde_json::json!("ignored")),
            ("count".to_string(), serde_json::json!(3)),
        ]),
    }];

    let normalized = config.normalize().expect("normalize config");
    let planner =
        normalized.members.iter().find(|member| member.id == "staff-planner").expect("planner");
    assert_eq!(planner.name, "Planner");
    assert_eq!(planner.delegate_agent.as_deref(), Some("main"));
    assert!(planner.acp_agent.is_none());
    assert_eq!(planner.ipc_agent_id.as_deref(), Some("ipc-planner"));
    assert_eq!(planner.current_task_ids, vec!["task-2".to_string(), "task-1".to_string()]);
    assert_eq!(planner.channel_bindings[0].channel_type, "matrix");
    assert_eq!(planner.channel_bindings[0].instance_id, "planner");
    assert_eq!(normalized.stage_workflow_bindings[0].workflow_uuid, "workflow-uuid");
    let action = normalized.stage_actions.first().expect("stage action");
    assert_eq!(action.workflow_uuid.as_deref(), Some("workflow-uuid"));
    assert_eq!(action.input_parameters.get("topic"), Some(&serde_json::json!("rust")));
    assert_eq!(action.input_parameters.get("count"), Some(&serde_json::json!(3)));
    assert!(!action.input_parameters.contains_key(""));
}

#[test]
fn staff_config_normalize_rejects_invalid_channel_bindings_stage_workflow_and_actions() {
    let mut unsupported_channel = super::TaskBoardStaffConfig::default();
    unsupported_channel.members[0].channel_bindings =
        vec![vw_config_types::staff::StaffChannelBindingConfig {
            channel_type: "slack".to_string(),
            instance_id: "default".to_string(),
        }];
    assert!(
        unsupported_channel
            .normalize()
            .expect_err("unsupported channel")
            .contains("unsupported task board staff channel binding type: slack")
    );

    let mut duplicate_channel = super::TaskBoardStaffConfig::default();
    duplicate_channel.members[0].channel_bindings = vec![
        vw_config_types::staff::StaffChannelBindingConfig {
            channel_type: "matrix".to_string(),
            instance_id: "default".to_string(),
        },
        vw_config_types::staff::StaffChannelBindingConfig {
            channel_type: "MATRIX".to_string(),
            instance_id: "other".to_string(),
        },
    ];
    assert!(
        duplicate_channel
            .normalize()
            .expect_err("duplicate channel")
            .contains("duplicate task board staff channel binding type: matrix")
    );

    let mut illegal_instance = super::TaskBoardStaffConfig::default();
    illegal_instance.members[0].channel_bindings =
        vec![vw_config_types::staff::StaffChannelBindingConfig {
            channel_type: "matrix".to_string(),
            instance_id: "bad/instance".to_string(),
        }];
    assert!(
        illegal_instance
            .normalize()
            .expect_err("illegal instance")
            .contains("must not contain ':', '/' or '\\'")
    );

    let mut duplicate_binding = super::TaskBoardStaffConfig::default();
    duplicate_binding.stage_workflow_bindings = vec![
        super::TaskBoardStageWorkflowBinding {
            status: crate::app::task::TaskStatus::Planning,
            workflow_uuid: "workflow-1".to_string(),
        },
        super::TaskBoardStageWorkflowBinding {
            status: crate::app::task::TaskStatus::Planning,
            workflow_uuid: "workflow-2".to_string(),
        },
    ];
    assert!(
        duplicate_binding
            .normalize()
            .expect_err("duplicate binding")
            .contains("duplicate task board stage workflow binding status: planning")
    );

    let mut empty_binding = super::TaskBoardStaffConfig::default();
    empty_binding.stage_workflow_bindings = vec![super::TaskBoardStageWorkflowBinding {
        status: crate::app::task::TaskStatus::Planning,
        workflow_uuid: "  ".to_string(),
    }];
    assert!(
        empty_binding
            .normalize()
            .expect_err("empty workflow uuid")
            .contains("workflow_uuid for planning cannot be empty")
    );

    let mut duplicate_action = super::TaskBoardStaffConfig::default();
    duplicate_action.stage_actions = vec![
        super::TaskBoardStageAction {
            status: crate::app::task::TaskStatus::Planning,
            capability_id: super::stage_capabilities::EXECUTE_TASK_CAPABILITY_ID.to_string(),
            workflow_uuid: None,
            input_parameters: Default::default(),
        },
        super::TaskBoardStageAction {
            status: crate::app::task::TaskStatus::Planning,
            capability_id: super::stage_capabilities::EXECUTE_TASK_CAPABILITY_ID.to_string(),
            workflow_uuid: None,
            input_parameters: Default::default(),
        },
    ];
    assert!(
        duplicate_action
            .normalize()
            .expect_err("duplicate action")
            .contains("duplicate task board stage action status: planning")
    );

    let mut missing_workflow_uuid = super::TaskBoardStaffConfig::default();
    missing_workflow_uuid.stage_actions = vec![super::TaskBoardStageAction {
        status: crate::app::task::TaskStatus::Planning,
        capability_id: super::stage_capabilities::SAVED_WORKFLOW_CAPABILITY_ID.to_string(),
        workflow_uuid: None,
        input_parameters: Default::default(),
    }];
    assert!(
        missing_workflow_uuid
            .normalize()
            .expect_err("missing workflow uuid")
            .contains("workflow_uuid for planning cannot be empty")
    );

    let mut forbidden_workflow_uuid = super::TaskBoardStaffConfig::default();
    forbidden_workflow_uuid.stage_actions = vec![super::TaskBoardStageAction {
        status: crate::app::task::TaskStatus::Planning,
        capability_id: super::stage_capabilities::EXECUTE_TASK_CAPABILITY_ID.to_string(),
        workflow_uuid: Some("workflow-1".to_string()),
        input_parameters: Default::default(),
    }];
    assert!(
        forbidden_workflow_uuid
            .normalize()
            .expect_err("forbidden workflow uuid")
            .contains("workflow_uuid for planning must be empty")
    );
}

#[test]
fn staff_config_projection_reports_invalid_gateway_member_and_stage_values() {
    let mut unknown_assignment_member = vw_config_types::staff::StaffConfig::default();
    unknown_assignment_member.project_state.member_task_assignments =
        vec![vw_config_types::staff::StaffMemberTaskAssignmentConfig {
            member_id: "missing".to_string(),
            task_ids: vec!["task-1".to_string()],
        }];
    let error = super::task_board_staff_config_from_staff_config(unknown_assignment_member)
        .expect_err("unknown member task assignment");
    assert!(
        error.contains("unknown staff member task assignment member_id: missing"),
        "unexpected validation error: {error}"
    );

    let mut invalid_role = vw_config_types::staff::StaffConfig::default();
    invalid_role.members[0].role = "mystery".to_string();
    assert!(
        super::task_board_staff_config_from_staff_config(invalid_role)
            .expect_err("invalid role")
            .contains("unknown task board staff role")
    );

    let mut invalid_level = vw_config_types::staff::StaffConfig::default();
    invalid_level.members[0].level = "executive".to_string();
    assert!(
        super::task_board_staff_config_from_staff_config(invalid_level)
            .expect_err("invalid level")
            .contains("unknown task board staff level")
    );

    let mut invalid_status = vw_config_types::staff::StaffConfig::default();
    invalid_status.members[0].status = "away".to_string();
    assert!(
        super::task_board_staff_config_from_staff_config(invalid_status)
            .expect_err("invalid status")
            .contains("unknown task board staff status")
    );
}

#[test]
fn retarget_for_workflow_id_prunes_stage_state_and_rejects_unknown_workflow_ids() {
    let content_idea =
        crate::app::task::TaskStatus::parse_key("content_idea").expect("content status");
    let mut config = super::TaskBoardStaffConfig::default();
    config.stage_assignments = vec![
        super::TaskBoardStageAssignment {
            status: crate::app::task::TaskStatus::CodeReview,
            primary_member_id: Some("staff-reviewer".to_string()),
            backup_member_ids: Vec::new(),
            manager_member_ids: Vec::new(),
            autonomous: true,
        },
        super::TaskBoardStageAssignment {
            status: content_idea,
            primary_member_id: Some("staff-orchestrator".to_string()),
            backup_member_ids: Vec::new(),
            manager_member_ids: Vec::new(),
            autonomous: true,
        },
    ];
    config.stage_workflow_bindings = vec![
        super::TaskBoardStageWorkflowBinding {
            status: crate::app::task::TaskStatus::CodeReview,
            workflow_uuid: "dev-workflow".to_string(),
        },
        super::TaskBoardStageWorkflowBinding {
            status: content_idea,
            workflow_uuid: "content-workflow".to_string(),
        },
    ];
    config.stage_actions = vec![
        super::TaskBoardStageAction {
            status: crate::app::task::TaskStatus::CodeReview,
            capability_id: super::stage_capabilities::CODE_REVIEW_CAPABILITY_ID.to_string(),
            workflow_uuid: None,
            input_parameters: Default::default(),
        },
        super::TaskBoardStageAction {
            status: content_idea,
            capability_id: super::stage_capabilities::ADVANCE_STATUS_CAPABILITY_ID.to_string(),
            workflow_uuid: None,
            input_parameters: Default::default(),
        },
    ];

    let retargeted =
        config.retarget_for_workflow_id("content_publishing").expect("retarget content workflow");

    assert!(retargeted.workflow.iter().any(|stage| stage.status == content_idea));
    assert!(
        retargeted.stage_assignments.iter().all(|assignment| assignment.status == content_idea)
    );
    assert!(
        retargeted.stage_workflow_bindings.iter().all(|binding| binding.status == content_idea)
    );
    assert!(retargeted.stage_actions.iter().all(|action| action.status == content_idea));
    assert!(
        super::TaskBoardStaffConfig::default()
            .normalize_for_workflow_id("missing_workflow")
            .expect_err("unknown normalize workflow")
            .contains("unknown task board workflow id: missing_workflow")
    );
    assert!(
        super::workflow_stages_for_workflow_id(" missing_workflow ")
            .expect_err("unknown workflow stages")
            .contains("unknown task board workflow id: missing_workflow")
    );
}
