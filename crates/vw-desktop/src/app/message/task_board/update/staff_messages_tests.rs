//! 员工交接本地消息的状态机校验测试。

use super::maybe_record_task_handoff_message;
use crate::app::state::TaskBoardViewMode;
use crate::app::task::staff::{
    TaskBoardStaffConfig, TaskBoardStaffEvent, TaskBoardStaffMember,
    TaskBoardStaffMessageDeliveryStatus, TaskBoardStaffRole, TaskBoardStageAssignment,
};
use crate::app::task::{Task, TaskStatus};
use serde_json::Value;

fn staff_member(id: &str, name: &str, role: TaskBoardStaffRole) -> TaskBoardStaffMember {
    TaskBoardStaffMember::new(id, name, role)
}

fn staff_config_for_review_handoff() -> TaskBoardStaffConfig {
    let mut config = TaskBoardStaffConfig::default();
    config.members = vec![
        staff_member("executor", "Executor", TaskBoardStaffRole::Executor),
        staff_member("reviewer", "Reviewer", TaskBoardStaffRole::Reviewer),
    ];
    config.stage_assignments = vec![
        TaskBoardStageAssignment {
            status: TaskStatus::Running,
            primary_member_id: Some("executor".to_string()),
            backup_member_ids: Vec::new(),
            manager_member_ids: Vec::new(),
            autonomous: false,
        },
        TaskBoardStageAssignment {
            status: TaskStatus::CodeReview,
            primary_member_id: Some("reviewer".to_string()),
            backup_member_ids: Vec::new(),
            manager_member_ids: Vec::new(),
            autonomous: false,
        },
    ];
    config
}

fn staff_app_with_task(task: Task, config: TaskBoardStaffConfig) -> crate::app::App {
    let (mut app, _task) = crate::app::App::new();
    app.task_board_view_mode = TaskBoardViewMode::Staff;
    app.task_board_staff_config = config;
    app.task_board_tasks = vec![task];
    app
}

#[test]
fn handoff_skips_invalid_or_unchanged_transition() {
    let config = staff_config_for_review_handoff();

    // 合法交接：Running -> CodeReview 沿状态机前向可达（Running -> CodeComplete -> CodeReview），
    // from 阶段员工为 Running 的 executor，to 阶段收件员工为 CodeReview 的 reviewer，应生成交接消息。
    let mut valid_task = Task::new(1);
    valid_task.status = TaskStatus::CodeReview;
    let valid_id = valid_task.id.clone();
    let mut app = staff_app_with_task(valid_task.clone(), config.clone());
    let _ = maybe_record_task_handoff_message(
        &mut app,
        &valid_task,
        TaskStatus::Running,
        TaskStatus::CodeReview,
    );
    let valid_messages = app
        .task_board_staff_events
        .iter()
        .filter(|event| {
            event.staff_message.as_ref().is_some_and(|message| message.task_id == valid_id)
        })
        .count();
    assert_eq!(valid_messages, 1, "合法状态机交接应生成一条交接消息");

    // 非法交接：Running -> Pending 不在任何意图的合法边上，状态机拒绝，不应生成交接消息。
    let mut invalid_task = Task::new(2);
    invalid_task.status = TaskStatus::Pending;
    let invalid_id = invalid_task.id.clone();
    let mut app = staff_app_with_task(invalid_task.clone(), config.clone());
    let _ = maybe_record_task_handoff_message(
        &mut app,
        &invalid_task,
        TaskStatus::Running,
        TaskStatus::Pending,
    );
    let invalid_messages = app
        .task_board_staff_events
        .iter()
        .filter(|event| {
            event.staff_message.as_ref().is_some_and(|message| message.task_id == invalid_id)
        })
        .count();
    assert_eq!(invalid_messages, 0, "非法状态机交接不应生成交接消息");
    let invalid_task_in_app =
        app.task_board_tasks.iter().find(|task| task.id == invalid_id).expect("invalid task");
    assert!(
        invalid_task_in_app
            .logs
            .iter()
            .any(|entry| entry.message.contains("状态机") && entry.message.contains("交接")),
        "非法交接应记录状态机跳过日志"
    );

    // 未变化交接：from == to 直接跳过，不生成交接消息。
    let mut unchanged_task = Task::new(3);
    unchanged_task.status = TaskStatus::CodeReview;
    let unchanged_id = unchanged_task.id.clone();
    let mut app = staff_app_with_task(unchanged_task.clone(), config);
    let _ = maybe_record_task_handoff_message(
        &mut app,
        &unchanged_task,
        TaskStatus::CodeReview,
        TaskStatus::CodeReview,
    );
    let unchanged_messages = app
        .task_board_staff_events
        .iter()
        .filter(|event| {
            event.staff_message.as_ref().is_some_and(|message| message.task_id == unchanged_id)
        })
        .count();
    assert_eq!(unchanged_messages, 0, "未变化状态不应生成交接消息");
}

#[test]
fn stage_assignment_falls_back_to_the_first_enabled_member() {
    let mut config = TaskBoardStaffConfig::default();
    let mut disabled = staff_member("primary", "Primary", TaskBoardStaffRole::Executor);
    disabled.enabled = false;
    config.members = vec![
        disabled,
        staff_member("backup", "Backup", TaskBoardStaffRole::Executor),
        staff_member("manager", "Manager", TaskBoardStaffRole::Orchestrator),
    ];
    config.stage_assignments = vec![TaskBoardStageAssignment {
        status: TaskStatus::Running,
        primary_member_id: Some("primary".to_string()),
        backup_member_ids: vec!["missing".to_string(), "backup".to_string()],
        manager_member_ids: vec!["manager".to_string()],
        autonomous: false,
    }];

    let selected = super::stage_assignment_member(&config, TaskStatus::Running)
        .expect("enabled backup should be selected");

    assert_eq!(selected.id, "backup");
    assert!(super::enabled_member(&config, "  ").is_none());
}

#[test]
fn handoff_body_truncation_counts_unicode_characters() {
    let body = "交".repeat(super::STAFF_HANDOFF_BODY_MAX_CHARS + 10);

    let truncated = super::truncate_handoff_body(body);

    assert_eq!(truncated.chars().count(), super::STAFF_HANDOFF_BODY_MAX_CHARS);
    assert!(truncated.chars().all(|character| character == '交'));
}

#[test]
fn ipc_payload_contains_handoff_metadata_and_rejects_oversized_body() {
    let task = Task::new(9);
    let from = staff_member("executor", "Executor", TaskBoardStaffRole::Executor);
    let to = staff_member("reviewer", "Reviewer", TaskBoardStaffRole::Reviewer);

    let payload = super::build_staff_handoff_ipc_payload(
        &task,
        TaskStatus::Running,
        TaskStatus::CodeReview,
        &from,
        &to,
        "handoff body",
    )
    .expect("valid payload");
    let value: Value = serde_json::from_str(&payload).expect("payload json");

    assert_eq!(value["kind"], "staff_handoff");
    assert_eq!(value["task_id"].as_str(), Some(task.id.as_str()));
    assert_eq!(value["from_member_id"], "executor");
    assert_eq!(value["to_member_id"], "reviewer");
    assert_eq!(value["from_status"], "running");
    assert_eq!(value["to_status"], "code_review");
    assert_eq!(value["body"], "handoff body");

    let oversized =
        "x".repeat(vw_gateway_client::vw_api_types::staff::STAFF_IPC_MESSAGE_PAYLOAD_LIMIT + 1);
    let error = super::build_staff_handoff_ipc_payload(
        &task,
        TaskStatus::Running,
        TaskStatus::CodeReview,
        &from,
        &to,
        &oversized,
    )
    .expect_err("oversized payload should fail");
    assert!(error.contains("payload must be at most"));
}

#[test]
fn ipc_delivery_result_updates_matching_message_and_task_logs() {
    let task = Task::new(10);
    let task_id = task.id.clone();
    let mut app = staff_app_with_task(task, TaskBoardStaffConfig::default());
    app.task_board_staff_events.push(TaskBoardStaffEvent::staff_message(
        task_id.clone(),
        "executor",
        "reviewer",
        "body",
        123,
        Some(TaskBoardStaffMessageDeliveryStatus::Pending),
    ));

    let _ = super::apply_staff_handoff_ipc_delivery_result(
        &mut app,
        task_id.clone(),
        "executor".to_string(),
        "reviewer".to_string(),
        "reviewer-agent".to_string(),
        123,
        Ok(()),
    );
    let message = app.task_board_staff_events[0].staff_message.as_ref().expect("message");
    assert_eq!(message.delivery_status, Some(TaskBoardStaffMessageDeliveryStatus::Delivered));
    assert!(
        app.task_board_tasks[0]
            .logs
            .iter()
            .any(|entry| entry.message.contains("IPC 交接消息已投递"))
    );

    let _ = super::apply_staff_handoff_ipc_delivery_result(
        &mut app,
        task_id,
        "executor".to_string(),
        "reviewer".to_string(),
        "reviewer-agent".to_string(),
        123,
        Err("network ".repeat(80)),
    );
    let message = app.task_board_staff_events[0].staff_message.as_ref().expect("message");
    assert_eq!(message.delivery_status, Some(TaskBoardStaffMessageDeliveryStatus::Failed));
    assert!(
        app.task_board_tasks[0]
            .logs
            .iter()
            .any(|entry| entry.message.contains("IPC 交接消息投递失败"))
    );
}
