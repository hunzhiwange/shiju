//! 任务看板控制区辅助函数的测试，覆盖工位状态、时长和日志摘要。
//! 边界数据验证状态配色、任务租约匹配以及空日志的回退文案。
#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("helpers_tests"));
}

fn all_worktree_states() -> [desktop_task_executor::executor::state::WorktreeState; 5] {
    [
        desktop_task_executor::executor::state::WorktreeState::Idle,
        desktop_task_executor::executor::state::WorktreeState::Busy,
        desktop_task_executor::executor::state::WorktreeState::Tainted,
        desktop_task_executor::executor::state::WorktreeState::Recycling,
        desktop_task_executor::executor::state::WorktreeState::Dead,
    ]
}

fn slot(task_id: Option<&str>) -> desktop_task_executor::executor::state::WorktreeSlotSnapshot {
    desktop_task_executor::executor::state::WorktreeSlotSnapshot {
        agent_id: None,
        run_id: None,
        id: "slot-1".to_string(),
        path: "/tmp/slot-1".to_string(),
        branch: "feature/task".to_string(),
        state: desktop_task_executor::executor::state::WorktreeState::Idle,
        leased_task_id: task_id.map(str::to_string),
        taint_reason: None,
    }
}

#[test]
fn worktree_state_text_helpers_cover_all_states() {
    let cases = [
        (
            desktop_task_executor::executor::state::WorktreeState::Idle,
            "空闲",
            "新任务到达后可立即执行；脏现场会在执行前统一整理",
            "待复用",
        ),
        (desktop_task_executor::executor::state::WorktreeState::Busy, "忙碌", "当前正在处理任务", "任务处理中"),
        (desktop_task_executor::executor::state::WorktreeState::Tainted, "污染", "需要人工或自动修复", "异常排查中"),
        (desktop_task_executor::executor::state::WorktreeState::Recycling, "回收中", "正在回收与整理", "回收整理中"),
        (desktop_task_executor::executor::state::WorktreeState::Dead, "失效", "当前暂不可用", "槽位停用中"),
    ];

    for (state, label, hint, actor_label) in cases {
        assert_eq!(super::worktree_state_label(state), label);
        assert_eq!(super::worktree_state_hint(state), hint);
        assert_eq!(super::worktree_state_actor(state).1, actor_label);
        assert_eq!(super::worktree_room_props(state).len(), 5);
    }
}

#[test]
fn running_dots_cycles_by_whole_seconds() {
    assert_eq!(super::running_dots(0), "·");
    assert_eq!(super::running_dots(999), "·");
    assert_eq!(super::running_dots(1_000), "··");
    assert_eq!(super::running_dots(2_000), "···");
    assert_eq!(super::running_dots(3_000), "·");
}

#[test]
fn format_duration_ms_uses_seconds_and_zero_fallback() {
    assert_eq!(super::format_duration_ms(0), "0s");
    assert_eq!(super::format_duration_ms(999), "0s");
    assert_eq!(super::format_duration_ms(61_000), "1m 1s");
}

#[test]
fn task_status_tag_colors_cover_builtin_and_custom_statuses() {
    let cases = [
        (
            desktop_task_model::TaskStatus::Pool,
            iced::Color::from_rgb8(107, 114, 128),
            iced::Color::from_rgb8(243, 244, 246),
        ),
        (
            desktop_task_model::TaskStatus::Pending,
            iced::Color::from_rgb8(37, 99, 235),
            iced::Color::from_rgb8(219, 234, 254),
        ),
        (
            desktop_task_model::TaskStatus::Planning,
            iced::Color::from_rgb8(79, 70, 229),
            iced::Color::from_rgb8(224, 231, 255),
        ),
        (
            desktop_task_model::TaskStatus::Running,
            iced::Color::from_rgb8(147, 51, 234),
            iced::Color::from_rgb8(243, 232, 255),
        ),
        (
            desktop_task_model::TaskStatus::Failed,
            iced::Color::from_rgb8(220, 38, 38),
            iced::Color::from_rgb8(254, 226, 226),
        ),
        (
            desktop_task_model::TaskStatus::Paused,
            iced::Color::from_rgb8(202, 138, 4),
            iced::Color::from_rgb8(254, 249, 195),
        ),
        (
            desktop_task_model::TaskStatus::CodeComplete,
            iced::Color::from_rgb8(5, 150, 105),
            iced::Color::from_rgb8(209, 250, 229),
        ),
        (
            desktop_task_model::TaskStatus::CodeReview,
            iced::Color::from_rgb8(217, 119, 6),
            iced::Color::from_rgb8(254, 243, 199),
        ),
        (
            desktop_task_model::TaskStatus::PrSubmitted,
            iced::Color::from_rgb8(8, 145, 178),
            iced::Color::from_rgb8(207, 250, 254),
        ),
        (
            desktop_task_model::TaskStatus::Completed,
            iced::Color::from_rgb8(22, 163, 74),
            iced::Color::from_rgb8(220, 252, 231),
        ),
        (
            desktop_task_model::TaskStatus::Archived,
            iced::Color::from_rgb8(100, 116, 139),
            iced::Color::from_rgb8(241, 245, 249),
        ),
        (
            desktop_task_model::TaskStatus::Custom("blocked"),
            iced::Color::from_rgb8(107, 114, 128),
            iced::Color::from_rgb8(243, 244, 246),
        ),
    ];

    for (status, expected_text, expected_background) in cases {
        assert_eq!(super::task_status_tag_colors(status), (expected_text, expected_background));
    }
}

#[test]
fn worktree_state_palette_uses_light_and_dark_branches() {
    let light = iced::Theme::Light;
    let dark = iced::Theme::Dark;

    for state in all_worktree_states() {
        let light_palette = super::worktree_state_palette(&light, state);
        let dark_palette = super::worktree_state_palette(&dark, state);

        assert_ne!(light_palette, dark_palette);
    }

    assert_eq!(
        super::worktree_state_palette(&light, desktop_task_executor::executor::state::WorktreeState::Idle).0,
        iced::Color::from_rgb(0.69, 0.92, 0.73)
    );
    assert_eq!(
        super::worktree_state_palette(&dark, desktop_task_executor::executor::state::WorktreeState::Dead).1,
        iced::Color::from_rgb(0.83, 0.85, 0.90)
    );
}

#[test]
fn truncate_log_line_trims_limits_and_handles_empty_input() {
    assert_eq!(super::truncate_log_line("  ok  ", 10), "ok");
    assert_eq!(super::truncate_log_line("abcdef", 3), "abc...");
    assert_eq!(super::truncate_log_line("  ", 10), "...");
    assert_eq!(super::truncate_log_line("界面日志", 2), "界面...");
    assert_eq!(super::truncate_log_line("abc", 0), "...");
}

#[test]
fn slot_task_matches_leased_task_id_only() {
    let (mut app, _task) = crate::control::test_support::TestApp::new();
    let mut task = desktop_task_model::Task::new(1);
    task.id = "task-1".to_string();
    app.task_board_tasks = vec![task];

    assert_eq!(
        super::slot_task(&app.view(), &slot(Some("task-1"))).map(|task| task.id.as_str()),
        Some("task-1")
    );
    assert!(super::slot_task(&app.view(), &slot(Some("missing"))).is_none());
    assert!(super::slot_task(&app.view(), &slot(None)).is_none());
}

#[test]
fn worktree_log_preview_lines_handles_empty_and_missing_task_paths() {
    let (app, _task) = crate::control::test_support::TestApp::new();

    assert_eq!(super::worktree_log_preview_lines(&app.view(), &slot(None), 10), vec!["等待任务日志..."]);
    assert_eq!(
        super::worktree_log_preview_lines(&app.view(), &slot(Some("task-without-record")), 10),
        vec!["任务 task-without-record"]
    );
}

#[test]
fn worktree_log_preview_lines_returns_started_when_task_has_no_visible_logs() {
    let (mut app, _task) = crate::control::test_support::TestApp::new();
    let mut task = desktop_task_model::Task::new(1);
    task.id = "task-1".to_string();
    task.logs = vec![desktop_task_model::TaskLogEntry::new_message(" \n ".to_string())];
    app.task_board_tasks = vec![task];

    assert_eq!(
        super::worktree_log_preview_lines(&app.view(), &slot(Some("task-1")), 10),
        vec!["任务已启动..."]
    );
}

#[test]
fn worktree_log_preview_lines_normalizes_truncates_and_keeps_latest_three() {
    let (mut app, _task) = crate::control::test_support::TestApp::new();
    let mut task = desktop_task_model::Task::new(1);
    task.id = "task-1".to_string();
    task.logs = vec![
        desktop_task_model::TaskLogEntry::new_message("one".to_string()),
        desktop_task_model::TaskLogEntry::new_message("two".to_string()),
        desktop_task_model::TaskLogEntry::new_message("three".to_string()),
        desktop_task_model::TaskLogEntry::new_message("four".to_string()),
        desktop_task_model::TaskLogEntry::new_message("five".to_string()),
        desktop_task_model::TaskLogEntry::new_message("six".to_string()),
        desktop_task_model::TaskLogEntry::new_message("seven".to_string()),
        desktop_task_model::TaskLogEntry::new_message(
            "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz1\nnext".to_string(),
        ),
    ];
    app.task_board_tasks = vec![task];

    assert_eq!(
        super::worktree_log_preview_lines(&app.view(), &slot(Some("task-1")), 123),
        vec![
            "six".to_string(),
            "seven".to_string(),
            "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz...".to_string(),
        ]
    );
}

#[test]
fn task_pool_employee_summary_retains_unavailable_run() {
    let mut app = crate::control::test_support::TestApp::new().0;
    app.agent_entries.clear();
    let mut slot = slot(Some("history-task"));
    slot.agent_id = Some("deleted-agent".into());
    slot.run_id = Some("history-run".into());
    let summary = super::employee_summary(&app.view(), &slot);
    assert!(summary.contains("deleted-agent"));
    assert!(summary.contains("history-task"));
    assert!(summary.contains("history-run"));
    assert!(!desktop_task_model::agent_activity::agent_available(
        "deleted-agent",
        &app.agent_entries
    ));
}
