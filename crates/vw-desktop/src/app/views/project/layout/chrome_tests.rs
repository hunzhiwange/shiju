use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, layout, mouse, renderer, widget::Tree};
use iced::{Font, Pixels, Rectangle, Size, Theme};

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("chrome_tests"));
}

#[test]
fn right_column_chrome_builds_with_positive_radius() {
    let content = iced::widget::text("right panel").into();
    let chrome = super::right_column_chrome(content, 8.0);

    assert_eq!(chrome.as_widget().size().width, iced::Length::Fill);
    assert_eq!(chrome.as_widget().size().height, iced::Length::Fill);
    assert!(chrome.as_widget().children().is_empty());
}

#[test]
fn right_column_chrome_builds_with_zero_radius() {
    let content = iced::widget::text("right panel").into();
    let chrome = super::right_column_chrome(content, 0.0);

    std::hint::black_box(chrome);
}

#[test]
fn right_column_chrome_builds_with_negative_radius() {
    let content = iced::widget::text("right panel").into();
    let chrome = super::right_column_chrome(content, -1.0);

    std::hint::black_box(chrome);
}

#[test]
fn right_column_chrome_builds_with_large_radius() {
    let content = iced::widget::text("right panel").into();
    let chrome = super::right_column_chrome(content, 100.0);

    std::hint::black_box(chrome);
}

#[test]
fn right_column_chrome_builds_with_very_small_positive_radius() {
    let content = iced::widget::text("right panel").into();
    let chrome = super::right_column_chrome(content, 0.001);

    std::hint::black_box(chrome);
}

#[test]
fn right_column_chrome_accepts_rich_element_content() {
    let content = iced::widget::column![iced::widget::text("a"), iced::widget::text("b")].into();
    let chrome = super::right_column_chrome(content, 6.0);

    std::hint::black_box(chrome);
}

#[test]
fn right_column_chrome_accepts_empty_container_content() {
    let content = iced::widget::container(iced::widget::text("")).into();
    let chrome = super::right_column_chrome(content, 8.0);

    std::hint::black_box(chrome);
}

#[tokio::test(flavor = "multi_thread")]
async fn right_column_chrome_draws_inner_and_outer_styles() {
    let mut renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("tiny-skia headless renderer should be available");
    let content = iced::widget::text("right panel").into();
    let mut chrome = super::right_column_chrome(content, 8.0);

    draw_element(&mut chrome, &mut renderer, &Theme::Dark);
}

#[tokio::test(flavor = "multi_thread")]
async fn right_column_chrome_covers_its_bottom_border() {
    let renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("tiny-skia headless renderer should be available");
    let content = iced::widget::text("right panel").into();
    let mut chrome = super::right_column_chrome(content, 8.0);
    let bounds = Size::new(320.0, 480.0);
    let limits = layout::Limits::new(Size::ZERO, bounds);
    let mut tree = Tree::new(chrome.as_widget());
    let node = chrome.as_widget_mut().layout(&mut tree, &renderer, &limits);
    let root = Layout::new(&node);
    let stack = root.children().next().expect("chrome should contain its stack");
    let mut layers = stack.children();
    layers.next().expect("chrome stack should contain the right column");
    let border_cover = layers.next().expect("chrome stack should contain the bottom border cover");
    let cover_strip =
        border_cover.children().next().expect("bottom border cover should contain its strip");

    assert!(
        (cover_strip.bounds().height - super::RIGHT_COLUMN_BOTTOM_BORDER_COVER_HEIGHT).abs()
            < 0.001
    );
    assert!(
        (cover_strip.bounds().y + cover_strip.bounds().height
            - (stack.bounds().y + stack.bounds().height))
            .abs()
            < 0.001
    );
}

#[test]
fn overlay_divider_builds_when_spacing_is_below_hit_width() {
    let divider = super::overlay_divider(2.0);

    assert_eq!(
        divider.as_widget().size().width,
        iced::Length::Fixed(super::HResizeHandle::HIT_WIDTH - 2.0)
    );
    assert_eq!(divider.as_widget().size().height, iced::Length::Fill);
    assert!(divider.as_widget().children().is_empty());
}

#[test]
fn overlay_divider_builds_when_spacing_matches_hit_width() {
    let divider = super::overlay_divider(super::HResizeHandle::HIT_WIDTH);

    assert_eq!(divider.as_widget().size().width, iced::Length::Fixed(1.0));
    assert_eq!(divider.as_widget().size().height, iced::Length::Fill);
}

#[test]
fn overlay_divider_builds_when_spacing_exceeds_hit_width() {
    let divider = super::overlay_divider(super::HResizeHandle::HIT_WIDTH + 8.0);

    assert_eq!(divider.as_widget().size().width, iced::Length::Fixed(1.0));
    assert_eq!(divider.as_widget().size().height, iced::Length::Fill);
}

#[test]
fn overlay_divider_builds_with_zero_spacing() {
    let divider = super::overlay_divider(0.0);

    assert_eq!(
        divider.as_widget().size().width,
        iced::Length::Fixed(super::HResizeHandle::HIT_WIDTH)
    );
}

#[test]
fn overlay_divider_builds_with_large_spacing() {
    let divider = super::overlay_divider(1_000.0);

    assert_eq!(divider.as_widget().size().width, iced::Length::Fixed(1.0));
}

#[test]
fn overlay_divider_builds_with_negative_spacing() {
    let divider = super::overlay_divider(-5.0);

    assert_eq!(
        divider.as_widget().size().width,
        iced::Length::Fixed(super::HResizeHandle::HIT_WIDTH + 5.0)
    );
}

#[test]
fn overlay_divider_width_is_full_width_when_spacing_is_zero() {
    let width = super::overlay_divider_width(0.0);

    assert_eq!(width, super::HResizeHandle::HIT_WIDTH);
}

#[test]
fn overlay_divider_width_clamps_to_one_when_spacing_matches_hit_width() {
    let width = super::overlay_divider_width(super::HResizeHandle::HIT_WIDTH);

    assert_eq!(width, 1.0);
}

#[test]
fn overlay_divider_width_clamps_to_one_when_spacing_exceeds_hit_width() {
    let width = super::overlay_divider_width(super::HResizeHandle::HIT_WIDTH + 10.0);

    assert_eq!(width, 1.0);
}

#[test]
fn overlay_divider_width_returns_one_for_exact_minimum_boundary() {
    let width = super::overlay_divider_width(super::HResizeHandle::HIT_WIDTH - 1.0);

    assert_eq!(width, 1.0);
}

#[test]
fn overlay_divider_width_returns_positive_width_when_spacing_is_below_hit_width() {
    let width = super::overlay_divider_width(2.0);

    assert_eq!(width, super::HResizeHandle::HIT_WIDTH - 2.0);
}

#[test]
fn overlay_divider_width_returns_large_width_when_spacing_is_negative() {
    let width = super::overlay_divider_width(-5.0);

    assert_eq!(width, super::HResizeHandle::HIT_WIDTH + 5.0);
}

#[test]
fn overlay_divider_width_clamps_with_f32_max_spacing() {
    let width = super::overlay_divider_width(f32::MAX);

    assert_eq!(width, 1.0);
}

#[test]
fn overlay_divider_width_clamps_nan_spacing_to_minimum() {
    let width = super::overlay_divider_width(f32::NAN);

    assert_eq!(width, 1.0);
}

#[test]
fn overlay_divider_width_clamps_positive_infinite_spacing_to_minimum() {
    let width = super::overlay_divider_width(f32::INFINITY);

    assert_eq!(width, 1.0);
}

#[test]
fn overlay_divider_width_preserves_negative_infinite_remainder() {
    let width = super::overlay_divider_width(f32::NEG_INFINITY);

    assert_eq!(width, f32::INFINITY);
}

#[test]
fn overlay_divider_width_never_below_one_for_any_spacing() {
    for spacing in [
        -100.0,
        -1.0,
        0.0,
        1.0,
        super::HResizeHandle::HIT_WIDTH,
        super::HResizeHandle::HIT_WIDTH + 1.0,
        100.0,
    ] {
        let width = super::overlay_divider_width(spacing);
        assert!(width >= 1.0, "width {} should never be below 1 for spacing {}", width, spacing);
    }
}

fn draw_element(
    element: &mut iced::Element<'_, crate::app::Message>,
    renderer: &mut iced::Renderer,
    theme: &Theme,
) {
    let bounds = Size::new(320.0, 480.0);
    let limits = layout::Limits::new(Size::ZERO, bounds);
    let mut tree = Tree::new(element.as_widget());
    let node = element.as_widget_mut().layout(&mut tree, renderer, &limits);
    let viewport = Rectangle::with_size(bounds);

    element.as_widget().draw(
        &tree,
        renderer,
        theme,
        &renderer::Style::default(),
        Layout::new(&node),
        mouse::Cursor::Unavailable,
        &viewport,
    );
}
