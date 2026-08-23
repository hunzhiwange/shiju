//! 工作树房间场景的测试，覆盖工位状态到区域和像素卡片的映射。
//! fixture 包含缺失任务、失败原因与 Unicode 标识，验证紧凑和宽屏构建。
use iced::{Element, Length};

use crate::app::task::{Task, TaskLogEntry, TaskStatus, WorktreeSlotSnapshot, WorktreeState};
use crate::app::{App, Message};

use super::*;

fn test_app(window_width: f32) -> App {
    let (mut app, _task) = App::new();
    app.window_size = (window_width, 800.0);
    app
}

fn slot(id: &str, state: WorktreeState) -> WorktreeSlotSnapshot {
    WorktreeSlotSnapshot {
        id: id.to_string(),
        path: format!("/tmp/{id}"),
        branch: format!("feature/{id}"),
        state,
        leased_task_id: matches!(state, WorktreeState::Busy).then(|| format!("task-{id}")),
        taint_reason: matches!(state, WorktreeState::Tainted)
            .then(|| "working tree contains changes".to_string()),
    }
}

fn task(id: &str, status: TaskStatus) -> Task {
    let mut task = Task::new(10);
    task.id = id.to_string();
    task.status = status;
    task
}

fn keep_element(element: Element<'_, Message>) {
    std::hint::black_box(element);
}

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("room_scene_tests"));
}

#[test]
fn room_plate_maps_operational_and_failure_states_to_zones() {
    let cases = [
        (WorktreeState::Idle, "A-slot-1"),
        (WorktreeState::Busy, "A-slot-1"),
        (WorktreeState::Tainted, "Q-slot-1"),
        (WorktreeState::Recycling, "R-slot-1"),
        (WorktreeState::Dead, "X-slot-1"),
    ];

    for (state, expected) in cases {
        assert_eq!(worktree_room_plate("slot-1", state), expected);
    }
}

#[test]
fn room_plate_preserves_empty_and_unicode_slot_ids() {
    assert_eq!(worktree_room_plate("", WorktreeState::Idle), "A-");
    assert_eq!(worktree_room_plate("工位-甲", WorktreeState::Dead), "X-工位-甲");
}

#[test]
fn room_scene_builds_all_states_with_state_specific_details() {
    let app = test_app(1440.0);

    for state in [
        WorktreeState::Idle,
        WorktreeState::Busy,
        WorktreeState::Tainted,
        WorktreeState::Recycling,
        WorktreeState::Dead,
    ] {
        let slot = slot("state", state);
        let scene = build_worktree_room_scene(&app, &slot);

        assert_eq!(scene.as_widget().size().width, Length::Fill);
        keep_element(scene);
    }
}

#[test]
fn room_scene_builds_compact_tight_and_wide_artwork() {
    let idle = slot("responsive", WorktreeState::Idle);

    for width in [1_200.0, 1_400.0, 1_600.0] {
        let app = test_app(width);
        keep_element(build_worktree_room_scene(&app, &idle));
    }
}

#[test]
fn tainted_room_builds_with_explicit_and_fallback_reasons() {
    let app = test_app(1_440.0);
    let explicit = slot("explicit", WorktreeState::Tainted);
    let mut fallback = slot("fallback", WorktreeState::Tainted);
    fallback.taint_reason = None;

    keep_element(build_worktree_room_scene(&app, &explicit));
    keep_element(build_worktree_room_scene(&app, &fallback));
}

#[test]
fn busy_room_builds_for_missing_running_and_finished_tasks() {
    let busy = slot("busy", WorktreeState::Busy);

    let missing_app = test_app(1_440.0);
    keep_element(build_worktree_room_scene(&missing_app, &busy));

    let mut running_app = test_app(1_440.0);
    let mut running = task("task-busy", TaskStatus::Running);
    running.execution_started_at_ms = Some(0);
    running_app.task_board_tasks = vec![running];
    keep_element(build_worktree_room_scene(&running_app, &busy));

    let mut finished_app = test_app(1_440.0);
    let mut finished = task("task-busy", TaskStatus::Completed);
    finished.last_execution_duration_ms = Some(61_000);
    finished_app.task_board_tasks = vec![finished];
    keep_element(build_worktree_room_scene(&finished_app, &busy));
}

#[test]
fn pixel_card_builds_all_states_with_fixed_card_geometry() {
    let app = test_app(1_440.0);

    for state in [
        WorktreeState::Idle,
        WorktreeState::Busy,
        WorktreeState::Tainted,
        WorktreeState::Recycling,
        WorktreeState::Dead,
    ] {
        let slot = slot("card", state);
        let card = build_worktree_pixel_card(&app, &slot);
        let size = card.as_widget().size();

        assert_eq!(size.width, Length::FillPortion(1));
        assert_eq!(size.height, Length::Fixed(268.0));
        keep_element(card);
    }
}

#[test]
fn pixel_card_builds_collapsed_and_expanded_panel_variants() {
    let idle = slot("panel", WorktreeState::Idle);
    let mut app = test_app(1_440.0);

    app.task_board_worktree_panel_expanded = false;
    keep_element(build_worktree_pixel_card(&app, &idle));

    app.task_board_worktree_panel_expanded = true;
    keep_element(build_worktree_pixel_card(&app, &idle));
}

#[test]
fn busy_room_builds_with_unassigned_owner_fallback() {
    let app = test_app(1_440.0);
    let mut busy = slot("unassigned", WorktreeState::Busy);
    busy.leased_task_id = None;

    keep_element(build_worktree_room_scene(&app, &busy));
    keep_element(build_worktree_pixel_card(&app, &busy));
}

#[test]
fn busy_room_builds_non_running_task_without_duration() {
    let mut app = test_app(1_440.0);
    app.task_board_tasks = vec![task("task-paused", TaskStatus::Paused)];
    let busy = slot("paused", WorktreeState::Busy);

    keep_element(build_worktree_room_scene(&app, &busy));
}

#[test]
fn busy_room_builds_log_bubble_from_recent_task_messages() {
    let mut app = test_app(1_440.0);
    let mut running = task("task-busy", TaskStatus::Running);
    running.logs = vec![
        TaskLogEntry::new_message("one".to_string()),
        TaskLogEntry::new_message("two".to_string()),
        TaskLogEntry::new_message("three".to_string()),
    ];
    app.task_board_tasks = vec![running];
    let busy = slot("busy", WorktreeState::Busy);

    keep_element(build_worktree_room_scene(&app, &busy));
    keep_element(build_worktree_pixel_card(&app, &busy));
}
