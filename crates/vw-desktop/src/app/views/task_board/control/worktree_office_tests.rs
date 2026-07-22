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
