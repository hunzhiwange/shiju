//! 项目区域缩放手柄的测试，覆盖命中范围、布局、鼠标反馈与绘制行为。
//! 记录型渲染器用于核对明暗主题下分隔线的实际填充参数。
use super::{
    HResizeHandle, SessionPanelLeftBorder, SessionPanelRightBorder, TopBorderCover, VResizeHandle,
    divider_line_color, is_dark_theme,
};
use iced::advanced::{Layout, Widget, layout, mouse, renderer, widget::Tree};
use iced::{Background, Color, Element, Length, Point, Rectangle, Size, Theme, Transformation};

#[derive(Debug, Clone, Copy, PartialEq)]
struct FillCall {
    bounds: Rectangle,
    background: Background,
}

#[derive(Debug, Default)]
struct RecordingRenderer {
    fills: Vec<FillCall>,
    layers: usize,
    transformations: usize,
    reset_bounds: Option<Rectangle>,
}

impl iced::advanced::Renderer for RecordingRenderer {
    fn start_layer(&mut self, _bounds: Rectangle) {
        self.layers += 1;
    }

    fn end_layer(&mut self) {
        self.layers = self.layers.saturating_sub(1);
    }

    fn start_transformation(&mut self, _transformation: Transformation) {
        self.transformations += 1;
    }

    fn end_transformation(&mut self) {
        self.transformations = self.transformations.saturating_sub(1);
    }

    fn fill_quad(&mut self, quad: renderer::Quad, background: impl Into<Background>) {
        self.fills.push(FillCall { bounds: quad.bounds, background: background.into() });
    }

    fn reset(&mut self, new_bounds: Rectangle) {
        self.reset_bounds = Some(new_bounds);
        self.fills.clear();
    }

    fn allocate_image(
        &mut self,
        _handle: &iced::advanced::image::Handle,
        callback: impl FnOnce(Result<iced::advanced::image::Allocation, iced::advanced::image::Error>)
        + Send
        + 'static,
    ) {
        callback(Err(iced::advanced::image::Error::Unsupported));
    }
}

fn limits(width: f32, height: f32) -> layout::Limits {
    layout::Limits::new(Size::ZERO, Size::new(width, height))
}

fn tree() -> Tree {
    Tree::empty()
}

fn layout_for(bounds: Rectangle) -> layout::Node {
    layout::Node::new(bounds.size()).move_to(Point::new(bounds.x, bounds.y))
}

fn color_background(call: &FillCall) -> Color {
    match call.background {
        Background::Color(color) => color,
        Background::Gradient(_) => panic!("expected color background"),
    }
}

#[test]
fn theme_detection_uses_background_brightness_threshold() {
    let light_theme = Theme::Light;
    let dark_theme = Theme::Dark;

    assert!(!is_dark_theme(&light_theme));
    assert!(is_dark_theme(&dark_theme));
}

#[test]
fn divider_line_color_matches_theme_brightness() {
    assert_eq!(divider_line_color(&Theme::Light), Color::from_rgba8(226, 226, 226, 1.0));
    assert_eq!(divider_line_color(&Theme::Dark), Color::from_rgb8(60, 60, 60));
}

#[test]
fn horizontal_resize_handle_reports_fixed_hit_width_and_fill_height() {
    let handle = HResizeHandle;
    let size = <HResizeHandle as Widget<(), Theme, RecordingRenderer>>::size(&handle);

    assert_eq!(size.width, Length::Fixed(HResizeHandle::HIT_WIDTH));
    assert_eq!(size.height, Length::Fill);
}

#[test]
fn horizontal_resize_handle_layout_uses_hit_width_and_available_height() {
    let mut handle = HResizeHandle;
    let mut state = tree();
    let renderer = RecordingRenderer::default();
    let node = <HResizeHandle as Widget<(), Theme, RecordingRenderer>>::layout(
        &mut handle,
        &mut state,
        &renderer,
        &limits(240.0, 88.0),
    );

    assert_eq!(node.size(), Size::new(HResizeHandle::HIT_WIDTH, 88.0));
}

#[test]
fn horizontal_resize_handle_mouse_interaction_tracks_cursor_bounds() {
    let handle = HResizeHandle;
    let state = tree();
    let renderer = RecordingRenderer::default();
    let node = layout_for(Rectangle { x: 10.0, y: 20.0, width: 6.0, height: 80.0 });
    let layout = Layout::new(&node);

    let over = <HResizeHandle as Widget<(), Theme, RecordingRenderer>>::mouse_interaction(
        &handle,
        &state,
        layout,
        mouse::Cursor::Available(Point::new(12.0, 40.0)),
        &Rectangle::INFINITE,
        &renderer,
    );
    let outside = <HResizeHandle as Widget<(), Theme, RecordingRenderer>>::mouse_interaction(
        &handle,
        &state,
        layout,
        mouse::Cursor::Available(Point::new(3.0, 40.0)),
        &Rectangle::INFINITE,
        &renderer,
    );

    assert_eq!(over, mouse::Interaction::ResizingHorizontally);
    assert_eq!(outside, mouse::Interaction::Idle);
}

#[test]
fn horizontal_resize_handle_draw_is_empty() {
    let handle = HResizeHandle;
    let state = tree();
    let mut renderer = RecordingRenderer::default();
    let node = layout_for(Rectangle { x: 0.0, y: 0.0, width: 6.0, height: 80.0 });

    <HResizeHandle as Widget<(), Theme, RecordingRenderer>>::draw(
        &handle,
        &state,
        &mut renderer,
        &Theme::Dark,
        &renderer::Style::default(),
        Layout::new(&node),
        mouse::Cursor::Unavailable,
        &Rectangle::INFINITE,
    );

    assert!(renderer.fills.is_empty());
}

#[test]
fn horizontal_resize_handle_converts_into_element() {
    let _element: Element<'_, (), Theme, RecordingRenderer> = HResizeHandle.into();
}

#[test]
fn vertical_resize_handle_reports_fill_width_and_fixed_hit_height() {
    let handle = VResizeHandle;
    let size = <VResizeHandle as Widget<(), Theme, RecordingRenderer>>::size(&handle);

    assert_eq!(size.width, Length::Fill);
    assert_eq!(size.height, Length::Fixed(VResizeHandle::HIT_HEIGHT));
}

#[test]
fn vertical_resize_handle_layout_uses_available_width_and_hit_height() {
    let mut handle = VResizeHandle;
    let mut state = tree();
    let renderer = RecordingRenderer::default();
    let node = <VResizeHandle as Widget<(), Theme, RecordingRenderer>>::layout(
        &mut handle,
        &mut state,
        &renderer,
        &limits(240.0, 88.0),
    );

    assert_eq!(node.size(), Size::new(240.0, VResizeHandle::HIT_HEIGHT));
}

#[test]
fn vertical_resize_handle_mouse_interaction_tracks_cursor_bounds() {
    let handle = VResizeHandle;
    let state = tree();
    let renderer = RecordingRenderer::default();
    let node = layout_for(Rectangle { x: 10.0, y: 20.0, width: 160.0, height: 1.0 });
    let layout = Layout::new(&node);

    let over = <VResizeHandle as Widget<(), Theme, RecordingRenderer>>::mouse_interaction(
        &handle,
        &state,
        layout,
        mouse::Cursor::Available(Point::new(20.0, 20.5)),
        &Rectangle::INFINITE,
        &renderer,
    );
    let outside = <VResizeHandle as Widget<(), Theme, RecordingRenderer>>::mouse_interaction(
        &handle,
        &state,
        layout,
        mouse::Cursor::Unavailable,
        &Rectangle::INFINITE,
        &renderer,
    );

    assert_eq!(over, mouse::Interaction::ResizingVertically);
    assert_eq!(outside, mouse::Interaction::Idle);
}

#[test]
fn vertical_resize_handle_draw_is_empty() {
    let handle = VResizeHandle;
    let state = tree();
    let mut renderer = RecordingRenderer::default();
    let node = layout_for(Rectangle { x: 0.0, y: 0.0, width: 160.0, height: 1.0 });

    <VResizeHandle as Widget<(), Theme, RecordingRenderer>>::draw(
        &handle,
        &state,
        &mut renderer,
        &Theme::Dark,
        &renderer::Style::default(),
        Layout::new(&node),
        mouse::Cursor::Unavailable,
        &Rectangle::INFINITE,
    );

    assert!(renderer.fills.is_empty());
}

#[test]
fn vertical_resize_handle_converts_into_element() {
    let _element: Element<'_, (), Theme, RecordingRenderer> = VResizeHandle.into();
}

#[test]
fn top_border_cover_reports_fill_size_and_layout() {
    let mut cover = TopBorderCover;
    let mut state = tree();
    let renderer = RecordingRenderer::default();

    let size = <TopBorderCover as Widget<(), Theme, RecordingRenderer>>::size(&cover);
    let node = <TopBorderCover as Widget<(), Theme, RecordingRenderer>>::layout(
        &mut cover,
        &mut state,
        &renderer,
        &limits(120.0, 48.0),
    );

    assert_eq!(size.width, Length::Fill);
    assert_eq!(size.height, Length::Fill);
    assert_eq!(node.size(), Size::new(120.0, 48.0));
}

#[test]
fn top_border_cover_draws_one_pixel_top_line() {
    let cover = TopBorderCover;
    let state = tree();
    let mut renderer = RecordingRenderer::default();
    let node = layout_for(Rectangle { x: 4.0, y: 8.0, width: 120.0, height: 48.0 });

    <TopBorderCover as Widget<(), Theme, RecordingRenderer>>::draw(
        &cover,
        &state,
        &mut renderer,
        &Theme::Light,
        &renderer::Style::default(),
        Layout::new(&node),
        mouse::Cursor::Unavailable,
        &Rectangle::INFINITE,
    );

    assert_eq!(renderer.fills.len(), 1);
    assert_eq!(renderer.fills[0].bounds, Rectangle { x: 4.0, y: 8.0, width: 120.0, height: 1.0 });
    assert_eq!(color_background(&renderer.fills[0]), divider_line_color(&Theme::Light));
}

#[test]
fn top_border_cover_converts_into_element() {
    let _element: Element<'_, (), Theme, RecordingRenderer> = TopBorderCover.into();
}

#[test]
fn session_panel_right_border_reports_fill_size_and_layout() {
    let mut border = SessionPanelRightBorder;
    let mut state = tree();
    let renderer = RecordingRenderer::default();

    let size = <SessionPanelRightBorder as Widget<(), Theme, RecordingRenderer>>::size(&border);
    let node = <SessionPanelRightBorder as Widget<(), Theme, RecordingRenderer>>::layout(
        &mut border,
        &mut state,
        &renderer,
        &limits(120.0, 48.0),
    );

    assert_eq!(size.width, Length::Fill);
    assert_eq!(size.height, Length::Fill);
    assert_eq!(node.size(), Size::new(120.0, 48.0));
}

#[test]
fn session_panel_right_border_draws_one_pixel_right_line() {
    let border = SessionPanelRightBorder;
    let state = tree();
    let mut renderer = RecordingRenderer::default();
    let node = layout_for(Rectangle { x: 4.0, y: 8.0, width: 120.0, height: 48.0 });

    <SessionPanelRightBorder as Widget<(), Theme, RecordingRenderer>>::draw(
        &border,
        &state,
        &mut renderer,
        &Theme::Dark,
        &renderer::Style::default(),
        Layout::new(&node),
        mouse::Cursor::Unavailable,
        &Rectangle::INFINITE,
    );

    assert_eq!(renderer.fills.len(), 1);
    assert_eq!(renderer.fills[0].bounds, Rectangle { x: 123.0, y: 8.0, width: 1.0, height: 48.0 });
    assert_eq!(color_background(&renderer.fills[0]), divider_line_color(&Theme::Dark));
}

#[test]
fn session_panel_right_border_converts_into_element() {
    let _element: Element<'_, (), Theme, RecordingRenderer> = SessionPanelRightBorder.into();
}

#[test]
fn session_panel_left_border_reports_fill_size_and_layout() {
    let mut border = SessionPanelLeftBorder;
    let mut state = tree();
    let renderer = RecordingRenderer::default();

    let size = <SessionPanelLeftBorder as Widget<(), Theme, RecordingRenderer>>::size(&border);
    let node = <SessionPanelLeftBorder as Widget<(), Theme, RecordingRenderer>>::layout(
        &mut border,
        &mut state,
        &renderer,
        &limits(120.0, 48.0),
    );

    assert_eq!(size.width, Length::Fill);
    assert_eq!(size.height, Length::Fill);
    assert_eq!(node.size(), Size::new(120.0, 48.0));
}

#[test]
fn session_panel_left_border_draws_one_pixel_left_line() {
    let border = SessionPanelLeftBorder;
    let state = tree();
    let mut renderer = RecordingRenderer::default();
    let node = layout_for(Rectangle { x: 4.0, y: 8.0, width: 120.0, height: 48.0 });

    <SessionPanelLeftBorder as Widget<(), Theme, RecordingRenderer>>::draw(
        &border,
        &state,
        &mut renderer,
        &Theme::Dark,
        &renderer::Style::default(),
        Layout::new(&node),
        mouse::Cursor::Unavailable,
        &Rectangle::INFINITE,
    );

    assert_eq!(renderer.fills.len(), 1);
    assert_eq!(renderer.fills[0].bounds, Rectangle { x: 4.0, y: 8.0, width: 1.0, height: 48.0 });
    assert_eq!(color_background(&renderer.fills[0]), divider_line_color(&Theme::Dark));
}

#[test]
fn session_panel_left_border_converts_into_element() {
    let _element: Element<'_, (), Theme, RecordingRenderer> = SessionPanelLeftBorder.into();
}

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("handles_tests"));
}
