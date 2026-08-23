//! 员工模式视图的组合测试，覆盖名册、泳道、动态流和分配面板。
//! 不同运行状态的 fixture 用于验证整体分栏及可选覆盖层构建。
use crate::app::state::{
    TaskBoardStaffAssignmentTab, TaskBoardStaffOrchestrationTab, TaskBoardViewMode,
};
use crate::app::task::staff::{
    TaskBoardStaffEvent, TaskBoardStaffMember, TaskBoardStaffMessageDeliveryStatus,
    TaskBoardStaffRole, TaskBoardStaffStatus,
};
use crate::app::task::{Task, TaskBoardStaffConfig, TaskStatus};

#[test]
fn staff_mode_view_builds_from_projection() {
    let app = app_with_staff_roster(1200.0, TaskBoardStaffConfig::default().members);

    let view = super::build_staff_mode_view(&app);

    assert_eq!(app.task_board_staff_projection.total_task_count, 1);
    let _ = std::hint::black_box(view);
}

#[test]
fn staff_mode_view_builds_roster_layout_variants() {
    let cases = [
        (1500.0, 24, staff_members(24)),
        (1100.0, 2, staff_members(2)),
        (720.0, 18, staff_members(18)),
    ];

    for (window_width, expected_members, members) in cases {
        let app = app_with_staff_roster(window_width, members);
        let view = super::build_staff_mode_view(&app);

        assert_eq!(app.task_board_staff_projection.member_loads.len(), expected_members);
        let _ = std::hint::black_box(view);
    }
}

#[test]
fn staff_mode_view_builds_task_flow_tabs_and_status_filter() {
    let mut app = app_with_staff_roster(1280.0, TaskBoardStaffConfig::default().members);

    app.task_board_staff_task_status_filter = vec![TaskStatus::Pool, TaskStatus::Planning];
    let view = super::build_staff_mode_view(&app);
    let _ = std::hint::black_box(view);
}

#[test]
fn staff_command_mention_candidates_filter_enabled_members() {
    let enabled = TaskBoardStaffMember::new(
        "staff-special-live",
        "Special Live",
        TaskBoardStaffRole::Executor,
    );
    let mut disabled = TaskBoardStaffMember::new(
        "staff-special-disabled",
        "Special Disabled",
        TaskBoardStaffRole::Executor,
    );
    disabled.enabled = false;
    let app = app_with_staff_roster(1280.0, vec![enabled, disabled]);

    let candidates =
        super::feed::staff_mention_candidates(&app.task_board_staff_projection, "@special");
    let ids = candidates.iter().map(|member| member.member_id.as_str()).collect::<Vec<_>>();
    assert_eq!(ids, vec!["staff-special-live"]);

    let closed =
        super::feed::staff_mention_candidates(&app.task_board_staff_projection, "@special 执行");
    assert!(closed.is_empty());
}

#[test]
fn staff_feed_renders_message_events() {
    let mut app = app_with_staff_roster(1280.0, TaskBoardStaffConfig::default().members);
    app.task_board_staff_events = vec![TaskBoardStaffEvent::staff_message(
        "T-feed-message",
        "staff-planner",
        "staff-executor",
        "规划交接完成，请接手执行。",
        1_000,
        Some(TaskBoardStaffMessageDeliveryStatus::Delivered),
    )];

    let content = super::feed::event_card_content_for_tests(&app.task_board_staff_events[0], 1_500);
    assert_eq!(content.status_label, "消息");
    assert_eq!(content.delivery_label.as_deref(), Some("已送达"));
    assert!(content.meta.contains("T-feed-message"));
    assert!(content.meta.contains("staff-planner -> staff-executor"));
    assert_eq!(content.message, "规划交接完成，请接手执行。");

    let view = super::build_staff_mode_view(&app);
    let _ = std::hint::black_box(view);
}

#[test]
fn staff_mode_view_builds_assignment_panel_tabs() {
    let mut app = app_with_staff_roster(1280.0, TaskBoardStaffConfig::default().members);
    app.task_board_staff_assignment_open = true;
    app.task_board_staff_assignment_tabs
        .insert(TaskStatus::Running, TaskBoardStaffAssignmentTab::Manager);
    app.task_board_staff_orchestration_tab = TaskBoardStaffOrchestrationTab::Staffing;

    let view = super::build_staff_mode_view(&app);

    let _ = std::hint::black_box(view);
}

#[test]
fn staff_scrollbar_reserves_requested_gutter() {
    assert_eq!(super::styles::STAFF_SCROLLBAR_WIDTH, 4);
    assert_eq!(super::styles::STAFF_SCROLLBAR_GUTTER, 7);
    assert!(super::styles::STAFF_SCROLLBAR_GUTTER >= super::styles::STAFF_SCROLLBAR_WIDTH);
}

fn app_with_staff_roster(window_width: f32, members: Vec<TaskBoardStaffMember>) -> crate::app::App {
    let (mut app, _task) = crate::app::App::new();
    let mut task = Task::new(5);
    task.id = "T-staff-layout".to_string();
    task.description = "Render staff mode layout from projection".to_string();
    task.status = TaskStatus::Planning;

    app.window_size = (window_width, 700.0);
    app.task_board_view_mode = TaskBoardViewMode::Staff;
    app.task_board_tasks = vec![task];
    app.task_board_staff_config =
        TaskBoardStaffConfig { members, ..TaskBoardStaffConfig::default() };
    app.task_board_staff_projection = crate::app::task::staff::build_staff_mode_projection(
        &app.task_board_tasks,
        &app.task_board_staff_config,
        &app.task_board_executor.running_tasks,
        &app.task_board_settings.workflow_id,
    )
    .expect("staff mode test projection");
    app
}

fn staff_members(count: usize) -> Vec<TaskBoardStaffMember> {
    let roles = TaskBoardStaffRole::all();
    (0..count)
        .map(|index| {
            let role = roles[index % roles.len()];
            let id = format!("staff-layout-{index}");
            let name = format!("Staff Layout Member {index} with a deliberately long display name");
            let mut member = TaskBoardStaffMember::new(&id, &name, role);
            member.capacity = 3;
            member.status = if index % 7 == 0 {
                TaskBoardStaffStatus::Blocked
            } else {
                TaskBoardStaffStatus::Online
            };
            member.acp_agent = Some(format!(
                "acp-agent-binding-for-member-{index}-with-long-unbroken-token-to-wrap"
            ));
            member.current_task_ids = vec![format!("external-task-{index}")];
            member
        })
        .collect()
}
