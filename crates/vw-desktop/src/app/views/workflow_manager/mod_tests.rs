//! 工作流管理器入口的测试，覆盖标签切换、空状态和面板组合。
//! 用例检查工作流列表与能力管理之间的更新和视图路由契约。
use crate::app::Message;
use crate::app::message::TaskBoardMessage;
use crate::app::task::staff::models::TaskBoardStageAction;
use crate::app::task::staff::stage_capabilities::{
    CODE_REVIEW_CAPABILITY_ID, GIT_MERGE_CAPABILITY_ID, SAVED_WORKFLOW_CAPABILITY_ID,
};
use crate::app::task::workflow::TaskWorkflowRegistry;
use crate::app::task::{
    DEFAULT_TASK_WORKFLOW_ID, OwnedStatusDefinition, OwnedTaskWorkflow, StatusOutcome, StatusRole,
    TaskStatus,
};
use crate::app::task::{TaskBoardStaffCapability, TaskBoardStaffConfig};
use iced_runtime::Action;
use iced_runtime::futures::futures::StreamExt;
use serde_json::json;
use std::collections::HashSet;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::{MutexGuard, mpsc};
use std::time::{Duration, Instant};
use vw_config_types::ui::{GatewayClientServerConfig, GatewayClientSystemSettingsConfig};
use vw_gateway_client::vw_api_types::task_lifecycle::TaskLifecycleWorkflowDto;

use super::{
    NextStatusOption, StatusRoleOption, WorkflowDraft, WorkflowDraftStatus, WorkflowManagerMessage,
    WorkflowManagerState, WorkflowManagerTab, apply_lifecycle_workflow_deleted,
    apply_lifecycle_workflow_reset, apply_lifecycle_workflow_saved,
    apply_lifecycle_workflows_loaded, lifecycle_save_command_from_draft,
    lifecycle_status_role_from_local, next_options, normalized_route_label, parse_route_target,
    stage_action_capability_select_message, stage_action_help_definitions, stage_action_rows,
    stage_actions_loading_detail, stage_actions_view_ready, status_role_help_definitions,
    unique_copy_id, update,
};

fn process_state_lock() -> MutexGuard<'static, ()> {
    crate::app::TEST_PROCESS_STATE_LOCK.lock().unwrap_or_else(|poison| poison.into_inner())
}

fn custom_status(key: &str, label: &str, role: StatusRole) -> OwnedStatusDefinition {
    OwnedStatusDefinition {
        status: TaskStatus::parse_key(key).expect("custom status key should parse"),
        key: key.to_string(),
        label: label.to_string(),
        role,
        enabled: true,
        is_terminal: false,
        auto_execution_candidate: false,
        occupies_execution_slot: false,
        is_scheduling_backlog: false,
        next: None,
        transitions: Vec::new(),
    }
}

fn custom_workflow(id: &str) -> OwnedTaskWorkflow {
    let mut intake = custom_status("triage", "Triage", StatusRole::Backlog);
    intake.next = TaskStatus::parse_key("qa_signoff");
    let mut qa = custom_status("qa_signoff", "QA Signoff", StatusRole::Review);
    qa.is_terminal = true;
    OwnedTaskWorkflow {
        id: id.to_string(),
        label: "QA Flow".to_string(),
        statuses: vec![intake, qa],
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn workflow_manager_saves_lifecycle_via_gateway() {
    let _guard = process_state_lock();
    let _cache = RuntimeLifecycleCacheGuard::new();
    let home = tempfile::TempDir::new().expect("temp home");
    let _home = EnvVarGuard::set("HOME", home.path());
    let (mut app, _startup) = crate::app::App::new();
    app.workflow_manager = WorkflowManagerState::default();
    let server = RecordingServer::json(
        vec![
            (200, lifecycle_workflow_json("handoff_flow", "Handoff Flow", false)),
            (200, json!([lifecycle_workflow_json("handoff_flow", "Handoff Flow", false)])),
        ],
        2,
    );
    write_gateway_bootstrap_config(home.path(), server.port());

    let _ = super::update_app(&mut app, WorkflowManagerMessage::StartNew);
    let _ = super::update_app(
        &mut app,
        WorkflowManagerMessage::DraftIdChanged("handoff_flow".to_string()),
    );
    let _ = super::update_app(
        &mut app,
        WorkflowManagerMessage::DraftLabelChanged("Handoff Flow".to_string()),
    );
    let _ = super::update_app(
        &mut app,
        WorkflowManagerMessage::AddDraftStatus(WorkflowDraftStatus::new(
            "ready_for_qa",
            "Ready for QA",
            StatusRole::ReviewReady,
        )),
    );
    let _ = super::update_app(
        &mut app,
        WorkflowManagerMessage::AddDraftStatus(WorkflowDraftStatus::new(
            "qa_done",
            "QA Done",
            StatusRole::Done,
        )),
    );

    let task = super::update_app(&mut app, WorkflowManagerMessage::SaveDraft);
    let messages = collect_task_messages_with_timeout(task).await;
    apply_workflow_manager_messages(&mut app, messages);
    let requests = server.finish();

    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].method, "POST");
    assert_eq!(requests[0].path, "/v1/task-lifecycle/workflows");
    assert_eq!(requests[0].body["workflow"]["id"], "handoff_flow");
    assert_eq!(requests[0].body["workflow"]["statuses"][0]["next"], "qa_done");
    assert_eq!(requests[1].method, "GET");
    assert_eq!(requests[1].path, "/v1/task-lifecycle/workflows");
    assert!(app.workflow_manager.workflow_ids().iter().any(|id| id == "handoff_flow"));
    assert!(app.workflow_manager.feedback().is_some_and(|feedback| feedback.is_success()));
}

#[tokio::test(flavor = "multi_thread")]
async fn workflow_manager_delete_reloads_gateway_lifecycles() {
    let _guard = process_state_lock();
    let _cache = RuntimeLifecycleCacheGuard::new();
    crate::app::task::replace_runtime_task_lifecycle_workflows_for_tests(vec![custom_workflow(
        "qa_flow",
    )])
    .expect("custom workflow cache should install");
    let home = tempfile::TempDir::new().expect("temp home");
    let _home = EnvVarGuard::set("HOME", home.path());
    let (mut app, _startup) = crate::app::App::new();
    app.workflow_manager = WorkflowManagerState::default();
    let server = RecordingServer::json(
        vec![
            (200, json!({"id": "qa_flow", "deleted": true})),
            (
                200,
                json!([lifecycle_workflow_json(DEFAULT_TASK_WORKFLOW_ID, "Dev lifecycle", true)]),
            ),
        ],
        2,
    );
    write_gateway_bootstrap_config(home.path(), server.port());

    let task = super::update_app(&mut app, WorkflowManagerMessage::DeleteSelected);
    let messages = collect_task_messages_with_timeout(task).await;
    assert!(messages.is_empty());
    assert!(matches!(
        app.workflow_manager.pending_confirmation.as_ref(),
        Some(super::WorkflowManagerConfirmation::DeleteWorkflow { workflow_id, .. })
            if workflow_id == "qa_flow"
    ));

    let task = super::update_app(&mut app, WorkflowManagerMessage::ConfirmPendingAction);
    let messages = collect_task_messages_with_timeout(task).await;
    apply_workflow_manager_messages(&mut app, messages);
    let requests = server.finish();

    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].method, "DELETE");
    assert_eq!(requests[0].path, "/v1/task-lifecycle/workflows/qa_flow");
    assert_eq!(requests[1].method, "GET");
    assert_eq!(requests[1].path, "/v1/task-lifecycle/workflows");
    assert!(!app.workflow_manager.workflow_ids().iter().any(|id| id == "qa_flow"));
    assert!(app.workflow_manager.feedback().is_some_and(|feedback| feedback.is_success()));
}

#[test]
fn builtin_workflow_delete_is_rejected_without_confirmation() {
    let mut registry = TaskWorkflowRegistry::new();
    let mut state = WorkflowManagerState::from_registry(&registry);

    update(
        &mut state,
        &mut registry,
        WorkflowManagerMessage::SelectWorkflow(DEFAULT_TASK_WORKFLOW_ID.to_string()),
    );
    update(&mut state, &mut registry, WorkflowManagerMessage::DeleteSelected);

    assert!(state.pending_confirmation.is_none());
    assert!(state.pending_effect.is_none());
    let feedback = state.feedback().expect("delete should emit feedback");
    assert!(feedback.is_error());
    assert!(feedback.message().contains("内置工作流不能删除"));
}

#[test]
fn workflow_manager_lists_builtin_and_custom() {
    let mut registry = TaskWorkflowRegistry::new();
    registry.register(custom_workflow("qa_flow")).expect("custom workflow should register");

    let state = WorkflowManagerState::from_registry(&registry);

    let ids = state.workflow_ids();
    assert!(ids.iter().any(|id| id == DEFAULT_TASK_WORKFLOW_ID));
    assert!(ids.iter().any(|id| id == "qa_flow"));
    assert_eq!(state.selected_workflow_id(), Some("qa_flow"));
}

#[test]
fn view_workflow_selects_item_and_leaves_draft_mode() {
    let registry = TaskWorkflowRegistry::new();
    let mut state = WorkflowManagerState::from_registry(&registry);

    update(&mut state, &mut TaskWorkflowRegistry::new(), WorkflowManagerMessage::StartNew);
    assert!(state.draft.is_some());

    update(
        &mut state,
        &mut TaskWorkflowRegistry::new(),
        WorkflowManagerMessage::ViewWorkflow(DEFAULT_TASK_WORKFLOW_ID.to_string()),
    );

    assert_eq!(state.selected_workflow_id(), Some(DEFAULT_TASK_WORKFLOW_ID));
    assert!(state.draft.is_none());
}

#[test]
fn status_role_help_uses_chinese_labels_and_english_values() {
    let mut state = WorkflowManagerState::default();
    assert!(!state.status_role_help_visible());

    update(
        &mut state,
        &mut TaskWorkflowRegistry::new(),
        WorkflowManagerMessage::ToggleStatusRoleHelp,
    );
    assert!(state.status_role_help_visible());

    assert_eq!(StatusRoleOption(StatusRole::ReviewReady).to_string(), "待审核 (ReviewReady)");
    assert_eq!(StatusRoleOption(StatusRole::Merging).to_string(), "收尾 (Merging)");

    let definitions = status_role_help_definitions();
    assert!(definitions.iter().any(|definition| {
        definition.value == "Queued" && definition.description.contains("待启动")
    }));
    assert!(definitions.iter().any(|definition| {
        definition.value == "Executing" && definition.description.contains("运行超时监控")
    }));
    assert!(definitions.iter().any(|definition| {
        definition.value == "ReviewReady" && definition.description.contains("阶段 Workflow")
    }));
}

#[test]
fn workflow_manager_stage_action_tab_lists_project_stage_actions() {
    let mut state = WorkflowManagerState::default();
    update(
        &mut state,
        &mut TaskWorkflowRegistry::new(),
        WorkflowManagerMessage::SelectTab(WorkflowManagerTab::StageActions),
    );
    assert_eq!(state.active_tab(), WorkflowManagerTab::StageActions);

    let mut config = TaskBoardStaffConfig::default()
        .retarget_for_workflow_id(DEFAULT_TASK_WORKFLOW_ID)
        .expect("default staff config should retarget");
    config.stage_actions = vec![
        TaskBoardStageAction {
            status: TaskStatus::CodeReview,
            capability_id: CODE_REVIEW_CAPABILITY_ID.to_string(),
            workflow_uuid: None,
            ..Default::default()
        },
        TaskBoardStageAction {
            status: TaskStatus::PrSubmitted,
            capability_id: GIT_MERGE_CAPABILITY_ID.to_string(),
            workflow_uuid: None,
            ..Default::default()
        },
        TaskBoardStageAction {
            status: TaskStatus::CodeComplete,
            capability_id: SAVED_WORKFLOW_CAPABILITY_ID.to_string(),
            workflow_uuid: Some("workflow-uuid-123".to_string()),
            ..Default::default()
        },
    ];
    let workflow_options = vec![vw_gateway_client::WorkflowRecordSummary {
        uuid: "workflow-uuid-123".to_string(),
        name: "业务验收流程".to_string(),
        description: String::new(),
        created_at_ms: 0,
        updated_at_ms: 0,
    }];

    let rows =
        stage_action_rows(DEFAULT_TASK_WORKFLOW_ID, &config.stage_actions, &workflow_options, None);

    let review = rows
        .iter()
        .find(|row| row.status == TaskStatus::CodeReview)
        .expect("review row should be listed");
    assert_eq!(review.capability_label, "代码审核 (code_review)");
    assert_eq!(review.capability_id.as_deref(), Some(CODE_REVIEW_CAPABILITY_ID));
    assert!(
        review
            .capability_options
            .iter()
            .any(|option| option.label == "已保存 Workflow (saved_workflow)")
    );

    let merge = rows
        .iter()
        .find(|row| row.status == TaskStatus::PrSubmitted)
        .expect("merge row should be listed");
    assert_eq!(merge.capability_label, "Git 合并 (git_merge)");

    let workflow = rows
        .iter()
        .find(|row| row.status == TaskStatus::CodeComplete)
        .expect("saved workflow row should be listed");
    assert_eq!(workflow.capability_label, "已保存 Workflow (saved_workflow)");
    assert_eq!(workflow.workflow_summary.as_deref(), Some("业务验收流程 · workflow-uuid-123"));

    match stage_action_capability_select_message(
        TaskStatus::CodeReview,
        Some(SAVED_WORKFLOW_CAPABILITY_ID.to_string()),
    ) {
        Message::TaskBoard(TaskBoardMessage::SelectStaffStageActionCapability {
            status,
            capability_id,
        }) => {
            assert_eq!(status, TaskStatus::CodeReview);
            assert_eq!(capability_id.as_deref(), Some(SAVED_WORKFLOW_CAPABILITY_ID));
        }
        other => panic!("unexpected stage action message: {other:?}"),
    }
}

#[test]
fn workflow_manager_stage_action_rows_follow_selected_workflow_without_project_filter() {
    let selected_status =
        TaskStatus::parse_key("campaign_review_ready").expect("marketing status should parse");
    let stage_actions = vec![TaskBoardStageAction {
        status: selected_status,
        capability_id: SAVED_WORKFLOW_CAPABILITY_ID.to_string(),
        workflow_uuid: Some("workflow-uuid-campaign".to_string()),
        ..Default::default()
    }];

    let rows = stage_action_rows("marketing_campaign", &stage_actions, &[], None);

    assert!(
        rows.iter().any(|row| row.status == selected_status),
        "selected workflow statuses should render without current project workflow filtering"
    );
    let row = rows
        .iter()
        .find(|row| row.status == selected_status)
        .expect("campaign review stage row should be present");
    assert_eq!(row.capability_id.as_deref(), Some(SAVED_WORKFLOW_CAPABILITY_ID));
    assert_eq!(row.workflow_summary.as_deref(), Some("已绑定 workflow-uuid-campaign"));
}

#[tokio::test(flavor = "multi_thread")]
async fn workflow_manager_stage_actions_tab_reloads_project_staff_config() {
    let _guard = process_state_lock();
    let (mut app, _task) = crate::app::App::new();

    let task = super::update_app(
        &mut app,
        WorkflowManagerMessage::SelectTab(WorkflowManagerTab::StageActions),
    );
    let messages = collect_task_messages(task).await;

    assert!(
        messages.iter().any(|message| {
            matches!(message, Message::TaskBoard(TaskBoardMessage::LoadStaffStageWorkflowOptions))
        }),
        "stage actions tab should load workflow options: {messages:?}"
    );
    assert!(
        messages.iter().any(|message| {
            matches!(message, Message::TaskBoard(TaskBoardMessage::ReloadStaffConfig))
                || matches!(
                    message,
                    Message::TaskBoard(TaskBoardMessage::ReloadStaffConfigForWorkflow {
                        workflow_id
                    }) if workflow_id == DEFAULT_TASK_WORKFLOW_ID
                )
        }),
        "stage actions tab should reload project staff config: {messages:?}"
    );
    assert!(
        messages.iter().any(|message| {
            matches!(
                message,
                Message::TaskBoard(TaskBoardMessage::LoadStaffCapabilityCatalogForWorkflow {
                    workflow_id
                }) if workflow_id == DEFAULT_TASK_WORKFLOW_ID
            )
        }),
        "stage actions tab should load selected workflow capabilities: {messages:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn workflow_manager_stage_actions_tab_uses_selected_workflow_for_staff_loads() {
    let _guard = process_state_lock();
    let (mut app, _task) = crate::app::App::new();
    app.task_board_settings.workflow_id = DEFAULT_TASK_WORKFLOW_ID.to_string();

    let _task = super::update_app(
        &mut app,
        WorkflowManagerMessage::SelectWorkflow("marketing_campaign".to_string()),
    );
    let task = super::update_app(
        &mut app,
        WorkflowManagerMessage::SelectTab(WorkflowManagerTab::StageActions),
    );
    let messages = collect_task_messages(task).await;

    assert!(
        messages.iter().any(|message| {
            matches!(
                message,
                Message::TaskBoard(TaskBoardMessage::ReloadStaffConfigForWorkflow {
                    workflow_id
                }) if workflow_id == "marketing_campaign"
            )
        }),
        "stage actions tab should reload selected workflow staff config: {messages:?}"
    );
    assert!(
        messages.iter().any(|message| {
            matches!(
                message,
                Message::TaskBoard(TaskBoardMessage::LoadStaffCapabilityCatalogForWorkflow {
                    workflow_id
                }) if workflow_id == "marketing_campaign"
            )
        }),
        "stage actions tab should load selected workflow capabilities: {messages:?}"
    );
}

#[test]
fn workflow_manager_stage_actions_waits_for_selected_workflow_state() {
    let _guard = process_state_lock();
    let (mut app, _task) = crate::app::App::new();
    app.task_board_staff_config_workflow_id = Some(DEFAULT_TASK_WORKFLOW_ID.to_string());
    app.task_board_staff_capability_catalog_workflow_id =
        Some(DEFAULT_TASK_WORKFLOW_ID.to_string());
    app.task_board_staff_capability_catalog =
        vec![TaskBoardStaffCapability::custom_saved_workflow(
            "dev_release".to_string(),
            "Dev Release".to_string(),
        )];

    let _task = super::update_app(
        &mut app,
        WorkflowManagerMessage::SelectWorkflow("marketing_campaign".to_string()),
    );

    assert!(!stage_actions_view_ready(&app, "marketing_campaign"));

    app.task_board_staff_config_workflow_id = Some("marketing_campaign".to_string());
    app.task_board_staff_capability_catalog_workflow_id = Some("marketing_campaign".to_string());
    app.task_board_staff_capability_catalog =
        vec![TaskBoardStaffCapability::custom_saved_workflow(
            "campaign_publish".to_string(),
            "Campaign Publish".to_string(),
        )];

    assert!(stage_actions_view_ready(&app, "marketing_campaign"));
}

#[test]
fn workflow_manager_stage_actions_scopes_loading_errors_to_selected_workflow() {
    let _guard = process_state_lock();
    let (mut app, _task) = crate::app::App::new();
    app.task_board_staff_config_workflow_id = Some("marketing_campaign".to_string());
    app.task_board_staff_config_load_error = Some(
        "task board workflow stage planning does not match workflow dev_lifecycle role mapping"
            .to_string(),
    );

    assert_eq!(
        stage_actions_loading_detail(&app, DEFAULT_TASK_WORKFLOW_ID),
        "正在加载当前工作流的阶段编排与能力目录"
    );

    app.task_board_staff_config_workflow_id = Some(DEFAULT_TASK_WORKFLOW_ID.to_string());

    assert_eq!(
        stage_actions_loading_detail(&app, DEFAULT_TASK_WORKFLOW_ID),
        "task board workflow stage planning does not match workflow dev_lifecycle role mapping"
    );

    let _task = super::update_app(
        &mut app,
        WorkflowManagerMessage::SelectTab(WorkflowManagerTab::StageActions),
    );

    assert!(app.task_board_staff_config_load_error.is_none());
}

#[test]
fn workflow_manager_stage_action_tab_builds_selected_workflow_orchestration_panel() {
    let _guard = process_state_lock();
    let (mut app, _task) = crate::app::App::new();

    let _task = super::update_app(
        &mut app,
        WorkflowManagerMessage::SelectWorkflow("marketing_campaign".to_string()),
    );
    let _task = super::update_app(
        &mut app,
        WorkflowManagerMessage::SelectTab(WorkflowManagerTab::StageActions),
    );
    let view = super::view(&app);
    let _ = std::hint::black_box(view);

    assert_eq!(app.workflow_manager.selected_workflow_id(), Some("marketing_campaign"));
    assert!(include_str!("mod.rs").contains("build_embedded_orchestration_panel"));
    assert!(include_str!("mod.rs").contains("SaveStaffStageAssignmentForWorkflow"));
}

#[test]
fn workflow_manager_stage_action_help_defines_roles_and_capabilities() {
    let mut state = WorkflowManagerState::default();
    assert!(!state.stage_action_help_visible());

    update(
        &mut state,
        &mut TaskWorkflowRegistry::new(),
        WorkflowManagerMessage::ToggleStageActionHelp,
    );
    assert!(state.stage_action_help_visible());

    let definitions = stage_action_help_definitions();
    assert!(definitions.iter().any(|definition| {
        definition.title == "调度关系"
            && definition.description.contains("StatusRole 决定能不能调度")
            && definition.description.contains("阶段能力决定具体执行")
    }));
    assert!(definitions.iter().any(|definition| {
        definition.title == "入参契约"
            && definition.description.contains("任务摘要、阶段 key、成员、优先级")
    }));
    assert!(definitions.iter().any(|definition| {
        definition.title == "结果契约"
            && definition.description.contains("结果映射到生命周期边")
            && definition.description.contains("succeed/failed")
    }));
    assert!(definitions.iter().any(|definition| {
        definition.title == "安全边界"
            && definition.description.contains("内置代码能力是标准能力")
            && definition.description.contains("已保存 Workflow 是业务自定义能力")
    }));
    assert!(definitions.iter().any(|definition| {
        definition.title == "代码审核 (code_review)"
            && definition.description.contains("任务摘要、变更范围、审核成员")
            && definition.description.contains("需要读取工作区差异")
    }));
}

#[test]
fn next_status_options_use_chinese_labels_and_english_keys() {
    let draft = WorkflowDraft {
        original_id: None,
        id: "qa_flow".to_string(),
        label: "QA Flow".to_string(),
        statuses: vec![
            WorkflowDraftStatus::new("ready_for_qa", "待审核", StatusRole::ReviewReady),
            WorkflowDraftStatus::new("qa_done", "完成", StatusRole::Done),
        ],
    };

    let options = next_options(&draft);
    let labels = options.iter().map(ToString::to_string).collect::<Vec<_>>();

    assert_eq!(labels, vec!["结束点 (End)", "待审核 (ready_for_qa)", "完成 (qa_done)"]);
    assert_eq!(
        NextStatusOption::selected(&options, Some("ready_for_qa".to_string())).to_string(),
        "待审核 (ready_for_qa)"
    );
}

#[test]
fn unique_copy_id_advances_past_existing_suffixes() {
    let used_ids = HashSet::from([
        "qa_flow_copy".to_string(),
        "qa_flow_copy_2".to_string(),
        "qa_flow_copy_3".to_string(),
    ]);

    assert_eq!(unique_copy_id(" qa_flow ", &used_ids), "qa_flow_copy_4");
    assert_eq!(unique_copy_id("new_flow", &used_ids), "new_flow_copy");
}

#[test]
fn draft_routes_protect_builtin_entries_and_allow_custom_entry_edits() {
    let mut status = WorkflowDraftStatus::new("review", "Review", StatusRole::Review);

    status.set_route_outcome_key(0, "changed".to_string());
    status.set_route_label(0, "Changed".to_string());
    status.remove_route(0);
    assert_eq!(status.routes[0].outcome_key, StatusOutcome::succeed().key());
    assert_eq!(status.routes[0].label, StatusOutcome::succeed().label());
    assert_eq!(status.routes.len(), 3);

    status.add_route();
    status.add_route();
    assert_eq!(status.routes[3].outcome_key, "manual_review");
    assert_eq!(status.routes[4].outcome_key, "manual_review_2");

    status.set_route_outcome_key(3, "security_review".to_string());
    status.set_route_label(3, "安全复核".to_string());
    status.set_route_target_key(3, Some("done".to_string()));
    assert_eq!(status.routes[3].outcome_key, "security_review");
    assert_eq!(status.routes[3].label, "安全复核");
    assert_eq!(status.routes[3].target_key.as_deref(), Some("done"));

    status.remove_route(3);
    assert_eq!(status.routes.len(), 4);
    assert!(status.routes.iter().all(|route| route.outcome_key != "security_review"));
}

#[test]
fn route_labels_normalize_defaults_and_preserve_custom_text() {
    let succeeded = StatusOutcome::succeed();

    assert_eq!(normalized_route_label("  ", &succeeded), None);
    assert_eq!(normalized_route_label(&succeeded.label(), &succeeded), None);
    assert_eq!(normalized_route_label("  发布成功  ", &succeeded), Some("发布成功".to_string()));
}

#[test]
fn copy_workflow_opens_editable_draft_with_unique_id() {
    let mut registry = TaskWorkflowRegistry::new();
    registry.register(custom_workflow("qa_flow")).expect("custom workflow should register");
    let mut state = WorkflowManagerState::from_registry(&registry);

    update(&mut state, &mut registry, WorkflowManagerMessage::CopyWorkflow("qa_flow".to_string()));

    let draft = state.draft.as_ref().expect("copy should open draft");
    assert_eq!(draft.original_id, None);
    assert_eq!(draft.id, "qa_flow_copy");
    assert_eq!(draft.label, "QA Flow 副本");
}

#[test]
fn builtin_workflow_can_open_edit_draft() {
    let mut registry = TaskWorkflowRegistry::new();
    let mut state = WorkflowManagerState::from_registry(&registry);

    update(&mut state, &mut registry, WorkflowManagerMessage::StartEditSelected);

    let draft = state.draft.as_ref().expect("built-in workflow should open for editing");
    assert_eq!(draft.original_id.as_deref(), Some(DEFAULT_TASK_WORKFLOW_ID));
}

#[test]
fn reset_builtin_workflow_requires_confirmation() {
    let mut registry = TaskWorkflowRegistry::new();
    let mut state = WorkflowManagerState::from_registry(&registry);

    update(
        &mut state,
        &mut registry,
        WorkflowManagerMessage::SelectWorkflow(DEFAULT_TASK_WORKFLOW_ID.to_string()),
    );
    update(&mut state, &mut registry, WorkflowManagerMessage::ResetSelected);

    assert!(matches!(
        state.pending_confirmation.as_ref(),
        Some(super::WorkflowManagerConfirmation::ResetWorkflow { .. })
    ));
    assert!(state.pending_effect.is_none());
    assert!(!state.lifecycles_loading());

    update(&mut state, &mut registry, WorkflowManagerMessage::ConfirmPendingAction);

    assert!(state.pending_confirmation.is_none());
    assert!(state.lifecycles_loading());
    assert!(matches!(
        state.pending_effect.as_ref(),
        Some(super::WorkflowManagerEffect::ResetLifecycle { workflow_id })
            if workflow_id == DEFAULT_TASK_WORKFLOW_ID
    ));
}

#[test]
fn remove_draft_status_requires_confirmation() {
    let mut registry = TaskWorkflowRegistry::new();
    let mut state = WorkflowManagerState::from_registry(&registry);

    update(&mut state, &mut registry, WorkflowManagerMessage::StartNew);
    update(
        &mut state,
        &mut registry,
        WorkflowManagerMessage::AddDraftStatus(WorkflowDraftStatus::new(
            "ready_for_qa",
            "Ready for QA",
            StatusRole::ReviewReady,
        )),
    );
    update(
        &mut state,
        &mut registry,
        WorkflowManagerMessage::AddDraftStatus(WorkflowDraftStatus::new(
            "qa_done",
            "QA Done",
            StatusRole::Done,
        )),
    );

    update(&mut state, &mut registry, WorkflowManagerMessage::RemoveDraftStatus(0));

    let draft = state.draft.as_ref().expect("draft should stay open");
    assert_eq!(draft.statuses.len(), 2);
    assert!(matches!(
        state.pending_confirmation.as_ref(),
        Some(super::WorkflowManagerConfirmation::DeleteStatus { index: 0, .. })
    ));

    update(&mut state, &mut registry, WorkflowManagerMessage::CancelPendingConfirmation);

    let draft = state.draft.as_ref().expect("draft should stay open");
    assert_eq!(draft.statuses.len(), 2);
    assert!(state.pending_confirmation.is_none());

    update(&mut state, &mut registry, WorkflowManagerMessage::RemoveDraftStatus(0));
    update(&mut state, &mut registry, WorkflowManagerMessage::ConfirmPendingAction);

    let draft = state.draft.as_ref().expect("draft should stay open");
    assert_eq!(draft.statuses.len(), 1);
    assert_eq!(draft.statuses[0].key, "qa_done");
    assert!(state.pending_confirmation.is_none());
}

#[test]
fn saving_valid_draft_builds_lifecycle_upsert_body() {
    let mut registry = TaskWorkflowRegistry::new();
    let mut state = WorkflowManagerState::from_registry(&registry);

    update(&mut state, &mut registry, WorkflowManagerMessage::StartNew);
    update(
        &mut state,
        &mut registry,
        WorkflowManagerMessage::DraftIdChanged("handoff_flow".to_string()),
    );
    update(
        &mut state,
        &mut registry,
        WorkflowManagerMessage::DraftLabelChanged("Handoff Flow".to_string()),
    );
    update(
        &mut state,
        &mut registry,
        WorkflowManagerMessage::AddDraftStatus(WorkflowDraftStatus::new(
            "ready_for_qa",
            "Ready for QA",
            StatusRole::ReviewReady,
        )),
    );
    update(
        &mut state,
        &mut registry,
        WorkflowManagerMessage::AddDraftStatus(WorkflowDraftStatus::new(
            "qa_done",
            "QA Done",
            StatusRole::Done,
        )),
    );
    update(
        &mut state,
        &mut registry,
        WorkflowManagerMessage::AddDraftStatus(WorkflowDraftStatus::new(
            "qa_failed",
            "QA Failed",
            StatusRole::Failed,
        )),
    );
    update(
        &mut state,
        &mut registry,
        WorkflowManagerMessage::SetDraftStatusRoute {
            status_index: 0,
            route_index: 1,
            target_key: Some("qa_failed".to_string()),
        },
    );
    update(
        &mut state,
        &mut registry,
        WorkflowManagerMessage::SetDraftStatusRoute {
            status_index: 1,
            route_index: 0,
            target_key: None,
        },
    );
    update(&mut state, &mut registry, WorkflowManagerMessage::ToggleDraftStatusEnabled(0));
    let draft = state.draft.as_ref().expect("draft should be open");
    let command = super::lifecycle_save_command_from_draft(draft, &registry)
        .expect("draft should build lifecycle upsert body");
    let workflow = command.body.workflow;

    assert_eq!(workflow.id, "handoff_flow");
    assert_eq!(workflow.label, "Handoff Flow");
    assert!(!workflow.statuses[0].enabled);
    assert_eq!(workflow.statuses[0].next.as_deref(), Some("qa_done"));
    assert_eq!(workflow.statuses[0].transitions.len(), 2);
    assert!(workflow.statuses[0].transitions.iter().any(|transition| {
        transition.outcome == StatusOutcome::failed().key() && transition.target == "qa_failed"
    }));
    assert!(workflow.statuses[1].terminal);
    assert!(workflow.statuses[2].terminal);
}

#[test]
fn draft_exposes_and_saves_retry_route() {
    let mut registry = TaskWorkflowRegistry::new();
    let mut state = WorkflowManagerState::from_registry(&registry);
    update(&mut state, &mut registry, WorkflowManagerMessage::StartNew);
    update(
        &mut state,
        &mut registry,
        WorkflowManagerMessage::DraftIdChanged("retry_flow".to_string()),
    );
    update(
        &mut state,
        &mut registry,
        WorkflowManagerMessage::DraftLabelChanged("Retry Flow".to_string()),
    );
    update(
        &mut state,
        &mut registry,
        WorkflowManagerMessage::AddDraftStatus(WorkflowDraftStatus::new(
            "retryable_stage",
            "Retryable Stage",
            StatusRole::Executing,
        )),
    );
    update(
        &mut state,
        &mut registry,
        WorkflowManagerMessage::AddDraftStatus(WorkflowDraftStatus::new(
            "retry_queue",
            "Retry Queue",
            StatusRole::Queued,
        )),
    );

    let retry_route_index = state.draft.as_ref().expect("draft should be open").statuses[0]
        .routes
        .iter()
        .position(|route| route.outcome_key == "retry")
        .expect("workflow draft should expose retry route");
    assert!(
        state.draft.as_ref().expect("draft should be open").statuses[0].routes[retry_route_index]
            .is_builtin()
    );

    update(
        &mut state,
        &mut registry,
        WorkflowManagerMessage::SetDraftStatusRoute {
            status_index: 0,
            route_index: retry_route_index,
            target_key: Some("retry_queue".to_string()),
        },
    );

    let draft = state.draft.as_ref().expect("draft should be open");
    let command = super::lifecycle_save_command_from_draft(draft, &registry)
        .expect("draft should build lifecycle upsert body");
    let failed = &command.body.workflow.statuses[0];
    assert!(
        failed.transitions.iter().any(|transition| {
            transition.outcome == "retry" && transition.target == "retry_queue"
        })
    );
}

#[test]
fn saving_custom_result_key_builds_lifecycle_route() {
    let mut registry = TaskWorkflowRegistry::new();
    let mut state = WorkflowManagerState::from_registry(&registry);

    update(&mut state, &mut registry, WorkflowManagerMessage::StartNew);
    update(
        &mut state,
        &mut registry,
        WorkflowManagerMessage::DraftIdChanged("publishing_flow".to_string()),
    );
    update(
        &mut state,
        &mut registry,
        WorkflowManagerMessage::DraftLabelChanged("Publishing Flow".to_string()),
    );
    update(
        &mut state,
        &mut registry,
        WorkflowManagerMessage::AddDraftStatus(WorkflowDraftStatus::new(
            "drafting",
            "Drafting",
            StatusRole::Executing,
        )),
    );
    update(
        &mut state,
        &mut registry,
        WorkflowManagerMessage::AddDraftStatus(WorkflowDraftStatus::new(
            "published",
            "Published",
            StatusRole::Done,
        )),
    );
    update(
        &mut state,
        &mut registry,
        WorkflowManagerMessage::AddDraftStatus(WorkflowDraftStatus::new(
            "platform_rejected",
            "Platform Rejected",
            StatusRole::Failed,
        )),
    );
    update(&mut state, &mut registry, WorkflowManagerMessage::AddDraftStatusRoute(0));
    let route_index = state.draft.as_ref().unwrap().statuses[0].routes.len() - 1;
    update(
        &mut state,
        &mut registry,
        WorkflowManagerMessage::DraftStatusRouteLabelChanged {
            status_index: 0,
            route_index,
            label: "平台驳回".to_string(),
        },
    );
    update(
        &mut state,
        &mut registry,
        WorkflowManagerMessage::DraftStatusRouteOutcomeChanged {
            status_index: 0,
            route_index,
            outcome_key: "platform_rejected".to_string(),
        },
    );
    update(
        &mut state,
        &mut registry,
        WorkflowManagerMessage::SetDraftStatusRoute {
            status_index: 0,
            route_index,
            target_key: Some("platform_rejected".to_string()),
        },
    );
    let draft = state.draft.as_ref().expect("draft should be open");
    let command = super::lifecycle_save_command_from_draft(draft, &registry)
        .expect("draft should build lifecycle upsert body");
    let workflow = command.body.workflow;
    let transition = workflow.statuses[0]
        .transitions
        .iter()
        .find(|transition| transition.outcome == "platform_rejected")
        .expect("custom route should be present");

    assert_eq!(transition.target, "platform_rejected");
    assert_eq!(transition.label.as_deref(), Some("平台驳回"));
}

#[test]
fn saving_invalid_draft_is_rejected_with_feedback() {
    let mut registry = TaskWorkflowRegistry::new();
    let mut state = WorkflowManagerState::from_registry(&registry);

    update(&mut state, &mut registry, WorkflowManagerMessage::StartNew);
    update(
        &mut state,
        &mut registry,
        WorkflowManagerMessage::DraftIdChanged("loop_flow".to_string()),
    );
    update(
        &mut state,
        &mut registry,
        WorkflowManagerMessage::DraftLabelChanged("Loop Flow".to_string()),
    );
    update(
        &mut state,
        &mut registry,
        WorkflowManagerMessage::AddDraftStatus(WorkflowDraftStatus::new(
            "loop_a",
            "Loop A",
            StatusRole::Queued,
        )),
    );
    update(
        &mut state,
        &mut registry,
        WorkflowManagerMessage::AddDraftStatus(WorkflowDraftStatus::new(
            "loop_b",
            "Loop B",
            StatusRole::Executing,
        )),
    );
    update(
        &mut state,
        &mut registry,
        WorkflowManagerMessage::SetDraftStatusRoute {
            status_index: 1,
            route_index: 0,
            target_key: Some("loop_a".to_string()),
        },
    );
    update(&mut state, &mut registry, WorkflowManagerMessage::SaveDraft);

    assert!(registry.workflow_by_id("loop_flow").is_none());
    let feedback = state.feedback().expect("invalid save should emit feedback");
    assert!(feedback.is_error());
    assert!(feedback.message().contains("成环"));
}

#[test]
fn workflow_manager_rejects_missing_selection_and_missing_draft_actions() {
    let mut registry = TaskWorkflowRegistry::new();
    let mut state = WorkflowManagerState::from_registry(&registry);

    update(&mut state, &mut registry, WorkflowManagerMessage::SaveDraft);
    assert!(state.feedback().is_some_and(|feedback| {
        feedback.is_error() && feedback.message().contains("没有正在编辑的工作流")
    }));

    update(&mut state, &mut registry, WorkflowManagerMessage::RemoveDraftStatus(9));
    assert!(state.feedback().is_some_and(|feedback| {
        feedback.is_error() && feedback.message().contains("要删除的阶段不存在")
    }));

    state.selected_workflow_id = None;
    update(&mut state, &mut registry, WorkflowManagerMessage::DeleteSelected);
    assert!(state.feedback().is_some_and(|feedback| {
        feedback.is_error() && feedback.message().contains("请先选择工作流")
    }));

    update(&mut state, &mut registry, WorkflowManagerMessage::ResetSelected);
    assert!(state.feedback().is_some_and(|feedback| {
        feedback.is_error() && feedback.message().contains("请先选择工作流")
    }));
}

#[test]
fn delete_status_confirmation_removes_matching_key_when_index_shifted() {
    let mut registry = TaskWorkflowRegistry::new();
    let mut state = WorkflowManagerState::from_registry(&registry);
    update(&mut state, &mut registry, WorkflowManagerMessage::StartNew);
    update(
        &mut state,
        &mut registry,
        WorkflowManagerMessage::AddDraftStatus(WorkflowDraftStatus::new(
            "ready_for_qa",
            "Ready for QA",
            StatusRole::ReviewReady,
        )),
    );
    update(
        &mut state,
        &mut registry,
        WorkflowManagerMessage::AddDraftStatus(WorkflowDraftStatus::new(
            "qa_done",
            "QA Done",
            StatusRole::Done,
        )),
    );

    state.pending_confirmation = Some(super::WorkflowManagerConfirmation::DeleteStatus {
        index: 0,
        key: "qa_done".to_string(),
        label: "QA Done".to_string(),
    });
    update(&mut state, &mut registry, WorkflowManagerMessage::ConfirmPendingAction);

    let draft = state.draft.as_ref().expect("draft should stay open");
    assert_eq!(draft.statuses.len(), 1);
    assert_eq!(draft.statuses[0].key, "ready_for_qa");
    assert!(state.pending_confirmation.is_none());
}

#[test]
fn editing_builtin_workflow_rejects_id_changes_and_empty_route_outcomes() {
    let registry = TaskWorkflowRegistry::new();
    let mut state = WorkflowManagerState::from_registry(&registry);

    update(&mut state, &mut TaskWorkflowRegistry::new(), WorkflowManagerMessage::StartEditSelected);
    update(
        &mut state,
        &mut TaskWorkflowRegistry::new(),
        WorkflowManagerMessage::DraftIdChanged("renamed_builtin".to_string()),
    );
    let draft = state.draft.as_ref().expect("built-in draft should be open");
    let error = lifecycle_save_command_from_draft(draft, &registry)
        .expect_err("renaming a built-in workflow should fail");
    assert!(error.to_string().contains("内置工作流 ID 不能修改"));

    let mut draft = WorkflowDraft {
        original_id: None,
        id: "empty_route_flow".to_string(),
        label: "Empty Route Flow".to_string(),
        statuses: vec![WorkflowDraftStatus::new("stage", "Stage", StatusRole::Executing)],
    };
    draft.statuses[0].add_route();
    draft.statuses[0].set_route_outcome_key(3, "  ".to_string());
    let error = lifecycle_save_command_from_draft(&draft, &registry)
        .expect_err("empty outcome key should fail");
    assert!(error.to_string().contains("结果 key 不能为空"));
}

#[test]
fn route_target_parsing_and_role_mapping_cover_terminal_and_custom_roles() {
    assert_eq!(parse_route_target("flow", "stage", &StatusOutcome::succeed(), None).unwrap(), None);
    assert_eq!(
        parse_route_target("flow", "stage", &StatusOutcome::succeed(), Some(" done_stage "))
            .unwrap(),
        Some(TaskStatus::Custom("done_stage"))
    );

    assert_eq!(
        lifecycle_status_role_from_local(StatusRole::Backlog),
        vw_gateway_client::vw_api_types::task_lifecycle::TaskLifecycleStatusRoleDto::Backlog
    );
    assert_eq!(
        lifecycle_status_role_from_local(StatusRole::Custom),
        vw_gateway_client::vw_api_types::task_lifecycle::TaskLifecycleStatusRoleDto::Custom
    );
}

#[test]
fn lifecycle_result_handlers_record_success_errors_and_selected_workflow() {
    let _guard = process_state_lock();
    let _cache = RuntimeLifecycleCacheGuard::new();
    let mut state = WorkflowManagerState::default();

    apply_lifecycle_workflows_loaded(&mut state, Err("gateway down".to_string()), true);
    assert_eq!(state.lifecycles_error(), Some("gateway down"));
    assert!(state.feedback().is_some_and(|feedback| {
        feedback.is_error() && feedback.message().contains("生命周期列表加载失败")
    }));

    let qa_workflow = lifecycle_workflow_dto("qa_flow", "QA Flow", false);
    apply_lifecycle_workflows_loaded(&mut state, Ok(vec![qa_workflow.clone()]), false);
    assert!(state.lifecycles_loaded());
    assert!(state.lifecycles_error().is_none());
    assert!(state.workflow_ids().iter().any(|id| id == "qa_flow"));

    state.draft = Some(WorkflowDraft::empty());
    apply_lifecycle_workflow_saved(&mut state, "qa_flow".to_string(), Ok(vec![qa_workflow]));
    assert!(state.draft.is_none());
    assert_eq!(state.selected_workflow_id(), Some("qa_flow"));
    assert!(state.feedback().is_some_and(|feedback| feedback.is_success()));

    apply_lifecycle_workflow_deleted(
        &mut state,
        "qa_flow".to_string(),
        Ok(vec![lifecycle_workflow_dto("other_flow", "Other Flow", false)]),
    );
    assert_ne!(state.selected_workflow_id(), Some("qa_flow"));
    assert!(state.feedback().is_some_and(|feedback| feedback.is_success()));

    apply_lifecycle_workflow_reset(
        &mut state,
        DEFAULT_TASK_WORKFLOW_ID.to_string(),
        Err("reset failed".to_string()),
    );
    assert_eq!(state.lifecycles_error(), Some("reset failed"));
    assert!(state.feedback().is_some_and(|feedback| {
        feedback.is_error() && feedback.message().contains("工作流重置失败")
    }));
}

#[derive(Debug)]
struct RecordedRequest {
    method: String,
    path: String,
    body: serde_json::Value,
}

struct RecordingServer {
    port: u16,
    requests: mpsc::Receiver<RecordedRequest>,
    handle: std::thread::JoinHandle<()>,
}

impl RecordingServer {
    fn json(responses: Vec<(u16, serde_json::Value)>, expected_requests: usize) -> Self {
        let responses = responses
            .into_iter()
            .map(|(status, body)| (status, body.to_string()))
            .collect::<Vec<_>>();
        Self::raw(responses, expected_requests)
    }

    fn raw(responses: Vec<(u16, String)>, expected_requests: usize) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("recording server should bind");
        listener.set_nonblocking(true).expect("recording server should be nonblocking");
        let port = listener.local_addr().expect("recording server addr").port();
        let (tx, rx) = mpsc::channel();
        let handle = std::thread::spawn(move || {
            let mut responses = responses.into_iter();
            let deadline = Instant::now() + Duration::from_secs(3);
            let mut handled = 0;
            while handled < expected_requests && Instant::now() < deadline {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        stream
                            .set_nonblocking(false)
                            .expect("accepted request stream should be blocking");
                        let request = read_recorded_request(&mut stream);
                        tx.send(request).expect("request should record");
                        let (status, body) =
                            responses.next().unwrap_or_else(|| (500, "{}".to_string()));
                        write_recorded_response(&mut stream, status, &body);
                        handled += 1;
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    Err(error) => panic!("recording server accept failed: {error}"),
                }
            }
        });
        Self { port, requests: rx, handle }
    }

    fn port(&self) -> u16 {
        self.port
    }

    fn finish(self) -> Vec<RecordedRequest> {
        self.handle.join().expect("recording server should join");
        self.requests.try_iter().collect()
    }
}

fn read_recorded_request(stream: &mut TcpStream) -> RecordedRequest {
    stream.set_read_timeout(Some(Duration::from_secs(1))).expect("request read timeout should set");
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 1024];
    let mut header_end = None;
    while header_end.is_none() {
        let read = stream.read(&mut buffer).expect("request should read");
        assert!(read > 0, "connection closed before headers");
        bytes.extend_from_slice(&buffer[..read]);
        header_end = find_header_end(&bytes);
    }

    let header_end = header_end.expect("request headers");
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

    let mut body_bytes = bytes[(header_end + 4)..].to_vec();
    while body_bytes.len() < content_length {
        let read = stream.read(&mut buffer).expect("request body should read");
        assert!(read > 0, "connection closed before body");
        body_bytes.extend_from_slice(&buffer[..read]);
    }
    body_bytes.truncate(content_length);
    let body = if body_bytes.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&body_bytes).expect("json request body")
    };

    RecordedRequest { method, path, body }
}

fn find_header_end(bytes: &[u8]) -> Option<usize> {
    bytes.windows(4).position(|window| window == b"\r\n\r\n")
}

fn write_recorded_response(stream: &mut TcpStream, status: u16, body: &str) {
    let reason = match status {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        404 => "Not Found",
        _ => "Error",
    };
    let response = format!(
        "HTTP/1.1 {status} {reason}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes()).expect("response should write");
}

async fn collect_task_messages_with_timeout(task: iced::Task<Message>) -> Vec<Message> {
    tokio::time::timeout(Duration::from_secs(1), collect_task_messages(task))
        .await
        .unwrap_or_default()
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

fn apply_workflow_manager_messages(app: &mut crate::app::App, messages: Vec<Message>) {
    for message in messages {
        if let Message::WorkflowManager(message) = message {
            let _ = super::update_app(app, message);
        }
    }
}

fn lifecycle_workflow_json(id: &str, label: &str, built_in: bool) -> serde_json::Value {
    json!({
        "id": id,
        "label": label,
        "built_in": built_in,
        "statuses": [
            lifecycle_status_json("ready_for_qa", "Ready for QA", "review_ready", false, Some("qa_done")),
            lifecycle_status_json("qa_done", "QA Done", "done", true, None),
        ],
    })
}

fn lifecycle_workflow_dto(id: &str, label: &str, built_in: bool) -> TaskLifecycleWorkflowDto {
    serde_json::from_value(lifecycle_workflow_json(id, label, built_in))
        .expect("workflow dto should deserialize")
}

fn lifecycle_status_json(
    key: &str,
    label: &str,
    role: &str,
    terminal: bool,
    next: Option<&str>,
) -> serde_json::Value {
    json!({
        "key": key,
        "label": label,
        "role": role,
        "terminal": terminal,
        "auto_execution_candidate": matches!(role, "backlog" | "queued" | "planning"),
        "occupies_execution_slot": matches!(role, "planning" | "executing"),
        "is_scheduling_backlog": matches!(role, "backlog" | "queued" | "planning"),
        "next": next,
        "transitions": [],
    })
}

fn write_gateway_bootstrap_config(home: &Path, port: u16) {
    let server = GatewayClientServerConfig {
        id: "stub".to_string(),
        name: "Stub".to_string(),
        host: "127.0.0.1".to_string(),
        port,
        platform_id: "10000".to_string(),
        company_id: "100000000".to_string(),
        bearer_token: String::new(),
        username: String::new(),
        password: String::new(),
        skey: String::new(),
    };
    let cfg = GatewayClientSystemSettingsConfig {
        host: "127.0.0.1".to_string(),
        port,
        platform_id: "10000".to_string(),
        company_id: "100000000".to_string(),
        bearer_token: String::new(),
        username: String::new(),
        password: String::new(),
        skey: String::new(),
        active_server_id: server.id.clone(),
        servers: vec![server],
    };
    let path = vw_config_types::paths::home_config_dir(home).join("gateway-client-bootstrap.json");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("gateway config dir should create");
    }
    std::fs::write(path, serde_json::to_string(&cfg).expect("gateway config should serialize"))
        .expect("gateway config should write");
}

struct EnvVarGuard {
    key: &'static str,
    previous: Option<std::ffi::OsString>,
}

impl EnvVarGuard {
    fn set(key: &'static str, value: impl AsRef<std::ffi::OsStr>) -> Self {
        let previous = std::env::var_os(key);
        unsafe { std::env::set_var(key, value) };
        Self { key, previous }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => unsafe { std::env::set_var(self.key, value) },
            None => unsafe { std::env::remove_var(self.key) },
        }
    }
}

struct RuntimeLifecycleCacheGuard;

impl RuntimeLifecycleCacheGuard {
    fn new() -> Self {
        crate::app::task::reset_runtime_task_lifecycle_cache()
            .expect("runtime lifecycle cache should reset");
        Self
    }
}

impl Drop for RuntimeLifecycleCacheGuard {
    fn drop(&mut self) {
        let _ = crate::app::task::reset_runtime_task_lifecycle_cache();
    }
}
