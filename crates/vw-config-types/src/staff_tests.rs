//! 验证员工、工作流、能力目录与任务分配配置的归一化和校验边界。
//! 用例覆盖默认编排、序列化往返、交叉引用清理以及只读校验错误族。

use crate::config::Config;
use crate::staff::{
    StaffCapabilitiesConfig, StaffCapabilityConfig, StaffCapabilityIoSummaryConfig,
    StaffCapabilityKindConfig, StaffChannelBindingConfig, StaffConfig, StaffMemberConfig,
    StaffMemberTaskAssignmentConfig, StaffProjectStateConfig, StaffStageActionConfig,
    StaffStageActionInputsConfig, StaffStageAssignmentConfig, StaffStageWorkflowBindingConfig,
    StaffWorkflowOrchestrationConfig, StaffWorkflowStageConfig,
};
use serde_json::json;
use std::collections::BTreeMap;

// 默认名册必须全部可委派给主代理，避免初次启动出现不可调度成员。
#[test]
fn default_staff_config_binds_default_members_to_main_agent() {
    let config = Config::default();

    assert!(!config.staff.members.is_empty());
    assert!(config.staff.members.iter().all(|member| {
        member.enabled
            && member.delegate_agent.as_deref() == Some("main")
            && member.acp_agent.is_none()
    }));
}

// 完整往返 fixture 同时保护成员能力、通道绑定和运行限制的持久化结构。
#[test]
fn staff_config_roundtrips_member_capabilities() {
    let config = Config {
        staff: StaffConfig {
            members: vec![StaffMemberConfig {
                id: "engineer".to_string(),
                name: "Engineer".to_string(),
                role: "implementation".to_string(),
                level: "senior".to_string(),
                status: "available".to_string(),
                enabled: true,
                capacity: 2,
                delegate_agent: Some("coder".to_string()),
                acp_agent: None,
                ipc_agent_id: None,
                channel_bindings: Vec::new(),
                capabilities: StaffCapabilitiesConfig {
                    agentic: true,
                    allowed_tools: vec!["shell".to_string(), "apply_patch".to_string()],
                    allowed_skills: vec!["rustcodegraph".to_string()],
                    max_iterations: 12,
                    max_parallel_tasks: Some(3),
                    max_active_minutes: Some(45),
                },
            }],
            workflow_stages: vec![StaffWorkflowStageConfig {
                id: "build".to_string(),
                name: "Build".to_string(),
                task_status: "in_progress".to_string(),
                role: Some("implementation".to_string()),
                enabled: true,
                display_only: false,
            }],
            workflow_orchestrations: BTreeMap::new(),
            project_state: StaffProjectStateConfig {
                capabilities: Vec::new(),
                stage_assignments: vec![StaffStageAssignmentConfig {
                    stage_id: "build".to_string(),
                    primary_member_id: Some("engineer".to_string()),
                    backup_member_ids: Vec::new(),
                    manager_member_ids: Vec::new(),
                    autonomous: true,
                }],
                stage_workflow_bindings: Vec::new(),
                stage_actions: Vec::new(),
                member_task_assignments: Vec::new(),
            },
        },
        ..Default::default()
    };

    let serialized = serde_json::to_string(&config).expect("serialize config");
    let roundtripped: Config = serde_json::from_str(&serialized).expect("deserialize config");
    let member = roundtripped
        .staff
        .members
        .iter()
        .find(|member| member.id == "engineer")
        .expect("engineer staff member");

    assert!(member.capabilities.agentic);
    assert_eq!(member.capabilities.allowed_tools, vec!["shell", "apply_patch"]);
    assert_eq!(member.capabilities.allowed_skills, vec!["rustcodegraph"]);
    assert_eq!(member.capabilities.max_iterations, 12);
    assert_eq!(member.capabilities.max_parallel_tasks, Some(3));
    assert_eq!(member.capabilities.max_active_minutes, Some(45));
    assert!(roundtripped.staff.project_state.stage_assignments[0].autonomous);
    assert!(roundtripped.agents.is_empty());
}

// 归一化迁移旧阶段绑定，并清理消息目标，确保只保留当前数据模型。
#[test]
fn staff_config_normalizes_message_targets_and_discards_legacy_stage_bindings() {
    let mut config = StaffConfig::default();
    let planner = config
        .members
        .iter_mut()
        .find(|member| member.id == "staff-planner")
        .expect("default planner");
    planner.ipc_agent_id = Some(" ipc-planner ".to_string());
    config.project_state.capabilities = vec![StaffCapabilityConfig {
        id: " legacy_capability ".to_string(),
        label: " Legacy Capability ".to_string(),
        capability_type: StaffCapabilityKindConfig::SavedWorkflow,
        built_in: false,
        input_summary: StaffCapabilityIoSummaryConfig::default(),
        result_summary: StaffCapabilityIoSummaryConfig::default(),
    }];
    config.project_state.stage_workflow_bindings = vec![StaffStageWorkflowBindingConfig {
        stage_id: " planning ".to_string(),
        workflow_uuid: " 018ff4b0-1d0d-7a2c-a30c-7bcf9ee1f001 ".to_string(),
    }];
    config.project_state.stage_actions = vec![StaffStageActionConfig {
        stage_id: " planning ".to_string(),
        capability_id: " legacy_capability ".to_string(),
        workflow_uuid: None,
        inputs: StaffStageActionInputsConfig::default(),
    }];

    let normalized = config.normalize().expect("normalize staff config");
    let planner = normalized
        .members
        .iter()
        .find(|member| member.id == "staff-planner")
        .expect("normalized planner");
    assert_eq!(planner.ipc_agent_id.as_deref(), Some("ipc-planner"));
    assert!(normalized.project_state.stage_workflow_bindings.is_empty());
    assert_eq!(normalized.project_state.capabilities[0].id, "legacy_capability");
    assert_eq!(normalized.project_state.stage_actions[0].stage_id, "planning");

    let mut empty_ipc_agent_id = normalized.clone();
    empty_ipc_agent_id.members[1].ipc_agent_id = Some(" ".to_string());
    let empty_ipc_agent_id = empty_ipc_agent_id.normalize().expect("normalize empty ipc id");
    assert_eq!(empty_ipc_agent_id.members[1].ipc_agent_id, None);
}

// 阶段动作以能力目录为边界，保存工作流动作还必须携带可用 UUID。
#[test]
fn staff_stage_actions_normalize_and_validate() {
    let mut config = StaffConfig::default();
    config.project_state.stage_actions = vec![
        StaffStageActionConfig {
            stage_id: " code_review ".to_string(),
            capability_id: " saved_workflow ".to_string(),
            workflow_uuid: Some(" 018ff4b0-1d0d-7a2c-a30c-7bcf9ee1f020 ".to_string()),
            inputs: StaffStageActionInputsConfig::default(),
        },
        StaffStageActionConfig {
            stage_id: " pr_submitted ".to_string(),
            capability_id: " git_merge ".to_string(),
            workflow_uuid: None,
            inputs: StaffStageActionInputsConfig::default(),
        },
    ];

    let normalized = config.normalize().expect("stage actions normalize");
    assert_eq!(
        normalized.project_state.stage_actions,
        vec![
            StaffStageActionConfig {
                stage_id: "code_review".to_string(),
                capability_id: "saved_workflow".to_string(),
                workflow_uuid: Some("018ff4b0-1d0d-7a2c-a30c-7bcf9ee1f020".to_string()),
                inputs: StaffStageActionInputsConfig {
                    workflow_uuid: Some("018ff4b0-1d0d-7a2c-a30c-7bcf9ee1f020".to_string()),
                    parameters: BTreeMap::new(),
                },
            },
            StaffStageActionConfig {
                stage_id: "pr_submitted".to_string(),
                capability_id: "git_merge".to_string(),
                workflow_uuid: None,
                inputs: StaffStageActionInputsConfig::default(),
            },
        ]
    );
    normalized.validate().expect("normalized stage actions validate");

    let mut empty_stage = normalized.clone();
    empty_stage.project_state.stage_actions[0].stage_id = " ".to_string();
    assert!(
        empty_stage
            .normalize()
            .expect_err("empty stage action stage")
            .contains("staff stage action stage_id cannot be empty")
    );

    let mut unknown_stage = normalized.clone();
    unknown_stage.project_state.stage_actions[0].stage_id = "missing".to_string();
    assert!(
        unknown_stage
            .normalize()
            .expect_err("unknown stage action stage")
            .contains("unknown staff stage action stage_id: missing")
    );

    let mut duplicate_stage = normalized.clone();
    duplicate_stage.project_state.stage_actions.push(StaffStageActionConfig {
        stage_id: "code_review".to_string(),
        capability_id: "manual".to_string(),
        workflow_uuid: None,
        inputs: StaffStageActionInputsConfig::default(),
    });
    assert!(
        duplicate_stage
            .normalize()
            .expect_err("duplicate stage action stage")
            .contains("duplicate staff stage action stage_id: code_review")
    );

    let mut empty_capability = normalized.clone();
    empty_capability.project_state.stage_actions[0].capability_id = " ".to_string();
    assert!(
        empty_capability
            .normalize()
            .expect_err("empty stage action capability")
            .contains("staff stage action capability_id for code_review cannot be empty")
    );

    let mut missing_saved_workflow_uuid = normalized.clone();
    missing_saved_workflow_uuid.project_state.stage_actions[0].workflow_uuid = None;
    missing_saved_workflow_uuid
        .normalize()
        .expect("config layer does not validate capability-specific workflow_uuid rules");

    let mut unknown_capability = normalized;
    unknown_capability.project_state.stage_actions[0].capability_id =
        "custom_capability".to_string();
    assert!(
        unknown_capability
            .normalize()
            .expect_err("config layer validates capability ids")
            .contains(
                "unknown staff stage action capability_id for code_review: custom_capability"
            )
    );
}

// 外部通道绑定要求受支持的类型、有效实例标识及成员内唯一性。
#[test]
fn staff_channel_binding_staff_member_channel_bindings_normalize_and_validate() {
    let mut config = StaffConfig::default();
    let member = config.members.first_mut().expect("default staff member");
    member.channel_bindings = vec![
        StaffChannelBindingConfig {
            channel_type: " matrix ".to_string(),
            instance_id: " planner ".to_string(),
        },
        StaffChannelBindingConfig {
            channel_type: "feishu".to_string(),
            instance_id: "planner-app".to_string(),
        },
        StaffChannelBindingConfig {
            channel_type: "dingtalk".to_string(),
            instance_id: "1".to_string(),
        },
    ];

    let normalized = config.normalize().expect("normalize staff channel bindings");
    let member = normalized.members.first().expect("normalized staff member");
    assert_eq!(
        member.channel_bindings,
        vec![
            StaffChannelBindingConfig {
                channel_type: "matrix".to_string(),
                instance_id: "planner".to_string(),
            },
            StaffChannelBindingConfig {
                channel_type: "feishu".to_string(),
                instance_id: "planner-app".to_string(),
            },
            StaffChannelBindingConfig {
                channel_type: "dingtalk".to_string(),
                instance_id: "1".to_string(),
            },
        ]
    );

    let mut duplicate = normalized.clone();
    duplicate.members[0].channel_bindings.push(StaffChannelBindingConfig {
        channel_type: "matrix".to_string(),
        instance_id: "reviewer".to_string(),
    });
    assert!(
        duplicate
            .normalize()
            .expect_err("duplicate channel binding")
            .contains("duplicate staff channel binding type: matrix")
    );

    let mut unsupported = normalized.clone();
    unsupported.members[0].channel_bindings[0].channel_type = "slack".to_string();
    assert!(
        unsupported
            .normalize()
            .expect_err("unsupported channel type")
            .contains("unsupported staff channel binding type: slack")
    );

    let mut unsafe_id = normalized;
    unsafe_id.members[0].channel_bindings[0].instance_id = "bad:id".to_string();
    assert!(
        unsafe_id
            .normalize()
            .expect_err("unsafe instance id")
            .contains("staff channel binding instance_id for matrix must not contain")
    );
}

#[test]
fn staff_config_normalize_deduplicates_capabilities_and_requires_enabled_binding() {
    let mut config = StaffConfig::default();
    let member = config.members.first_mut().expect("default staff member");
    member.delegate_agent = Some(" main ".to_string());
    member.capabilities.allowed_tools =
        vec![" shell ".to_string(), String::new(), "shell".to_string()];
    member.capabilities.allowed_skills =
        vec![" rustcodegraph ".to_string(), "rustcodegraph".to_string()];

    let normalized = config.normalize().expect("normalize staff config");
    let member = normalized.members.first().expect("normalized staff member");

    assert_eq!(member.delegate_agent.as_deref(), Some("main"));
    assert_eq!(member.capabilities.allowed_tools, vec!["shell"]);
    assert_eq!(member.capabilities.allowed_skills, vec!["rustcodegraph"]);

    let mut invalid = StaffConfig::default();
    let member = invalid.members.first_mut().expect("default staff member");
    member.delegate_agent = None;
    member.acp_agent = Some(" ".to_string());

    let err = invalid.normalize().expect_err("enabled staff member without binding");
    assert!(err.contains("must bind a delegate_agent or acp_agent"));
}

// 能力目录和项目状态执行序列化往返，确保网关持久化不丢结构化输入输出。
#[test]
fn staff_capability_catalog_roundtrips_and_validates() {
    let mut config = StaffConfig::default();
    config.project_state.capabilities = vec![StaffCapabilityConfig {
        id: " product_review ".to_string(),
        label: " Product Review ".to_string(),
        capability_type: StaffCapabilityKindConfig::SavedWorkflow,
        built_in: false,
        input_summary: StaffCapabilityIoSummaryConfig {
            description: " Workflow UUID plus task summary ".to_string(),
            fields: vec![" workflow_uuid ".to_string(), "task_summary".to_string()],
        },
        result_summary: StaffCapabilityIoSummaryConfig {
            description: " Maps workflow result to lifecycle stage ".to_string(),
            fields: vec!["status".to_string()],
        },
    }];
    config.project_state.stage_actions = vec![StaffStageActionConfig {
        stage_id: " code_review ".to_string(),
        capability_id: " product_review ".to_string(),
        workflow_uuid: None,
        inputs: StaffStageActionInputsConfig {
            workflow_uuid: Some(" workflow-product-review ".to_string()),
            parameters: BTreeMap::from([(" focus ".to_string(), json!("release"))]),
        },
    }];

    let normalized = config.normalize().expect("capability catalog normalizes");
    let capability = &normalized.project_state.capabilities[0];
    assert_eq!(capability.id, "product_review");
    assert_eq!(capability.label, "Product Review");
    assert_eq!(capability.input_summary.description, "Workflow UUID plus task summary");
    assert_eq!(capability.input_summary.fields, vec!["workflow_uuid", "task_summary"]);
    assert_eq!(capability.result_summary.fields, vec!["status"]);
    normalized.validate().expect("capability catalog validates readonly");

    let encoded = serde_json::to_string(&normalized).expect("serialize staff config");
    assert!(encoded.contains("\"capabilities\""));
    let decoded: StaffConfig = serde_json::from_str(&encoded).expect("decode staff config");
    assert_eq!(decoded.project_state.capabilities, normalized.project_state.capabilities);
    assert_eq!(decoded.project_state.stage_actions, normalized.project_state.stage_actions);

    let mut overrides_builtin = normalized.clone();
    overrides_builtin.project_state.capabilities[0].id = "code_review".to_string();
    assert!(
        overrides_builtin
            .normalize()
            .expect_err("custom capability cannot override built-in")
            .contains("staff capability id cannot override built-in capability: code_review")
    );

    let mut unknown_capability = normalized.clone();
    unknown_capability.project_state.stage_actions[0].capability_id = "missing".to_string();
    assert!(
        unknown_capability
            .normalize()
            .expect_err("unknown stage action capability")
            .contains("unknown staff stage action capability_id for code_review: missing")
    );
}

#[test]
fn staff_config_roundtrips_workflow_state_by_workflow_id() {
    let config = StaffConfig {
        workflow_orchestrations: BTreeMap::from([
            (
                " dev_lifecycle ".to_string(),
                StaffWorkflowOrchestrationConfig {
                    capabilities: vec![StaffCapabilityConfig {
                        id: " product_review ".to_string(),
                        label: " Product Review ".to_string(),
                        capability_type: StaffCapabilityKindConfig::SavedWorkflow,
                        built_in: false,
                        input_summary: StaffCapabilityIoSummaryConfig {
                            description: " Workflow UUID and review focus ".to_string(),
                            fields: vec![" workflow_uuid ".to_string(), "focus".to_string()],
                        },
                        result_summary: StaffCapabilityIoSummaryConfig {
                            description: " Review result ".to_string(),
                            fields: vec!["status".to_string()],
                        },
                    }],
                    stage_assignments: vec![StaffStageAssignmentConfig {
                        stage_id: " code_review ".to_string(),
                        primary_member_id: Some(" staff-reviewer ".to_string()),
                        backup_member_ids: Vec::new(),
                        manager_member_ids: Vec::new(),
                        autonomous: true,
                    }],
                    stage_workflow_bindings: vec![StaffStageWorkflowBindingConfig {
                        stage_id: " planning ".to_string(),
                        workflow_uuid: " workflow-planning ".to_string(),
                    }],
                    stage_actions: vec![StaffStageActionConfig {
                        stage_id: " code_review ".to_string(),
                        capability_id: " product_review ".to_string(),
                        workflow_uuid: None,
                        inputs: StaffStageActionInputsConfig {
                            workflow_uuid: Some(" workflow-product-review ".to_string()),
                            parameters: BTreeMap::from([(" focus ".to_string(), json!("release"))]),
                        },
                    }],
                },
            ),
            (
                " marketing_campaign ".to_string(),
                StaffWorkflowOrchestrationConfig {
                    capabilities: vec![StaffCapabilityConfig {
                        id: " campaign_publish ".to_string(),
                        label: " Campaign Publish ".to_string(),
                        capability_type: StaffCapabilityKindConfig::SavedWorkflow,
                        built_in: false,
                        input_summary: StaffCapabilityIoSummaryConfig::default(),
                        result_summary: StaffCapabilityIoSummaryConfig::default(),
                    }],
                    stage_assignments: vec![StaffStageAssignmentConfig {
                        stage_id: " campaign_planning ".to_string(),
                        primary_member_id: Some(" staff-planner ".to_string()),
                        backup_member_ids: Vec::new(),
                        manager_member_ids: Vec::new(),
                        autonomous: true,
                    }],
                    stage_workflow_bindings: vec![StaffStageWorkflowBindingConfig {
                        stage_id: " campaign_running ".to_string(),
                        workflow_uuid: " workflow-campaign-running ".to_string(),
                    }],
                    stage_actions: vec![StaffStageActionConfig {
                        stage_id: " campaign_reviewing ".to_string(),
                        capability_id: " campaign_publish ".to_string(),
                        workflow_uuid: None,
                        inputs: StaffStageActionInputsConfig::default(),
                    }],
                },
            ),
        ]),
        project_state: StaffProjectStateConfig {
            member_task_assignments: vec![crate::staff::StaffMemberTaskAssignmentConfig {
                member_id: "staff-reviewer".to_string(),
                task_ids: vec!["task-1".to_string()],
            }],
            ..StaffProjectStateConfig::default()
        },
        ..StaffConfig::default()
    };

    let normalized = config.normalize().expect("workflow orchestration normalizes");
    normalized.validate().expect("workflow orchestration validates");
    let orchestration = normalized
        .workflow_orchestrations
        .get("dev_lifecycle")
        .expect("workflow orchestration by normalized workflow id");

    assert_eq!(orchestration.capabilities[0].id, "product_review");
    assert_eq!(orchestration.stage_assignments[0].stage_id, "code_review");
    assert!(orchestration.stage_workflow_bindings.is_empty());
    assert!(orchestration.stage_actions.iter().all(|action| {
        !(action.stage_id == "planning" && action.capability_id == "saved_workflow")
    }));
    assert!(orchestration.stage_actions.iter().any(|action| {
        action.stage_id == "code_review"
            && action.capability_id == "product_review"
            && action.inputs.workflow_uuid.as_deref() == Some("workflow-product-review")
    }));
    let campaign = normalized
        .workflow_orchestrations
        .get("marketing_campaign")
        .expect("custom workflow orchestration survives normalize");
    assert_eq!(campaign.capabilities[0].id, "campaign_publish");
    assert_eq!(campaign.stage_assignments[0].stage_id, "campaign_planning");
    assert!(campaign.stage_workflow_bindings.is_empty());
    assert!(campaign.stage_actions.iter().all(|action| {
        !(action.stage_id == "campaign_running" && action.capability_id == "saved_workflow")
    }));
    assert!(campaign.stage_actions.iter().any(|action| {
        action.stage_id == "campaign_reviewing" && action.capability_id == "campaign_publish"
    }));

    let encoded = serde_json::to_value(&normalized).expect("serialize staff config");
    let workflow_state = &encoded["workflow_orchestrations"]["dev_lifecycle"];
    assert!(workflow_state.get("capabilities").is_some());
    assert!(workflow_state.get("stage_assignments").is_some());
    assert!(workflow_state.get("stage_workflow_bindings").is_none());
    assert!(workflow_state.get("stage_actions").is_some());
    assert!(workflow_state.get("member_task_assignments").is_none());

    let decoded: StaffConfig = serde_json::from_value(encoded).expect("decode staff config");
    assert_eq!(decoded.workflow_orchestrations, normalized.workflow_orchestrations);
    assert_eq!(decoded.project_state.member_task_assignments[0].task_ids, vec!["task-1"]);
}

#[test]
fn staff_stage_action_inputs_and_outcomes_roundtrip() {
    let legacy_payload = r#"{
        "members": [],
        "workflow_stages": [],
        "project_state": {
            "stage_workflow_bindings": [{
                "stage_id": "code_review",
                "workflow_uuid": "workflow-from-binding"
            }],
            "stage_actions": [{
                "stage_id": "pr_submitted",
                "capability_id": "saved_workflow",
                "workflow_uuid": "workflow-from-action"
            }]
        }
    }"#;
    let config: StaffConfig = serde_json::from_str(legacy_payload).expect("payload decodes");
    let normalized = config.normalize().expect("stage action workflow uuid maps into inputs");

    assert!(normalized.project_state.stage_workflow_bindings.is_empty());
    assert!(
        normalized
            .project_state
            .stage_actions
            .iter()
            .all(|action| action.stage_id != "code_review")
    );

    let pr_submitted = normalized
        .project_state
        .stage_actions
        .iter()
        .find(|action| action.stage_id == "pr_submitted")
        .expect("stage action survives");
    assert_eq!(pr_submitted.inputs.workflow_uuid.as_deref(), Some("workflow-from-action"));
    assert_eq!(pr_submitted.workflow_uuid.as_deref(), Some("workflow-from-action"));
}

// 成员任务归属允许清理空项，但同一任务不能同时归属于多个成员。
#[test]
fn member_task_assignments_normalize_empty_entries_and_reject_cross_member_duplicates() {
    let mut config = StaffConfig::default();
    config.project_state.member_task_assignments = vec![
        StaffMemberTaskAssignmentConfig {
            member_id: " staff-planner ".to_string(),
            task_ids: vec![
                " task-2 ".to_string(),
                String::new(),
                "task-1".to_string(),
                "task-1".to_string(),
            ],
        },
        StaffMemberTaskAssignmentConfig {
            member_id: "staff-executor".to_string(),
            task_ids: vec![" ".to_string()],
        },
    ];

    let normalized = config.normalize().expect("member task assignments normalize");
    assert_eq!(
        normalized.project_state.member_task_assignments,
        vec![StaffMemberTaskAssignmentConfig {
            member_id: "staff-planner".to_string(),
            task_ids: vec!["task-2".to_string(), "task-1".to_string()],
        }]
    );

    let mut duplicate_task = normalized;
    duplicate_task.project_state.member_task_assignments.push(StaffMemberTaskAssignmentConfig {
        member_id: "staff-executor".to_string(),
        task_ids: vec!["task-1".to_string()],
    });

    assert!(
        duplicate_task
            .normalize()
            .expect_err("one task cannot be assigned to multiple staff members")
            .contains("duplicate staff member task assignment task id: task-1")
    );
}

// 后续错误族用例分别锁定成员、阶段、编排、能力和任务引用的诊断文本。
#[test]
fn staff_members_normalize_defaults_and_report_readonly_identity_errors() {
    let mut config = StaffConfig::default();
    let member = config.members.first_mut().expect("default staff member");
    member.id = " custom-member ".to_string();
    member.name = " ".to_string();
    member.role = " ".to_string();
    member.level = "\n".to_string();
    member.status = " ".to_string();
    member.delegate_agent = Some(" ".to_string());
    member.acp_agent = Some(" acp-reviewer ".to_string());

    let normalized = config.normalize().expect("member defaults normalize");
    let member = normalized.members.first().expect("normalized member");
    assert_eq!(member.id, "custom-member");
    assert_eq!(member.name, "custom-member");
    assert_eq!(member.role, "member");
    assert_eq!(member.level, "member");
    assert_eq!(member.status, "online");
    assert_eq!(member.delegate_agent, None);
    assert_eq!(member.acp_agent.as_deref(), Some("acp-reviewer"));

    let mut empty_id = StaffConfig::default();
    empty_id.members[0].id = " ".to_string();
    assert_normalize_error(empty_id, "staff member id cannot be empty");

    let mut duplicate_id = StaffConfig::default();
    duplicate_id.members[1].id = duplicate_id.members[0].id.clone();
    assert_normalize_error(duplicate_id, "duplicate staff member id: staff-orchestrator");

    let mut empty_members = StaffConfig::default();
    empty_members.members.clear();
    assert_validate_error(&empty_members, "staff config must contain at least one member");

    let mut zero_capacity = StaffConfig::default();
    zero_capacity.members[0].capacity = 0;
    assert_validate_error(&zero_capacity, "staff member staff-orchestrator capacity cannot be 0");

    let mut duplicate_tool = StaffConfig::default();
    duplicate_tool.members[0].capabilities.allowed_tools =
        vec!["shell".to_string(), "shell".to_string()];
    assert_validate_error(&duplicate_tool, "duplicate staff allowed tool: shell");
}

#[test]
fn staff_workflow_stages_normalize_defaults_and_report_readonly_stage_errors() {
    let config = StaffConfig {
        workflow_stages: vec![
            StaffWorkflowStageConfig {
                id: " qa ".to_string(),
                name: " ".to_string(),
                task_status: " qa_status ".to_string(),
                role: Some(" reviewer ".to_string()),
                enabled: true,
                display_only: false,
            },
            StaffWorkflowStageConfig {
                id: " blocked ".to_string(),
                name: "Blocked".to_string(),
                task_status: " blocked ".to_string(),
                role: Some(" orchestrator ".to_string()),
                enabled: false,
                display_only: false,
            },
        ],
        ..StaffConfig::default()
    };

    let normalized = config.normalize().expect("workflow stages normalize");
    assert_eq!(normalized.workflow_stages[0].id, "qa");
    assert_eq!(normalized.workflow_stages[0].name, "qa");
    assert_eq!(normalized.workflow_stages[0].task_status, "qa_status");
    assert_eq!(normalized.workflow_stages[0].role.as_deref(), Some("reviewer"));
    assert_eq!(normalized.workflow_stages[1].role, None);
    assert!(normalized.workflow_stages[1].display_only);

    let mut empty_id = StaffConfig::default();
    empty_id.workflow_stages[0].id = " ".to_string();
    assert_normalize_error(empty_id, "staff workflow stage id cannot be empty");

    let mut duplicate_id = StaffConfig::default();
    duplicate_id.workflow_stages[1].id = duplicate_id.workflow_stages[0].id.clone();
    assert_normalize_error(duplicate_id, "duplicate staff workflow stage id: pool");

    let mut empty_status = StaffConfig::default();
    empty_status.workflow_stages[0].task_status = " ".to_string();
    assert_normalize_error(empty_status, "staff workflow stage pool task_status cannot be empty");

    let mut duplicate_status = StaffConfig::default();
    duplicate_status.workflow_stages[1].task_status =
        duplicate_status.workflow_stages[0].task_status.clone();
    assert_normalize_error(duplicate_status, "duplicate staff workflow task_status: pool");

    let mut no_stages = StaffConfig::default();
    no_stages.workflow_stages.clear();
    assert_validate_error(&no_stages, "staff config must contain at least one workflow stage");

    let mut display_only_with_role = StaffConfig::default();
    display_only_with_role.workflow_stages[0].display_only = true;
    assert_validate_error(
        &display_only_with_role,
        "display-only staff workflow stage cannot own role: pool",
    );

    let mut stage_without_role = StaffConfig::default();
    stage_without_role.workflow_stages[0].role = None;
    assert_validate_error(&stage_without_role, "staff workflow stage requires a role: pool");

    let mut disabled_with_role = StaffConfig::default();
    disabled_with_role.workflow_stages[0].enabled = false;
    assert_validate_error(
        &disabled_with_role,
        "disabled staff workflow stage cannot own role: pool",
    );
}

#[test]
fn staff_stage_assignments_report_stage_and_member_validation_errors() {
    let mut config = StaffConfig::default();
    config.project_state.stage_assignments = vec![StaffStageAssignmentConfig {
        stage_id: " running ".to_string(),
        primary_member_id: Some(" staff-executor ".to_string()),
        backup_member_ids: vec![" staff-reviewer ".to_string()],
        manager_member_ids: vec![" staff-orchestrator ".to_string()],
        autonomous: true,
    }];

    let normalized = config.normalize().expect("stage assignments normalize");
    assert_eq!(normalized.project_state.stage_assignments[0].stage_id, "running");
    assert_eq!(
        normalized.project_state.stage_assignments[0].primary_member_id.as_deref(),
        Some("staff-executor")
    );
    assert_eq!(
        normalized.project_state.stage_assignments[0].backup_member_ids,
        vec!["staff-reviewer"]
    );

    let mut empty_stage = StaffConfig::default();
    empty_stage.project_state.stage_assignments = vec![StaffStageAssignmentConfig {
        stage_id: " ".to_string(),
        primary_member_id: Some("staff-executor".to_string()),
        backup_member_ids: Vec::new(),
        manager_member_ids: Vec::new(),
        autonomous: false,
    }];
    assert_normalize_error(empty_stage, "staff stage assignment stage_id cannot be empty");

    let mut unknown_stage = normalized.clone();
    unknown_stage.project_state.stage_assignments[0].stage_id = "missing".to_string();
    assert_normalize_error(unknown_stage, "unknown staff stage assignment stage_id: missing");

    let mut duplicate_stage = normalized.clone();
    duplicate_stage
        .project_state
        .stage_assignments
        .push(normalized.project_state.stage_assignments[0].clone());
    assert_normalize_error(duplicate_stage, "duplicate staff stage assignment stage_id: running");

    let mut empty_member = normalized.clone();
    empty_member.project_state.stage_assignments[0].primary_member_id = Some(" ".to_string());
    assert_validate_error(&empty_member, "staff stage assignment member id cannot be empty");

    let mut unknown_member = normalized.clone();
    unknown_member.project_state.stage_assignments[0].backup_member_ids =
        vec!["unknown-member".to_string()];
    assert_normalize_error(
        unknown_member,
        "unknown staff stage assignment member id: unknown-member",
    );

    let mut duplicate_member = normalized;
    duplicate_member.project_state.stage_assignments[0].backup_member_ids =
        vec!["staff-executor".to_string()];
    assert_normalize_error(
        duplicate_member,
        "duplicate staff stage assignment member id: staff-executor",
    );
}

#[test]
fn staff_workflow_orchestrations_report_trimmed_key_assignment_and_action_errors() {
    let action = StaffStageActionConfig {
        stage_id: "review".to_string(),
        capability_id: "manual".to_string(),
        workflow_uuid: None,
        inputs: StaffStageActionInputsConfig::default(),
    };
    let assignment = StaffStageAssignmentConfig {
        stage_id: "review".to_string(),
        primary_member_id: Some("staff-reviewer".to_string()),
        backup_member_ids: Vec::new(),
        manager_member_ids: Vec::new(),
        autonomous: true,
    };

    let duplicate_workflow = StaffConfig {
        workflow_orchestrations: BTreeMap::from([
            (" review_flow ".to_string(), StaffWorkflowOrchestrationConfig::default()),
            ("review_flow".to_string(), StaffWorkflowOrchestrationConfig::default()),
        ]),
        ..StaffConfig::default()
    };
    assert_normalize_error(
        duplicate_workflow,
        "duplicate staff workflow orchestration workflow_id: review_flow",
    );

    let empty_workflow = StaffConfig {
        workflow_orchestrations: BTreeMap::from([(
            " ".to_string(),
            StaffWorkflowOrchestrationConfig::default(),
        )]),
        ..StaffConfig::default()
    };
    assert_normalize_error(
        empty_workflow,
        "staff workflow orchestration workflow_id cannot be empty",
    );

    let duplicate_assignment = StaffConfig {
        workflow_orchestrations: BTreeMap::from([(
            "flow".to_string(),
            StaffWorkflowOrchestrationConfig {
                stage_assignments: vec![assignment.clone(), assignment],
                ..StaffWorkflowOrchestrationConfig::default()
            },
        )]),
        ..StaffConfig::default()
    };
    assert_normalize_error(
        duplicate_assignment,
        "duplicate staff stage assignment stage_id: review",
    );

    let unknown_member = StaffConfig {
        workflow_orchestrations: BTreeMap::from([(
            "flow".to_string(),
            StaffWorkflowOrchestrationConfig {
                stage_assignments: vec![StaffStageAssignmentConfig {
                    stage_id: "review".to_string(),
                    primary_member_id: Some("missing".to_string()),
                    backup_member_ids: Vec::new(),
                    manager_member_ids: Vec::new(),
                    autonomous: false,
                }],
                ..StaffWorkflowOrchestrationConfig::default()
            },
        )]),
        ..StaffConfig::default()
    };
    assert_normalize_error(unknown_member, "unknown staff stage assignment member id: missing");

    let duplicate_action = StaffConfig {
        workflow_orchestrations: BTreeMap::from([(
            "flow".to_string(),
            StaffWorkflowOrchestrationConfig {
                stage_actions: vec![action.clone(), action],
                ..StaffWorkflowOrchestrationConfig::default()
            },
        )]),
        ..StaffConfig::default()
    };
    assert_normalize_error(duplicate_action, "duplicate staff stage action stage_id: review");
}

#[test]
fn staff_capability_catalog_and_runtime_limits_report_all_error_families() {
    let mut empty_id = StaffConfig::default();
    empty_id.project_state.capabilities = vec![StaffCapabilityConfig {
        id: " ".to_string(),
        label: "Empty".to_string(),
        capability_type: StaffCapabilityKindConfig::SavedWorkflow,
        built_in: false,
        input_summary: StaffCapabilityIoSummaryConfig::default(),
        result_summary: StaffCapabilityIoSummaryConfig::default(),
    }];
    assert_normalize_error(empty_id, "staff capability id cannot be empty");

    let mut duplicate_id = StaffConfig::default();
    duplicate_id.project_state.capabilities = vec![
        capability("review", "Review", false),
        capability(" review ", "Duplicate Review", false),
    ];
    assert_normalize_error(duplicate_id, "duplicate staff capability id: review");

    let mut unknown_builtin = StaffConfig::default();
    unknown_builtin.project_state.capabilities =
        vec![capability("custom_builtin", "Custom Builtin", true)];
    assert_normalize_error(
        unknown_builtin,
        "staff built-in capability id is not registered: custom_builtin",
    );

    let mut empty_label = StaffConfig::default();
    empty_label.project_state.capabilities = vec![capability("review", " ", false)];
    assert_validate_error(&empty_label, "staff capability label for review cannot be empty");

    let mut duplicate_input_field = StaffConfig::default();
    duplicate_input_field.project_state.capabilities = vec![StaffCapabilityConfig {
        input_summary: StaffCapabilityIoSummaryConfig {
            description: "inputs".to_string(),
            fields: vec!["task".to_string(), "task".to_string()],
        },
        ..capability("review", "Review", false)
    }];
    assert_validate_error(
        &duplicate_input_field,
        "duplicate staff capability input_summary field for review: task",
    );

    let mut duplicate_result_field = StaffConfig::default();
    duplicate_result_field.project_state.capabilities = vec![StaffCapabilityConfig {
        result_summary: StaffCapabilityIoSummaryConfig {
            description: "results".to_string(),
            fields: vec![" ".to_string()],
        },
        ..capability("review", "Review", false)
    }];
    assert_validate_error(
        &duplicate_result_field,
        "staff capability result_summary field for review cannot be empty",
    );

    let mut runtime =
        StaffCapabilitiesConfig { max_iterations: 0, ..StaffCapabilitiesConfig::default() };
    assert!(runtime.validate().unwrap_err().contains("max_iterations cannot be 0"));
    runtime.max_iterations = 1;
    runtime.max_parallel_tasks = Some(0);
    assert!(runtime.validate().unwrap_err().contains("max_parallel_tasks cannot be 0"));
    runtime.max_parallel_tasks = Some(1);
    runtime.max_active_minutes = Some(0);
    assert!(runtime.validate().unwrap_err().contains("max_active_minutes cannot be 0"));
}

#[test]
fn staff_stage_actions_report_workflow_uuid_and_parameter_validation_errors() {
    let mut conflict = StaffConfig::default();
    conflict.project_state.stage_actions = vec![StaffStageActionConfig {
        stage_id: "code_review".to_string(),
        capability_id: "saved_workflow".to_string(),
        workflow_uuid: Some("workflow-a".to_string()),
        inputs: StaffStageActionInputsConfig {
            workflow_uuid: Some("workflow-b".to_string()),
            parameters: BTreeMap::new(),
        },
    }];
    assert_normalize_error(
        conflict,
        "staff stage action workflow_uuid for code_review conflicts with inputs.workflow_uuid",
    );

    let mut duplicate_parameter_key = StaffConfig::default();
    duplicate_parameter_key.project_state.stage_actions = vec![StaffStageActionConfig {
        stage_id: "code_review".to_string(),
        capability_id: "saved_workflow".to_string(),
        workflow_uuid: None,
        inputs: StaffStageActionInputsConfig {
            workflow_uuid: Some("workflow-review".to_string()),
            parameters: BTreeMap::from([
                (" focus ".to_string(), json!("release")),
                ("focus".to_string(), json!("security")),
            ]),
        },
    }];
    assert_normalize_error(
        duplicate_parameter_key,
        "duplicate staff stage action input parameter for code_review key: focus",
    );

    let mut empty_parameter_key = StaffConfig::default();
    empty_parameter_key.project_state.stage_actions = vec![StaffStageActionConfig {
        stage_id: "code_review".to_string(),
        capability_id: "saved_workflow".to_string(),
        workflow_uuid: None,
        inputs: StaffStageActionInputsConfig {
            workflow_uuid: Some("workflow-review".to_string()),
            parameters: BTreeMap::from([(" ".to_string(), json!(true))]),
        },
    }];
    assert_normalize_error(
        empty_parameter_key,
        "staff stage action input parameter for code_review key cannot be empty",
    );

    let mut blank_action_workflow = StaffConfig::default();
    blank_action_workflow.project_state.stage_actions = vec![StaffStageActionConfig {
        stage_id: "code_review".to_string(),
        capability_id: "saved_workflow".to_string(),
        workflow_uuid: Some(" ".to_string()),
        inputs: StaffStageActionInputsConfig::default(),
    }];
    assert_validate_error(
        &blank_action_workflow,
        "staff stage action workflow_uuid for code_review cannot be empty when provided",
    );

    let mut blank_input_workflow = StaffConfig::default();
    blank_input_workflow.project_state.stage_actions = vec![StaffStageActionConfig {
        stage_id: "code_review".to_string(),
        capability_id: "saved_workflow".to_string(),
        workflow_uuid: None,
        inputs: StaffStageActionInputsConfig {
            workflow_uuid: Some(" ".to_string()),
            parameters: BTreeMap::new(),
        },
    }];
    assert_validate_error(
        &blank_input_workflow,
        "staff stage action inputs.workflow_uuid for code_review cannot be empty when provided",
    );
}

#[test]
fn staff_member_task_assignments_report_member_and_readonly_task_errors() {
    let mut unknown_member = StaffConfig::default();
    unknown_member.project_state.member_task_assignments = vec![StaffMemberTaskAssignmentConfig {
        member_id: "missing".to_string(),
        task_ids: vec!["task-1".to_string()],
    }];
    assert_normalize_error(
        unknown_member,
        "unknown staff member task assignment member_id: missing",
    );

    let mut duplicate_member = StaffConfig::default();
    duplicate_member.project_state.member_task_assignments = vec![
        StaffMemberTaskAssignmentConfig {
            member_id: "staff-planner".to_string(),
            task_ids: vec!["task-1".to_string()],
        },
        StaffMemberTaskAssignmentConfig {
            member_id: " staff-planner ".to_string(),
            task_ids: vec!["task-2".to_string()],
        },
    ];
    assert_normalize_error(
        duplicate_member,
        "duplicate staff member task assignment member_id: staff-planner",
    );

    let mut empty_member = StaffConfig::default();
    empty_member.project_state.member_task_assignments = vec![StaffMemberTaskAssignmentConfig {
        member_id: " ".to_string(),
        task_ids: vec!["task-1".to_string()],
    }];
    assert_validate_error(&empty_member, "staff member task assignment member_id cannot be empty");

    let mut empty_task = StaffConfig::default();
    empty_task.project_state.member_task_assignments = vec![StaffMemberTaskAssignmentConfig {
        member_id: "staff-planner".to_string(),
        task_ids: vec![" ".to_string()],
    }];
    assert_validate_error(&empty_task, "staff member task assignment task id cannot be empty");
}

// fixture 构造器集中填充能力结构，使错误用例只突出正在验证的字段。
fn capability(id: &str, label: &str, built_in: bool) -> StaffCapabilityConfig {
    StaffCapabilityConfig {
        id: id.to_string(),
        label: label.to_string(),
        capability_type: if built_in {
            StaffCapabilityKindConfig::BuiltIn
        } else {
            StaffCapabilityKindConfig::SavedWorkflow
        },
        built_in,
        input_summary: StaffCapabilityIoSummaryConfig::default(),
        result_summary: StaffCapabilityIoSummaryConfig::default(),
    }
}

// 同一非法配置必须在可变归一化与只读校验入口返回一致错误。
fn assert_normalize_error(config: StaffConfig, expected: &str) {
    let error = config.normalize().expect_err("config should fail normalize");
    assert!(error.contains(expected), "expected {expected:?}, got {error:?}");
}

fn assert_validate_error(config: &StaffConfig, expected: &str) {
    let error = config.validate().expect_err("config should fail validate");
    assert!(error.contains(expected), "expected {expected:?}, got {error:?}");
}
