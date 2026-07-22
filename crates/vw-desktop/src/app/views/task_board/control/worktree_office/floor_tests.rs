use iced::advanced::renderer::Headless;
use iced::{Background, Color, Element, Length, Theme};

use crate::app::task::{WorktreePoolSnapshot, WorktreeSlotSnapshot, WorktreeState};
use crate::app::{App, Message};

use super::*;

fn test_app(window_width: f32) -> App {
    let (mut app, _task) = App::new();
    app.window_size = (window_width, 800.0);
    app
}

fn keep_element(element: Element<'_, Message>) {
    std::hint::black_box(element);
}

fn slot(id: &str, state: WorktreeState) -> WorktreeSlotSnapshot {
    WorktreeSlotSnapshot {
        id: id.to_string(),
        path: format!("/tmp/{id}"),
        branch: format!("feature/{id}"),
        state,
        leased_task_id: matches!(state, WorktreeState::Busy).then(|| format!("task-{id}")),
        taint_reason: matches!(state, WorktreeState::Tainted).then(|| "dirty checkout".to_string()),
    }
}

fn snapshot(
    idle_count: usize,
    busy_count: usize,
    tainted_count: usize,
    recycling_count: usize,
    dead_count: usize,
) -> WorktreePoolSnapshot {
    WorktreePoolSnapshot {
        repo_root: "/repo".to_string(),
        pool_root: "/repo/.worktrees".to_string(),
        base_branch: "main".to_string(),
        idle_count,
        busy_count,
        tainted_count,
        recycling_count,
        dead_count,
        merge_target_locks: Vec::new(),
        slots: Vec::new(),
    }
}

fn background_color(style: iced::widget::container::Style) -> Color {
    match style.background {
        Some(Background::Color(color)) => color,
        other => panic!("expected color background, got {other:?}"),
    }
}

fn color_tuple(color: Color) -> (f32, f32, f32, f32) {
    (color.r, color.g, color.b, color.a)
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
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("floor_tests"));
}

#[test]
fn floor_theme_helpers_cover_dark_light_and_text_alpha() {
    assert!(!is_dark_floor_theme(&Theme::Light));
    assert!(is_dark_floor_theme(&Theme::Dark));

    let style = floor_text_alpha_style(&Theme::Light, 0.42);
    let color = style.color.expect("text alpha style should set color");

    assert!(color.a > 0.0);
    assert!(color.a <= 0.42);
}

#[test]
fn container_style_helpers_distinguish_dark_and_light_backgrounds() {
    let helpers: [fn(&Theme) -> iced::widget::container::Style; 6] = [
        build_worktree_floor_strip_style,
        build_worktree_floor_texture_fill_style,
        build_worktree_corridor_style,
        build_empty_worktree_section_style,
        build_worktree_pixel_section_style,
        build_worktree_pixel_floor_style,
    ];

    for helper in helpers {
        let light = helper(&Theme::Light);
        let dark = helper(&Theme::Dark);

        assert_ne!(color_tuple(background_color(light)), color_tuple(background_color(dark)));
    }
}

#[test]
fn bordered_style_helpers_keep_visible_borders() {
    let helpers: [fn(&Theme) -> iced::widget::container::Style; 5] = [
        build_worktree_floor_strip_style,
        build_worktree_corridor_style,
        build_empty_worktree_section_style,
        build_worktree_pixel_section_style,
        build_worktree_pixel_floor_style,
    ];

    for helper in helpers {
        let style = helper(&Theme::Light);

        assert_eq!(style.border.width, 1.0);
        assert!(style.border.color.a > 0.0);
    }
}

#[test]
fn floor_decorations_build_as_fill_width_elements() {
    let strip = build_worktree_floor_strip("测试地板");
    let texture = build_worktree_floor_texture_fill();

    assert_eq!(strip.as_widget().size().width, Length::Fill);
    assert_eq!(texture.as_widget().size().width, Length::Fill);
}

#[test]
fn corridor_covers_busy_maintenance_and_idle_states() {
    let busy = snapshot(1, 2, 0, 0, 0);
    let maintenance = snapshot(0, 0, 1, 1, 1);
    let idle = snapshot(3, 0, 0, 0, 0);

    let hidden = build_worktree_corridor(&maintenance, true);
    assert_eq!(hidden.as_widget().size().width, Length::Fixed(190.0));

    let visible = build_worktree_corridor(&busy, false);
    assert_eq!(visible.as_widget().size().width, Length::Fixed(120.0));

    keep_element(hidden);
    keep_element(visible);
    keep_element(build_worktree_corridor(&idle, false));
}

#[test]
fn corridor_summary_reports_status_hint_width_and_flow_mode() {
    let busy = worktree_corridor_summary(&snapshot(1, 2, 0, 0, 0), false);
    assert_eq!(busy.status, "任务流动中");
    assert_eq!(busy.maintenance_hint, "运维修复间记录: 0");
    assert_eq!(busy.width, 120);
    assert!(busy.busy_flow);

    let hidden_maintenance = worktree_corridor_summary(&snapshot(0, 0, 2, 1, 1), true);
    assert_eq!(hidden_maintenance.status, "维护队列处理中");
    assert_eq!(hidden_maintenance.maintenance_hint, "运维修复间已收起\n污染:2 回收:1 停机:1");
    assert_eq!(hidden_maintenance.width, 190);
    assert!(!hidden_maintenance.busy_flow);

    let idle = worktree_corridor_summary(&snapshot(3, 0, 0, 0, 0), false);
    assert_eq!(idle.status, "走廊空闲");
    assert_eq!(idle.maintenance_hint, "运维修复间记录: 0");
    assert_eq!(idle.width, 120);
    assert!(!idle.busy_flow);
}

#[test]
fn pixel_section_builds_empty_grid_and_filled_rows() {
    let app = test_app(1280.0);
    let idle = slot("idle", WorktreeState::Idle);
    let busy = slot("busy", WorktreeState::Busy);
    let dead = slot("dead", WorktreeState::Dead);

    keep_element(build_worktree_pixel_section(&app, "空区域", "暂无工位", Vec::new(), 0));
    keep_element(build_worktree_pixel_section(
        &app,
        "任务办公区",
        "三张卡片会触发行补位",
        vec![&idle, &busy, &dead],
        2,
    ));
}

#[test]
fn pixel_floor_builds_narrow_and_wide_maintenance_layouts() {
    let idle = slot("idle", WorktreeState::Idle);
    let busy = slot("busy", WorktreeState::Busy);
    let tainted = slot("tainted", WorktreeState::Tainted);
    let recycling = slot("recycling", WorktreeState::Recycling);
    let pool = snapshot(1, 1, 1, 1, 0);

    let narrow_app = test_app(1200.0);
    keep_element(build_worktree_pixel_floor(
        &narrow_app,
        &pool,
        vec![&idle, &busy],
        vec![&tainted],
    ));
    keep_element(build_worktree_pixel_floor(&narrow_app, &pool, vec![&idle], Vec::new()));

    let wide_app = test_app(1500.0);
    keep_element(build_worktree_pixel_floor(
        &wide_app,
        &pool,
        vec![&idle, &busy],
        vec![&tainted, &recycling],
    ));
    keep_element(build_worktree_pixel_floor(&wide_app, &pool, vec![&idle, &busy], Vec::new()));
}

#[tokio::test(flavor = "multi_thread")]
async fn pixel_floor_draws_wide_layout_at_exact_width_threshold() {
    let app = test_app(1360.0);
    let idle = slot("idle", WorktreeState::Idle);
    let busy = slot("busy", WorktreeState::Busy);
    let tainted = slot("tainted", WorktreeState::Tainted);
    let pool = snapshot(1, 1, 1, 0, 0);

    assert_element_draws(build_worktree_pixel_floor(
        &app,
        &pool,
        vec![&idle, &busy],
        vec![&tainted],
    ))
    .await;
}
