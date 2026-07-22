use crate::app::state::TaskBoardViewMode;
use crate::app::state::{TaskBoardStaffAssignmentTab, TaskBoardStaffOrchestrationTab};
use crate::app::task::staff::models::{
    TaskBoardStaffCapability, TaskBoardStaffCapabilityKind, TaskBoardStaffCapabilitySummary,
    TaskBoardStageAction, TaskBoardStageAssignment,
};
use crate::app::task::staff::stage_capabilities::{
    CODE_REVIEW_CAPABILITY_ID, GIT_MERGE_CAPABILITY_ID, SAVED_WORKFLOW_CAPABILITY_ID,
};
use crate::app::task::{TaskBoardStaffConfig, TaskStatus};

const ASSIGNMENT_SOURCE: &str = include_str!("assignment.rs");

#[test]
fn staff_assignment_panel_shows_saved_workflow_control() {
    let (mut app, _task) = crate::app::App::new();
    app.window_size = (1280.0, 700.0);
    app.task_board_view_mode = TaskBoardViewMode::Staff;
    app.task_board_staff_assignment_open = true;
    app.task_board_staff_config = TaskBoardStaffConfig {
        stage_actions: vec![TaskBoardStageAction {
            status: TaskStatus::Running,
            capability_id: SAVED_WORKFLOW_CAPABILITY_ID.to_string(),
            workflow_uuid: Some("018ff4b0-1d0d-7a2c-a30c-7bcf9ee1f004".to_string()),
            ..Default::default()
        }],
        ..TaskBoardStaffConfig::default()
    };
    app.task_board_staff_workflow_options = vec![vw_gateway_client::WorkflowRecordSummary {
        uuid: "018ff4b0-1d0d-7a2c-a30c-7bcf9ee1f004".to_string(),
        name: "Run implementation workflow".to_string(),
        description: String::new(),
        created_at_ms: 1,
        updated_at_ms: 1,
    }];

    let view = super::assignment::build_assignment_panel(
        &app,
        iced::Length::Fixed(360.0),
        iced::Length::Fixed(500.0),
    );
    let _ = std::hint::black_box(view);

    let options = super::assignment::workflow_binding_options(
        &app.task_board_staff_workflow_options,
        Some("018ff4b0-1d0d-7a2c-a30c-7bcf9ee1f004"),
    );
    let labels = options.iter().map(ToString::to_string).collect::<Vec<_>>();
    assert_eq!(labels[0], "未选择 Workflow");
    assert!(labels.iter().any(|label| label.contains("Run implementation workflow")));
    assert!(ASSIGNMENT_SOURCE.contains("阶段能力"));
    assert!(ASSIGNMENT_SOURCE.contains("SelectStaffStageActionWorkflow"));
    assert!(ASSIGNMENT_SOURCE.contains("CreateStaffStageActionWorkflow"));
    assert!(ASSIGNMENT_SOURCE.contains("新建 Workflow"));
    assert!(ASSIGNMENT_SOURCE.contains("pick_list"));
}

#[test]
fn stage_action_assignment_ui_lists_capabilities_with_chinese_labels() {
    let (mut app, _task) = crate::app::App::new();
    app.window_size = (1280.0, 700.0);
    app.task_board_view_mode = TaskBoardViewMode::Staff;
    app.task_board_staff_assignment_open = true;
    app.task_board_staff_config = TaskBoardStaffConfig::default();

    let view = super::assignment::build_assignment_panel(
        &app,
        iced::Length::Fixed(360.0),
        iced::Length::Fixed(500.0),
    );
    let _ = std::hint::black_box(view);

    let review_labels = super::assignment::stage_action_capability_options(&[])
        .into_iter()
        .map(|option| option.to_string())
        .collect::<Vec<_>>();
    assert!(review_labels.contains(&format!("代码审核 ({CODE_REVIEW_CAPABILITY_ID})")));
    assert!(review_labels.contains(&format!("已保存 Workflow ({SAVED_WORKFLOW_CAPABILITY_ID})")));
    assert!(review_labels.contains(&format!("Git 合并 ({GIT_MERGE_CAPABILITY_ID})")));

    let merge_labels = super::assignment::stage_action_capability_options(&[])
        .into_iter()
        .map(|option| option.to_string())
        .collect::<Vec<_>>();
    assert!(merge_labels.contains(&format!("Git 合并 ({GIT_MERGE_CAPABILITY_ID})")));
    assert!(merge_labels.contains(&format!("已保存 Workflow ({SAVED_WORKFLOW_CAPABILITY_ID})")));

    assert!(ASSIGNMENT_SOURCE.contains("阶段能力"));
    assert!(ASSIGNMENT_SOURCE.contains("SelectStaffStageActionCapability"));
    assert!(ASSIGNMENT_SOURCE.contains("SelectStaffStageActionWorkflow"));
}

#[test]
fn stage_assignment_uses_catalog_without_role_filtering() {
    let marketing_labels = super::assignment::stage_action_capability_options(&[])
        .into_iter()
        .map(|option| option.to_string())
        .collect::<Vec<_>>();

    assert!(marketing_labels.contains(&format!("代码审核 ({CODE_REVIEW_CAPABILITY_ID})")));
    assert!(marketing_labels.contains(&format!("Git 合并 ({GIT_MERGE_CAPABILITY_ID})")));
    assert!(
        marketing_labels.contains(&format!("已保存 Workflow ({SAVED_WORKFLOW_CAPABILITY_ID})"))
    );

    let recruiting_labels = super::assignment::stage_action_capability_options(&[])
        .into_iter()
        .map(|option| option.to_string())
        .collect::<Vec<_>>();

    assert!(recruiting_labels.contains(&format!("代码审核 ({CODE_REVIEW_CAPABILITY_ID})")));
    assert!(recruiting_labels.contains(&format!("Git 合并 ({GIT_MERGE_CAPABILITY_ID})")));
    assert!(
        recruiting_labels.contains(&format!("已保存 Workflow ({SAVED_WORKFLOW_CAPABILITY_ID})"))
    );
    assert!(ASSIGNMENT_SOURCE.contains("无能力绑定"));
}

#[test]
fn embedded_stage_assignment_save_is_disabled_without_changes() {
    let (app, _task) = crate::app::App::new();
    let config = app
        .task_board_staff_config
        .clone()
        .retarget_for_workflow_id(&app.task_board_settings.workflow_id)
        .expect("default workflow should retarget");

    assert!(!super::assignment::stage_assignment_has_unsaved_changes(
        &app,
        &config,
        &app.task_board_settings.workflow_id,
    ));
    assert!(ASSIGNMENT_SOURCE.contains("stage_assignment_has_unsaved_changes"));
}

#[test]
fn embedded_stage_assignment_save_is_enabled_after_member_change() {
    let (mut app, _task) = crate::app::App::new();
    let config = app
        .task_board_staff_config
        .clone()
        .retarget_for_workflow_id(&app.task_board_settings.workflow_id)
        .expect("default workflow should retarget");

    app.task_board_staff_assignment_draft = Some(vec![TaskBoardStageAssignment {
        status: TaskStatus::Running,
        primary_member_id: Some("staff-executor".to_string()),
        backup_member_ids: Vec::new(),
        manager_member_ids: Vec::new(),
        autonomous: false,
    }]);

    assert!(super::assignment::stage_assignment_has_unsaved_changes(
        &app,
        &config,
        &app.task_board_settings.workflow_id,
    ));
}

#[test]
fn embedded_stage_assignment_save_is_disabled_after_reverting_to_saved() {
    let (mut app, _task) = crate::app::App::new();
    let saved_assignment = TaskBoardStageAssignment {
        status: TaskStatus::Running,
        primary_member_id: Some("staff-executor".to_string()),
        backup_member_ids: Vec::new(),
        manager_member_ids: Vec::new(),
        autonomous: true,
    };
    app.task_board_staff_config.stage_assignments = vec![saved_assignment.clone()];
    app.task_board_staff_assignment_draft = Some(vec![saved_assignment]);
    let config = app
        .task_board_staff_config
        .clone()
        .retarget_for_workflow_id(&app.task_board_settings.workflow_id)
        .expect("default workflow should retarget");

    assert!(!super::assignment::stage_assignment_has_unsaved_changes(
        &app,
        &config,
        &app.task_board_settings.workflow_id,
    ));
}

#[test]
fn workflow_binding_options_preserve_missing_selection_and_blank_name() {
    let records = vec![vw_gateway_client::WorkflowRecordSummary {
        uuid: "known-workflow".to_string(),
        name: "   ".to_string(),
        description: String::new(),
        created_at_ms: 1,
        updated_at_ms: 1,
    }];

    let labels = super::assignment::workflow_binding_options(&records, Some("missing-workflow"))
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();

    assert_eq!(
        labels,
        vec![
            "未选择 Workflow".to_string(),
            "known-workflow".to_string(),
            "已绑定 missing-workflow".to_string(),
        ]
    );
}

#[test]
fn unknown_workflow_does_not_report_unsaved_stage_assignment_changes() {
    let (mut app, _task) = crate::app::App::new();
    app.task_board_staff_assignment_draft = Some(vec![TaskBoardStageAssignment {
        status: TaskStatus::Running,
        primary_member_id: Some("staff-executor".to_string()),
        backup_member_ids: Vec::new(),
        manager_member_ids: Vec::new(),
        autonomous: true,
    }]);

    assert!(!super::assignment::stage_assignment_has_unsaved_changes(
        &app,
        &app.task_board_staff_config,
        "missing-workflow",
    ));
}

#[test]
fn embedded_stage_assignment_save_is_enabled_after_stage_action_change() {
    let (mut app, _task) = crate::app::App::new();
    let mut config = app
        .task_board_staff_config
        .clone()
        .retarget_for_workflow_id(&app.task_board_settings.workflow_id)
        .expect("default workflow should retarget");
    config.stage_actions.clear();
    app.task_board_staff_stage_action_draft = Some(vec![TaskBoardStageAction {
        status: TaskStatus::Running,
        capability_id: CODE_REVIEW_CAPABILITY_ID.to_string(),
        ..Default::default()
    }]);

    assert!(super::assignment::stage_assignment_has_unsaved_changes(
        &app,
        &config,
        &app.task_board_settings.workflow_id,
    ));
}

#[test]
fn assignment_panel_builds_dispatch_states_for_custom_capability_workflow_errors_and_reset_confirm()
{
    let (mut app, _task) = crate::app::App::new();
    app.task_board_staff_orchestration_tab = TaskBoardStaffOrchestrationTab::Dispatch;
    app.task_board_staff_stage_action_reset_confirm = true;
    app.task_board_staff_workflow_options_error = Some("gateway down".to_string());
    app.task_board_staff_capability_catalog = vec![custom_release_capability(true)];
    app.task_board_staff_stage_action_draft = Some(vec![TaskBoardStageAction {
        status: TaskStatus::Running,
        capability_id: "release_workflow".to_string(),
        workflow_uuid: Some("missing-workflow".to_string()),
        input_parameters: std::collections::BTreeMap::from([
            ("channel".to_string(), serde_json::json!("stable")),
            ("count".to_string(), serde_json::json!(3)),
        ]),
    }]);
    app.task_board_staff_assignment_draft = Some(vec![TaskBoardStageAssignment {
        status: TaskStatus::Running,
        primary_member_id: Some("staff-executor".to_string()),
        backup_member_ids: Vec::new(),
        manager_member_ids: Vec::new(),
        autonomous: true,
    }]);

    let view = super::assignment::build_assignment_panel(
        &app,
        iced::Length::Fixed(420.0),
        iced::Length::Fixed(540.0),
    );
    let _ = std::hint::black_box(view);

    let options = super::assignment::stage_action_capability_options(
        &app.task_board_staff_capability_catalog,
    );
    assert_eq!(options[0].to_string(), "无能力绑定");
    assert!(options.iter().any(|option| option.to_string() == "Release (release_workflow)"));

    let workflow_options = super::assignment::workflow_binding_options(
        &app.task_board_staff_workflow_options,
        Some("missing-workflow"),
    );
    assert!(workflow_options.iter().any(|option| option.to_string() == "已绑定 missing-workflow"));
}

#[test]
fn assignment_panel_builds_staffing_states_for_empty_members_and_each_assignment_tab() {
    let (mut app, _task) = crate::app::App::new();
    let mut config = app.task_board_staff_config.clone();
    for member in &mut config.members {
        member.enabled = false;
    }
    config.stage_assignments = vec![TaskBoardStageAssignment {
        status: TaskStatus::Running,
        primary_member_id: Some("staff-executor".to_string()),
        backup_member_ids: vec!["staff-reviewer".to_string()],
        manager_member_ids: vec!["staff-orchestrator".to_string()],
        autonomous: false,
    }];
    app.task_board_staff_orchestration_tab = TaskBoardStaffOrchestrationTab::Staffing;

    for tab in [
        TaskBoardStaffAssignmentTab::Primary,
        TaskBoardStaffAssignmentTab::Manager,
        TaskBoardStaffAssignmentTab::Backup,
    ] {
        app.task_board_staff_assignment_tabs.insert(TaskStatus::Running, tab);
        let view = super::assignment::build_embedded_orchestration_panel(
            &app,
            &config,
            &app.task_board_settings.workflow_id,
            iced::Length::Fixed(420.0),
            iced::Length::Fixed(540.0),
            Some(crate::app::Message::None),
        );
        let _ = std::hint::black_box(view);
    }
}

#[test]
fn assignment_panel_builds_empty_workflow_fallback_and_embedded_unsaved_save_state() {
    let (mut app, _task) = crate::app::App::new();
    let mut config = app.task_board_staff_config.clone();
    config.workflow.clear();
    config.stage_assignments.clear();
    config.stage_actions.clear();
    app.task_board_staff_assignment_draft = Some(vec![TaskBoardStageAssignment {
        status: TaskStatus::Planning,
        primary_member_id: Some("staff-planner".to_string()),
        backup_member_ids: Vec::new(),
        manager_member_ids: Vec::new(),
        autonomous: false,
    }]);

    let view = super::assignment::build_embedded_orchestration_panel(
        &app,
        &config,
        &app.task_board_settings.workflow_id,
        iced::Length::Fixed(380.0),
        iced::Length::Fixed(500.0),
        Some(crate::app::Message::None),
    );
    let _ = std::hint::black_box(view);

    assert!(super::assignment::stage_assignment_has_unsaved_changes(
        &app,
        &config,
        &app.task_board_settings.workflow_id,
    ));
}

#[test]
fn stage_assignment_unsaved_changes_ignores_empty_out_of_workflow_and_order_only_drafts() {
    let (mut app, _task) = crate::app::App::new();
    let out_of_workflow_status =
        TaskStatus::parse_key("content_idea").expect("custom content status");
    let mut config = app
        .task_board_staff_config
        .clone()
        .retarget_for_workflow_id(&app.task_board_settings.workflow_id)
        .expect("retarget default workflow");
    config.stage_assignments = vec![
        TaskBoardStageAssignment {
            status: TaskStatus::Running,
            primary_member_id: Some("staff-executor".to_string()),
            backup_member_ids: Vec::new(),
            manager_member_ids: Vec::new(),
            autonomous: false,
        },
        TaskBoardStageAssignment {
            status: TaskStatus::Planning,
            primary_member_id: Some("staff-planner".to_string()),
            backup_member_ids: Vec::new(),
            manager_member_ids: Vec::new(),
            autonomous: true,
        },
    ];
    config.stage_actions = vec![TaskBoardStageAction {
        status: TaskStatus::Running,
        capability_id: CODE_REVIEW_CAPABILITY_ID.to_string(),
        ..Default::default()
    }];
    app.task_board_staff_assignment_draft = Some(vec![
        TaskBoardStageAssignment {
            status: out_of_workflow_status,
            primary_member_id: Some("ignored".to_string()),
            backup_member_ids: Vec::new(),
            manager_member_ids: Vec::new(),
            autonomous: true,
        },
        config.stage_assignments[1].clone(),
        TaskBoardStageAssignment {
            status: TaskStatus::CodeReview,
            primary_member_id: None,
            backup_member_ids: Vec::new(),
            manager_member_ids: Vec::new(),
            autonomous: false,
        },
        config.stage_assignments[0].clone(),
    ]);
    app.task_board_staff_stage_action_draft = Some(vec![
        TaskBoardStageAction {
            status: out_of_workflow_status,
            capability_id: SAVED_WORKFLOW_CAPABILITY_ID.to_string(),
            workflow_uuid: Some("ignored".to_string()),
            ..Default::default()
        },
        config.stage_actions[0].clone(),
        TaskBoardStageAction { status: TaskStatus::Planning, ..Default::default() },
    ]);

    assert!(!super::assignment::stage_assignment_has_unsaved_changes(
        &app,
        &config,
        &app.task_board_settings.workflow_id,
    ));
}

fn custom_release_capability(requires_workflow_uuid: bool) -> TaskBoardStaffCapability {
    TaskBoardStaffCapability {
        id: "release_workflow".to_string(),
        label: "Release".to_string(),
        capability_type: TaskBoardStaffCapabilityKind::SavedWorkflow,
        built_in: false,
        supported_roles: Vec::new(),
        requires_workflow_uuid,
        input_summary: TaskBoardStaffCapabilitySummary {
            description: "Inputs".to_string(),
            fields: vec!["workflow_uuid".to_string(), "channel".to_string(), "count".to_string()],
        },
        result_summary: TaskBoardStaffCapabilitySummary::default(),
        input_contract: "workflow + channel".to_string(),
        result_semantics: "release summary".to_string(),
        permission_hint: "workflow scope".to_string(),
    }
}
