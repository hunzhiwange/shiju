use super::{
    SideOverlay, SideOverlayElement, compute_side_overlay_position, side_overlay_max_width,
};
use iced::advanced::{
    Clipboard, Layout, Renderer, Shell, Widget, layout, mouse, overlay, renderer, widget,
};
use iced::{Background, Element, Event, Length, Point, Rectangle, Size, Theme, Transformation};

#[derive(Clone, Debug, PartialEq, Eq)]
enum TestMessage {
    ChildUpdated,
    Close,
}

#[derive(Debug, Clone, Copy)]
struct TestContent {
    width: f32,
    height: f32,
    interaction: mouse::Interaction,
}

impl Default for TestContent {
    fn default() -> Self {
        Self { width: 120.0, height: 40.0, interaction: mouse::Interaction::Pointer }
    }
}

impl Widget<TestMessage, Theme, RecordingRenderer> for TestContent {
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fixed(self.width), Length::Fixed(self.height))
    }

    fn size_hint(&self) -> Size<Length> {
        self.size()
    }

    fn layout(
        &mut self,
        _tree: &mut widget::Tree,
        _renderer: &RecordingRenderer,
        _limits: &layout::Limits,
    ) -> layout::Node {
        layout::Node::new(Size::new(self.width, self.height))
    }

    fn draw(
        &self,
        _tree: &widget::Tree,
        renderer: &mut RecordingRenderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        renderer.fill_quad(
            renderer::Quad { bounds: layout.bounds(), ..renderer::Quad::default() },
            Background::Color(iced::Color::BLACK),
        );
    }

    fn update(
        &mut self,
        _tree: &mut widget::Tree,
        _event: &Event,
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _renderer: &RecordingRenderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, TestMessage>,
        _viewport: &Rectangle,
    ) {
        shell.publish(TestMessage::ChildUpdated);
    }

    fn mouse_interaction(
        &self,
        _tree: &widget::Tree,
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &RecordingRenderer,
    ) -> mouse::Interaction {
        self.interaction
    }
}

impl<'a> From<TestContent> for Element<'a, TestMessage, Theme, RecordingRenderer> {
    fn from(content: TestContent) -> Self {
        Element::new(content)
    }
}

#[derive(Debug, Default)]
struct RecordingRenderer {
    fills: Vec<Rectangle>,
    reset_bounds: Option<Rectangle>,
}

impl Renderer for RecordingRenderer {
    fn start_layer(&mut self, _bounds: Rectangle) {}

    fn end_layer(&mut self) {}

    fn start_transformation(&mut self, _transformation: Transformation) {}

    fn end_transformation(&mut self) {}

    fn fill_quad(&mut self, quad: renderer::Quad, _background: impl Into<Background>) {
        self.fills.push(quad.bounds);
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

fn content(size: Size) -> Element<'static, TestMessage, Theme, RecordingRenderer> {
    TestContent { width: size.width, height: size.height, ..TestContent::default() }.into()
}

fn side_overlay() -> SideOverlay<'static, TestMessage, Theme, RecordingRenderer> {
    SideOverlay::new(content(Size::new(120.0, 40.0)), content(Size::new(80.0, 50.0)))
}

fn tree_for(widget: &SideOverlay<'static, TestMessage, Theme, RecordingRenderer>) -> widget::Tree {
    widget::Tree::new(widget as &dyn Widget<TestMessage, Theme, RecordingRenderer>)
}

fn layout_for(bounds: Rectangle) -> layout::Node {
    layout::Node::new(bounds.size()).move_to(Point::new(bounds.x, bounds.y))
}

fn update_overlay_once(
    event: Event,
    cursor: mouse::Cursor,
    on_close: Option<TestMessage>,
) -> (Vec<TestMessage>, bool) {
    let mut overlay_content = content(Size::new(80.0, 50.0));
    let mut tree = widget::Tree::new(&overlay_content);
    let renderer = RecordingRenderer::default();
    let mut clipboard = iced::advanced::clipboard::Null;
    let node = layout_for(Rectangle { x: 90.0, y: 30.0, width: 80.0, height: 50.0 });
    let mut messages = Vec::new();
    let mut shell = Shell::new(&mut messages);
    let mut overlay = SideOverlayElement {
        position: Point::new(20.0, 30.0),
        target_bounds: Rectangle { x: 20.0, y: 30.0, width: 60.0, height: 24.0 },
        viewport: Rectangle::with_size(Size::new(240.0, 180.0)),
        gap: 8.0,
        min_x: 0.0,
        min_y: 0.0,
        snap_within_viewport: true,
        align_y_start: false,
        overlay: &mut overlay_content,
        tree: &mut tree,
        on_close,
    };

    overlay::Overlay::update(
        &mut overlay,
        &event,
        Layout::new(&node),
        cursor,
        &renderer,
        &mut clipboard,
        &mut shell,
    );

    let event_captured = shell.is_event_captured();
    drop(shell);
    (messages, event_captured)
}

#[test]
fn new_sets_hidden_defaults() {
    let overlay = side_overlay();

    assert!(!overlay.show);
    assert_eq!(overlay.gap, 0.0);
    assert_eq!(overlay.min_x, 0.0);
    assert_eq!(overlay.min_y, 0.0);
    assert!(overlay.snap_within_viewport);
    assert!(!overlay.align_y_start);
    assert!(overlay.on_close.is_none());
}

#[test]
fn builder_updates_all_flags() {
    let overlay = side_overlay()
        .show(true)
        .gap(8.0)
        .min_x(12.0)
        .min_y(16.0)
        .align_y_start(true)
        .snap_within_viewport(false)
        .on_close(TestMessage::Close);

    assert!(overlay.show);
    assert_eq!(overlay.gap, 8.0);
    assert_eq!(overlay.min_x, 12.0);
    assert_eq!(overlay.min_y, 16.0);
    assert!(overlay.align_y_start);
    assert!(!overlay.snap_within_viewport);
    assert_eq!(overlay.on_close, Some(TestMessage::Close));
}

#[test]
fn builder_converts_to_element_with_all_options() {
    let overlay = side_overlay()
        .show(true)
        .gap(8.0)
        .min_x(12.0)
        .min_y(16.0)
        .align_y_start(true)
        .snap_within_viewport(false)
        .on_close(TestMessage::Close);

    let element: Element<'_, TestMessage, Theme, RecordingRenderer> = overlay.into();

    std::hint::black_box(element);
}

#[test]
fn widget_reports_two_children_and_diff_keeps_them() {
    let overlay = side_overlay();
    let children = <SideOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::children(&overlay);

    let mut tree = widget::Tree::empty();
    tree.children.push(widget::Tree::empty());
    tree.children.push(widget::Tree::empty());

    <SideOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::diff(&overlay, &mut tree);

    assert_eq!(children.len(), 2);
    assert_eq!(tree.children.len(), 2);
}

#[test]
fn widget_size_and_layout_delegate_to_content() {
    let mut overlay = side_overlay();
    let mut tree = tree_for(&overlay);
    let renderer = RecordingRenderer::default();

    let size = <SideOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::size(&overlay);
    let hint = <SideOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::size_hint(&overlay);
    let node = <SideOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::layout(
        &mut overlay,
        &mut tree,
        &renderer,
        &layout::Limits::new(Size::ZERO, Size::new(300.0, 200.0)),
    );

    assert_eq!(size, Size::new(Length::Fixed(120.0), Length::Fixed(40.0)));
    assert_eq!(hint, size);
    assert_eq!(node.size(), Size::new(120.0, 40.0));
}

#[test]
fn widget_draw_update_and_interaction_delegate_to_content() {
    let mut overlay = side_overlay();
    let mut tree = tree_for(&overlay);
    let renderer = RecordingRenderer::default();
    let mut drawing_renderer = RecordingRenderer::default();
    let mut clipboard = iced::advanced::clipboard::Null;
    let node = layout_for(Rectangle { x: 4.0, y: 8.0, width: 120.0, height: 40.0 });
    let mut messages = Vec::new();
    let mut shell = Shell::new(&mut messages);

    <SideOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::draw(
        &overlay,
        &tree,
        &mut drawing_renderer,
        &Theme::Dark,
        &renderer::Style::default(),
        Layout::new(&node),
        mouse::Cursor::Unavailable,
        &Rectangle::INFINITE,
    );

    <SideOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::update(
        &mut overlay,
        &mut tree,
        &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        Layout::new(&node),
        mouse::Cursor::Available(Point::new(8.0, 12.0)),
        &renderer,
        &mut clipboard,
        &mut shell,
        &Rectangle::INFINITE,
    );

    let interaction = <SideOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::mouse_interaction(
        &overlay,
        &tree,
        Layout::new(&node),
        mouse::Cursor::Unavailable,
        &Rectangle::INFINITE,
        &renderer,
    );

    assert_eq!(
        drawing_renderer.fills,
        vec![Rectangle { x: 4.0, y: 8.0, width: 120.0, height: 40.0 }]
    );
    assert_eq!(messages, vec![TestMessage::ChildUpdated]);
    assert_eq!(interaction, mouse::Interaction::Pointer);
}

#[test]
fn widget_overlay_is_none_when_hidden_and_some_when_shown() {
    let renderer = RecordingRenderer::default();
    let node = layout_for(Rectangle { x: 10.0, y: 20.0, width: 120.0, height: 40.0 });
    let mut hidden = side_overlay();
    let mut hidden_tree = tree_for(&hidden);
    let mut shown = side_overlay().show(true);
    let mut shown_tree = tree_for(&shown);

    let hidden_overlay = <SideOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::overlay(
        &mut hidden,
        &mut hidden_tree,
        Layout::new(&node),
        &renderer,
        &Rectangle::with_size(Size::new(320.0, 240.0)),
        iced::Vector::new(0.0, 0.0),
    );
    let shown_overlay = <SideOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::overlay(
        &mut shown,
        &mut shown_tree,
        Layout::new(&node),
        &renderer,
        &Rectangle::with_size(Size::new(320.0, 240.0)),
        iced::Vector::new(5.0, 6.0),
    );

    assert!(hidden_overlay.is_none());
    assert!(shown_overlay.is_some());
}

#[test]
fn widget_overlay_preserves_content_overlay_when_side_is_hidden() {
    let renderer = RecordingRenderer::default();
    let node = layout_for(Rectangle { x: 10.0, y: 20.0, width: 120.0, height: 40.0 });
    let content_overlay = side_overlay().show(true);
    let mut outer = SideOverlay::new(content_overlay, content(Size::new(80.0, 50.0)));
    let mut tree = tree_for(&outer);

    let overlay = <SideOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::overlay(
        &mut outer,
        &mut tree,
        Layout::new(&node),
        &renderer,
        &Rectangle::with_size(Size::new(320.0, 240.0)),
        iced::Vector::new(0.0, 0.0),
    );

    assert!(overlay.is_some());
}

#[test]
fn overlay_element_layout_positions_content() {
    let mut overlay_content = content(Size::new(80.0, 50.0));
    let mut tree = widget::Tree::new(&overlay_content);
    let renderer = RecordingRenderer::default();
    let mut overlay = SideOverlayElement {
        position: Point::new(20.0, 30.0),
        target_bounds: Rectangle { x: 20.0, y: 30.0, width: 60.0, height: 24.0 },
        viewport: Rectangle::with_size(Size::new(240.0, 180.0)),
        gap: 8.0,
        min_x: 12.0,
        min_y: 16.0,
        snap_within_viewport: true,
        align_y_start: false,
        overlay: &mut overlay_content,
        tree: &mut tree,
        on_close: None,
    };

    let node = overlay::Overlay::layout(&mut overlay, &renderer, Size::new(240.0, 180.0));

    assert_eq!(node.bounds(), Rectangle { x: 88.0, y: 30.0, width: 80.0, height: 50.0 });
}

#[test]
fn overlay_element_layout_uses_left_side_when_right_is_tight() {
    let mut overlay_content = content(Size::new(140.0, 80.0));
    let mut tree = widget::Tree::new(&overlay_content);
    let renderer = RecordingRenderer::default();
    let mut overlay = SideOverlayElement {
        position: Point::new(220.0, 50.0),
        target_bounds: Rectangle { x: 220.0, y: 50.0, width: 70.0, height: 24.0 },
        viewport: Rectangle::with_size(Size::new(320.0, 220.0)),
        gap: 8.0,
        min_x: 0.0,
        min_y: 0.0,
        snap_within_viewport: true,
        align_y_start: false,
        overlay: &mut overlay_content,
        tree: &mut tree,
        on_close: None,
    };

    let node = overlay::Overlay::layout(&mut overlay, &renderer, Size::new(320.0, 220.0));

    assert_eq!(node.bounds(), Rectangle { x: 72.0, y: 50.0, width: 140.0, height: 80.0 });
}

#[test]
fn overlay_element_click_outside_publishes_close_and_captures() {
    let mut overlay_content = content(Size::new(80.0, 50.0));
    let mut tree = widget::Tree::new(&overlay_content);
    let renderer = RecordingRenderer::default();
    let mut clipboard = iced::advanced::clipboard::Null;
    let node = layout_for(Rectangle { x: 90.0, y: 30.0, width: 80.0, height: 50.0 });
    let mut messages = Vec::new();
    let mut shell = Shell::new(&mut messages);
    let mut overlay = SideOverlayElement {
        position: Point::new(20.0, 30.0),
        target_bounds: Rectangle { x: 20.0, y: 30.0, width: 60.0, height: 24.0 },
        viewport: Rectangle::with_size(Size::new(240.0, 180.0)),
        gap: 8.0,
        min_x: 0.0,
        min_y: 0.0,
        snap_within_viewport: true,
        align_y_start: false,
        overlay: &mut overlay_content,
        tree: &mut tree,
        on_close: Some(TestMessage::Close),
    };

    overlay::Overlay::update(
        &mut overlay,
        &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        Layout::new(&node),
        mouse::Cursor::Available(Point::new(5.0, 5.0)),
        &renderer,
        &mut clipboard,
        &mut shell,
    );

    let event_captured = shell.is_event_captured();
    assert_eq!(messages, vec![TestMessage::Close, TestMessage::ChildUpdated]);
    assert!(event_captured);
}

#[test]
fn overlay_element_click_inside_target_does_not_close() {
    let mut overlay_content = content(Size::new(80.0, 50.0));
    let mut tree = widget::Tree::new(&overlay_content);
    let renderer = RecordingRenderer::default();
    let mut clipboard = iced::advanced::clipboard::Null;
    let node = layout_for(Rectangle { x: 90.0, y: 30.0, width: 80.0, height: 50.0 });
    let mut messages = Vec::new();
    let mut shell = Shell::new(&mut messages);
    let mut overlay = SideOverlayElement {
        position: Point::new(20.0, 30.0),
        target_bounds: Rectangle { x: 20.0, y: 30.0, width: 60.0, height: 24.0 },
        viewport: Rectangle::with_size(Size::new(240.0, 180.0)),
        gap: 8.0,
        min_x: 0.0,
        min_y: 0.0,
        snap_within_viewport: true,
        align_y_start: false,
        overlay: &mut overlay_content,
        tree: &mut tree,
        on_close: Some(TestMessage::Close),
    };

    overlay::Overlay::update(
        &mut overlay,
        &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        Layout::new(&node),
        mouse::Cursor::Available(Point::new(30.0, 40.0)),
        &renderer,
        &mut clipboard,
        &mut shell,
    );

    let event_captured = shell.is_event_captured();
    assert_eq!(messages, vec![TestMessage::ChildUpdated]);
    assert!(!event_captured);
}

#[test]
fn overlay_element_mouse_inside_overlay_captures_without_close() {
    let mut overlay_content = content(Size::new(80.0, 50.0));
    let mut tree = widget::Tree::new(&overlay_content);
    let renderer = RecordingRenderer::default();
    let mut clipboard = iced::advanced::clipboard::Null;
    let node = layout_for(Rectangle { x: 90.0, y: 30.0, width: 80.0, height: 50.0 });
    let mut messages = Vec::new();
    let mut shell = Shell::new(&mut messages);
    let mut overlay = SideOverlayElement {
        position: Point::new(20.0, 30.0),
        target_bounds: Rectangle { x: 20.0, y: 30.0, width: 60.0, height: 24.0 },
        viewport: Rectangle::with_size(Size::new(240.0, 180.0)),
        gap: 8.0,
        min_x: 0.0,
        min_y: 0.0,
        snap_within_viewport: true,
        align_y_start: false,
        overlay: &mut overlay_content,
        tree: &mut tree,
        on_close: Some(TestMessage::Close),
    };

    overlay::Overlay::update(
        &mut overlay,
        &Event::Mouse(mouse::Event::CursorMoved { position: Point::new(100.0, 40.0) }),
        Layout::new(&node),
        mouse::Cursor::Available(Point::new(100.0, 40.0)),
        &renderer,
        &mut clipboard,
        &mut shell,
    );

    let event_captured = shell.is_event_captured();
    assert_eq!(messages, vec![TestMessage::ChildUpdated]);
    assert!(event_captured);
}

#[test]
fn overlay_element_right_click_outside_does_not_close_or_capture() {
    let (messages, event_captured) = update_overlay_once(
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)),
        mouse::Cursor::Available(Point::new(5.0, 5.0)),
        Some(TestMessage::Close),
    );

    assert_eq!(messages, vec![TestMessage::ChildUpdated]);
    assert!(!event_captured);
}

#[test]
fn overlay_element_left_click_without_close_message_is_forwarded() {
    let (messages, event_captured) = update_overlay_once(
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        mouse::Cursor::Available(Point::new(5.0, 5.0)),
        None,
    );

    assert_eq!(messages, vec![TestMessage::ChildUpdated]);
    assert!(!event_captured);
}

#[test]
fn overlay_element_left_click_with_unavailable_cursor_is_forwarded() {
    let (messages, event_captured) = update_overlay_once(
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        mouse::Cursor::Unavailable,
        Some(TestMessage::Close),
    );

    assert_eq!(messages, vec![TestMessage::ChildUpdated]);
    assert!(!event_captured);
}

#[test]
fn overlay_element_left_click_inside_overlay_does_not_close_but_captures() {
    let (messages, event_captured) = update_overlay_once(
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        mouse::Cursor::Available(Point::new(100.0, 40.0)),
        Some(TestMessage::Close),
    );

    assert_eq!(messages, vec![TestMessage::ChildUpdated]);
    assert!(event_captured);
}

#[test]
fn overlay_element_interaction_and_draw_delegate_to_content() {
    let mut overlay_content: Element<'_, TestMessage, Theme, RecordingRenderer> =
        TestContent { width: 80.0, height: 50.0, interaction: mouse::Interaction::Text }.into();
    let mut tree = widget::Tree::new(&overlay_content);
    let renderer = RecordingRenderer::default();
    let mut drawing_renderer = RecordingRenderer::default();
    let node = layout_for(Rectangle { x: 90.0, y: 30.0, width: 80.0, height: 50.0 });
    let overlay = SideOverlayElement {
        position: Point::new(20.0, 30.0),
        target_bounds: Rectangle { x: 20.0, y: 30.0, width: 60.0, height: 24.0 },
        viewport: Rectangle::with_size(Size::new(240.0, 180.0)),
        gap: 8.0,
        min_x: 0.0,
        min_y: 0.0,
        snap_within_viewport: true,
        align_y_start: false,
        overlay: &mut overlay_content,
        tree: &mut tree,
        on_close: None,
    };

    let interaction = overlay::Overlay::mouse_interaction(
        &overlay,
        Layout::new(&node),
        mouse::Cursor::Available(Point::new(100.0, 40.0)),
        &renderer,
    );
    overlay::Overlay::draw(
        &overlay,
        &mut drawing_renderer,
        &Theme::Dark,
        &renderer::Style::default(),
        Layout::new(&node),
        mouse::Cursor::Unavailable,
    );

    assert_eq!(interaction, mouse::Interaction::Text);
    assert_eq!(
        drawing_renderer.fills,
        vec![Rectangle { x: 90.0, y: 30.0, width: 80.0, height: 50.0 }]
    );
}

#[test]
fn max_width_uses_larger_side_when_snapping() {
    let bounds = Size::new(320.0, 200.0);
    let position = Point::new(220.0, 40.0);
    let target = Rectangle { x: 220.0, y: 40.0, width: 60.0, height: 24.0 };

    let width = side_overlay_max_width(bounds, position, target, 8.0, true);

    assert_eq!(width, 212.0);
}

#[test]
fn max_width_uses_right_side_when_it_is_larger() {
    let bounds = Size::new(320.0, 200.0);
    let position = Point::new(20.0, 40.0);
    let target = Rectangle { x: 20.0, y: 40.0, width: 60.0, height: 24.0 };

    let width = side_overlay_max_width(bounds, position, target, 8.0, true);

    assert_eq!(width, 232.0);
}

#[test]
fn max_width_never_goes_negative_when_snapping() {
    let bounds = Size::new(80.0, 200.0);
    let position = Point::new(20.0, 40.0);
    let target = Rectangle { x: 20.0, y: 40.0, width: 60.0, height: 24.0 };

    let width = side_overlay_max_width(bounds, position, target, 48.0, true);

    assert_eq!(width, 0.0);
}

#[test]
fn max_width_uses_full_bounds_without_snapping() {
    let bounds = Size::new(320.0, 200.0);
    let position = Point::new(220.0, 40.0);
    let target = Rectangle { x: 220.0, y: 40.0, width: 60.0, height: 24.0 };

    let width = side_overlay_max_width(bounds, position, target, 8.0, false);

    assert_eq!(width, 320.0);
}

#[test]
fn position_prefers_right_when_there_is_room() {
    let point = compute_side_overlay_position(
        Point::new(40.0, 30.0),
        Rectangle { x: 40.0, y: 30.0, width: 80.0, height: 24.0 },
        Rectangle::with_size(Size::new(320.0, 220.0)),
        Size::new(120.0, 80.0),
        10.0,
        0.0,
        0.0,
        true,
        false,
    );

    assert_eq!(point, Point::new(130.0, 30.0));
}

#[test]
fn position_prefers_right_when_right_side_is_larger_even_without_full_room() {
    let point = compute_side_overlay_position(
        Point::new(70.0, 30.0),
        Rectangle { x: 70.0, y: 30.0, width: 20.0, height: 24.0 },
        Rectangle::with_size(Size::new(170.0, 220.0)),
        Size::new(80.0, 60.0),
        10.0,
        0.0,
        0.0,
        true,
        false,
    );

    assert_eq!(point, Point::new(90.0, 30.0));
}

#[test]
fn position_prefers_right_when_both_sides_are_tied() {
    let point = compute_side_overlay_position(
        Point::new(90.0, 30.0),
        Rectangle { x: 90.0, y: 30.0, width: 20.0, height: 24.0 },
        Rectangle::with_size(Size::new(200.0, 220.0)),
        Size::new(90.0, 60.0),
        10.0,
        0.0,
        0.0,
        true,
        false,
    );

    assert_eq!(point, Point::new(110.0, 30.0));
}

#[test]
fn position_falls_back_left_when_right_side_is_tight() {
    let point = compute_side_overlay_position(
        Point::new(220.0, 50.0),
        Rectangle { x: 220.0, y: 50.0, width: 70.0, height: 24.0 },
        Rectangle::with_size(Size::new(320.0, 220.0)),
        Size::new(140.0, 80.0),
        8.0,
        0.0,
        0.0,
        true,
        false,
    );

    assert_eq!(point, Point::new(72.0, 50.0));
}

#[test]
fn position_clamps_to_minimums_and_viewport() {
    let point = compute_side_overlay_position(
        Point::new(8.0, 190.0),
        Rectangle { x: 8.0, y: 190.0, width: 24.0, height: 20.0 },
        Rectangle::with_size(Size::new(180.0, 220.0)),
        Size::new(220.0, 90.0),
        8.0,
        12.0,
        18.0,
        true,
        false,
    );

    assert_eq!(point, Point::new(12.0, 130.0));
}

#[test]
fn position_clamps_to_min_y_when_overlay_is_taller_than_viewport() {
    let point = compute_side_overlay_position(
        Point::new(40.0, 90.0),
        Rectangle { x: 40.0, y: 90.0, width: 30.0, height: 20.0 },
        Rectangle::with_size(Size::new(220.0, 80.0)),
        Size::new(60.0, 160.0),
        4.0,
        0.0,
        24.0,
        true,
        false,
    );

    assert_eq!(point, Point::new(74.0, 24.0));
}

#[test]
fn position_aligns_to_min_y_when_requested() {
    let point = compute_side_overlay_position(
        Point::new(40.0, 90.0),
        Rectangle { x: 40.0, y: 90.0, width: 30.0, height: 20.0 },
        Rectangle::with_size(Size::new(220.0, 180.0)),
        Size::new(60.0, 40.0),
        4.0,
        0.0,
        24.0,
        true,
        true,
    );

    assert_eq!(point, Point::new(74.0, 24.0));
}

#[test]
fn position_can_extend_without_snapping() {
    let point = compute_side_overlay_position(
        Point::new(190.0, 170.0),
        Rectangle { x: 190.0, y: 170.0, width: 40.0, height: 20.0 },
        Rectangle::with_size(Size::new(220.0, 180.0)),
        Size::new(80.0, 40.0),
        6.0,
        10.0,
        10.0,
        false,
        false,
    );

    assert_eq!(point, Point::new(104.0, 170.0));
}

#[test]
fn position_without_snapping_still_respects_min_y() {
    let point = compute_side_overlay_position(
        Point::new(40.0, 4.0),
        Rectangle { x: 40.0, y: 4.0, width: 30.0, height: 20.0 },
        Rectangle::with_size(Size::new(220.0, 180.0)),
        Size::new(60.0, 40.0),
        4.0,
        20.0,
        24.0,
        false,
        false,
    );

    assert_eq!(point, Point::new(74.0, 24.0));
}

#[test]
fn position_without_snapping_can_extend_left_of_min_x() {
    let point = compute_side_overlay_position(
        Point::new(90.0, 30.0),
        Rectangle { x: 90.0, y: 30.0, width: 20.0, height: 24.0 },
        Rectangle::with_size(Size::new(100.0, 180.0)),
        Size::new(80.0, 40.0),
        4.0,
        20.0,
        0.0,
        false,
        false,
    );

    assert_eq!(point, Point::new(6.0, 30.0));
}
