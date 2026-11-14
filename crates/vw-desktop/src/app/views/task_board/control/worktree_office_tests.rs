//! 工作树办公室面板的集成测试，覆盖空快照及全部工位状态分组。
//! 真实绘制用于确认整组房间在明暗主题下均保持有效布局。
use iced::advanced::renderer::Headless;
use iced::{Background, Color, Length, Theme};

use super::*;
use crate::app::task::{Task, WorktreeSlotSnapshot};

const EPSILON: f32 = 0.000_01;

fn test_app() -> App {
    App::new().0
}

fn task(id: &str, status: TaskStatus, deleted: bool, archived: bool) -> Task {
    let mut task = Task::new(1);
    task.id = id.to_string();
    task.status = status;
    task.deleted = deleted;
    task.archived = archived;
    task
}

fn slot(id: &str, state: WorktreeState) -> WorktreeSlotSnapshot {
    WorktreeSlotSnapshot {
        agent_id: None,
        run_id: None,
        id: id.to_string(),
        path: format!("/tmp/{id}"),
        branch: format!("task/{id}"),
        state,
        leased_task_id: matches!(state, WorktreeState::Busy).then(|| format!("task-{id}")),
        taint_reason: matches!(state, WorktreeState::Tainted).then(|| "dirty tree".to_string()),
    }
}

fn snapshot(slots: Vec<WorktreeSlotSnapshot>) -> WorktreePoolSnapshot {
    WorktreePoolSnapshot {
        repo_root: "/repo".to_string(),
        pool_root: "/repo/.worktrees".to_string(),
        base_branch: "main".to_string(),
        idle_count: slots.iter().filter(|slot| slot.state == WorktreeState::Idle).count(),
        busy_count: slots.iter().filter(|slot| slot.state == WorktreeState::Busy).count(),
        tainted_count: slots.iter().filter(|slot| slot.state == WorktreeState::Tainted).count(),
        recycling_count: slots.iter().filter(|slot| slot.state == WorktreeState::Recycling).count(),
        dead_count: slots.iter().filter(|slot| slot.state == WorktreeState::Dead).count(),
        merge_target_locks: Vec::new(),
        slots,
    }
}

fn background_color(style: &iced::widget::container::Style) -> Color {
    match &style.background {
        Some(Background::Color(color)) => *color,
        other => panic!("expected color background, got {other:?}"),
    }
}

fn assert_close(actual: f32, expected: f32) {
    assert!((actual - expected).abs() <= EPSILON, "expected {actual} to be close to {expected}");
}

async fn assert_element_draws(mut element: Element<'_, Message>) {
    let mut renderer = <iced::Renderer as Headless>::new(
        iced::Font::DEFAULT,
        iced::Pixels(16.0),
        Some("tiny-skia"),
    )
    .await
    .expect("tiny-skia headless renderer should be available");
    let mut tree = iced::advanced::widget::Tree::new(element.as_widget());
    let viewport = iced::Rectangle::with_size(iced::Size::new(640.0, 480.0));
    let node = element.as_widget_mut().layout(
        &mut tree,
        &renderer,
        &iced::advanced::layout::Limits::new(iced::Size::ZERO, viewport.size()),
    );

    element.as_widget().draw(
        &tree,
        &mut renderer,
        &Theme::Dark,
        &iced::advanced::renderer::Style::default(),
        iced::advanced::Layout::new(&node),
        iced::mouse::Cursor::Unavailable,
        &viewport,
    );

    let pixels = renderer.screenshot(iced::Size::new(640, 480), 1.0, Color::TRANSPARENT);
    assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] > 0));
}

#[test]
fn office_style_distinguishes_light_and_dark_themes() {
    let light = worktree_office_style(&Theme::Light);
    let dark = worktree_office_style(&Theme::Dark);

    assert_ne!(background_color(&light), background_color(&dark));
    assert_close(light.border.width, 1.0);
    assert_close(dark.border.width, 1.0);
    assert_close(light.border.radius.top_left, 2.0);
    assert_close(dark.border.radius.top_left, 2.0);
    assert_close(light.border.color.a, 0.35);
    assert_close(dark.border.color.a, 0.42);
}

#[test]
fn office_builds_empty_snapshot_as_fill_width_panel() {
    let app = test_app();
    let snapshot = snapshot(Vec::new());

    let office = build_worktree_pixel_office(&app, &snapshot);

    assert_eq!(office.as_widget().size().width, Length::Fill);
    assert_eq!(office.as_widget().children().len(), 4);
}

#[test]
fn office_builds_all_worktree_state_groups() {
    let app = test_app();
    let snapshot = snapshot(vec![
        slot("idle", WorktreeState::Idle),
        slot("busy", WorktreeState::Busy),
        slot("tainted", WorktreeState::Tainted),
        slot("recycling", WorktreeState::Recycling),
        slot("dead", WorktreeState::Dead),
    ]);

    let office = build_worktree_pixel_office(&app, &snapshot);
    let active_ids = active_worktree_slots(&snapshot)
        .into_iter()
        .map(|slot| slot.id.as_str())
        .collect::<Vec<_>>();
    let maintenance_ids = maintenance_worktree_slots(&snapshot)
        .into_iter()
        .map(|slot| slot.id.as_str())
        .collect::<Vec<_>>();

    assert_eq!(office.as_widget().size().width, Length::Fill);
    assert_eq!(active_ids, ["idle", "busy"]);
    assert_eq!(maintenance_ids, ["tainted", "recycling", "dead"]);
}

#[tokio::test(flavor = "multi_thread")]
async fn office_draws_all_worktree_state_groups() {
    let mut app = test_app();
    app.window_size = (1440.0, 900.0);
    app.task_board_tasks = vec![
        task("pool", TaskStatus::Pool, false, false),
        task("running", TaskStatus::Running, false, false),
        task("completed", TaskStatus::Completed, false, false),
    ];
    let snapshot = snapshot(vec![
        slot("idle", WorktreeState::Idle),
        slot("busy", WorktreeState::Busy),
        slot("tainted", WorktreeState::Tainted),
        slot("recycling", WorktreeState::Recycling),
        slot("dead", WorktreeState::Dead),
    ]);

    assert_element_draws(build_worktree_pixel_office(&app, &snapshot)).await;
}

#[test]
fn office_counts_visible_tasks_and_ignores_deleted_or_archived_tasks() {
    let mut app = test_app();
    let statuses = [
        TaskStatus::Pool,
        TaskStatus::Pending,
        TaskStatus::Planning,
        TaskStatus::Running,
        TaskStatus::Paused,
        TaskStatus::Failed,
        TaskStatus::CodeComplete,
        TaskStatus::CodeReview,
        TaskStatus::PrSubmitted,
        TaskStatus::Completed,
    ];
    for (index, status) in statuses.into_iter().enumerate() {
        app.task_board_tasks.push(task(&format!("visible-{index}"), status, false, false));
        app.task_board_tasks.push(task(&format!("deleted-{index}"), status, true, false));
        app.task_board_tasks.push(task(&format!("archived-{index}"), status, false, true));
    }
    let snapshot = snapshot(Vec::new());

    let office = build_worktree_pixel_office(&app, &snapshot);

    assert_eq!(office.as_widget().size().width, Length::Fill);
    for status in statuses {
        assert_eq!(visible_task_status_count(&app, status), 1);
    }
}

#[test]
fn task_pool_office_non_git_displays_agents_and_runs() {
    let mut app = test_app();
    let project = tempfile::tempdir().unwrap();
    app.project_path = Some(project.path().to_string_lossy().into_owned());
    app.project_is_git_repo = false;
    assert!(super::super::should_render_worktree_controls(&app));
    let mut employee = slot("logical-1", WorktreeState::Busy);
    employee.agent_id = Some("main".into());
    employee.run_id = Some("run-1".into());
    let summary = super::super::helpers::employee_summary(&app, &employee);
    assert!(summary.contains("run-1"));
    assert!(summary.contains("main"));
}

#[test]
fn task_pool_office_resource_changes_do_not_change_employee_count() {
    let app = test_app();
    let mut seats = snapshot(vec![slot("seat", WorktreeState::Busy)]);
    let before = active_worktree_slots(&seats).len();
    seats.slots[0].path.clear();
    seats.slots[0].branch.clear();
    assert_eq!(active_worktree_slots(&seats).len(), before);
    let _ = build_worktree_pixel_office(&app, &seats);
}

#[test]
fn task_pool_office_empty_and_disabled_agents_render() {
    let app = test_app();
    let mut employee = slot("seat", WorktreeState::Busy);
    employee.agent_id = Some("disabled-agent".into());
    let summary = super::super::helpers::employee_summary(&app, &employee);
    assert!(summary.contains("disabled-agent"));
    let seats = snapshot(vec![employee]);
    let _ = build_worktree_pixel_office(&app, &seats);
    let empty = snapshot(vec![]);
    let _ = build_worktree_pixel_office(&app, &empty);
}

#[test]
fn task_pool_office_capacity_and_all_employee_interactions_are_preserved() {
    let mut app = test_app();
    let project = tempfile::tempdir().unwrap();
    app.project_path = Some(project.path().to_string_lossy().into_owned());
    app.task_board_worktree_snapshot = Some(snapshot(vec![
        slot("idle", WorktreeState::Idle),
        slot("busy", WorktreeState::Busy),
        slot("tainted", WorktreeState::Tainted),
        slot("recycling", WorktreeState::Recycling),
        slot("dead", WorktreeState::Dead),
    ]));
    use crate::app::message::{TaskBoardMessage, task_board::update};
    app.task_board_worktree_desired_capacity = Some(17);
    for (message, kind) in [
        (TaskBoardMessage::CleanAllWorktreesPressed, "cleanup"),
        (TaskBoardMessage::DeleteAllWorktreesPressed, "delete"),
        (TaskBoardMessage::CommitMergeAllWorktreesPressed, "merge"),
    ] {
        let _ = update(&mut app, message);
        assert_eq!(app.task_board_worktree_manual_confirm_kind, Some(kind));
        let _ = update(&mut app, TaskBoardMessage::CancelWorktreeManualConfirm);
        assert_eq!(app.task_board_worktree_manual_confirm_kind, None);
        assert_eq!(app.task_board_worktree_desired_capacity, Some(17));
        let press = || match kind {
            "cleanup" => TaskBoardMessage::CleanAllWorktreesPressed,
            "delete" => TaskBoardMessage::DeleteAllWorktreesPressed,
            _ => TaskBoardMessage::CommitMergeAllWorktreesPressed,
        };
        let _ = update(&mut app, press());
        let _ = update(&mut app, press());
        assert_eq!(app.task_board_worktree_manual_action_kind, Some(kind));
        assert!(!app.task_board_worktree_action_logs.is_empty());
        let error = Err("显式任务未配置".to_string());
        let completed = match kind {
            "cleanup" => TaskBoardMessage::CleanAllWorktreesCompleted(error),
            "delete" => TaskBoardMessage::DeleteAllWorktreesCompleted(error),
            _ => TaskBoardMessage::CommitMergeAllWorktreesCompleted(error),
        };
        let _ = update(&mut app, completed);
        assert!(app.task_board_worktree_manual_action_kind.is_none());
        assert!(
            app.task_board_worktree_action_logs.iter().any(|line| line.contains("显式任务未配置"))
        );
        assert_eq!(app.task_board_worktree_desired_capacity, Some(17));
    }
    let _ =
        update(&mut app, TaskBoardMessage::RecycleWorktreeSlotPressed { slot_id: "busy".into() });
    assert!(app.task_board_worktree_manual_action_kind.is_none());
    let _ =
        update(&mut app, TaskBoardMessage::RecycleWorktreeSlotPressed { slot_id: "dead".into() });
    assert_eq!(app.task_board_worktree_manual_action_kind, Some("slot_recycle"));
    // 丢弃未轮询的异步命令，测试不连接生产网关。
    app.task_board_worktree_manual_action_kind = None;
    let expanded = app.task_board_worktree_panel_expanded;
    let _ = update(&mut app, TaskBoardMessage::ToggleWorktreePanelExpanded);
    assert_eq!(app.task_board_worktree_panel_expanded, !expanded);
    let _ = update(&mut app, TaskBoardMessage::ToggleWorktreePixelOffice(false));
    assert!(!app.task_board_worktree_pixel_office);
    for expanded in [false, true] {
        for pixel in [false, true] {
            app.task_board_worktree_panel_expanded = expanded;
            app.task_board_worktree_pixel_office = pixel;
            let _ = super::super::build_control_buttons(&app);
            assert_eq!(app.task_board_worktree_snapshot.as_ref().unwrap().slots.len(), 5);
        }
    }
}
