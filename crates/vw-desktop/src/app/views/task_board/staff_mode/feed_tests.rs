//! 员工模式动态流的测试，覆盖事件排序、摘要格式和空状态。
//! fixture 包含不同事件类型与时间边界，验证过滤及主题化卡片构建。
use super::*;
use crate::app::task::TaskExecutorBackend;
use crate::app::task::staff::{
    TaskBoardStaffActionAvailability, TaskBoardStaffLane, TaskBoardStaffMemberLoad,
    TaskBoardStaffRole, TaskBoardStaffStatus,
};

#[test]
fn active_staff_mention_query_handles_prefix_latest_mention_and_closed_query() {
    assert_eq!(active_staff_mention_query("   @planner"), Some("planner"));
    assert_eq!(active_staff_mention_query("转交 @reviewer"), Some("reviewer"));
    assert_eq!(active_staff_mention_query("@planner 转交"), None);
    assert_eq!(active_staff_mention_query("没有提及"), None);
}

#[test]
fn task_status_filters_exclude_archived_by_default_and_honor_explicit_selection() {
    assert!(task_status_filter_matches(&[], TaskStatus::Running));
    assert!(!task_status_filter_matches(&[], TaskStatus::Archived));
    assert!(task_status_filter_matches(&[TaskStatus::Archived], TaskStatus::Archived,));
    assert!(!task_status_filter_matches(&[TaskStatus::Running], TaskStatus::Paused,));

    let counts = vec![(TaskStatus::Running, 3), (TaskStatus::Paused, 2), (TaskStatus::Archived, 7)];
    assert_eq!(task_status_count(&counts, TaskStatus::Running), 3);
    assert_eq!(task_status_count(&counts, TaskStatus::Failed), 0);
    assert_eq!(non_archived_task_status_count(&counts), 5);
}

#[test]
fn feed_labels_cover_member_status_and_elapsed_time_boundaries() {
    assert_eq!(feed_subtitle(Some("执行员"), &[]), "按 执行员 筛选当前任务");
    assert_eq!(
        feed_subtitle(Some("执行员"), &[TaskStatus::Running]),
        "按 执行员 和 执行中 筛选任务",
    );
    assert_eq!(
        feed_subtitle(None, &[TaskStatus::Running, TaskStatus::Paused]),
        "按 2 类任务 筛选当前任务",
    );

    assert_eq!(updated_at_label(59_999, 0), "刚刚更新");
    assert_eq!(updated_at_label(60_000, 0), "1 分钟前");
    assert_eq!(updated_at_label(3_600_000, 0), "1 小时前");
    assert_eq!(updated_at_label(86_400_000, 0), "1 天前");
    assert_eq!(updated_at_label(1_000, 2_000), "刚刚更新");
}

#[test]
fn feed_filters_sort_assignments_and_count_statuses_by_member() {
    let projection = projection(
        vec![
            assignment(
                "T-old",
                TaskStatus::Running,
                Some("exec-1"),
                Some(TaskBoardStaffRole::Executor),
                1_000,
                3,
            ),
            assignment(
                "T-new",
                TaskStatus::Running,
                Some("exec-1"),
                Some(TaskBoardStaffRole::Executor),
                3_000,
                9,
            ),
            assignment(
                "T-tie",
                TaskStatus::Running,
                Some("exec-1"),
                Some(TaskBoardStaffRole::Executor),
                3_000,
                1,
            ),
            assignment(
                "T-paused",
                TaskStatus::Paused,
                Some("reviewer-1"),
                Some(TaskBoardStaffRole::Reviewer),
                2_000,
                1,
            ),
            assignment(
                "T-archived",
                TaskStatus::Archived,
                Some("exec-1"),
                Some(TaskBoardStaffRole::Executor),
                9_000,
                1,
            ),
        ],
        Vec::new(),
    );

    let all_visible = filtered_assignments(&projection, None, &[]);
    assert_eq!(
        all_visible.iter().map(|assignment| assignment.task_id.as_str()).collect::<Vec<_>>(),
        vec!["T-tie", "T-new", "T-paused", "T-old"]
    );

    let running_for_exec =
        filtered_assignments(&projection, Some("exec-1"), &[TaskStatus::Running]);
    assert_eq!(
        running_for_exec.iter().map(|assignment| assignment.task_id.as_str()).collect::<Vec<_>>(),
        vec!["T-tie", "T-new", "T-old"]
    );

    let counts = task_status_counts(&projection, Some("exec-1"));
    assert_eq!(task_status_count(&counts, TaskStatus::Running), 3);
    assert_eq!(task_status_count(&counts, TaskStatus::Archived), 1);
    assert_eq!(task_status_count(&counts, TaskStatus::Paused), 0);
}

#[test]
fn feed_command_helpers_create_expected_staff_messages() {
    let projection = projection(
        Vec::new(),
        vec![member_load("exec-1", "Executor", TaskBoardStaffRole::Executor, true, 0, 2)],
    );
    let now_ms = 42_000;

    let unassigned = assignment(
        "T-assign",
        TaskStatus::Planning,
        None,
        Some(TaskBoardStaffRole::Executor),
        1_000,
        1,
    );
    assert!(matches!(
        assign_or_take_over_message(&unassigned, &projection, now_ms),
        Some(Message::TaskBoard(TaskBoardMessage::SubmitStaffCommand(
            TaskBoardStaffCommand::AssignTask { task_id, member_id, .. }
        ))) if task_id == "T-assign" && member_id == "exec-1"
    ));

    let takeover = assignment(
        "T-takeover",
        TaskStatus::Planning,
        Some("exec-1"),
        Some(TaskBoardStaffRole::Executor),
        1_000,
        1,
    );
    assert!(matches!(
        assign_or_take_over_message(&takeover, &projection, now_ms),
        Some(Message::TaskBoard(TaskBoardMessage::SubmitStaffCommand(
            TaskBoardStaffCommand::TakeOverTask { task_id, member_id, .. }
        ))) if task_id == "T-takeover" && member_id == "exec-1"
    ));

    assert!(matches!(
        start_message(&takeover, now_ms),
        Some(Message::TaskBoard(TaskBoardMessage::SubmitStaffCommand(
            TaskBoardStaffCommand::StartTask { task_id, member_id, .. }
        ))) if task_id == "T-takeover" && member_id == "exec-1"
    ));

    let pause = assignment_with_actions(
        "T-pause",
        TaskStatus::Running,
        Some("exec-1"),
        Some(TaskBoardStaffRole::Executor),
        1_000,
        1,
        TaskBoardStaffActionAvailability { can_pause: true, ..actions() },
    );
    assert!(matches!(
        pause_or_resume_message(&pause, now_ms),
        Some(Message::TaskBoard(TaskBoardMessage::SubmitStaffCommand(
            TaskBoardStaffCommand::PauseTask { task_id, member_id, .. }
        ))) if task_id == "T-pause" && member_id == "exec-1"
    ));

    let resume = assignment_with_actions(
        "T-resume",
        TaskStatus::Paused,
        Some("exec-1"),
        Some(TaskBoardStaffRole::Executor),
        1_000,
        1,
        TaskBoardStaffActionAvailability { can_resume: true, ..actions() },
    );
    assert!(matches!(
        pause_or_resume_message(&resume, now_ms),
        Some(Message::TaskBoard(TaskBoardMessage::SubmitStaffCommand(
            TaskBoardStaffCommand::ResumeTask { task_id, member_id, .. }
        ))) if task_id == "T-resume" && member_id == "exec-1"
    ));

    let logs = open_logs_command(&takeover, now_ms);
    assert!(
        matches!(logs, TaskBoardStaffCommand::OpenTaskLogs { task_id, .. } if task_id == "T-takeover")
    );
}

#[test]
fn feed_builds_empty_filtered_queued_and_mention_overlay_states() {
    let members = vec![
        member_load("planner-1", "Planner One", TaskBoardStaffRole::Planner, true, 0, 2),
        member_load("exec-1", "Executor One", TaskBoardStaffRole::Executor, true, 1, 2),
        member_load(
            "exec-disabled",
            "Executor Disabled",
            TaskBoardStaffRole::Executor,
            false,
            0,
            2,
        ),
    ];
    let empty_projection = projection(Vec::new(), members.clone());
    let empty_queue = TaskBoardStaffCommandQueue::default();
    let _ = build_feed_panel(
        &empty_projection,
        &empty_queue,
        &[],
        TaskBoardStaffFeedTab::Tasks,
        None,
        &[],
        "@exec",
        StaffMatrixBridgeSnapshot { enabled: false, cursor: 0, processed_count: 0 },
        Length::Fixed(320.0),
    );

    let assignment = assignment(
        "T-card",
        TaskStatus::Running,
        Some("exec-1"),
        Some(TaskBoardStaffRole::Executor),
        2_000,
        1,
    );
    let projection = projection(vec![assignment.clone()], members);
    let mut queue = TaskBoardStaffCommandQueue::default();
    queue.enqueue(
        TaskBoardStaffCommand::AssignTask {
            command_id: "queued-assign".to_string(),
            task_id: "T-card".to_string(),
            member_id: "exec-1".to_string(),
        },
        1,
    );
    assert!(queue.has_active_task_command("T-card", TaskBoardStaffCommandGroup::Assignment));

    let _ = build_assignment_card(&assignment, &projection, &queue, 4_000);
    let _ = build_feed_panel(
        &projection,
        &queue,
        &[],
        TaskBoardStaffFeedTab::Tasks,
        Some("exec-1"),
        &[TaskStatus::Running],
        "@exec",
        StaffMatrixBridgeSnapshot { enabled: true, cursor: 7, processed_count: 3 },
        Length::Fixed(360.0),
    );
    let _ = task_filter_bar(
        Some("exec-1"),
        Some("Executor One"),
        &[TaskStatus::Archived],
        task_status_counts(&projection, Some("exec-1")),
        0,
        0,
        9_000,
    );
    let _ = matrix_bridge_chip(StaffMatrixBridgeSnapshot {
        enabled: true,
        cursor: 7,
        processed_count: 3,
    });
    let _ = build_staff_command_bar(&projection, "@exec");

    let candidates = staff_mention_candidates(&projection, "@");
    assert_eq!(
        candidates.iter().map(|member| member.member_id.as_str()).collect::<Vec<_>>(),
        vec!["exec-1", "planner-1"]
    );
    let _ = build_staff_mention_dropdown(candidates);
}

#[test]
fn feed_event_content_and_task_preview_cover_fallbacks() {
    let command_event = TaskBoardStaffEvent::command_event(
        "cmd-1",
        None,
        None,
        crate::app::task::staff::TaskBoardStaffCommandStatus::Pending,
        1_000,
        "queued",
        None,
    );
    let command_content = event_card_content_for_tests(&command_event, 62_000);
    assert_eq!(command_content.status_label, "待处理");
    assert_eq!(command_content.delivery_label, None);
    assert!(command_content.meta.contains("全局 · 队列 · 1 分钟前"));
    assert_eq!(command_content.message, "queued");

    let message_event = TaskBoardStaffEvent {
        kind: TaskBoardStaffEventKind::Message,
        command_id: String::new(),
        task_id: Some("T-message".to_string()),
        member_id: Some("exec-1".to_string()),
        status: crate::app::task::staff::TaskBoardStaffEventStatus::Message,
        timestamp_ms: 1_000,
        message: "fallback body".to_string(),
        orchestration: None,
        staff_message: None,
    };
    let message_content = event_card_content_for_tests(&message_event, 1_500);
    assert_eq!(message_content.status_label, "消息");
    assert_eq!(message_content.delivery_label, None);
    assert!(message_content.meta.contains("T-message · exec-1 · 刚刚更新"));
    assert_eq!(message_content.message, "fallback body");

    let fallback = assignment_with_description("T-empty", " \n\t ");
    assert_eq!(task_preview(&fallback), "T-empty");

    let long =
        assignment_with_description("T-long", &format!("{}\n{}", "x".repeat(90), "y".repeat(90)));
    let preview = task_preview(&long);
    assert_eq!(preview.chars().count(), TASK_PREVIEW_CHARS + 1);
    assert!(preview.ends_with('…'));
    assert!(!preview.contains('\n'));
}

fn projection(
    assignments: Vec<TaskBoardStaffAssignment>,
    member_loads: Vec<TaskBoardStaffMemberLoad>,
) -> TaskBoardStaffProjection {
    let task_count = assignments.len();
    let blocked_count = assignments.iter().filter(|assignment| assignment.blocked).count();
    TaskBoardStaffProjection {
        lanes: vec![TaskBoardStaffLane {
            status: TaskStatus::Running,
            role: Some(TaskBoardStaffRole::Executor),
            display_only: false,
            assignments,
            task_count,
            blocked_count,
        }],
        member_loads,
        unassigned: Vec::new(),
        total_task_count: task_count,
        blocked_task_count: blocked_count,
    }
}

fn assignment(
    task_id: &str,
    status: TaskStatus,
    member_id: Option<&str>,
    role: Option<TaskBoardStaffRole>,
    updated_at_ms: u64,
    order: u32,
) -> TaskBoardStaffAssignment {
    assignment_with_actions(task_id, status, member_id, role, updated_at_ms, order, actions())
}

fn assignment_with_actions(
    task_id: &str,
    status: TaskStatus,
    member_id: Option<&str>,
    role: Option<TaskBoardStaffRole>,
    updated_at_ms: u64,
    order: u32,
    actions: TaskBoardStaffActionAvailability,
) -> TaskBoardStaffAssignment {
    TaskBoardStaffAssignment {
        task_id: task_id.to_string(),
        task_status: status,
        task_description: format!("{task_id} description"),
        priority: 2,
        order,
        updated_at_ms,
        role,
        member_id: member_id.map(ToOwned::to_owned),
        member_name: member_id.map(|value| format!("{value} name")),
        member_role: role,
        display_only: false,
        blocked: false,
        block_reason: None,
        actions,
    }
}

fn assignment_with_description(task_id: &str, description: &str) -> TaskBoardStaffAssignment {
    let mut assignment = assignment(task_id, TaskStatus::Running, None, None, 0, 0);
    assignment.task_description = description.to_string();
    assignment
}

fn actions() -> TaskBoardStaffActionAvailability {
    TaskBoardStaffActionAvailability {
        can_assign: true,
        can_start: true,
        can_pause: false,
        can_resume: false,
        can_open_logs: true,
        can_filter_member: true,
    }
}

fn member_load(
    member_id: &str,
    member_name: &str,
    role: TaskBoardStaffRole,
    enabled: bool,
    load_count: usize,
    capacity: u32,
) -> TaskBoardStaffMemberLoad {
    TaskBoardStaffMemberLoad {
        member_id: member_id.to_string(),
        member_name: member_name.to_string(),
        role,
        status: TaskBoardStaffStatus::Online,
        enabled,
        capacity,
        executor_backend: Some(TaskExecutorBackend::Codex),
        delegate_agent: None,
        acp_agent: None,
        task_ids: Vec::new(),
        running_task_ids: Vec::new(),
        blocked_task_ids: Vec::new(),
        load_count,
        over_capacity: load_count >= capacity as usize,
    }
}
