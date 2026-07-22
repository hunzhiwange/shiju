use iced::advanced::renderer::Headless;
use iced::{Background, Color, Length, Theme};

use super::*;
use crate::app::task::{TaskStatus, WorktreeState};

const EPSILON: f32 = 0.000_01;

fn assert_close(actual: f32, expected: f32) {
    assert!((actual - expected).abs() <= EPSILON, "expected {actual} to be close to {expected}");
}

fn assert_color_close(actual: Color, expected: Color) {
    assert_close(actual.r, expected.r);
    assert_close(actual.g, expected.g);
    assert_close(actual.b, expected.b);
    assert_close(actual.a, expected.a);
}

fn background_color(style: iced::widget::container::Style) -> Color {
    match style.background {
        Some(Background::Color(color)) => color,
        other => panic!("expected color background, got {other:?}"),
    }
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
    let viewport = iced::Rectangle::with_size(iced::Size::new(320.0, 120.0));
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

    let pixels = renderer.screenshot(iced::Size::new(320, 120), 1.0, Color::TRANSPARENT);
    assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] > 0));
}

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("summary_tests"));
}

#[test]
fn worktree_summary_strip_builds_fixed_two_line_tile() {
    for state in [
        WorktreeState::Idle,
        WorktreeState::Busy,
        WorktreeState::Tainted,
        WorktreeState::Recycling,
        WorktreeState::Dead,
    ] {
        let element = build_worktree_summary_strip("状态", 42, state);

        assert_eq!(element.as_widget().size().width, Length::FillPortion(1));
        let children = element.as_widget().children();
        assert_eq!(children.len(), 2);
    }
}

#[test]
fn task_status_summary_strip_builds_fixed_two_line_tile() {
    for status in [
        TaskStatus::Pool,
        TaskStatus::Pending,
        TaskStatus::Planning,
        TaskStatus::Running,
        TaskStatus::Failed,
        TaskStatus::Paused,
        TaskStatus::CodeComplete,
        TaskStatus::CodeReview,
        TaskStatus::PrSubmitted,
        TaskStatus::Completed,
        TaskStatus::Archived,
        TaskStatus::Custom("custom"),
    ] {
        let element = build_task_status_summary_strip("任务", 7, status);

        assert_eq!(element.as_widget().size().width, Length::FillPortion(1));
        let children = element.as_widget().children();
        assert_eq!(children.len(), 2);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn summary_strips_draw_state_and_status_styles() {
    let element = iced::widget::row![
        build_worktree_summary_strip("繁忙", 2, WorktreeState::Busy),
        build_task_status_summary_strip("失败", 1, TaskStatus::Failed),
    ]
    .spacing(4)
    .width(Length::Fill)
    .into();

    assert_element_draws(element).await;
}

#[test]
fn worktree_summary_styles_follow_state_palette() {
    for theme in [Theme::Light, Theme::Dark] {
        for state in [
            WorktreeState::Idle,
            WorktreeState::Busy,
            WorktreeState::Tainted,
            WorktreeState::Recycling,
            WorktreeState::Dead,
        ] {
            let (bg, text_color, border_color) =
                super::super::super::helpers::worktree_state_palette(&theme, state);
            let label = worktree_label_style(&theme, state);
            let value = worktree_value_style(&theme, state);
            let container = worktree_container_style(&theme, state);

            assert_color_close(label.color.expect("label color"), text_color.scale_alpha(0.72));
            assert_color_close(value.color.expect("value color"), text_color);
            assert_color_close(background_color(container), bg.scale_alpha(0.9));
            assert_color_close(container.border.color, border_color.scale_alpha(0.8));
            assert_close(container.border.width, 1.0);
            assert_close(container.border.radius.top_left, 2.0);
        }
    }
}

#[test]
fn task_status_summary_styles_follow_status_tag_colors() {
    for status in [
        TaskStatus::Pool,
        TaskStatus::Pending,
        TaskStatus::Planning,
        TaskStatus::Running,
        TaskStatus::Failed,
        TaskStatus::Paused,
        TaskStatus::CodeComplete,
        TaskStatus::CodeReview,
        TaskStatus::PrSubmitted,
        TaskStatus::Completed,
        TaskStatus::Archived,
        TaskStatus::Custom("custom"),
    ] {
        let (text_color, bg_color) = super::super::super::helpers::task_status_tag_colors(status);
        let label = task_status_label_style(status);
        let value = task_status_value_style(status);
        let container = task_status_container_style(status);

        assert_color_close(label.color.expect("label color"), text_color.scale_alpha(0.84));
        assert_color_close(value.color.expect("value color"), text_color);
        assert_color_close(background_color(container), bg_color.scale_alpha(0.92));
        assert_color_close(container.border.color, text_color.scale_alpha(0.42));
        assert_close(container.border.width, 1.0);
        assert_close(container.border.radius.top_left, 2.0);
    }
}
