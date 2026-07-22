use iced::advanced::renderer::Headless;
use iced::{Background, Color, Point, Theme};

use crate::app::task::{Task, TaskStatus};

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("drag_tests"));
}

fn color(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgb8(r, g, b)
}

fn task_with(priority: u32, prompt: &str, status: TaskStatus) -> Task {
    let mut task = Task::new(priority);
    task.id = format!("task-{priority}");
    task.prompt = prompt.to_string();
    task.status = status;
    task
}

async fn assert_element_draws(mut element: iced::Element<'_, crate::app::Message>) {
    let mut renderer = <iced::Renderer as Headless>::new(
        iced::Font::DEFAULT,
        iced::Pixels(16.0),
        Some("tiny-skia"),
    )
    .await
    .expect("tiny-skia headless renderer should be available");
    let mut tree = iced::advanced::widget::Tree::new(element.as_widget());
    let viewport = iced::Rectangle::with_size(iced::Size::new(320.0, 180.0));
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

    let pixels = renderer.screenshot(iced::Size::new(320, 180), 1.0, Color::TRANSPARENT);
    assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] > 0));
}

#[test]
fn status_tag_colors_cover_builtin_and_custom_statuses() {
    let expected = [
        (TaskStatus::Pool, color(107, 114, 128), color(243, 244, 246)),
        (TaskStatus::Pending, color(37, 99, 235), color(219, 234, 254)),
        (TaskStatus::Planning, color(79, 70, 229), color(224, 231, 255)),
        (TaskStatus::Running, color(147, 51, 234), color(243, 232, 255)),
        (TaskStatus::Failed, color(220, 38, 38), color(254, 226, 226)),
        (TaskStatus::Paused, color(202, 138, 4), color(254, 249, 195)),
        (TaskStatus::CodeComplete, color(5, 150, 105), color(209, 250, 229)),
        (TaskStatus::CodeReview, color(217, 119, 6), color(254, 243, 199)),
        (TaskStatus::PrSubmitted, color(8, 145, 178), color(207, 250, 254)),
        (TaskStatus::Completed, color(22, 163, 74), color(220, 252, 231)),
        (TaskStatus::Archived, color(100, 116, 139), color(241, 245, 249)),
        (
            TaskStatus::parse_key("waiting_review").expect("custom status"),
            color(107, 114, 128),
            color(243, 244, 246),
        ),
    ];

    for (status, expected_text, expected_background) in expected {
        let (text, background) = super::task_status_tag_colors(status);

        assert_eq!(text, expected_text);
        assert_eq!(background, expected_background);
    }
}

#[test]
fn priority_badge_color_uses_high_medium_and_neutral_bands() {
    assert_eq!(super::task_priority_badge_color(1), color(239, 68, 68));
    assert_eq!(super::task_priority_badge_color(100), color(239, 68, 68));
    assert_eq!(super::task_priority_badge_color(101), color(245, 158, 11));
    assert_eq!(super::task_priority_badge_color(500), color(245, 158, 11));
    assert_eq!(super::task_priority_badge_color(501), color(107, 114, 128));
}

#[test]
fn preview_content_falls_back_only_for_blank_prompts() {
    assert_eq!(super::task_preview_content(""), "（无内容）");
    assert_eq!(super::task_preview_content(" \n\t "), "（无内容）");
    assert_eq!(
        super::task_preview_content("  keep surrounding space  "),
        "  keep surrounding space  "
    );
}

#[test]
fn drag_preview_padding_offsets_and_clamps_to_window_bounds() {
    let padding = super::drag_preview_padding(Point::new(320.0, 180.0), (1200.0, 800.0));

    assert_eq!(padding.top, 140.0);
    assert_eq!(padding.right, 0.0);
    assert_eq!(padding.bottom, 0.0);
    assert_eq!(padding.left, 220.0);

    let min_padding = super::drag_preview_padding(Point::new(-20.0, -30.0), (1200.0, 800.0));
    assert_eq!(min_padding.top, -40.0);
    assert_eq!(min_padding.left, -100.0);

    let max_padding = super::drag_preview_padding(Point::new(2_000.0, 2_000.0), (500.0, 120.0));
    assert_eq!(max_padding.top, 0.0);
    assert_eq!(max_padding.left, 180.0);

    let tiny_padding = super::drag_preview_padding(Point::new(20.0, 80.0), (120.0, 30.0));
    assert_eq!(tiny_padding.top, -40.0);
    assert_eq!(tiny_padding.left, -100.0);
}

#[test]
fn badge_styles_preserve_requested_colors() {
    let priority = color(239, 68, 68);
    let priority_style = super::priority_badge_style(priority);
    assert_eq!(priority_style.background, Some(Background::Color(priority.scale_alpha(0.2))));
    assert_eq!(priority_style.text_color, Some(priority));

    let status_text = color(37, 99, 235);
    let status_background = color(219, 234, 254);
    let status_style = super::status_badge_style(status_text, status_background);
    assert_eq!(status_style.background, Some(Background::Color(status_background)));
    assert_eq!(status_style.text_color, Some(status_text));
}

#[test]
fn theme_styles_use_theme_surface_text_and_blue_drag_accent() {
    let theme = Theme::Dark;

    let text_style = super::task_id_text_style(&theme);
    assert_eq!(text_style.color, Some(theme.extended_palette().background.base.text));

    let card_style = super::drag_preview_card_style(&theme);
    assert_eq!(card_style.background, Some(Background::Color(theme.palette().background)));
    assert_eq!(card_style.border.width, 2.0);
    assert_eq!(card_style.border.color, color(59, 130, 246));
    assert_eq!(card_style.shadow.color, color(59, 130, 246).scale_alpha(0.3));
    assert_eq!(card_style.shadow.offset, iced::Vector::new(0.0, 8.0));
    assert_eq!(card_style.shadow.blur_radius, 16.0);
}

#[test]
fn drag_preview_card_builds_for_priority_and_prompt_variants() {
    for task in [
        task_with(100, "", TaskStatus::Pending),
        task_with(500, "medium priority", TaskStatus::Running),
        task_with(501, "neutral priority", TaskStatus::Completed),
    ] {
        let _element = super::build_drag_preview_card(&task);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn drag_preview_card_draws_badges_and_theme_styles() {
    let task = task_with(100, "render the dragged task", TaskStatus::Running);

    assert_element_draws(super::build_drag_preview_card(&task)).await;
}

#[test]
fn drag_preview_layer_builds_empty_layer_without_active_drag() {
    let (app, _task) = crate::app::App::new();

    let _element = super::build_drag_preview_layer(&app);
}

#[test]
fn drag_preview_layer_builds_empty_layer_when_dragged_task_is_missing() {
    let (mut app, _task) = crate::app::App::new();
    app.task_board_dragging = Some(("missing".to_string(), TaskStatus::Pool));

    let _element = super::build_drag_preview_layer(&app);
}

#[test]
fn drag_preview_layer_builds_card_for_matching_dragged_task() {
    let (mut app, _task) = crate::app::App::new();
    let task = task_with(42, "drag me", TaskStatus::Planning);
    app.task_board_dragging = Some((task.id.clone(), task.status));
    app.task_board_tasks = vec![task];
    app.cursor_position = Point::new(9_999.0, -20.0);
    app.window_size = (180.0, 36.0);

    let _element = super::build_drag_preview_layer(&app);
}
