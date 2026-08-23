// 验证员工模式工作流投影、配置重载与错误拒绝。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use crate::app::Message;
use crate::app::components::system_settings::SystemTab;
use crate::app::state::{
    TaskBoardStaffAssignmentTab, TaskBoardStaffFeedTab, TaskBoardStaffOrchestrationTab,
    TaskBoardViewMode,
};
use crate::app::task::staff::models::TaskBoardStageAction;
use crate::app::task::staff::stage_capabilities::{
    ADVANCE_STATUS_CAPABILITY_ID, CODE_REVIEW_CAPABILITY_ID, EXECUTE_TASK_CAPABILITY_ID,
    GIT_MERGE_CAPABILITY_ID, SAVED_WORKFLOW_CAPABILITY_ID,
};
use crate::app::task::staff::{
    TaskBoardStaffCapability, TaskBoardStaffCapabilityDraftField, TaskBoardStaffCapabilityKind,
    TaskBoardStaffCapabilitySummary, TaskBoardStaffCapabilitySummaryKind, TaskBoardStaffEvent,
    TaskBoardStaffEventKind, TaskBoardStaffLevel, TaskBoardStaffMember,
    TaskBoardStaffMessageDeliveryStatus, TaskBoardStaffRole,
};
use crate::app::task::workflow::StatusOutcome;
use crate::app::task::{
    Task, TaskBoardStaffCommand, TaskBoardStaffCommandAction, TaskBoardStaffConfig,
    TaskBoardStageAssignment, TaskBoardStageWorkflowBinding, TaskExecutorBackend, TaskStatus,
};
use iced_runtime::Action;
use iced_runtime::futures::futures::StreamExt;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::time::{Duration, Instant};
use vw_config_types::staff::{
    StaffConfig, StaffMemberTaskAssignmentConfig, StaffStageActionConfig,
    StaffStageActionInputsConfig, StaffStageAssignmentConfig, StaffStageWorkflowBindingConfig,
    StaffWorkflowOrchestrationConfig,
};
use vw_config_types::ui::{GatewayClientServerConfig, GatewayClientSystemSettingsConfig};

const UPDATE_SOURCE: &str = include_str!("update.rs");

fn ensure_task_board_project_path(app: &mut crate::app::App) -> String {
    if let Some(project_path) = &app.project_path {
        return project_path.clone();
    }

    let project_path = "/tmp/vibewindow-task-board-staff-test".to_string();
    app.project_path = Some(project_path.clone());
    project_path
}

// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。
#[test]
fn toggle_staff_mode_preserves_default_kanban() {
    let project = tempfile::TempDir::new().expect("temp project");
    let (mut app, _task) = crate::app::App::new();

    assert_eq!(app.task_board_view_mode, TaskBoardViewMode::Kanban);

    app.project_path = Some(project.path().to_string_lossy().to_string());
    let mut task = Task::new(10);
    task.id = "T-staff-toggle".to_string();
    task.status = TaskStatus::Pending;
    app.task_board_tasks = vec![task];

    let _task = super::update(&mut app, super::TaskBoardMessage::ToggleStaffMode(true));
    assert_eq!(app.task_board_view_mode, TaskBoardViewMode::Staff);

    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::StaffConfigLoaded(Ok(TaskBoardStaffConfig::default())),
    );
    assert!(app.task_board_staff_config_load_error.is_none());
    assert_eq!(app.task_board_staff_projection.total_task_count, 1);

    let _task = super::update(&mut app, super::TaskBoardMessage::RefreshStaffProjection);
    assert_eq!(app.task_board_staff_projection.total_task_count, 1);

    let _task = super::update(&mut app, super::TaskBoardMessage::ToggleStaffMode(false));
    assert_eq!(app.task_board_view_mode, TaskBoardViewMode::Kanban);
    assert_eq!(app.task_board_tasks.len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn staff_mode_refresh_does_not_run_auto_staff_schedule() {
    fn auto_schedule_fixture(view_mode: TaskBoardViewMode) -> crate::app::App {
        let (mut app, _task) = crate::app::App::new();
        app.task_board_view_mode = view_mode;
        app.task_board_staff_config = TaskBoardStaffConfig::default();
        app.task_board_staff_config.stage_assignments = vec![TaskBoardStageAssignment {
            status: TaskStatus::Pending,
            primary_member_id: Some("staff-orchestrator".to_string()),
            backup_member_ids: Vec::new(),
            manager_member_ids: Vec::new(),
            autonomous: true,
        }];
        let member = app
            .task_board_staff_config
            .members
            .iter_mut()
            .find(|member| member.id == "staff-orchestrator")
            .expect("default orchestrator");
        member.current_task_ids = vec!["T-implicit-staff-auto".to_string()];
        let mut task = Task::new(10);
        task.id = "T-implicit-staff-auto".to_string();
        task.status = TaskStatus::Pending;
        app.task_board_tasks = vec![task];
        app
    }

    fn assert_no_auto_schedule_messages(label: &str, messages: &[Message]) {
        assert!(
            messages.iter().all(|message| !matches!(
                message,
                Message::TaskBoard(super::TaskBoardMessage::StaffAutoSchedulePlanLoaded { .. })
                    | Message::TaskBoard(super::TaskBoardMessage::SubmitStaffCommandSilently(_))
            )),
            "{label} emitted staff auto schedule messages: {messages:?}"
        );
    }

    let mut app = auto_schedule_fixture(TaskBoardViewMode::Kanban);
    let task = super::update(&mut app, super::TaskBoardMessage::ToggleStaffMode(true));
    let messages = collect_task_messages(task).await;
    assert_no_auto_schedule_messages("toggle staff mode", &messages);
    assert_eq!(app.task_board_staff_command_queue.pending_len(), 0);

    let mut app = auto_schedule_fixture(TaskBoardViewMode::Staff);
    let config = app.task_board_staff_config.clone();
    let task = super::update(&mut app, super::TaskBoardMessage::StaffConfigLoaded(Ok(config)));
    let messages = collect_task_messages(task).await;
    assert_no_auto_schedule_messages("staff config load", &messages);
    assert_eq!(app.task_board_staff_command_queue.pending_len(), 0);

    let mut app = auto_schedule_fixture(TaskBoardViewMode::Staff);
    let task = super::update(&mut app, super::TaskBoardMessage::RefreshStaffProjection);
    let messages = collect_task_messages(task).await;
    assert_no_auto_schedule_messages("refresh staff projection", &messages);
    assert_eq!(app.task_board_staff_command_queue.pending_len(), 0);
}

#[test]
fn staff_config_reload_paths_clear_stale_load_error() {
    let project = tempfile::TempDir::new().expect("temp project");
    let (mut app, _task) = crate::app::App::new();
    app.project_path = Some(project.path().to_string_lossy().to_string());
    app.task_board_view_mode = TaskBoardViewMode::Staff;

    app.task_board_staff_config_load_error = Some("stale".to_string());
    let _task = super::update(&mut app, super::TaskBoardMessage::ReloadStaffConfig);
    assert!(app.task_board_staff_config_load_error.is_none());

    app.task_board_staff_config_load_error = Some("stale".to_string());
    let _task = super::update(&mut app, super::TaskBoardMessage::LoadTasks);
    assert!(app.task_board_staff_config_load_error.is_none());
}

#[test]
fn staff_config_loaded_for_workflow_retargets_persisted_stage_roles() {
    let (mut app, _task) = crate::app::App::new();
    let mut config = TaskBoardStaffConfig::default();
    let planning = config
        .workflow
        .iter_mut()
        .find(|stage| stage.status == TaskStatus::Planning)
        .expect("planning stage");
    planning.role = None;
    planning.display_only = true;

    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::StaffConfigLoadedForWorkflow {
            workflow_id: "dev_lifecycle".to_string(),
            result: Ok(config),
        },
    );

    assert!(app.task_board_staff_config_load_error.is_none());
    assert_eq!(
        app.task_board_staff_config.role_for_status(TaskStatus::Planning),
        Some(TaskBoardStaffRole::Planner)
    );
    assert_eq!(app.task_board_staff_config_workflow_id.as_deref(), Some("dev_lifecycle"));
}

#[test]
fn task_board_staff_projection_uses_selected_workflow_id() {
    let (mut app, _task) = crate::app::App::new();
    app.task_board_view_mode = TaskBoardViewMode::Staff;
    app.task_board_settings.workflow_id = "marketing_campaign".to_string();

    let config =
        crate::app::task::staff::task_board_staff_config_from_staff_config_for_workflow_id(
            StaffConfig::default(),
            &app.task_board_settings.workflow_id,
        )
        .expect("staff config projects for selected workflow");
    let _task = super::update(&mut app, super::TaskBoardMessage::StaffConfigLoaded(Ok(config)));

    let lane_statuses = app
        .task_board_staff_projection
        .lanes
        .iter()
        .map(|lane| lane.status.to_string_key())
        .collect::<Vec<_>>();
    assert_eq!(
        lane_statuses,
        vec![
            "campaign_ideas",
            "campaign_queued",
            "campaign_planning",
            "campaign_running",
            "campaign_blocked",
            "campaign_paused",
            "campaign_review_ready",
            "campaign_reviewing",
            "campaign_closing",
            "campaign_done",
            "campaign_archived",
        ]
    );
}

#[test]
fn task_board_unknown_lifecycle_reports_error_without_fallback() {
    let (mut app, _task) = crate::app::App::new();
    app.task_board_view_mode = TaskBoardViewMode::Staff;
    app.task_board_settings.workflow_id = "missing_gateway_lifecycle".to_string();
    app.task_board_staff_config = TaskBoardStaffConfig::default();

    let mut task = Task::new(42);
    task.id = "T-missing-lifecycle".to_string();
    task.status = TaskStatus::Pending;
    app.task_board_tasks = vec![task];

    let _task = super::update(&mut app, super::TaskBoardMessage::RefreshStaffProjection);

    let error = app
        .task_board_staff_config_load_error
        .as_deref()
        .expect("unknown lifecycle should report load error");
    assert!(error.contains("missing_gateway_lifecycle"), "{error}");
    assert!(
        app.task_board_staff_projection.lanes.is_empty(),
        "unknown lifecycle must not render default lanes: {:?}",
        app.task_board_staff_projection
            .lanes
            .iter()
            .map(|lane| lane.status.to_string_key())
            .collect::<Vec<_>>()
    );
}

#[test]
fn staff_matrix_config_action_opens_channels_settings_on_matrix_tab() {
    let (mut app, _task) = crate::app::App::new();
    app.channels_settings.matrix = None;

    let _task = super::update(&mut app, super::TaskBoardMessage::OpenMatrixTaskPoolConfig);

    assert!(app.show_system_settings);
    assert_eq!(app.system_settings_tab, SystemTab::Channels);
    assert!(app.channels_settings.matrix.is_none());
    assert_eq!(app.channels_settings.active_tab, crate::app::state::ChannelsSettingsTab::Matrix);
    assert!(app.channels_settings.save_error.is_none());
}

#[test]
fn staff_mode_loads_and_persists_staff_config_through_gateway() {
    let mut staff_config = StaffConfig::default();
    let executor = staff_config
        .members
        .iter_mut()
        .find(|member| member.id == "staff-executor")
        .expect("default executor");
    executor.name = "Gateway Executor".to_string();
    executor.status = "working".to_string();
    executor.capacity = 5;
    executor.delegate_agent = Some("main".to_string());
    executor.acp_agent = Some("codex".to_string());
    executor.ipc_agent_id = Some("ipc-executor".to_string());
    staff_config.workflow_orchestrations.insert(
        "dev_lifecycle".to_string(),
        StaffWorkflowOrchestrationConfig {
            stage_assignments: vec![StaffStageAssignmentConfig {
                stage_id: "running".to_string(),
                primary_member_id: Some("staff-executor".to_string()),
                backup_member_ids: Vec::new(),
                manager_member_ids: Vec::new(),
                autonomous: true,
            }],
            stage_actions: vec![StaffStageActionConfig {
                stage_id: "running".to_string(),
                capability_id: SAVED_WORKFLOW_CAPABILITY_ID.to_string(),
                workflow_uuid: Some("018ff4b0-1d0d-7a2c-a30c-7bcf9ee1f010".to_string()),
                inputs: StaffStageActionInputsConfig {
                    workflow_uuid: Some("018ff4b0-1d0d-7a2c-a30c-7bcf9ee1f010".to_string()),
                    ..StaffStageActionInputsConfig::default()
                },
            }],
            ..StaffWorkflowOrchestrationConfig::default()
        },
    );
    staff_config.project_state.member_task_assignments = vec![StaffMemberTaskAssignmentConfig {
        member_id: "staff-executor".to_string(),
        task_ids: vec!["T-gateway".to_string()],
    }];

    let mut projection =
        crate::app::task::staff::task_board_staff_config_from_staff_config(staff_config.clone())
            .expect("gateway staff config projects into task board staff config");

    let projected_executor = projection
        .members
        .iter()
        .find(|member| member.id == "staff-executor")
        .expect("projected executor");
    assert_eq!(projected_executor.name, "Gateway Executor");
    assert_eq!(projected_executor.status, crate::app::task::staff::TaskBoardStaffStatus::Working);
    assert_eq!(projected_executor.capacity, 5);
    assert_eq!(projected_executor.delegate_agent.as_deref(), Some("main"));
    assert_eq!(projected_executor.acp_agent.as_deref(), Some("codex"));
    assert_eq!(projected_executor.ipc_agent_id.as_deref(), Some("ipc-executor"));
    assert_eq!(projected_executor.current_task_ids, vec!["T-gateway"]);
    assert_eq!(
        projection.stage_assignments[0],
        TaskBoardStageAssignment {
            status: TaskStatus::Running,
            primary_member_id: Some("staff-executor".to_string()),
            backup_member_ids: Vec::new(),
            manager_member_ids: Vec::new(),
            autonomous: true,
        }
    );
    assert_eq!(
        projection.stage_actions,
        vec![TaskBoardStageAction {
            status: TaskStatus::Running,
            capability_id: SAVED_WORKFLOW_CAPABILITY_ID.to_string(),
            workflow_uuid: Some("018ff4b0-1d0d-7a2c-a30c-7bcf9ee1f010".to_string()),
            ..Default::default()
        }]
    );

    projection.stage_assignments = vec![TaskBoardStageAssignment {
        status: TaskStatus::CodeReview,
        primary_member_id: Some("staff-reviewer".to_string()),
        backup_member_ids: Vec::new(),
        manager_member_ids: Vec::new(),
        autonomous: true,
    }];
    projection.stage_actions = vec![TaskBoardStageAction {
        status: TaskStatus::CodeReview,
        capability_id: SAVED_WORKFLOW_CAPABILITY_ID.to_string(),
        workflow_uuid: Some("018ff4b0-1d0d-7a2c-a30c-7bcf9ee1f011".to_string()),
        ..Default::default()
    }];
    for member in &mut projection.members {
        member.current_task_ids.clear();
    }
    let reviewer = projection
        .members
        .iter_mut()
        .find(|member| member.id == "staff-reviewer")
        .expect("projected reviewer");
    reviewer.ipc_agent_id = Some("ipc-reviewer".to_string());
    reviewer.current_task_ids.push("T-review".to_string());

    crate::app::task::staff::apply_task_board_staff_config_to_staff_config(
        &mut staff_config,
        &projection,
    )
    .expect("task board staff config writes back into gateway staff config");

    assert_eq!(staff_config.project_state.stage_assignments.len(), 1);
    assert_eq!(staff_config.project_state.stage_assignments[0].stage_id, "code_review");
    assert_eq!(
        staff_config.project_state.member_task_assignments,
        vec![StaffMemberTaskAssignmentConfig {
            member_id: "staff-reviewer".to_string(),
            task_ids: vec!["T-review".to_string()],
        }]
    );
    assert_eq!(
        staff_config.project_state.stage_actions,
        vec![StaffStageActionConfig {
            stage_id: "code_review".to_string(),
            capability_id: SAVED_WORKFLOW_CAPABILITY_ID.to_string(),
            workflow_uuid: Some("018ff4b0-1d0d-7a2c-a30c-7bcf9ee1f011".to_string()),
            inputs: StaffStageActionInputsConfig {
                workflow_uuid: Some("018ff4b0-1d0d-7a2c-a30c-7bcf9ee1f011".to_string()),
                ..StaffStageActionInputsConfig::default()
            },
        }]
    );
    assert!(staff_config.project_state.stage_workflow_bindings.is_empty());
    let orchestration =
        staff_config.workflow_orchestrations.get("dev_lifecycle").expect("workflow orchestration");
    assert!(orchestration.stage_workflow_bindings.is_empty());
    assert_eq!(orchestration.stage_actions, staff_config.project_state.stage_actions);
    let executor = staff_config
        .members
        .iter()
        .find(|member| member.id == "staff-executor")
        .expect("executor should remain configured");
    assert_eq!(executor.capacity, 5);
    assert_eq!(executor.delegate_agent.as_deref(), Some("main"));
    assert_eq!(executor.acp_agent.as_deref(), Some("codex"));
    let reviewer = staff_config
        .members
        .iter()
        .find(|member| member.id == "staff-reviewer")
        .expect("reviewer should remain configured");
    assert_eq!(reviewer.ipc_agent_id.as_deref(), Some("ipc-reviewer"));
    assert!(UPDATE_SOURCE.contains("load_staff_config_result"));
    assert!(UPDATE_SOURCE.contains("staff_workflow_orchestration"));
    assert!(UPDATE_SOURCE.contains("save_staff_workflow_orchestration"));
    assert!(!UPDATE_SOURCE.contains("load_task_board_staff_config"));
    assert!(!UPDATE_SOURCE.contains("save_task_board_staff_config"));
    assert!(!UPDATE_SOURCE.contains("_staff_config.json"));
}

#[tokio::test(flavor = "multi_thread")]
async fn desktop_staff_orchestration_uses_workflow_level_gateway_paths() {
    let _guard = super::TEST_GATEWAY_ENV_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
    let project = tempfile::TempDir::new().expect("temp project");
    let (mut app, _task) = crate::app::App::new();
    app.project_path = Some(project.path().to_string_lossy().to_string());
    app.task_board_settings.workflow_id = "marketing_campaign".to_string();
    app.task_board_staff_config = TaskBoardStaffConfig::default()
        .retarget_for_workflow_id("marketing_campaign")
        .expect("marketing staff config");
    app.task_board_staff_capability_draft = vec![TaskBoardStaffCapability::custom_saved_workflow(
        "campaign_publish".to_string(),
        "Campaign Publish".to_string(),
    )];

    let response = staff_orchestration_gateway_response_json().to_string();
    let gateway = StaffIpcGateway::start_raw(
        std::iter::repeat_with(|| (200, response.clone())).take(12).collect(),
    );
    let workflow_id = app.task_board_settings.workflow_id.clone();
    let loaded = super::load_gateway_staff_projection(
        project.path().to_string_lossy().to_string(),
        workflow_id.clone(),
    )
    .await
    .expect("load workflow staff config");
    assert_eq!(
        loaded.workflow.iter().map(|stage| stage.status.to_string_key()).collect::<Vec<_>>()[0],
        "campaign_ideas"
    );

    let campaign_status =
        TaskStatus::parse_key("campaign_queued").expect("marketing status should parse");
    let _task = super::update(&mut app, super::TaskBoardMessage::OpenStaffStageAssignment);
    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::SelectStaffStagePrimaryMember {
            status: campaign_status,
            member_id: Some("staff-executor".to_string()),
        },
    );
    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::ToggleStaffStageAutonomous {
            status: campaign_status,
            enabled: true,
        },
    );
    let save_assignment =
        super::update(&mut app, super::TaskBoardMessage::SaveStaffStageAssignment);
    let _messages = collect_task_messages(save_assignment).await;
    let mut requests = gateway.take_requests();

    let capability_gateway = StaffIpcGateway::start_matching(vec![(
        "/v1/staff/projects/proj_1/workflows/marketing_campaign/capabilities",
        200,
        json!({
            "capabilities": [release_capability_json(
                "Campaign Publish",
                "Workflow UUID",
                "summary",
            )]
        }),
    )]);
    let save_catalog = super::update(&mut app, super::TaskBoardMessage::SaveStaffCapabilityCatalog);
    let _messages = collect_task_messages(save_catalog).await;
    requests.extend(capability_gateway.take_requests());
    let paths = requests.iter().map(|request| request.path.as_str()).collect::<Vec<_>>();

    assert!(
        requests.iter().any(|request| {
            request.method == "GET"
                && request.path
                    == "/v1/staff/projects/proj_1/workflows/marketing_campaign/orchestration"
        }),
        "loading should use workflow orchestration path: {paths:?}"
    );
    assert!(
        requests.iter().any(|request| {
            request.method == "PUT"
                && request.path
                    == "/v1/staff/projects/proj_1/workflows/marketing_campaign/orchestration"
                && request.body["workflow_id"] == "marketing_campaign"
                && request.body["orchestration"]["stage_assignments"].as_array().is_some_and(
                    |assignments| {
                        assignments
                            .iter()
                            .any(|assignment| assignment["stage_id"] == "campaign_queued")
                    },
                )
        }),
        "stage assignment save should use workflow orchestration path: {requests:?}"
    );
    assert!(
        requests.iter().any(|request| {
            request.method == "PUT"
                && request.path
                    == "/v1/staff/projects/proj_1/workflows/marketing_campaign/capabilities"
                && request.body["capabilities"][0]["id"] == "campaign_publish"
        }),
        "capability catalog save should use workflow capability path: {requests:?}"
    );
    assert!(
        requests.iter().all(|request| { !request.path.starts_with("/v1/staff/workflows/") }),
        "desktop orchestration should use project-scoped paths: {paths:?}"
    );
}

#[test]
fn staff_stage_actions_projection_ignores_legacy_workflow_bindings() {
    let mut staff_config = StaffConfig::default();
    staff_config.project_state.stage_workflow_bindings = vec![StaffStageWorkflowBindingConfig {
        stage_id: "running".to_string(),
        workflow_uuid: "018ff4b0-1d0d-7a2c-a30c-7bcf9ee1f021".to_string(),
    }];

    let mut projection =
        crate::app::task::staff::task_board_staff_config_from_staff_config(staff_config.clone())
            .expect("staff config projects into task board staff config");
    assert!(!projection.stage_actions.is_empty());
    assert!(projection.stage_workflow_bindings.is_empty());
    assert!(
        projection.stage_actions.iter().all(|action| {
            action.workflow_uuid.as_deref() != Some("018ff4b0-1d0d-7a2c-a30c-7bcf9ee1f021")
        }),
        "legacy workflow bindings should not become stage actions"
    );

    let marketing_projection =
        crate::app::task::staff::task_board_staff_config_from_staff_config_for_workflow_id(
            StaffConfig::default(),
            "marketing_campaign",
        )
        .expect("marketing workflow projects staff config");
    assert!(
        marketing_projection.stage_actions.iter().all(|action| {
            action.capability_id != CODE_REVIEW_CAPABILITY_ID
                && action.capability_id != GIT_MERGE_CAPABILITY_ID
        }),
        "business workflows should not default code stage actions"
    );

    projection.stage_actions = vec![
        TaskBoardStageAction {
            status: TaskStatus::CodeReview,
            capability_id: SAVED_WORKFLOW_CAPABILITY_ID.to_string(),
            workflow_uuid: Some("018ff4b0-1d0d-7a2c-a30c-7bcf9ee1f022".to_string()),
            ..Default::default()
        },
        TaskBoardStageAction {
            status: TaskStatus::PrSubmitted,
            capability_id: GIT_MERGE_CAPABILITY_ID.to_string(),
            workflow_uuid: None,
            ..Default::default()
        },
    ];
    projection.stage_workflow_bindings.clear();
    crate::app::task::staff::apply_task_board_staff_config_to_staff_config(
        &mut staff_config,
        &projection,
    )
    .expect("stage actions write back into gateway staff config");
    assert_eq!(
        staff_config.project_state.stage_actions,
        vec![
            StaffStageActionConfig {
                stage_id: "code_review".to_string(),
                capability_id: SAVED_WORKFLOW_CAPABILITY_ID.to_string(),
                workflow_uuid: Some("018ff4b0-1d0d-7a2c-a30c-7bcf9ee1f022".to_string()),
                inputs: StaffStageActionInputsConfig {
                    workflow_uuid: Some("018ff4b0-1d0d-7a2c-a30c-7bcf9ee1f022".to_string()),
                    ..StaffStageActionInputsConfig::default()
                },
            },
            StaffStageActionConfig {
                stage_id: "pr_submitted".to_string(),
                capability_id: GIT_MERGE_CAPABILITY_ID.to_string(),
                workflow_uuid: None,
                inputs: StaffStageActionInputsConfig::default(),
            },
        ]
    );
    assert!(staff_config.project_state.stage_workflow_bindings.is_empty());
    let orchestration =
        staff_config.workflow_orchestrations.get("dev_lifecycle").expect("workflow orchestration");
    assert!(orchestration.stage_workflow_bindings.is_empty());
    assert_eq!(orchestration.stage_actions, staff_config.project_state.stage_actions);

    let mut unknown_capability = TaskBoardStaffConfig::default();
    unknown_capability.stage_actions = vec![TaskBoardStageAction {
        status: TaskStatus::Running,
        capability_id: "missing".to_string(),
        workflow_uuid: None,
        ..Default::default()
    }];
    unknown_capability
        .normalize()
        .expect("project catalog capability ids pass desktop normalization");

    let mut built_in_with_uuid = TaskBoardStaffConfig::default();
    built_in_with_uuid.stage_actions = vec![TaskBoardStageAction {
        status: TaskStatus::CodeReview,
        capability_id: CODE_REVIEW_CAPABILITY_ID.to_string(),
        workflow_uuid: Some("018ff4b0-1d0d-7a2c-a30c-7bcf9ee1f023".to_string()),
        ..Default::default()
    }];
    assert!(
        built_in_with_uuid
            .normalize()
            .expect_err("built-in stage action rejects workflow uuid")
            .contains("task board stage action workflow_uuid for code_review must be empty")
    );
}

#[test]
fn staff_stage_actions_reset_requires_confirmation_and_restores_dev_defaults() {
    let (mut app, _task) = crate::app::App::new();
    app.task_board_staff_config.stage_actions = vec![
        TaskBoardStageAction {
            status: TaskStatus::CodeComplete,
            capability_id: ADVANCE_STATUS_CAPABILITY_ID.to_string(),
            workflow_uuid: None,
            ..Default::default()
        },
        TaskBoardStageAction {
            status: TaskStatus::PrSubmitted,
            capability_id: GIT_MERGE_CAPABILITY_ID.to_string(),
            workflow_uuid: None,
            ..Default::default()
        },
    ];
    app.task_board_staff_config.stage_workflow_bindings = vec![TaskBoardStageWorkflowBinding {
        status: TaskStatus::CodeReview,
        workflow_uuid: "018ff4b0-1d0d-7a2c-a30c-7bcf9ee1f050".to_string(),
    }];

    let _task = super::update(&mut app, super::TaskBoardMessage::OpenStaffStageAssignment);
    assert_eq!(
        stage_action_capability_in_draft(&app, TaskStatus::CodeComplete).as_deref(),
        Some(ADVANCE_STATUS_CAPABILITY_ID)
    );

    let _task = super::update(&mut app, super::TaskBoardMessage::RequestResetStaffStageActions);
    assert!(app.task_board_staff_stage_action_reset_confirm);
    assert_eq!(
        stage_action_capability_in_draft(&app, TaskStatus::CodeComplete).as_deref(),
        Some(ADVANCE_STATUS_CAPABILITY_ID)
    );

    let _task = super::update(&mut app, super::TaskBoardMessage::CancelResetStaffStageActions);
    assert!(!app.task_board_staff_stage_action_reset_confirm);
    assert_eq!(
        stage_action_capability_in_draft(&app, TaskStatus::CodeComplete).as_deref(),
        Some(ADVANCE_STATUS_CAPABILITY_ID)
    );

    let _task = super::update(&mut app, super::TaskBoardMessage::RequestResetStaffStageActions);
    let _task = super::update(&mut app, super::TaskBoardMessage::ConfirmResetStaffStageActions);
    assert!(!app.task_board_staff_stage_action_reset_confirm);
    assert_eq!(
        stage_action_capability_in_draft(&app, TaskStatus::CodeComplete).as_deref(),
        Some(ADVANCE_STATUS_CAPABILITY_ID)
    );
    assert_eq!(
        stage_action_capability_in_draft(&app, TaskStatus::CodeReview).as_deref(),
        Some(CODE_REVIEW_CAPABILITY_ID)
    );
    assert_eq!(
        stage_action_capability_in_draft(&app, TaskStatus::PrSubmitted).as_deref(),
        Some(GIT_MERGE_CAPABILITY_ID)
    );
    assert!(app.task_board_staff_workflow_binding_draft.as_ref().is_some_and(Vec::is_empty));
    assert_eq!(
        stage_assignment_in_draft(&app, TaskStatus::Pool)
            .and_then(|assignment| { assignment.primary_member_id.as_deref() }),
        Some("staff-orchestrator")
    );
    assert!(
        stage_assignment_in_draft(&app, TaskStatus::CodeComplete)
            .is_some_and(|assignment| assignment.autonomous)
    );
    assert_eq!(
        stage_assignment_in_draft(&app, TaskStatus::CodeReview)
            .and_then(|assignment| { assignment.primary_member_id.as_deref() }),
        Some("staff-reviewer")
    );
    assert_eq!(
        stage_assignment_in_draft(&app, TaskStatus::PrSubmitted)
            .and_then(|assignment| { assignment.primary_member_id.as_deref() }),
        Some("staff-merger")
    );
}

#[test]
fn staff_stage_actions_reset_without_save_reopens_saved_config() {
    let (mut app, _task) = crate::app::App::new();
    app.task_board_staff_config.stage_actions = vec![TaskBoardStageAction {
        status: TaskStatus::Running,
        capability_id: SAVED_WORKFLOW_CAPABILITY_ID.to_string(),
        workflow_uuid: Some("018ff4b0-1d0d-7a2c-a30c-7bcf9ee1f051".to_string()),
        ..Default::default()
    }];

    let _task = super::update(&mut app, super::TaskBoardMessage::OpenStaffStageAssignment);
    let _task = super::update(&mut app, super::TaskBoardMessage::RequestResetStaffStageActions);
    let _task = super::update(&mut app, super::TaskBoardMessage::ConfirmResetStaffStageActions);
    assert_eq!(
        stage_action_capability_in_draft(&app, TaskStatus::CodeReview).as_deref(),
        Some(CODE_REVIEW_CAPABILITY_ID)
    );

    let _task = super::update(&mut app, super::TaskBoardMessage::CloseStaffStageAssignment);
    let _task = super::update(&mut app, super::TaskBoardMessage::OpenStaffStageAssignment);

    assert_eq!(
        stage_action_capability_in_draft(&app, TaskStatus::Running).as_deref(),
        Some(SAVED_WORKFLOW_CAPABILITY_ID)
    );
    assert!(stage_action_capability_in_draft(&app, TaskStatus::CodeReview).is_none());
}

#[test]
fn staff_stage_actions_reset_restores_business_builtin_defaults() {
    assert_business_builtin_reset_defaults(
        "marketing_campaign",
        "campaign_ideas",
        "campaign_queued",
        "campaign_planning",
        "campaign_running",
        "campaign_review_ready",
        "campaign_reviewing",
        "campaign_closing",
    );
    assert_business_builtin_reset_defaults(
        "recruiting_pipeline",
        "recruiting_pool",
        "recruiting_queued",
        "recruiting_screening",
        "recruiting_interviewing",
        "recruiting_review_ready",
        "recruiting_reviewing",
        "recruiting_offer",
    );
}

fn assert_business_builtin_reset_defaults(
    workflow_id: &str,
    backlog_key: &str,
    queued_key: &str,
    planning_key: &str,
    executing_key: &str,
    review_ready_key: &str,
    review_key: &str,
    merging_key: &str,
) {
    let (mut app, _task) = crate::app::App::new();
    let backlog = TaskStatus::parse_key(backlog_key).expect("backlog status");
    let queued = TaskStatus::parse_key(queued_key).expect("queued status");
    let planning = TaskStatus::parse_key(planning_key).expect("planning status");
    let executing = TaskStatus::parse_key(executing_key).expect("executing status");
    let review_ready = TaskStatus::parse_key(review_ready_key).expect("review ready status");
    let review = TaskStatus::parse_key(review_key).expect("review status");
    let merging = TaskStatus::parse_key(merging_key).expect("merging status");
    app.task_board_settings.workflow_id = workflow_id.to_string();
    app.task_board_staff_config =
        crate::app::task::staff::task_board_staff_config_from_staff_config_for_workflow_id(
            StaffConfig::default(),
            workflow_id,
        )
        .expect("business staff config");
    app.task_board_staff_config.stage_actions = vec![TaskBoardStageAction {
        status: review,
        capability_id: CODE_REVIEW_CAPABILITY_ID.to_string(),
        workflow_uuid: None,
        ..Default::default()
    }];
    app.task_board_staff_config.stage_assignments.clear();

    let _task = super::update(&mut app, super::TaskBoardMessage::OpenStaffStageAssignment);
    let _task = super::update(&mut app, super::TaskBoardMessage::RequestResetStaffStageActions);
    let _task = super::update(&mut app, super::TaskBoardMessage::ConfirmResetStaffStageActions);

    assert_eq!(
        stage_action_capability_in_draft(&app, backlog).as_deref(),
        Some(ADVANCE_STATUS_CAPABILITY_ID)
    );
    assert_eq!(
        stage_action_capability_in_draft(&app, queued).as_deref(),
        Some(EXECUTE_TASK_CAPABILITY_ID)
    );
    assert_eq!(
        stage_action_capability_in_draft(&app, planning).as_deref(),
        Some(EXECUTE_TASK_CAPABILITY_ID)
    );
    assert!(stage_action_capability_in_draft(&app, executing).is_none());
    assert_eq!(
        stage_action_capability_in_draft(&app, review_ready).as_deref(),
        Some(ADVANCE_STATUS_CAPABILITY_ID)
    );
    assert_eq!(
        stage_action_capability_in_draft(&app, review).as_deref(),
        Some(ADVANCE_STATUS_CAPABILITY_ID)
    );
    assert_eq!(
        stage_action_capability_in_draft(&app, merging).as_deref(),
        Some(ADVANCE_STATUS_CAPABILITY_ID)
    );
    assert_eq!(
        stage_assignment_in_draft(&app, backlog)
            .and_then(|assignment| { assignment.primary_member_id.as_deref() }),
        Some("staff-orchestrator")
    );
    assert_eq!(
        stage_assignment_in_draft(&app, review)
            .and_then(|assignment| { assignment.primary_member_id.as_deref() }),
        Some("staff-reviewer")
    );
    assert_eq!(
        stage_assignment_in_draft(&app, merging)
            .and_then(|assignment| { assignment.primary_member_id.as_deref() }),
        Some("staff-merger")
    );
    assert!(
        stage_assignment_in_draft(&app, review).is_some_and(|assignment| assignment.autonomous)
    );
    assert!(
        stage_assignment_in_draft(&app, merging).is_some_and(|assignment| assignment.autonomous)
    );
}

fn stage_action_capability_in_draft(app: &crate::app::App, status: TaskStatus) -> Option<String> {
    app.task_board_staff_stage_action_draft
        .as_ref()
        .and_then(|actions| actions.iter().find(|action| action.status == status))
        .map(|action| action.capability_id.clone())
}

fn stage_assignment_in_draft(
    app: &crate::app::App,
    status: TaskStatus,
) -> Option<&TaskBoardStageAssignment> {
    app.task_board_staff_assignment_draft
        .as_ref()
        .and_then(|assignments| assignments.iter().find(|assignment| assignment.status == status))
}

#[tokio::test(flavor = "multi_thread")]
async fn desktop_staff_capability_catalog_loads_and_saves_draft() {
    let _guard = super::TEST_GATEWAY_ENV_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
    let (mut app, _task) = crate::app::App::new();
    let project_path = ensure_task_board_project_path(&mut app);
    app.task_board_view_mode = TaskBoardViewMode::Staff;

    let gateway = StaffIpcGateway::start_matching(vec![
        (
            "/v1/staff/projects/proj_1/workflows/dev_lifecycle/capabilities",
            200,
            json!({
                "capabilities": [
                    manual_capability_json(),
                    release_capability_json("Release Workflow", "Workflow UUID", "summary")
                ]
            }),
        ),
        (
            "/v1/staff/projects/proj_1/workflows/dev_lifecycle/capabilities",
            500,
            json!({"error": "save rejected"}),
        ),
        (
            "/v1/staff/projects/proj_1/workflows/dev_lifecycle/capabilities",
            200,
            json!({
                "capabilities": [
                    manual_capability_json(),
                    release_capability_json(
                        "Release Automation",
                        "Workflow UUID and release channel",
                        "summary, deployed_url"
                    )
                ]
            }),
        ),
        (
            "/v1/config",
            200,
            serde_json::to_value(vw_config_types::config::Config::default())
                .expect("default config json"),
        ),
        (
            "/v1/staff/projects/proj_1/workflows/dev_lifecycle/orchestration",
            200,
            json!({
                "workflow_id": "dev_lifecycle",
                "orchestration": {}
            }),
        ),
    ]);

    let load_task = super::update(&mut app, super::TaskBoardMessage::LoadStaffCapabilityCatalog);
    apply_task_board_messages(&mut app, collect_task_messages(load_task).await);

    assert!(!app.task_board_staff_capability_catalog_loading);
    assert!(app.task_board_staff_capability_catalog_error.is_none());
    assert_eq!(app.task_board_staff_capability_catalog.len(), 2);
    assert_eq!(app.task_board_staff_capability_draft.len(), 2);
    assert!(
        app.task_board_staff_capability_catalog
            .iter()
            .any(|capability| capability.id == "manual" && capability.built_in)
    );

    let built_in_label = app
        .task_board_staff_capability_draft
        .iter()
        .find(|capability| capability.id == "manual")
        .expect("manual capability")
        .label
        .clone();
    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::UpdateStaffCapabilityDraft {
            capability_id: "manual".to_string(),
            field: TaskBoardStaffCapabilityDraftField::Label,
            value: "Should Stay Read Only".to_string(),
        },
    );
    assert_eq!(
        app.task_board_staff_capability_draft
            .iter()
            .find(|capability| capability.id == "manual")
            .expect("manual capability remains")
            .label,
        built_in_label
    );

    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::UpdateStaffCapabilityDraft {
            capability_id: "release_workflow".to_string(),
            field: TaskBoardStaffCapabilityDraftField::Label,
            value: "Release Automation".to_string(),
        },
    );
    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::UpdateStaffCapabilitySummaryDraft {
            capability_id: "release_workflow".to_string(),
            summary: TaskBoardStaffCapabilitySummaryKind::Input,
            description: "Workflow UUID and release channel".to_string(),
            fields: vec!["workflow_uuid".to_string(), "release_channel".to_string()],
        },
    );
    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::UpdateStaffCapabilitySummaryDraft {
            capability_id: "release_workflow".to_string(),
            summary: TaskBoardStaffCapabilitySummaryKind::Result,
            description: "Release automation result".to_string(),
            fields: vec!["summary".to_string(), "deployed_url".to_string()],
        },
    );
    let _task = super::update(&mut app, super::TaskBoardMessage::AddStaffCapabilityDraft);
    assert!(
        app.task_board_staff_capability_draft
            .iter()
            .any(|capability| !capability.built_in
                && capability.id.starts_with("custom_capability_"))
    );
    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::DeleteStaffCapabilityDraft { capability_id: "manual".to_string() },
    );
    assert!(
        app.task_board_staff_capability_draft.iter().any(|capability| capability.id == "manual")
    );

    let failed_save = super::update(&mut app, super::TaskBoardMessage::SaveStaffCapabilityCatalog);
    apply_task_board_messages(&mut app, collect_task_messages(failed_save).await);
    assert!(!app.task_board_staff_capability_saving);
    assert!(
        app.task_board_staff_capability_catalog_error
            .as_deref()
            .is_some_and(|error| error.contains("save rejected"))
    );
    assert!(
        app.task_board_staff_capability_draft
            .iter()
            .any(|capability| capability.id == "release_workflow"
                && capability.label == "Release Automation")
    );
    let toast = app.active_toast.as_ref().expect("failure should show a toast");
    assert_eq!(toast.kind, crate::app::state::ToastKind::Error);

    app.active_toast = None;
    let saved = super::update(&mut app, super::TaskBoardMessage::SaveStaffCapabilityCatalog);
    apply_task_board_messages_with_followups(&mut app, collect_task_messages(saved).await).await;

    assert!(!app.task_board_staff_capability_saving);
    assert!(app.task_board_staff_capability_catalog_error.is_none());
    assert!(
        app.task_board_staff_capability_catalog
            .iter()
            .any(|capability| capability.id == "release_workflow"
                && capability.label == "Release Automation"
                && capability.result_summary.fields.contains(&"deployed_url".to_string()))
    );
    assert_eq!(
        app.active_toast.as_ref().map(|toast| toast.kind),
        Some(crate::app::state::ToastKind::Success)
    );

    let requests = gateway.take_requests();
    let capability_requests = requests
        .iter()
        .filter(|request| {
            request.path == "/v1/staff/projects/proj_1/workflows/dev_lifecycle/capabilities"
        })
        .collect::<Vec<_>>();
    assert_eq!(capability_requests.len(), 3, "requests: {requests:?}");
    assert_eq!(capability_requests[0].method, "GET");
    assert_eq!(capability_requests[1].method, "PUT");
    assert_eq!(capability_requests[2].method, "PUT");
    assert_eq!(capability_requests[1].body["capabilities"][0]["id"], "release_workflow");
    assert_eq!(
        capability_requests[1].body["capabilities"][0]["input_summary"]["fields"],
        json!(["workflow_uuid", "release_channel"])
    );
    assert_eq!(
        capability_requests[1].body["capabilities"][0]["result_summary"]["fields"],
        json!(["summary", "deployed_url"])
    );
    assert!(
        requests.iter().any(|request| request.path == "/v1/config" && request.method == "GET"),
        "successful save should refresh staff projection: {requests:?}"
    );
    assert!(
        requests.iter().any(|request| {
            request.path == "/v1/staff/projects/proj_1/workflows/dev_lifecycle/orchestration"
                && request.method == "GET"
        }),
        "successful save should refresh workflow orchestration: {requests:?}"
    );
    assert_eq!(app.project_path.as_deref(), Some(project_path.as_str()));
}

fn manual_capability_json() -> Value {
    json!({
        "capability_id": "manual",
        "label": "人工处理",
        "type": "built_in",
        "built_in": true,
        "supported_roles": ["queued"],
        "requires_workflow_uuid": false,
        "input_summary": {
            "description": "Task context",
            "fields": ["task_id"]
        },
        "result_summary": {
            "description": "Manual result",
            "fields": ["note"]
        },
        "input_contract": "Task context",
        "result_semantics": "Manual result",
        "permission_hint": "none"
    })
}

fn release_capability_json(label: &str, input_description: &str, result_fields: &str) -> Value {
    let input_fields = if input_description.contains("release channel") {
        vec!["workflow_uuid", "release_channel"]
    } else {
        vec!["workflow_uuid"]
    };
    let result_fields = result_fields
        .split(',')
        .map(str::trim)
        .filter(|field| !field.is_empty())
        .collect::<Vec<_>>();

    json!({
        "capability_id": "release_workflow",
        "label": label,
        "type": "saved_workflow",
        "built_in": false,
        "supported_roles": ["queued"],
        "requires_workflow_uuid": true,
        "input_summary": {
            "description": input_description,
            "fields": input_fields
        },
        "result_summary": {
            "description": "Release automation result",
            "fields": result_fields
        },
        "input_contract": input_description,
        "result_semantics": "Release automation result",
        "permission_hint": "Use scoped workflow access"
    })
}

#[test]
fn stage_action_start_dev_review_runs_code_review() {
    let (mut app, _task) = crate::app::App::new();
    app.task_board_staff_config = TaskBoardStaffConfig::default();
    app.task_board_staff_config.stage_actions = vec![TaskBoardStageAction {
        status: TaskStatus::CodeReview,
        capability_id: CODE_REVIEW_CAPABILITY_ID.to_string(),
        workflow_uuid: None,
        ..Default::default()
    }];

    let mut task = Task::new(10);
    task.id = "T-dev-review".to_string();
    task.status = TaskStatus::CodeReview;
    app.task_board_tasks = vec![task];

    let command = TaskBoardStaffCommand::StartTask {
        command_id: "cmd-dev-review".to_string(),
        task_id: "T-dev-review".to_string(),
        member_id: "staff-reviewer".to_string(),
    };
    let action = super::staff_command_action(&app, &command).expect("dev review action");

    assert_eq!(
        action,
        TaskBoardStaffCommandAction::RunCodeReview { task_id: "T-dev-review".to_string() }
    );
}

#[test]
fn stage_action_start_dev_merge_runs_git_merge() {
    let (mut app, _task) = crate::app::App::new();
    app.task_board_staff_config = TaskBoardStaffConfig::default();
    app.task_board_staff_config.stage_actions = vec![TaskBoardStageAction {
        status: TaskStatus::PrSubmitted,
        capability_id: GIT_MERGE_CAPABILITY_ID.to_string(),
        workflow_uuid: None,
        ..Default::default()
    }];

    let mut task = Task::new(10);
    task.id = "T-dev-merge".to_string();
    task.status = TaskStatus::PrSubmitted;
    app.task_board_tasks = vec![task];

    let command = TaskBoardStaffCommand::StartTask {
        command_id: "cmd-dev-merge".to_string(),
        task_id: "T-dev-merge".to_string(),
        member_id: "staff-merger".to_string(),
    };
    let action = super::staff_command_action(&app, &command).expect("dev merge action");

    assert_eq!(
        action,
        TaskBoardStaffCommandAction::RunMerge { task_id: "T-dev-merge".to_string() }
    );
}

// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。
#[test]
fn stage_action_start_recruiting_offer_default_advances_status() {
    let (mut app, _task) = crate::app::App::new();
    let offer = TaskStatus::parse_key("recruiting_offer").expect("recruiting offer status");
    let hired = TaskStatus::parse_key("recruiting_hired").expect("recruiting hired status");
    app.task_board_settings.workflow_id = "recruiting_pipeline".to_string();
    app.task_board_staff_config =
        crate::app::task::staff::task_board_staff_config_from_staff_config_for_workflow_id(
            StaffConfig::default(),
            &app.task_board_settings.workflow_id,
        )
        .expect("recruiting staff config");

    let mut task = Task::new(10);
    task.id = "T-recruiting-offer".to_string();
    task.status = offer;
    app.task_board_tasks = vec![task];

    let command = TaskBoardStaffCommand::StartTask {
        command_id: "cmd-recruiting-offer".to_string(),
        task_id: "T-recruiting-offer".to_string(),
        member_id: "staff-merger".to_string(),
    };
    let action =
        super::staff_command_action(&app, &command).expect("recruiting offer should advance");

    assert_eq!(
        action,
        TaskBoardStaffCommandAction::ChangeTaskStatus {
            task_id: "T-recruiting-offer".to_string(),
            new_status: hired,
        }
    );
    assert!(
        app.task_board_manual_merge_queue.is_empty(),
        "recruiting offer must not enqueue Git merge by default"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn stage_action_start_recruiting_offer_with_workflow_runs_saved_workflow() {
    let _guard = super::TEST_GATEWAY_ENV_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
    let (mut app, _task) = crate::app::App::new();
    let offer = TaskStatus::parse_key("recruiting_offer").expect("recruiting offer status");
    let hired = TaskStatus::parse_key("recruiting_hired").expect("recruiting hired status");
    let workflow_uuid = "018ff4b0-1d0d-7a2c-a30c-7bcf9ee1f100";
    app.task_board_view_mode = TaskBoardViewMode::Staff;
    app.task_board_settings.workflow_id = "recruiting_pipeline".to_string();
    let _project_path = ensure_task_board_project_path(&mut app);
    app.task_board_staff_config =
        crate::app::task::staff::task_board_staff_config_from_staff_config_for_workflow_id(
            StaffConfig::default(),
            &app.task_board_settings.workflow_id,
        )
        .expect("recruiting staff config");
    app.task_board_staff_config.stage_actions = vec![TaskBoardStageAction {
        status: offer,
        capability_id: SAVED_WORKFLOW_CAPABILITY_ID.to_string(),
        workflow_uuid: Some(workflow_uuid.to_string()),
        ..Default::default()
    }];

    let mut task = Task::new(10);
    task.id = "T-recruiting-offer".to_string();
    task.status = offer;
    task.description = "Candidate accepted compensation band".to_string();
    task.prompt = "private candidate packet secret-token diff --git worktree".to_string();
    app.task_board_tasks = vec![task];

    let command = TaskBoardStaffCommand::StartTask {
        command_id: "cmd-recruiting-workflow".to_string(),
        task_id: "T-recruiting-offer".to_string(),
        member_id: "staff-merger".to_string(),
    };
    let action = super::staff_command_action(&app, &command).expect("saved workflow action");
    assert_eq!(
        action,
        TaskBoardStaffCommandAction::RunStageWorkflow {
            task_id: "T-recruiting-offer".to_string(),
            status: offer,
            workflow_uuid: workflow_uuid.to_string(),
            member_id: "staff-merger".to_string(),
        }
    );

    let gateway = StaffIpcGateway::start_raw(vec![(
        200,
        [
            sse(json!({
                "event": "workflow_started",
                "task_id": "run-recruiting-1",
                "workflow_run_id": "run-recruiting-1",
                "data": {"id": "run-recruiting-1"}
            })),
            sse(json!({
                "event": "message",
                "id": "run-recruiting-1",
                "task_id": "run-recruiting-1",
                "message_id": "msg-recruiting-1",
                "conversation_id": "conv-recruiting-1",
                "answer": "Offer 已确认，进入入职阶段",
                "created_at": 10
            })),
            sse(json!({
                "event": "workflow_finished",
                "task_id": "run-recruiting-1",
                "workflow_run_id": "run-recruiting-1",
                "data": {"status": "succeeded"}
            })),
        ]
        .join(""),
    )]);
    let (_message, tasks) =
        super::apply_staff_command_action(&mut app, action).expect("apply action");
    assert_eq!(tasks.len(), 1);
    let messages = collect_task_messages(iced::Task::batch(tasks)).await;
    apply_task_board_messages(&mut app, messages);
    let requests = gateway.take_requests();

    let task = app
        .task_board_tasks
        .iter()
        .find(|task| task.id == "T-recruiting-offer")
        .expect("task remains");
    assert_eq!(task.status, hired);
    assert!(
        task.logs
            .iter()
            .any(|entry| { entry.status_from == Some(offer) && entry.status_to == Some(hired) })
    );

    assert_eq!(requests.len(), 1, "expected one workflow gateway request: {requests:?}");
    let request = &requests[0];
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, format!("/v1/workflow/applications/{workflow_uuid}/chat-messages"));
    assert_eq!(request.body["inputs"]["task_id"], "T-recruiting-offer");
    assert_eq!(request.body["inputs"]["status_key"], "recruiting_offer");
    assert_eq!(request.body["inputs"]["staff_member_id"], "staff-merger");
    assert_eq!(request.body["user"], "staff-merger");
    let request_body = request.body.to_string();
    assert!(request_body.contains("Candidate accepted compensation band"));
    assert!(!request_body.contains("secret-token"));
    assert!(!request_body.contains("diff --git"));
    assert!(!request_body.contains("worktree"));
}

#[test]
fn staff_stage_assignment_persists_workflow_bindings() {
    let (mut app, _task) = crate::app::App::new();
    app.task_board_staff_config = TaskBoardStaffConfig::default();
    app.task_board_staff_config.stage_workflow_bindings = vec![TaskBoardStageWorkflowBinding {
        status: TaskStatus::Running,
        workflow_uuid: "018ff4b0-1d0d-7a2c-a30c-7bcf9ee1f001".to_string(),
    }];

    let _task = super::update(&mut app, super::TaskBoardMessage::OpenStaffStageAssignment);
    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::StaffStageWorkflowOptionsLoaded(Ok(vec![
            vw_gateway_client::WorkflowRecordSummary {
                uuid: "018ff4b0-1d0d-7a2c-a30c-7bcf9ee1f003".to_string(),
                name: "Code review handoff".to_string(),
                description: String::new(),
                created_at_ms: 1,
                updated_at_ms: 1,
            },
        ])),
    );
    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::SelectStaffStageWorkflowBinding {
            status: TaskStatus::Running,
            workflow_uuid: None,
        },
    );
    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::SelectStaffStageWorkflowBinding {
            status: TaskStatus::CodeReview,
            workflow_uuid: Some("018ff4b0-1d0d-7a2c-a30c-7bcf9ee1f003".to_string()),
        },
    );
    let _task = super::update(&mut app, super::TaskBoardMessage::SaveStaffStageAssignment);

    assert_eq!(
        app.task_board_staff_config.stage_workflow_bindings,
        vec![TaskBoardStageWorkflowBinding {
            status: TaskStatus::CodeReview,
            workflow_uuid: "018ff4b0-1d0d-7a2c-a30c-7bcf9ee1f003".to_string(),
        }]
    );
    assert!(!app.task_board_staff_assignment_open);
    assert!(app.task_board_staff_assignment_draft.is_none());
    assert!(app.task_board_staff_workflow_binding_draft.is_none());

    let mut staff_config = StaffConfig::default();
    crate::app::task::staff::apply_task_board_staff_config_to_staff_config(
        &mut staff_config,
        &app.task_board_staff_config,
    )
    .expect("workflow binding writes into staff project state");
    assert_eq!(
        staff_config.project_state.stage_actions,
        vec![StaffStageActionConfig {
            stage_id: "code_review".to_string(),
            capability_id: SAVED_WORKFLOW_CAPABILITY_ID.to_string(),
            workflow_uuid: Some("018ff4b0-1d0d-7a2c-a30c-7bcf9ee1f003".to_string()),
            inputs: StaffStageActionInputsConfig {
                workflow_uuid: Some("018ff4b0-1d0d-7a2c-a30c-7bcf9ee1f003".to_string()),
                ..StaffStageActionInputsConfig::default()
            },
        }]
    );
    assert!(staff_config.project_state.stage_workflow_bindings.is_empty());
}

#[test]
fn stage_action_assignment_persists_builtin_and_workflow_actions() {
    let (mut app, _task) = crate::app::App::new();
    app.task_board_staff_config = TaskBoardStaffConfig::default();
    app.task_board_staff_config.stage_actions = vec![TaskBoardStageAction {
        status: TaskStatus::CodeReview,
        capability_id: SAVED_WORKFLOW_CAPABILITY_ID.to_string(),
        workflow_uuid: Some("018ff4b0-1d0d-7a2c-a30c-7bcf9ee1f030".to_string()),
        ..Default::default()
    }];
    app.task_board_staff_config.stage_workflow_bindings = vec![TaskBoardStageWorkflowBinding {
        status: TaskStatus::CodeReview,
        workflow_uuid: "018ff4b0-1d0d-7a2c-a30c-7bcf9ee1f030".to_string(),
    }];

    let _task = super::update(&mut app, super::TaskBoardMessage::OpenStaffStageAssignment);
    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::SelectStaffStageActionCapability {
            status: TaskStatus::Running,
            capability_id: Some(SAVED_WORKFLOW_CAPABILITY_ID.to_string()),
        },
    );
    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::SelectStaffStageActionWorkflow {
            status: TaskStatus::Running,
            workflow_uuid: Some("018ff4b0-1d0d-7a2c-a30c-7bcf9ee1f031".to_string()),
        },
    );
    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::SelectStaffStageActionCapability {
            status: TaskStatus::CodeReview,
            capability_id: Some(CODE_REVIEW_CAPABILITY_ID.to_string()),
        },
    );
    let _task = super::update(&mut app, super::TaskBoardMessage::SaveStaffStageAssignment);

    assert_eq!(
        app.task_board_staff_config.stage_actions,
        vec![
            TaskBoardStageAction {
                status: TaskStatus::Running,
                capability_id: SAVED_WORKFLOW_CAPABILITY_ID.to_string(),
                workflow_uuid: Some("018ff4b0-1d0d-7a2c-a30c-7bcf9ee1f031".to_string()),
                ..Default::default()
            },
            TaskBoardStageAction {
                status: TaskStatus::CodeReview,
                capability_id: CODE_REVIEW_CAPABILITY_ID.to_string(),
                workflow_uuid: None,
                ..Default::default()
            },
        ]
    );
    assert_eq!(
        app.task_board_staff_config.stage_workflow_bindings,
        vec![TaskBoardStageWorkflowBinding {
            status: TaskStatus::Running,
            workflow_uuid: "018ff4b0-1d0d-7a2c-a30c-7bcf9ee1f031".to_string(),
        }]
    );
    assert!(!app.task_board_staff_assignment_open);
    assert!(app.task_board_staff_assignment_draft.is_none());
    assert!(app.task_board_staff_stage_action_draft.is_none());
    assert!(app.task_board_staff_workflow_binding_draft.is_none());

    let mut staff_config = StaffConfig::default();
    crate::app::task::staff::apply_task_board_staff_config_to_staff_config(
        &mut staff_config,
        &app.task_board_staff_config,
    )
    .expect("stage actions write into staff project state");
    assert_eq!(
        staff_config.project_state.stage_actions,
        vec![
            StaffStageActionConfig {
                stage_id: "running".to_string(),
                capability_id: SAVED_WORKFLOW_CAPABILITY_ID.to_string(),
                workflow_uuid: Some("018ff4b0-1d0d-7a2c-a30c-7bcf9ee1f031".to_string()),
                inputs: StaffStageActionInputsConfig {
                    workflow_uuid: Some("018ff4b0-1d0d-7a2c-a30c-7bcf9ee1f031".to_string()),
                    ..StaffStageActionInputsConfig::default()
                },
            },
            StaffStageActionConfig {
                stage_id: "code_review".to_string(),
                capability_id: CODE_REVIEW_CAPABILITY_ID.to_string(),
                workflow_uuid: None,
                inputs: StaffStageActionInputsConfig::default(),
            },
        ]
    );
    assert_eq!(
        staff_config
            .workflow_orchestrations
            .get("dev_lifecycle")
            .expect("workflow orchestration")
            .stage_actions,
        staff_config.project_state.stage_actions
    );
    assert!(staff_config.project_state.stage_workflow_bindings.is_empty());
}

#[test]
fn stage_assignment_edits_capability_inputs() {
    let (mut app, _task) = crate::app::App::new();
    let campaign_queued = TaskStatus::parse_key("campaign_queued").expect("campaign queued status");
    let campaign_planning =
        TaskStatus::parse_key("campaign_planning").expect("campaign planning status");
    let workflow_uuid = "018ff4b0-1d0d-7a2c-a30c-7bcf9ee1f040";
    app.task_board_settings.workflow_id = "marketing_campaign".to_string();
    app.task_board_staff_config =
        crate::app::task::staff::task_board_staff_config_from_staff_config_for_workflow_id(
            StaffConfig::default(),
            &app.task_board_settings.workflow_id,
        )
        .expect("marketing staff config");

    let _task = super::update(&mut app, super::TaskBoardMessage::OpenStaffStageAssignment);
    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::SelectStaffStageActionCapability {
            status: campaign_queued,
            capability_id: Some(CODE_REVIEW_CAPABILITY_ID.to_string()),
        },
    );
    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::UpdateStaffStageActionParameter {
            status: campaign_queued,
            key: "review_scope".to_string(),
            value: "landing page copy".to_string(),
        },
    );
    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::SelectStaffStageActionCapability {
            status: campaign_planning,
            capability_id: Some(SAVED_WORKFLOW_CAPABILITY_ID.to_string()),
        },
    );
    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::SelectStaffStageActionWorkflow {
            status: campaign_planning,
            workflow_uuid: Some(workflow_uuid.to_string()),
        },
    );
    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::UpdateStaffStageActionParameter {
            status: campaign_planning,
            key: "brief".to_string(),
            value: "summer launch".to_string(),
        },
    );
    let _task = super::update(&mut app, super::TaskBoardMessage::SaveStaffStageAssignment);

    assert_eq!(app.task_board_staff_config.stage_actions.len(), 7);
    assert!(app.task_board_staff_config.stage_actions.iter().any(|action| {
        action.status == campaign_queued
            && action.capability_id == CODE_REVIEW_CAPABILITY_ID
            && action.input_parameters
                == BTreeMap::from([("review_scope".to_string(), json!("landing page copy"))])
    }));
    assert!(app.task_board_staff_config.stage_actions.iter().any(|action| {
        action.status == campaign_planning
            && action.capability_id == SAVED_WORKFLOW_CAPABILITY_ID
            && action.workflow_uuid.as_deref() == Some(workflow_uuid)
            && action.input_parameters
                == BTreeMap::from([("brief".to_string(), json!("summer launch"))])
    }));

    let mut staff_config = StaffConfig::default();
    crate::app::task::staff::apply_task_board_staff_config_to_staff_config_for_workflow_id(
        &mut staff_config,
        &app.task_board_staff_config,
        &app.task_board_settings.workflow_id,
    )
    .expect("stage action inputs write into staff project state");
    assert_eq!(staff_config.project_state.stage_actions.len(), 7);
    assert!(staff_config.project_state.stage_actions.iter().any(|action| {
        action.stage_id == "campaign_queued"
            && action.capability_id == CODE_REVIEW_CAPABILITY_ID
            && action.inputs.parameters
                == BTreeMap::from([("review_scope".to_string(), json!("landing page copy"))])
    }));
    assert!(staff_config.project_state.stage_actions.iter().any(|action| {
        action.stage_id == "campaign_planning"
            && action.capability_id == SAVED_WORKFLOW_CAPABILITY_ID
            && action.workflow_uuid.as_deref() == Some(workflow_uuid)
            && action.inputs.parameters
                == BTreeMap::from([("brief".to_string(), json!("summer launch"))])
    }));
    assert_eq!(
        app.task_board_staff_config.stage_workflow_bindings,
        vec![TaskBoardStageWorkflowBinding {
            status: campaign_planning,
            workflow_uuid: workflow_uuid.to_string(),
        }]
    );
}

#[test]
fn stage_action_workflow_generator_uses_capability_io() {
    let (mut app, _task) = crate::app::App::new();
    let campaign_planning =
        TaskStatus::parse_key("campaign_planning").expect("campaign planning status");
    app.task_board_settings.workflow_id = "marketing_campaign".to_string();
    app.task_board_staff_config =
        crate::app::task::staff::task_board_staff_config_from_staff_config_for_workflow_id(
            StaffConfig::default(),
            &app.task_board_settings.workflow_id,
        )
        .expect("marketing staff config");
    app.task_board_staff_capability_catalog = vec![TaskBoardStaffCapability {
        id: "campaign_brief_workflow".to_string(),
        label: "活动简报 Workflow".to_string(),
        capability_type: TaskBoardStaffCapabilityKind::SavedWorkflow,
        built_in: false,
        supported_roles: Vec::new(),
        requires_workflow_uuid: true,
        input_summary: TaskBoardStaffCapabilitySummary {
            description: "营销 brief 入参".to_string(),
            fields: vec![
                "workflow_uuid".to_string(),
                "brief".to_string(),
                "target audience".to_string(),
            ],
        },
        result_summary: TaskBoardStaffCapabilitySummary {
            description: "营销产出".to_string(),
            fields: vec!["outcome".to_string(), "copy_notes".to_string()],
        },
        input_contract: String::new(),
        result_semantics: String::new(),
        permission_hint: String::new(),
    }];
    app.task_board_staff_stage_action_draft = Some(vec![TaskBoardStageAction {
        status: campaign_planning,
        capability_id: "campaign_brief_workflow".to_string(),
        workflow_uuid: None,
        input_parameters: BTreeMap::from([
            ("brief".to_string(), json!("summer launch")),
            ("campaign channel".to_string(), json!("email")),
        ]),
    }]);

    let spec = super::build_stage_action_workflow_spec(
        &app,
        campaign_planning,
        &app.task_board_settings.workflow_id,
    )
    .expect("workflow spec");
    let yaml = super::generated_stage_workflow_yaml(&spec).expect("workflow yaml");

    assert!(spec.input_fields.contains(&"task_summary".to_string()));
    assert!(spec.input_fields.contains(&"brief".to_string()));
    assert!(spec.input_fields.contains(&"target_audience".to_string()));
    assert!(spec.input_fields.contains(&"campaign_channel".to_string()));
    assert!(!spec.input_fields.contains(&"workflow_uuid".to_string()));
    assert!(spec.result_fields.contains(&"copy_notes".to_string()));
    assert!(yaml.contains("按任务流管理中的成功/失败边自动推进"));
    assert!(yaml.contains("campaign_channel"));
    assert!(yaml.contains("copy_notes"));
    assert!(yaml.contains("{{#start-node.task_summary#}}"));
}

#[test]
fn stage_workflow_request_merges_custom_parameters_without_overriding_context() {
    let workflow =
        crate::app::task::task_workflow_by_id("dev_lifecycle").expect("default workflow");
    let member = TaskBoardStaffConfig::default()
        .members
        .into_iter()
        .find(|member| member.id == "staff-executor")
        .expect("executor member");
    let mut task = Task::new(7);
    task.id = "T-stage-workflow".to_string();
    task.description = "实现阶段 Workflow 参数".to_string();
    task.status = TaskStatus::Pending;
    let stage_action = TaskBoardStageAction {
        status: TaskStatus::Pending,
        capability_id: SAVED_WORKFLOW_CAPABILITY_ID.to_string(),
        workflow_uuid: Some("018ff4b0-1d0d-7a2c-a30c-7bcf9ee1f040".to_string()),
        input_parameters: BTreeMap::from([
            ("release_channel".to_string(), json!("beta")),
            ("priority".to_string(), json!("should-not-override")),
            ("workflow_uuid".to_string(), json!("blocked")),
        ]),
    };

    let request = super::build_stage_workflow_request(
        &task,
        &workflow,
        TaskStatus::Pending,
        &member,
        Some(&stage_action),
    );

    assert_eq!(request.inputs.get("release_channel"), Some(&json!("beta")));
    assert_eq!(request.inputs.get("priority"), Some(&json!(7)));
    assert!(!request.inputs.contains_key("workflow_uuid"));
}

#[tokio::test(flavor = "multi_thread")]
async fn stage_workflow_binding_runs_application_for_custom_review_stage() {
    let _guard = super::TEST_GATEWAY_ENV_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
    let (mut app, _task) = crate::app::App::new();
    let review_ready = TaskStatus::parse_key("campaign_review_ready").expect("review ready status");
    let reviewing = TaskStatus::parse_key("campaign_reviewing").expect("reviewing status");
    let workflow_uuid = "018ff4b0-1d0d-7a2c-a30c-7bcf9ee1f099";
    app.task_board_view_mode = TaskBoardViewMode::Staff;
    app.task_board_settings.workflow_id = "marketing_campaign".to_string();
    let _project_path = ensure_task_board_project_path(&mut app);
    let mut staff_config = StaffConfig::default();
    staff_config.workflow_orchestrations.insert(
        "marketing_campaign".to_string(),
        StaffWorkflowOrchestrationConfig {
            stage_actions: vec![StaffStageActionConfig {
                stage_id: "campaign_review_ready".to_string(),
                capability_id: SAVED_WORKFLOW_CAPABILITY_ID.to_string(),
                workflow_uuid: Some(workflow_uuid.to_string()),
                inputs: StaffStageActionInputsConfig {
                    workflow_uuid: Some(workflow_uuid.to_string()),
                    ..StaffStageActionInputsConfig::default()
                },
            }],
            ..StaffWorkflowOrchestrationConfig::default()
        },
    );
    app.task_board_staff_config =
        crate::app::task::staff::task_board_staff_config_from_staff_config_for_workflow_id(
            staff_config,
            &app.task_board_settings.workflow_id,
        )
        .expect("marketing staff config");
    assert!(app.task_board_staff_config.stage_actions.iter().any(|action| {
        action.status == review_ready
            && action.capability_id == SAVED_WORKFLOW_CAPABILITY_ID
            && action.workflow_uuid.as_deref() == Some(workflow_uuid)
    }));

    let mut task = Task::new(10);
    task.id = "T-campaign-review".to_string();
    task.status = review_ready;
    task.description = "Campaign recap for launch week".to_string();
    task.prompt = "full hidden payload secret-token diff --git worktree".to_string();
    task.logs.push(crate::app::task::TaskLogEntry::new_message(
        "operator note secret-token".to_string(),
    ));
    app.task_board_tasks = vec![task];

    let command = TaskBoardStaffCommand::StartTask {
        command_id: "cmd-stage-workflow".to_string(),
        task_id: "T-campaign-review".to_string(),
        member_id: "staff-reviewer".to_string(),
    };
    let action = super::staff_command_action(&app, &command).expect("stage workflow action");
    assert_eq!(
        action,
        TaskBoardStaffCommandAction::RunStageWorkflow {
            task_id: "T-campaign-review".to_string(),
            status: review_ready,
            workflow_uuid: workflow_uuid.to_string(),
            member_id: "staff-reviewer".to_string(),
        }
    );

    let gateway = StaffIpcGateway::start_raw(vec![(
        200,
        [
            sse(json!({
                "event": "workflow_started",
                "task_id": "run-stage-1",
                "workflow_run_id": "run-stage-1",
                "data": {"id": "run-stage-1"}
            })),
            sse(json!({
                "event": "message",
                "id": "run-stage-1",
                "task_id": "run-stage-1",
                "message_id": "msg-stage-1",
                "conversation_id": "conv-stage-1",
                "answer": "复盘摘要：继续进入复盘中",
                "created_at": 10
            })),
            sse(json!({
                "event": "workflow_finished",
                "task_id": "run-stage-1",
                "workflow_run_id": "run-stage-1",
                "data": {"status": "succeeded"}
            })),
        ]
        .join(""),
    )]);
    let (_message, tasks) =
        super::apply_staff_command_action(&mut app, action).expect("apply action");
    assert_eq!(tasks.len(), 1);
    let messages = collect_task_messages(iced::Task::batch(tasks)).await;
    apply_task_board_messages(&mut app, messages);
    let requests = gateway.take_requests();

    let task = app
        .task_board_tasks
        .iter()
        .find(|task| task.id == "T-campaign-review")
        .expect("task remains");
    assert_eq!(task.status, reviewing);
    assert!(
        task.logs.iter().any(|entry| {
            entry.message.contains("阶段 workflow 完成")
                && entry.message.contains("复盘摘要：继续进入复盘中")
        }),
        "task logs should record workflow summary: {:?}",
        task.logs
    );
    assert!(task.logs.iter().any(|entry| {
        entry.status_from == Some(review_ready) && entry.status_to == Some(reviewing)
    }));
    assert_eq!(
        crate::app::task::workflow::task_workflow_by_id(&app.task_board_settings.workflow_id)
            .expect("workflow")
            .next_status_for_outcome(review_ready, StatusOutcome::succeed()),
        Some(reviewing)
    );

    assert_eq!(requests.len(), 1, "expected one workflow gateway request: {requests:?}");
    let request = &requests[0];
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, format!("/v1/workflow/applications/{workflow_uuid}/chat-messages"));
    assert_eq!(request.body["inputs"]["task_id"], "T-campaign-review");
    assert_eq!(request.body["inputs"]["status_key"], "campaign_review_ready");
    assert_eq!(request.body["inputs"]["staff_member_id"], "staff-reviewer");
    assert_eq!(request.body["user"], "staff-reviewer");
    let request_body = request.body.to_string();
    assert!(request_body.contains("Campaign recap for launch week"));
    assert!(!request_body.contains("secret-token"));
    assert!(!request_body.contains("diff --git"));
    assert!(!request_body.contains("worktree"));
}

#[test]
fn staff_command_mapping_assigns_member_without_changing_task_data() {
    let (mut app, _task) = crate::app::App::new();
    let mut task = Task::new(10);
    task.id = "T-staff-assign".to_string();
    task.status = TaskStatus::Pending;
    task.assignee = "VibeWindow".to_string();
    app.task_board_tasks = vec![task];
    app.task_board_staff_config = TaskBoardStaffConfig::default();

    let command = TaskBoardStaffCommand::AssignTask {
        command_id: "cmd-assign".to_string(),
        task_id: "T-staff-assign".to_string(),
        member_id: "staff-orchestrator".to_string(),
    };
    let action = super::staff_command_action(&app, &command).expect("assign action");

    assert_eq!(
        action,
        TaskBoardStaffCommandAction::AssignTask {
            task_id: "T-staff-assign".to_string(),
            member_id: "staff-orchestrator".to_string(),
        }
    );

    let (_message, tasks) =
        super::apply_staff_command_action(&mut app, action).expect("apply assign");

    assert!(tasks.is_empty());
    assert_eq!(app.task_board_tasks[0].assignee, "VibeWindow");
    let orchestrator = app
        .task_board_staff_config
        .members
        .iter()
        .find(|member| member.id == "staff-orchestrator")
        .expect("orchestrator member");
    assert_eq!(orchestrator.current_task_ids, vec!["T-staff-assign".to_string()]);
}

#[test]
fn staff_command_mapping_filters_member_without_changing_tasks() {
    let (mut app, _task) = crate::app::App::new();
    app.task_board_staff_config = TaskBoardStaffConfig::default();
    let command = TaskBoardStaffCommand::FilterByMember {
        command_id: "cmd-filter".to_string(),
        member_id: "staff-planner".to_string(),
    };
    let action = super::staff_command_action(&app, &command).expect("filter action");

    let (_message, tasks) =
        super::apply_staff_command_action(&mut app, action).expect("apply filter");

    assert!(tasks.is_empty());
    assert_eq!(app.task_board_staff_filter_member_id.as_deref(), Some("staff-planner"));
    assert!(app.task_board_tasks.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn staff_mode_task_created_keeps_filtered_member_as_filter_only() {
    let (mut app, _task) = crate::app::App::new();
    let project_path = ensure_task_board_project_path(&mut app);
    app.task_board_view_mode = TaskBoardViewMode::Staff;
    app.task_board_staff_config = TaskBoardStaffConfig::default();
    app.task_board_staff_filter_member_id = Some("staff-orchestrator".to_string());
    app.task_board_close_after_create = true;

    let mut task = Task::new(10);
    task.id = "T-created-filtered".to_string();
    task.status = TaskStatus::Pool;

    let post_create =
        super::update(&mut app, super::TaskBoardMessage::TaskCreated { project_path, task });
    apply_task_board_messages(&mut app, collect_task_messages(post_create).await);

    assert_eq!(app.task_board_tasks.len(), 1);
    assert_eq!(app.task_board_staff_filter_member_id.as_deref(), Some("staff-orchestrator"));
    assert_eq!(app.task_board_staff_command_queue.pending_len(), 0);
}

#[test]
fn staff_merge_action_queues_manual_merge() {
    let (mut app, _task) = crate::app::App::new();
    app.task_board_staff_config = TaskBoardStaffConfig::default();

    let (_message, tasks) = super::apply_staff_command_action(
        &mut app,
        TaskBoardStaffCommandAction::RunMerge { task_id: "T-staff-merge".to_string() },
    )
    .expect("apply merge");

    assert_eq!(app.task_board_manual_merge_queue, vec!["T-staff-merge".to_string()]);
    assert_eq!(tasks.len(), 1);
}

#[test]
fn staff_execution_defaults_fill_task_defaults_from_assigned_member() {
    let mut config = TaskBoardStaffConfig::default();
    let member = config
        .members
        .iter_mut()
        .find(|member| member.id == "staff-executor")
        .expect("executor member");
    member.current_task_ids = vec!["T-staff-exec".to_string()];
    member.delegate_agent = Some("executor-agent".to_string());
    member.acp_agent = Some("claude".to_string());
    member.executor_backend = Some(TaskExecutorBackend::Codex);

    let mut task = Task::new(10);
    task.id = "T-staff-exec".to_string();

    let defaults =
        super::apply_staff_execution_defaults(&config, &mut task).expect("staff defaults applied");

    assert_eq!(task.agent.as_deref(), Some("executor-agent"));
    assert_eq!(task.acp_agent.as_deref(), Some("claude"));
    assert_eq!(defaults.member_id, "staff-executor");
    assert_eq!(defaults.delegate_agent.as_deref(), Some("executor-agent"));
    assert_eq!(defaults.acp_agent.as_deref(), Some("claude"));
}

#[test]
fn staff_execution_defaults_clear_default_main_when_member_uses_acp_only() {
    let mut config = TaskBoardStaffConfig::default();
    let member = config
        .members
        .iter_mut()
        .find(|member| member.id == "staff-executor")
        .expect("executor member");
    member.current_task_ids = vec!["T-staff-acp-only".to_string()];
    member.delegate_agent = None;
    member.acp_agent = Some("codex".to_string());

    let mut task = Task::new(10);
    task.id = "T-staff-acp-only".to_string();
    task.agent = Some(crate::app::task::TASK_AGENT_MAIN.to_string());

    let defaults =
        super::apply_staff_execution_defaults(&config, &mut task).expect("staff defaults applied");

    assert_eq!(task.agent, None);
    assert_eq!(task.acp_agent.as_deref(), Some("codex"));
    assert_eq!(defaults.delegate_agent, None);
    assert_eq!(defaults.acp_agent.as_deref(), Some("codex"));
    assert!(defaults.cleared_default_delegate_agent);
}

#[test]
fn staff_execution_defaults_preserve_explicit_task_execution_fields() {
    let mut config = TaskBoardStaffConfig::default();
    let member = config
        .members
        .iter_mut()
        .find(|member| member.id == "staff-executor")
        .expect("executor member");
    member.current_task_ids = vec!["T-staff-explicit".to_string()];
    member.delegate_agent = Some("executor-agent".to_string());
    member.acp_agent = Some("claude".to_string());

    let mut task = Task::new(10);
    task.id = "T-staff-explicit".to_string();
    task.agent = Some("reviewer-agent".to_string());
    task.acp_agent = Some("codex".to_string());

    let defaults = super::apply_staff_execution_defaults(&config, &mut task);

    assert!(defaults.is_none());
    assert_eq!(task.agent.as_deref(), Some("reviewer-agent"));
    assert_eq!(task.acp_agent.as_deref(), Some("codex"));
}

#[test]
fn staff_command_input_enqueues_assign_then_start_for_mentioned_member() {
    let (mut app, _task) = crate::app::App::new();
    let mut task = Task::new(10);
    task.id = "T-input-run".to_string();
    task.status = TaskStatus::Pending;
    app.task_board_tasks = vec![task];
    app.task_board_staff_config = TaskBoardStaffConfig::default();
    app.task_board_staff_config.members.push(TaskBoardStaffMember::new(
        "staff-orchestrator-alt",
        "Alt",
        TaskBoardStaffRole::Orchestrator,
    ));

    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::UpdateStaffCommandInput("@Alt 执行 T-input-run".to_string()),
    );
    let _task = super::update(&mut app, super::TaskBoardMessage::SubmitStaffCommandInput);

    assert!(app.task_board_staff_command_input.is_empty());
    assert_eq!(app.task_board_staff_command_queue.pending_len(), 2);
    assert_eq!(app.task_board_staff_events.len(), 2);
    assert!(app.task_board_staff_events[0].command_id.contains("assign"));
    assert!(app.task_board_staff_events[1].command_id.contains("start"));
}

// 行为契约：使用代表性场景固定该分支的可观察结果。
// 断言聚焦输入、状态与返回任务，不复述内部控制流程。
#[test]
fn staff_command_manual_submit_shows_enqueue_toast() {
    let (mut app, _task) = crate::app::App::new();
    let mut task = Task::new(10);
    task.id = "T-manual-start".to_string();
    task.status = TaskStatus::Pending;
    app.task_board_tasks = vec![task];
    app.task_board_staff_config = TaskBoardStaffConfig::default();

    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::SubmitStaffCommand(TaskBoardStaffCommand::StartTask {
            command_id: "cmd-manual-start".to_string(),
            task_id: "T-manual-start".to_string(),
            member_id: "staff-orchestrator".to_string(),
        }),
    );

    let toast = app.active_toast.as_ref().expect("enqueue should show a toast");
    assert_eq!(toast.kind, crate::app::state::ToastKind::Info);
    assert!(toast.message.contains("开始任务"));
    assert!(toast.message.contains("已进入员工命令队列"));
}

#[test]
fn staff_command_queue_tick_shows_applied_toast() {
    let (mut app, _task) = crate::app::App::new();
    let mut task = Task::new(10);
    task.id = "T-apply-start".to_string();
    task.status = TaskStatus::Pending;
    app.task_board_tasks = vec![task];
    app.task_board_staff_config = TaskBoardStaffConfig::default();

    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::SubmitStaffCommand(TaskBoardStaffCommand::StartTask {
            command_id: "cmd-apply-start".to_string(),
            task_id: "T-apply-start".to_string(),
            member_id: "staff-orchestrator".to_string(),
        }),
    );
    let _task = super::update(&mut app, super::TaskBoardMessage::StaffCommandQueueTick);

    let toast = app.active_toast.as_ref().expect("applied command should show a toast");
    assert_eq!(toast.kind, crate::app::state::ToastKind::Success);
    assert!(toast.message.contains("T-apply-start"));
    assert!(toast.message.contains("执行路径"));
}

#[test]
fn staff_command_member_candidate_prefills_start_command() {
    let (mut app, _task) = crate::app::App::new();
    app.task_board_staff_config = TaskBoardStaffConfig::default();

    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::UpdateStaffCommandInput("@pla T-input-run".to_string()),
    );
    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::SelectStaffCommandMember {
            member_id: "staff-planner".to_string(),
        },
    );

    assert_eq!(app.task_board_staff_command_input, "@Planner 执行 T-input-run");
}

#[test]
fn staff_command_input_uses_single_startable_task_for_member() {
    let (mut app, _task) = crate::app::App::new();
    let mut task = Task::new(10);
    task.id = "T-input-single".to_string();
    task.status = TaskStatus::Pending;
    app.task_board_tasks = vec![task];
    app.task_board_staff_config = TaskBoardStaffConfig::default();
    app.task_board_staff_config
        .members
        .iter_mut()
        .find(|member| member.id == "staff-orchestrator")
        .expect("orchestrator")
        .current_task_ids = vec!["T-input-single".to_string()];

    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::UpdateStaffCommandInput("@Orchestrator 执行".to_string()),
    );
    let _task = super::update(&mut app, super::TaskBoardMessage::SubmitStaffCommandInput);

    assert!(app.task_board_staff_command_input.is_empty());
    assert_eq!(app.task_board_staff_command_queue.pending_len(), 1);
    assert_eq!(app.task_board_staff_events.len(), 1);
    assert!(app.task_board_staff_events[0].command_id.contains("start"));
}

#[test]
fn staff_feed_tab_and_task_status_filter_update_independent_staff_view_state() {
    let (mut app, _task) = crate::app::App::new();

    assert_eq!(app.task_board_staff_feed_tab, TaskBoardStaffFeedTab::Tasks);
    assert!(app.task_board_staff_task_status_filter.is_empty());

    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::SelectStaffFeedTab(TaskBoardStaffFeedTab::Tasks),
    );
    assert_eq!(app.task_board_staff_feed_tab, TaskBoardStaffFeedTab::Tasks);

    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::ToggleStaffTaskStatusFilter(TaskStatus::Running),
    );
    assert_eq!(app.task_board_staff_task_status_filter, vec![TaskStatus::Running]);
    assert_eq!(app.task_board_filter_status, None);

    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::ToggleStaffTaskStatusFilter(TaskStatus::Archived),
    );
    assert_eq!(app.task_board_staff_task_status_filter, vec![TaskStatus::Archived]);

    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::ToggleStaffTaskStatusFilter(TaskStatus::Pool),
    );
    assert_eq!(app.task_board_staff_task_status_filter, vec![TaskStatus::Pool]);

    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::SelectStaffTaskStatusTab(TaskStatus::CodeReview),
    );
    assert_eq!(app.task_board_staff_task_status_filter, vec![TaskStatus::CodeReview]);

    let _task = super::update(&mut app, super::TaskBoardMessage::SelectAllStaffTaskStatuses);
    assert!(app.task_board_staff_task_status_filter.is_empty());
}

#[test]
fn staff_stage_assignment_save_updates_config_and_projection() {
    let project = tempfile::TempDir::new().expect("temp project");
    let (mut app, _task) = crate::app::App::new();
    app.project_path = Some(project.path().to_string_lossy().to_string());
    app.task_board_view_mode = TaskBoardViewMode::Kanban;

    let mut config = TaskBoardStaffConfig::default();
    let mut executor = TaskBoardStaffMember::new(
        "staff-executor-alt",
        "Executor Alt",
        TaskBoardStaffRole::Executor,
    );
    executor.level = TaskBoardStaffLevel::Executor;
    config.members.push(executor);
    app.task_board_staff_config = config;

    let mut task = Task::new(10);
    task.id = "T-running".to_string();
    task.status = TaskStatus::Running;
    app.task_board_tasks = vec![task];

    let _task = super::update(&mut app, super::TaskBoardMessage::OpenStaffStageAssignment);
    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::SelectStaffStagePrimaryMember {
            status: TaskStatus::Running,
            member_id: Some("staff-executor-alt".to_string()),
        },
    );
    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::ToggleStaffStageAutonomous {
            status: TaskStatus::Running,
            enabled: true,
        },
    );
    let _task = super::update(&mut app, super::TaskBoardMessage::SaveStaffStageAssignment);

    let assignment = app
        .task_board_staff_config
        .stage_assignments
        .iter()
        .find(|assignment| assignment.status == TaskStatus::Running)
        .expect("running assignment");
    assert_eq!(assignment.primary_member_id.as_deref(), Some("staff-executor-alt"));
    assert!(assignment.autonomous);
    assert!(!app.task_board_staff_assignment_open);
    assert!(app.task_board_staff_assignment_draft.is_none());
    assert_eq!(app.task_board_view_mode, TaskBoardViewMode::Kanban);

    let running_lane = app
        .task_board_staff_projection
        .lanes
        .iter()
        .find(|lane| lane.status == TaskStatus::Running)
        .expect("running lane");
    assert_eq!(running_lane.assignments[0].member_id.as_deref(), Some("staff-executor-alt"));
}

#[test]
fn staff_stage_assignment_save_syncs_staff_settings_project_state() {
    let (mut app, _task) = crate::app::App::new();

    let _task = super::update(&mut app, super::TaskBoardMessage::OpenStaffStageAssignment);
    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::SelectStaffStagePrimaryMember {
            status: TaskStatus::Running,
            member_id: Some("staff-executor".to_string()),
        },
    );
    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::ToggleStaffStageAutonomous {
            status: TaskStatus::Running,
            enabled: true,
        },
    );
    let _task = super::update(&mut app, super::TaskBoardMessage::SaveStaffStageAssignment);

    assert_eq!(
        app.staff_settings.config.workflow_orchestrations["dev_lifecycle"].stage_assignments,
        vec![StaffStageAssignmentConfig {
            stage_id: "running".to_string(),
            primary_member_id: Some("staff-executor".to_string()),
            backup_member_ids: Vec::new(),
            manager_member_ids: Vec::new(),
            autonomous: true,
        }]
    );
}

#[test]
fn staff_stage_assignment_tab_updates_per_status_picker_tab() {
    let (mut app, _task) = crate::app::App::new();

    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::SelectStaffStageAssignmentTab {
            status: TaskStatus::Running,
            tab: TaskBoardStaffAssignmentTab::Manager,
        },
    );
    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::SelectStaffStageAssignmentTab {
            status: TaskStatus::CodeReview,
            tab: TaskBoardStaffAssignmentTab::Backup,
        },
    );

    assert_eq!(
        app.task_board_staff_assignment_tabs.get(&TaskStatus::Running),
        Some(&TaskBoardStaffAssignmentTab::Manager)
    );
    assert_eq!(
        app.task_board_staff_assignment_tabs.get(&TaskStatus::CodeReview),
        Some(&TaskBoardStaffAssignmentTab::Backup)
    );
}

#[test]
fn staff_orchestration_tab_updates_visible_section() {
    let (mut app, _task) = crate::app::App::new();

    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::SelectStaffOrchestrationTab(
            TaskBoardStaffOrchestrationTab::Staffing,
        ),
    );

    assert_eq!(app.task_board_staff_orchestration_tab, TaskBoardStaffOrchestrationTab::Staffing);
}

#[test]
fn staff_stage_assignment_save_rejects_unknown_member() {
    let (mut app, _task) = crate::app::App::new();
    app.task_board_staff_config = TaskBoardStaffConfig::default();

    let _task = super::update(&mut app, super::TaskBoardMessage::OpenStaffStageAssignment);
    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::SelectStaffStagePrimaryMember {
            status: TaskStatus::Running,
            member_id: Some("missing-staff".to_string()),
        },
    );
    let _task = super::update(&mut app, super::TaskBoardMessage::SaveStaffStageAssignment);

    assert!(app.task_board_staff_config.stage_assignments.is_empty());
    assert!(app.task_board_staff_assignment_open);
    assert!(app.task_board_staff_assignment_draft.is_some());
    assert_eq!(
        app.active_toast.as_ref().map(|toast| toast.kind),
        Some(crate::app::state::ToastKind::Error)
    );
}

#[test]
fn staff_stage_assignment_close_discards_draft() {
    let (mut app, _task) = crate::app::App::new();
    app.task_board_staff_config = TaskBoardStaffConfig::default();

    let _task = super::update(&mut app, super::TaskBoardMessage::OpenStaffStageAssignment);
    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::SelectStaffStagePrimaryMember {
            status: TaskStatus::Running,
            member_id: Some("staff-executor".to_string()),
        },
    );
    let _task = super::update(&mut app, super::TaskBoardMessage::CloseStaffStageAssignment);

    assert!(app.task_board_staff_config.stage_assignments.is_empty());
    assert!(!app.task_board_staff_assignment_open);
    assert!(app.task_board_staff_assignment_draft.is_none());
}

#[test]
fn task_execution_success_waits_for_stage_action_without_legacy_review_switch() {
    let project = tempfile::TempDir::new().expect("temp project");
    create_handoff_repo(project.path());
    let (mut app, _task) = crate::app::App::new();
    app.project_path = Some(project.path().to_string_lossy().to_string());
    app.task_board_view_mode = TaskBoardViewMode::Staff;
    app.task_board_staff_config = staff_handoff_config();

    let mut task = Task::new(10);
    task.id = "T-handoff-exec".to_string();
    task.status = TaskStatus::Running;
    task.assignee = "staff-executor".to_string();
    task.merge_source_branch = Some("feature/handoff".to_string());
    task.merge_target_branch = Some("main".to_string());
    task.selected_worktree_path =
        Some(project.path().join("worktree").to_string_lossy().to_string());
    app.task_board_tasks = vec![task];
    app.task_board_executor.running_tasks = vec!["T-handoff-exec".to_string()];

    let project_path = ensure_task_board_project_path(&mut app);
    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::TaskExecutionCompleted {
            project_path,
            task_id: "T-handoff-exec".to_string(),
            result: Ok("done\nsecret-token\ndiff --git sensitive".to_string()),
        },
    );

    let task = app
        .task_board_tasks
        .iter()
        .find(|task| task.id == "T-handoff-exec")
        .expect("task should remain");
    assert_eq!(task.status, TaskStatus::CodeComplete);
    assert!(app.task_board_staff_events.is_empty());
}

#[test]
fn task_review_success_emits_staff_handoff_message() {
    let project = tempfile::TempDir::new().expect("temp project");
    let (mut app, _task) = crate::app::App::new();
    app.project_path = Some(project.path().to_string_lossy().to_string());
    app.task_board_view_mode = TaskBoardViewMode::Staff;
    app.task_board_staff_config = staff_handoff_config();

    let mut task = Task::new(10);
    task.id = "T-handoff-review".to_string();
    task.status = TaskStatus::CodeReview;
    task.assignee = "staff-reviewer".to_string();
    task.merge_source_branch = Some("feature/handoff".to_string());
    task.merge_target_branch = Some("main".to_string());
    task.selected_worktree_path =
        Some(project.path().join("worktree").to_string_lossy().to_string());
    app.task_board_tasks = vec![task];
    app.task_board_executor.running_tasks = vec!["T-handoff-review".to_string()];

    let project_path = ensure_task_board_project_path(&mut app);
    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::TaskCodeReviewCompleted {
            project_path,
            task_id: "T-handoff-review".to_string(),
            result: Ok(r#"review ok {"status":"ok"} secret-token diff --git"#.to_string()),
        },
    );

    assert_staff_handoff_message(
        &app,
        "T-handoff-review",
        "staff-reviewer",
        "staff-merger",
        "代码审核",
        "合并代码",
    );
}

#[test]
fn task_merge_success_emits_staff_handoff_message() {
    let (mut app, _task) = crate::app::App::new();
    app.task_board_view_mode = TaskBoardViewMode::Staff;
    app.task_board_staff_config = staff_handoff_config();

    let mut task = Task::new(10);
    task.id = "T-handoff-merge".to_string();
    task.status = TaskStatus::PrSubmitted;
    task.assignee = "staff-merger".to_string();
    app.task_board_tasks = vec![task];
    app.task_board_executor.running_tasks = vec!["T-handoff-merge".to_string()];

    let project_path = ensure_task_board_project_path(&mut app);
    let _task = super::update(
        &mut app,
        super::TaskBoardMessage::TaskMergeCompleted {
            project_path,
            task_id: "T-handoff-merge".to_string(),
            result: Ok("merged secret-token diff --git".to_string()),
        },
    );

    assert_staff_handoff_message(
        &app,
        "T-handoff-merge",
        "staff-merger",
        "staff-orchestrator",
        "合并代码",
        "任务完成",
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn staff_handoff_with_ipc_target_schedules_gateway_delivery() {
    let _guard = super::TEST_GATEWAY_ENV_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
    let (mut app, _task) = crate::app::App::new();
    app.task_board_view_mode = TaskBoardViewMode::Staff;
    app.task_board_staff_config = staff_handoff_config();
    set_staff_ipc_target(&mut app.task_board_staff_config, "staff-orchestrator", "agent-next");

    let mut task = Task::new(10);
    task.id = "T-handoff-ipc".to_string();
    task.status = TaskStatus::PrSubmitted;
    task.assignee = "staff-merger".to_string();
    app.task_board_tasks = vec![task];
    app.task_board_executor.running_tasks = vec!["T-handoff-ipc".to_string()];

    let gateway = StaffIpcGateway::start_matching(vec![
        ("/v1/projects/resolve", 200, project_resolve_response_json()),
        (
            "/v1/agent-scheduler/projects/proj_1/commands/bulk",
            200,
            json!({"accepted": 1, "rejected": 0, "results": [{"accepted": true}]}),
        ),
        ("/v1/staff/ipc/messages", 200, json!({"accepted": true})),
    ]);
    let project_path = ensure_task_board_project_path(&mut app);
    let task = super::update(
        &mut app,
        super::TaskBoardMessage::TaskMergeCompleted {
            project_path,
            task_id: "T-handoff-ipc".to_string(),
            result: Ok("merged secret-token diff --git".to_string()),
        },
    );
    apply_task_board_messages(&mut app, collect_task_messages(task).await);
    let requests = gateway.take_requests();

    let event = staff_handoff_message_event(&app, "T-handoff-ipc");
    let message = event.staff_message.as_ref().expect("staff message payload");
    assert_eq!(message.delivery_status, Some(TaskBoardStaffMessageDeliveryStatus::Delivered));
    assert!(
        requests
            .iter()
            .any(|request| request.path == "/v1/agent-scheduler/projects/proj_1/commands/bulk"),
        "merge result should be persisted through Agent Scheduler: {requests:?}"
    );
    let ipc_requests = requests
        .iter()
        .filter(|request| request.path == "/v1/staff/ipc/messages")
        .collect::<Vec<_>>();
    assert_eq!(ipc_requests.len(), 1, "expected one IPC gateway request: {requests:?}");
    let request = ipc_requests[0];
    assert_eq!(request.method, "POST");
    assert_eq!(request.body["to_agent"], "agent-next");
    assert_eq!(request.body["task_id"], "T-handoff-ipc");
    assert_eq!(request.body["from_member_id"], "staff-merger");
    assert_eq!(request.body["to_member_id"], "staff-orchestrator");

    let payload = request.body["payload"].as_str().expect("payload string");
    let payload_json: Value = serde_json::from_str(payload).expect("payload json");
    assert_eq!(payload_json["kind"], "staff_handoff");
    assert_eq!(payload_json["task_id"], "T-handoff-ipc");
    assert_eq!(payload_json["from_status"], "pr_submitted");
    assert_eq!(payload_json["to_status"], "completed");
    assert!(!payload.contains("secret-token"));
    assert!(!payload.contains("diff --git"));
    assert!(!payload.contains("worktree"));
    assert!(
        payload.chars().count()
            <= vw_gateway_client::vw_api_types::staff::STAFF_IPC_MESSAGE_PAYLOAD_LIMIT
    );
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[tokio::test(flavor = "multi_thread")]
async fn staff_handoff_ipc_failure_keeps_task_status() {
    let _guard = super::TEST_GATEWAY_ENV_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
    let (mut app, _task) = crate::app::App::new();
    app.task_board_view_mode = TaskBoardViewMode::Staff;
    app.task_board_staff_config = staff_handoff_config();
    set_staff_ipc_target(&mut app.task_board_staff_config, "staff-orchestrator", "agent-next");

    let mut task = Task::new(10);
    task.id = "T-handoff-ipc-failure".to_string();
    task.status = TaskStatus::PrSubmitted;
    task.assignee = "staff-merger".to_string();
    app.task_board_tasks = vec![task];
    app.task_board_executor.running_tasks = vec!["T-handoff-ipc-failure".to_string()];

    let gateway = StaffIpcGateway::start_matching(vec![
        ("/v1/projects/resolve", 200, project_resolve_response_json()),
        (
            "/v1/agent-scheduler/projects/proj_1/commands/bulk",
            200,
            json!({"accepted": 1, "rejected": 0, "results": [{"accepted": true}]}),
        ),
        ("/v1/staff/ipc/messages", 200, json!({"accepted": false})),
    ]);
    let project_path = ensure_task_board_project_path(&mut app);
    let task = super::update(
        &mut app,
        super::TaskBoardMessage::TaskMergeCompleted {
            project_path,
            task_id: "T-handoff-ipc-failure".to_string(),
            result: Ok("merged".to_string()),
        },
    );
    apply_task_board_messages(&mut app, collect_task_messages(task).await);
    let requests = gateway.take_requests();

    let task = app
        .task_board_tasks
        .iter()
        .find(|task| task.id == "T-handoff-ipc-failure")
        .expect("task should remain");
    assert_eq!(task.status, TaskStatus::Completed);
    assert!(
        task.logs.iter().any(|entry| entry.message.contains("IPC 交接消息投递失败")),
        "task logs should record nonfatal IPC failure: {:?}",
        task.logs
    );
    let event = staff_handoff_message_event(&app, "T-handoff-ipc-failure");
    let message = event.staff_message.as_ref().expect("staff message payload");
    assert_eq!(message.delivery_status, Some(TaskBoardStaffMessageDeliveryStatus::Failed));
    assert_eq!(
        requests.iter().filter(|request| request.path == "/v1/staff/ipc/messages").count(),
        1,
        "expected one IPC gateway request: {requests:?}"
    );
}

fn staff_handoff_config() -> TaskBoardStaffConfig {
    let mut config = TaskBoardStaffConfig::default();
    for member in &mut config.members {
        member.current_task_ids.clear();
    }
    config.stage_assignments = vec![
        staff_stage_assignment(TaskStatus::Running, "staff-executor"),
        staff_stage_assignment(TaskStatus::CodeReview, "staff-reviewer"),
        staff_stage_assignment(TaskStatus::PrSubmitted, "staff-merger"),
        staff_stage_assignment(TaskStatus::Completed, "staff-orchestrator"),
    ];
    config
}

fn staff_stage_assignment(status: TaskStatus, member_id: &str) -> TaskBoardStageAssignment {
    TaskBoardStageAssignment {
        status,
        primary_member_id: Some(member_id.to_string()),
        backup_member_ids: Vec::new(),
        manager_member_ids: Vec::new(),
        autonomous: true,
    }
}

fn create_handoff_repo(path: &std::path::Path) {
    run_git(path, &["init", "-b", "main"]);
    run_git(path, &["config", "user.email", "staff@example.test"]);
    run_git(path, &["config", "user.name", "Staff Test"]);
    std::fs::write(path.join("README.md"), "seed\n").expect("seed file");
    run_git(path, &["add", "README.md"]);
    run_git(path, &["commit", "-m", "seed"]);
    run_git(path, &["checkout", "-b", "feature/handoff"]);
    std::fs::write(path.join("README.md"), "seed\nhandoff\n").expect("updated file");
    run_git(path, &["add", "README.md"]);
    run_git(path, &["commit", "-m", "handoff"]);
    run_git(path, &["checkout", "main"]);
}

fn run_git(path: &std::path::Path, args: &[&str]) {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(path)
        .output()
        .expect("git command should run");
    assert!(
        output.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
}

fn assert_staff_handoff_message(
    app: &crate::app::App,
    task_id: &str,
    from_member_id: &str,
    to_member_id: &str,
    from_stage: &str,
    to_stage: &str,
) {
    let event = app
        .task_board_staff_events
        .iter()
        .find(|event| event.task_id.as_deref() == Some(task_id))
        .expect("staff handoff message event");
    assert_eq!(event.kind, TaskBoardStaffEventKind::Message);
    let message = event.staff_message.as_ref().expect("staff message payload");
    assert_eq!(message.task_id, task_id);
    assert_eq!(message.from_member_id, from_member_id);
    assert_eq!(message.to_member_id, to_member_id);
    assert!(message.body.contains(task_id));
    assert!(message.body.contains(from_stage));
    assert!(message.body.contains(to_stage));
    assert!(message.body.contains(from_member_id));
    assert!(message.body.contains(to_member_id));
    assert!(!message.body.contains("secret-token"));
    assert!(!message.body.contains("diff --git"));
    assert!(!message.body.contains("worktree"));
    assert!(message.body.chars().count() <= 240);
}

#[derive(Debug, Clone)]
struct RecordedStaffIpcRequest {
    method: String,
    path: String,
    body: Value,
}

struct EnvGuard {
    key: &'static str,
    previous: Option<OsString>,
}

impl EnvGuard {
    fn set(key: &'static str, value: impl AsRef<OsStr>) -> Self {
        let previous = std::env::var_os(key);
        unsafe { std::env::set_var(key, value) };
        Self { key, previous }
    }

    fn remove(key: &'static str) -> Self {
        let previous = std::env::var_os(key);
        unsafe { std::env::remove_var(key) };
        Self { key, previous }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => unsafe { std::env::set_var(self.key, value) },
            None => unsafe { std::env::remove_var(self.key) },
        }
    }
}

struct StaffIpcGateway {
    _home: tempfile::TempDir,
    _home_guard: EnvGuard,
    _config_guard: EnvGuard,
    _config_dir_guard: EnvGuard,
    rx: mpsc::Receiver<RecordedStaffIpcRequest>,
    handle: std::thread::JoinHandle<()>,
}

impl StaffIpcGateway {
    fn start_matching(responses: Vec<(&'static str, u16, Value)>) -> Self {
        Self::start_matching_raw(
            responses
                .into_iter()
                .map(|(path, status, body)| (path.to_string(), status, body.to_string()))
                .collect(),
        )
    }

    fn start_raw(responses: Vec<(u16, String)>) -> Self {
        let home = tempfile::tempdir().expect("temp home");
        let home_guard = EnvGuard::set("HOME", home.path());
        let config_guard = EnvGuard::remove("VIBEWINDOW_CONFIG");
        let config_dir_guard = EnvGuard::remove("VIBEWINDOW_CONFIG_DIR");
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind gateway stub");
        listener.set_nonblocking(true).expect("nonblocking gateway stub");
        let port = listener.local_addr().expect("local addr").port();
        write_staff_ipc_gateway_bootstrap_config(home.path(), port);

        let (tx, rx) = mpsc::channel();
        let handle = std::thread::spawn(move || {
            let mut responses = std::collections::VecDeque::from(responses);
            let deadline = Instant::now() + Duration::from_secs(2);
            while !responses.is_empty() && Instant::now() < deadline {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let request = read_staff_ipc_request(&mut stream);
                        let (status, body) = responses.pop_front().expect("scenario response");
                        tx.send(request).expect("record request");
                        write_staff_ipc_response(&mut stream, status, &body);
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    Err(error) => panic!("accept gateway stub request: {error}"),
                }
            }
        });

        Self {
            _home: home,
            _home_guard: home_guard,
            _config_guard: config_guard,
            _config_dir_guard: config_dir_guard,
            rx,
            handle,
        }
    }

    fn start_matching_raw(responses: Vec<(String, u16, String)>) -> Self {
        let home = tempfile::tempdir().expect("temp home");
        let home_guard = EnvGuard::set("HOME", home.path());
        let config_guard = EnvGuard::remove("VIBEWINDOW_CONFIG");
        let config_dir_guard = EnvGuard::remove("VIBEWINDOW_CONFIG_DIR");
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind gateway stub");
        listener.set_nonblocking(true).expect("nonblocking gateway stub");
        let port = listener.local_addr().expect("local addr").port();
        write_staff_ipc_gateway_bootstrap_config(home.path(), port);

        let (tx, rx) = mpsc::channel();
        let handle = std::thread::spawn(move || {
            let mut responses = responses;
            let deadline = Instant::now() + Duration::from_secs(2);
            while !responses.is_empty() && Instant::now() < deadline {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let request = read_staff_ipc_request(&mut stream);
                        let matched_response = responses
                            .iter()
                            .position(|(path, _, _)| path == &request.path)
                            .map(|response_index| {
                                let (_path, status, body) = responses.remove(response_index);
                                (status, body)
                            })
                            .unwrap_or_else(|| fallback_staff_ipc_gateway_response(&request.path));
                        tx.send(request).expect("record request");
                        write_staff_ipc_response(
                            &mut stream,
                            matched_response.0,
                            &matched_response.1,
                        );
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    Err(error) => panic!("accept gateway stub request: {error}"),
                }
            }
        });

        Self {
            _home: home,
            _home_guard: home_guard,
            _config_guard: config_guard,
            _config_dir_guard: config_dir_guard,
            rx,
            handle,
        }
    }

    fn take_requests(self) -> Vec<RecordedStaffIpcRequest> {
        self.handle.join().expect("gateway stub thread");
        let mut requests = Vec::new();
        while let Ok(request) = self.rx.try_recv() {
            requests.push(request);
        }
        requests
    }
}

fn project_resolve_response_json() -> Value {
    json!({
        "project": {
            "id": "proj_1",
            "name": "handoff",
            "directory": "/tmp/vibewindow-handoff",
            "display_path": "/tmp/vibewindow-handoff",
            "status": "ready",
            "created_at_ms": 1,
            "updated_at_ms": 2,
            "default_branch": null,
            "current_branch": null,
            "git": {
                "is_repo": true,
                "has_uncommitted_changes": false,
                "ahead": null,
                "behind": null
            },
            "active_worktree_id": null,
            "session_count": null
        }
    })
}

fn staff_orchestration_gateway_response_json() -> Value {
    let mut value = serde_json::to_value(vw_config_types::config::Config::default())
        .expect("default config should serialize");
    let object = value.as_object_mut().expect("default config should be an object");
    let workflow = crate::app::task::workflow::task_workflow_by_id("marketing_campaign")
        .expect("marketing workflow");
    object.insert("project".to_string(), project_resolve_response_json()["project"].clone());
    object.insert("project_id".to_string(), json!("proj_1"));
    object.insert(
        "workflow".to_string(),
        serde_json::to_value(crate::app::task::workflow::task_workflow_snapshot_for_gateway(
            &workflow,
        ))
        .expect("workflow snapshot should serialize"),
    );
    object.insert("project_state".to_string(), staff_project_state_json());
    object.insert("workflow_id".to_string(), json!("marketing_campaign"));
    object.insert("orchestration".to_string(), staff_workflow_orchestration_json());
    object.insert("capabilities".to_string(), json!([]));
    value
}

fn staff_project_state_json() -> Value {
    json!({
        "capabilities": [],
        "stage_assignments": [],
        "stage_workflow_bindings": [],
        "stage_actions": [],
        "member_task_assignments": []
    })
}

fn staff_workflow_orchestration_json() -> Value {
    json!({
        "capabilities": [],
        "stage_assignments": [],
        "stage_workflow_bindings": [],
        "stage_actions": []
    })
}

fn fallback_staff_ipc_gateway_response(path: &str) -> (u16, String) {
    if path == "/v1/projects/resolve" {
        return (200, project_resolve_response_json().to_string());
    }
    if path.contains("/commands/bulk") {
        return (
            200,
            json!({"accepted": 1, "rejected": 0, "results": [{"accepted": true}]}).to_string(),
        );
    }
    (404, json!({"error": "unexpected test gateway request", "path": path}).to_string())
}

fn write_staff_ipc_gateway_bootstrap_config(home: &std::path::Path, port: u16) {
    let mut config = GatewayClientSystemSettingsConfig::default();
    config.set_servers(
        vec![GatewayClientServerConfig {
            id: "staff-ipc-test".to_string(),
            name: "Staff IPC Test".to_string(),
            host: "127.0.0.1".to_string(),
            port,
            bearer_token: "test-token".to_string(),
            ..GatewayClientServerConfig::default()
        }],
        "staff-ipc-test".to_string(),
    );
    let path = vw_config_types::paths::home_config_dir(home).join("gateway-client-bootstrap.json");
    std::fs::create_dir_all(path.parent().expect("config parent")).expect("create config dir");
    std::fs::write(path, serde_json::to_string(&config).expect("serialize config"))
        .expect("write config");
}

fn read_staff_ipc_request(stream: &mut TcpStream) -> RecordedStaffIpcRequest {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 1024];
    let mut header_end = None;
    let deadline = Instant::now() + Duration::from_secs(2);
    while header_end.is_none() {
        let read = match stream.read(&mut buffer) {
            Ok(read) => read,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                assert!(Instant::now() < deadline, "timed out waiting for request headers");
                std::thread::sleep(Duration::from_millis(5));
                continue;
            }
            Err(error) => panic!("read request: {error}"),
        };
        assert!(read > 0, "connection closed before headers");
        bytes.extend_from_slice(&buffer[..read]);
        header_end = bytes.windows(4).position(|window| window == b"\r\n\r\n");
    }

    let header_end = header_end.expect("headers");
    let headers = String::from_utf8_lossy(&bytes[..header_end]).to_string();
    let mut lines = headers.lines();
    let request_line = lines.next().expect("request line");
    let mut parts = request_line.split_whitespace();
    let method = parts.next().expect("method").to_string();
    let path = parts.next().expect("path").to_string();
    let content_length = lines
        .filter_map(|line| line.split_once(':'))
        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, value)| value.trim().parse::<usize>().ok())
        .unwrap_or(0);

    let mut body = bytes[(header_end + 4)..].to_vec();
    while body.len() < content_length {
        let read = match stream.read(&mut buffer) {
            Ok(read) => read,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                assert!(Instant::now() < deadline, "timed out waiting for request body");
                std::thread::sleep(Duration::from_millis(5));
                continue;
            }
            Err(error) => panic!("read body: {error}"),
        };
        assert!(read > 0, "connection closed before body");
        body.extend_from_slice(&buffer[..read]);
    }
    body.truncate(content_length);
    let body = if body.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&body).expect("request json body")
    };

    RecordedStaffIpcRequest { method, path, body }
}

fn write_staff_ipc_response(stream: &mut TcpStream, status: u16, body: &str) {
    let response = format!(
        "HTTP/1.1 {status} OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes()).expect("write response");
}

fn sse(payload: serde_json::Value) -> String {
    format!("data: {payload}\n\n")
}

async fn collect_task_messages(task: iced::Task<Message>) -> Vec<Message> {
    let Some(mut stream) = iced_runtime::task::into_stream(task.collect()) else {
        return Vec::new();
    };
    while let Some(action) = stream.next().await {
        if let Action::Output(messages) = action {
            return messages;
        }
    }
    Vec::new()
}

fn apply_task_board_messages(app: &mut crate::app::App, messages: Vec<Message>) {
    for message in messages {
        if let Message::TaskBoard(task_message) = message {
            let _task = super::update(app, task_message);
        }
    }
}

async fn apply_task_board_messages_with_followups(
    app: &mut crate::app::App,
    messages: Vec<Message>,
) {
    for message in messages {
        if let Message::TaskBoard(task_message) = message {
            let followup = super::update(app, task_message);
            apply_task_board_messages(app, collect_task_messages(followup).await);
        }
    }
}

fn set_staff_ipc_target(config: &mut TaskBoardStaffConfig, member_id: &str, agent_id: &str) {
    let member =
        config.members.iter_mut().find(|member| member.id == member_id).expect("staff member");
    member.ipc_agent_id = Some(agent_id.to_string());
}

fn staff_handoff_message_event<'a>(
    app: &'a crate::app::App,
    task_id: &str,
) -> &'a TaskBoardStaffEvent {
    app.task_board_staff_events
        .iter()
        .find(|event| event.task_id.as_deref() == Some(task_id))
        .expect("staff handoff message event")
}
