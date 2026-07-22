use super::{
    LeftOverlay, LeftOverlayElement, PointLeftOverlay, PointLeftOverlayElement,
    compute_left_position, compute_point_left_position,
};
use iced::advanced::{
    Clipboard, Layout, Renderer as _, Shell, Widget, layout, mouse, overlay, renderer, widget,
};
use iced::widget::text;
use iced::{
    Background, Color, Element, Event, Length, Point, Rectangle, Size, Theme, Transformation,
};

#[derive(Debug, Clone, PartialEq)]
enum TestMessage {
    Close,
    ChildUpdated,
}

fn label(value: &'static str) -> Element<'static, TestMessage> {
    text(value).into()
}

#[derive(Debug, Clone, Copy)]
struct TestContent {
    width: f32,
    height: f32,
    interaction: mouse::Interaction,
    publish_update: bool,
}

impl TestContent {
    fn new(width: f32, height: f32) -> Self {
        Self { width, height, interaction: mouse::Interaction::Text, publish_update: false }
    }

    fn with_update(mut self) -> Self {
        self.publish_update = true;
        self
    }

    fn with_interaction(mut self, interaction: mouse::Interaction) -> Self {
        self.interaction = interaction;
        self
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
        if self.publish_update {
            shell.publish(TestMessage::ChildUpdated);
        }
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
            Background::Color(Color::BLACK),
        );
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

fn fixed(width: f32, height: f32) -> TestContent {
    TestContent::new(width, height)
}

fn left_overlay_widget() -> LeftOverlay<'static, TestMessage, Theme, RecordingRenderer> {
    LeftOverlay::new(
        fixed(120.0, 40.0).with_update().with_interaction(mouse::Interaction::Pointer),
        fixed(70.0, 20.0).with_interaction(mouse::Interaction::Crosshair),
    )
}

fn point_left_overlay_widget() -> PointLeftOverlay<'static, TestMessage, Theme, RecordingRenderer> {
    PointLeftOverlay::new(
        fixed(120.0, 40.0).with_update().with_interaction(mouse::Interaction::Pointer),
        fixed(70.0, 20.0).with_interaction(mouse::Interaction::Crosshair),
    )
}

fn left_tree(widget: &LeftOverlay<'static, TestMessage, Theme, RecordingRenderer>) -> widget::Tree {
    let mut tree = widget::Tree::empty();
    tree.children = <LeftOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::children(widget);
    tree
}

fn point_left_tree(
    widget: &PointLeftOverlay<'static, TestMessage, Theme, RecordingRenderer>,
) -> widget::Tree {
    let mut tree = widget::Tree::empty();
    tree.children = <PointLeftOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::children(widget);
    tree
}

fn node_at(bounds: Rectangle) -> layout::Node {
    layout::Node::new(bounds.size()).move_to(Point::new(bounds.x, bounds.y))
}

fn layout_limits() -> layout::Limits {
    layout::Limits::new(Size::ZERO, Size::new(500.0, 400.0))
}

#[test]
fn left_overlay_new_sets_hidden_defaults() {
    let overlay = LeftOverlay::new(label("content"), label("overlay"));

    assert!(!overlay.show);
    assert_eq!(overlay.gap, 0.0);
    assert!(overlay.snap_within_viewport);
    assert!(overlay.on_close.is_none());
}

#[test]
fn left_overlay_builder_updates_all_flags() {
    let overlay = LeftOverlay::new(label("content"), label("overlay"))
        .show(true)
        .gap(6.0)
        .snap_within_viewport(false)
        .on_close(TestMessage::Close);

    assert!(overlay.show);
    assert_eq!(overlay.gap, 6.0);
    assert!(!overlay.snap_within_viewport);
    assert_eq!(overlay.on_close, Some(TestMessage::Close));
}

#[test]
fn point_left_overlay_new_sets_hidden_defaults() {
    let overlay = PointLeftOverlay::new(label("content"), label("overlay"));

    assert!(!overlay.show);
    assert_eq!(overlay.anchor, Point::ORIGIN);
    assert_eq!(overlay.gap, 0.0);
    assert!(overlay.snap_within_viewport);
    assert!(overlay.on_close.is_none());
}

#[test]
fn point_left_overlay_builder_updates_all_flags() {
    let overlay = PointLeftOverlay::new(label("content"), label("overlay"))
        .show(true)
        .anchor(Point::new(24.0, 80.0))
        .gap(10.0)
        .snap_within_viewport(false)
        .on_close(TestMessage::Close);

    assert!(overlay.show);
    assert_eq!(overlay.anchor, Point::new(24.0, 80.0));
    assert_eq!(overlay.gap, 10.0);
    assert!(!overlay.snap_within_viewport);
    assert_eq!(overlay.on_close, Some(TestMessage::Close));
}

#[test]
fn left_position_centers_on_target_height_without_snap() {
    let position = compute_left_position(
        Point::new(90.0, 40.0),
        Rectangle { x: 90.0, y: 40.0, width: 30.0, height: 50.0 },
        Rectangle::with_size(Size::new(200.0, 120.0)),
        Size::new(70.0, 20.0),
        8.0,
        false,
    );

    assert_eq!(position, Point::new(12.0, 55.0));
}

#[test]
fn left_position_snaps_inside_viewport() {
    let position = compute_left_position(
        Point::new(40.0, 112.0),
        Rectangle { x: 40.0, y: 112.0, width: 30.0, height: 50.0 },
        Rectangle::with_size(Size::new(120.0, 120.0)),
        Size::new(70.0, 40.0),
        8.0,
        true,
    );

    assert_eq!(position, Point::new(0.0, 80.0));
}

#[test]
fn left_position_snaps_to_lower_right_edges() {
    let position = compute_left_position(
        Point::new(250.0, 200.0),
        Rectangle { x: 250.0, y: 200.0, width: 30.0, height: 40.0 },
        Rectangle::with_size(Size::new(180.0, 120.0)),
        Size::new(70.0, 40.0),
        10.0,
        true,
    );

    assert_eq!(position, Point::new(110.0, 80.0));
}

#[test]
fn left_position_snaps_oversized_overlay_to_origin() {
    let position = compute_left_position(
        Point::new(200.0, 150.0),
        Rectangle { x: 200.0, y: 150.0, width: 20.0, height: 20.0 },
        Rectangle::with_size(Size::new(50.0, 30.0)),
        Size::new(80.0, 40.0),
        0.0,
        true,
    );

    assert_eq!(position, Point::ORIGIN);
}

#[test]
fn point_left_position_centers_on_anchor() {
    let position = compute_point_left_position(
        Point::new(100.0, 60.0),
        Rectangle::with_size(Size::new(180.0, 120.0)),
        Size::new(70.0, 40.0),
        8.0,
        false,
    );

    assert_eq!(position, Point::new(22.0, 40.0));
}

#[test]
fn point_left_position_snaps_inside_viewport() {
    let position = compute_point_left_position(
        Point::new(30.0, 118.0),
        Rectangle::with_size(Size::new(180.0, 120.0)),
        Size::new(70.0, 40.0),
        8.0,
        true,
    );

    assert_eq!(position, Point::new(0.0, 80.0));
}

#[test]
fn point_left_position_snaps_to_lower_right_edges() {
    let position = compute_point_left_position(
        Point::new(250.0, 200.0),
        Rectangle::with_size(Size::new(180.0, 120.0)),
        Size::new(70.0, 40.0),
        10.0,
        true,
    );

    assert_eq!(position, Point::new(110.0, 80.0));
}

#[test]
fn point_left_position_snaps_oversized_overlay_to_origin() {
    let position = compute_point_left_position(
        Point::new(200.0, 150.0),
        Rectangle::with_size(Size::new(50.0, 30.0)),
        Size::new(80.0, 40.0),
        0.0,
        true,
    );

    assert_eq!(position, Point::ORIGIN);
}

#[test]
fn left_overlays_convert_to_elements() {
    let _: Element<'static, TestMessage, Theme> =
        LeftOverlay::new(label("content"), label("overlay")).into();
    let _: Element<'static, TestMessage, Theme> =
        PointLeftOverlay::new(label("content"), label("overlay")).into();
}

#[test]
fn left_overlay_widget_delegates_to_content() {
    let mut widget = left_overlay_widget();
    let mut tree = left_tree(&widget);
    let renderer = RecordingRenderer::default();
    let limits = layout_limits();

    assert_eq!(tree.children.len(), 2);

    <LeftOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::diff(&widget, &mut tree);

    let size = <LeftOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::size(&widget);
    let hint = <LeftOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::size_hint(&widget);
    let node = <LeftOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::layout(&mut widget, &mut tree, &renderer, &limits);

    assert_eq!(size, Size::new(Length::Fixed(120.0), Length::Fixed(40.0)));
    assert_eq!(hint, size);
    assert_eq!(node.bounds(), Rectangle { x: 0.0, y: 0.0, width: 120.0, height: 40.0 });
}

#[test]
fn left_overlay_widget_draw_update_and_interaction_delegate_to_content() {
    let mut widget = left_overlay_widget();
    let mut tree = left_tree(&widget);
    let renderer = RecordingRenderer::default();
    let mut draw_renderer = RecordingRenderer::default();
    let mut clipboard = iced::advanced::clipboard::Null;
    let node = node_at(Rectangle { x: 10.0, y: 20.0, width: 120.0, height: 40.0 });
    let mut messages = Vec::new();
    let mut shell = Shell::new(&mut messages);

    <LeftOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::draw(
        &widget,
        &tree,
        &mut draw_renderer,
        &Theme::Dark,
        &renderer::Style::default(),
        Layout::new(&node),
        mouse::Cursor::Unavailable,
        &Rectangle::INFINITE,
    );

    <LeftOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::update(
        &mut widget,
        &mut tree,
        &Event::Mouse(mouse::Event::CursorMoved { position: Point::new(12.0, 24.0) }),
        Layout::new(&node),
        mouse::Cursor::Available(Point::new(12.0, 24.0)),
        &renderer,
        &mut clipboard,
        &mut shell,
        &Rectangle::INFINITE,
    );

    let interaction = <LeftOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::mouse_interaction(
        &widget,
        &tree,
        Layout::new(&node),
        mouse::Cursor::Unavailable,
        &Rectangle::INFINITE,
        &renderer,
    );

    drop(shell);
    assert_eq!(draw_renderer.fills, vec![node.bounds()]);
    assert_eq!(messages, vec![TestMessage::ChildUpdated]);
    assert_eq!(interaction, mouse::Interaction::Pointer);
}

#[test]
fn left_overlay_widget_only_builds_overlay_when_shown() {
    let renderer = RecordingRenderer::default();
    let viewport = Rectangle::with_size(Size::new(300.0, 200.0));
    let node = node_at(Rectangle { x: 90.0, y: 40.0, width: 30.0, height: 50.0 });
    let mut hidden = left_overlay_widget();
    let mut hidden_tree = left_tree(&hidden);

    let hidden_overlay = <LeftOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::overlay(
        &mut hidden,
        &mut hidden_tree,
        Layout::new(&node),
        &renderer,
        &viewport,
        iced::Vector::ZERO,
    );

    let mut shown = left_overlay_widget().show(true).gap(8.0);
    let mut shown_tree = left_tree(&shown);
    let shown_overlay = <LeftOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::overlay(
        &mut shown,
        &mut shown_tree,
        Layout::new(&node),
        &renderer,
        &viewport,
        iced::Vector::new(5.0, 6.0),
    );

    assert!(hidden_overlay.is_none());
    assert!(shown_overlay.is_some());
}

#[test]
fn point_left_overlay_widget_delegates_to_content() {
    let mut widget = point_left_overlay_widget();
    let mut tree = point_left_tree(&widget);
    let renderer = RecordingRenderer::default();
    let limits = layout_limits();

    assert_eq!(tree.children.len(), 2);

    <PointLeftOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::diff(&widget, &mut tree);

    let size = <PointLeftOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::size(&widget);
    let hint = <PointLeftOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::size_hint(&widget);
    let node = <PointLeftOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::layout(&mut widget, &mut tree, &renderer, &limits);

    assert_eq!(size, Size::new(Length::Fixed(120.0), Length::Fixed(40.0)));
    assert_eq!(hint, size);
    assert_eq!(node.bounds(), Rectangle { x: 0.0, y: 0.0, width: 120.0, height: 40.0 });
}

#[test]
fn point_left_overlay_widget_draw_update_and_interaction_delegate_to_content() {
    let mut widget = point_left_overlay_widget();
    let mut tree = point_left_tree(&widget);
    let renderer = RecordingRenderer::default();
    let mut draw_renderer = RecordingRenderer::default();
    let mut clipboard = iced::advanced::clipboard::Null;
    let node = node_at(Rectangle { x: 10.0, y: 20.0, width: 120.0, height: 40.0 });
    let mut messages = Vec::new();
    let mut shell = Shell::new(&mut messages);

    <PointLeftOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::draw(
        &widget,
        &tree,
        &mut draw_renderer,
        &Theme::Dark,
        &renderer::Style::default(),
        Layout::new(&node),
        mouse::Cursor::Unavailable,
        &Rectangle::INFINITE,
    );

    <PointLeftOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::update(
        &mut widget,
        &mut tree,
        &Event::Mouse(mouse::Event::CursorMoved { position: Point::new(12.0, 24.0) }),
        Layout::new(&node),
        mouse::Cursor::Available(Point::new(12.0, 24.0)),
        &renderer,
        &mut clipboard,
        &mut shell,
        &Rectangle::INFINITE,
    );

    let interaction = <PointLeftOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::mouse_interaction(
        &widget,
        &tree,
        Layout::new(&node),
        mouse::Cursor::Unavailable,
        &Rectangle::INFINITE,
        &renderer,
    );

    drop(shell);
    assert_eq!(draw_renderer.fills, vec![node.bounds()]);
    assert_eq!(messages, vec![TestMessage::ChildUpdated]);
    assert_eq!(interaction, mouse::Interaction::Pointer);
}

#[test]
fn point_left_overlay_widget_only_builds_overlay_when_shown() {
    let renderer = RecordingRenderer::default();
    let viewport = Rectangle::with_size(Size::new(300.0, 200.0));
    let node = node_at(Rectangle { x: 90.0, y: 40.0, width: 30.0, height: 50.0 });
    let mut hidden = point_left_overlay_widget();
    let mut hidden_tree = point_left_tree(&hidden);

    let hidden_overlay = <PointLeftOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::overlay(
        &mut hidden,
        &mut hidden_tree,
        Layout::new(&node),
        &renderer,
        &viewport,
        iced::Vector::ZERO,
    );

    let mut shown = point_left_overlay_widget().show(true).anchor(Point::new(24.0, 80.0));
    let mut shown_tree = point_left_tree(&shown);
    let shown_overlay = <PointLeftOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::overlay(
        &mut shown,
        &mut shown_tree,
        Layout::new(&node),
        &renderer,
        &viewport,
        iced::Vector::new(5.0, 6.0),
    );

    assert!(hidden_overlay.is_none());
    assert!(shown_overlay.is_some());
}

#[test]
fn left_overlay_element_layout_uses_target_center_and_gap() {
    let renderer = RecordingRenderer::default();
    let mut overlay: Element<'static, TestMessage, Theme, RecordingRenderer> =
        fixed(70.0, 20.0).into();
    let mut tree = widget::Tree::new(&overlay);
    let mut element = LeftOverlayElement {
        position: Point::new(90.0, 40.0),
        target_bounds: Rectangle { x: 90.0, y: 40.0, width: 30.0, height: 50.0 },
        viewport: Rectangle::with_size(Size::new(200.0, 120.0)),
        gap: 8.0,
        snap_within_viewport: false,
        overlay: &mut overlay,
        tree: &mut tree,
        on_close: None,
    };

    let node = overlay::Overlay::layout(&mut element, &renderer, Size::new(200.0, 120.0));

    assert_eq!(node.bounds(), Rectangle { x: 12.0, y: 55.0, width: 70.0, height: 20.0 });
}

#[test]
fn left_overlay_element_layout_snaps_to_viewport() {
    let renderer = RecordingRenderer::default();
    let mut overlay: Element<'static, TestMessage, Theme, RecordingRenderer> =
        fixed(70.0, 40.0).into();
    let mut tree = widget::Tree::new(&overlay);
    let mut element = LeftOverlayElement {
        position: Point::new(40.0, 112.0),
        target_bounds: Rectangle { x: 40.0, y: 112.0, width: 30.0, height: 50.0 },
        viewport: Rectangle::with_size(Size::new(120.0, 120.0)),
        gap: 8.0,
        snap_within_viewport: true,
        overlay: &mut overlay,
        tree: &mut tree,
        on_close: None,
    };

    let node = overlay::Overlay::layout(&mut element, &renderer, Size::new(120.0, 120.0));

    assert_eq!(node.bounds(), Rectangle { x: 0.0, y: 80.0, width: 70.0, height: 40.0 });
}

#[test]
fn left_overlay_element_closes_on_click_outside_target_and_overlay() {
    let renderer = RecordingRenderer::default();
    let mut clipboard = iced::advanced::clipboard::Null;
    let mut overlay: Element<'static, TestMessage, Theme, RecordingRenderer> =
        fixed(70.0, 20.0).into();
    let mut tree = widget::Tree::new(&overlay);
    let mut element = LeftOverlayElement {
        position: Point::new(90.0, 40.0),
        target_bounds: Rectangle { x: 90.0, y: 40.0, width: 30.0, height: 50.0 },
        viewport: Rectangle::with_size(Size::new(200.0, 120.0)),
        gap: 8.0,
        snap_within_viewport: false,
        overlay: &mut overlay,
        tree: &mut tree,
        on_close: Some(TestMessage::Close),
    };
    let node = node_at(Rectangle { x: 12.0, y: 55.0, width: 70.0, height: 20.0 });
    let mut messages = Vec::new();
    let mut shell = Shell::new(&mut messages);

    overlay::Overlay::update(
        &mut element,
        &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        Layout::new(&node),
        mouse::Cursor::Available(Point::new(5.0, 5.0)),
        &renderer,
        &mut clipboard,
        &mut shell,
    );

    let captured = shell.is_event_captured();
    drop(shell);
    assert_eq!(messages, vec![TestMessage::Close]);
    assert!(captured);
}

#[test]
fn left_overlay_element_keeps_clicks_inside_target_or_overlay_open() {
    let renderer = RecordingRenderer::default();
    let mut clipboard = iced::advanced::clipboard::Null;
    let node = node_at(Rectangle { x: 12.0, y: 55.0, width: 70.0, height: 20.0 });

    for cursor_position in [Point::new(95.0, 45.0), Point::new(20.0, 60.0)] {
        let mut overlay: Element<'static, TestMessage, Theme, RecordingRenderer> =
            fixed(70.0, 20.0).into();
        let mut tree = widget::Tree::new(&overlay);
        let mut element = LeftOverlayElement {
            position: Point::new(90.0, 40.0),
            target_bounds: Rectangle { x: 90.0, y: 40.0, width: 30.0, height: 50.0 },
            viewport: Rectangle::with_size(Size::new(200.0, 120.0)),
            gap: 8.0,
            snap_within_viewport: false,
            overlay: &mut overlay,
            tree: &mut tree,
            on_close: Some(TestMessage::Close),
        };
        let mut messages = Vec::new();
        let mut shell = Shell::new(&mut messages);

        overlay::Overlay::update(
            &mut element,
            &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Layout::new(&node),
            mouse::Cursor::Available(cursor_position),
            &renderer,
            &mut clipboard,
            &mut shell,
        );

        let captured = shell.is_event_captured();
        drop(shell);
        assert!(messages.is_empty());
        assert!(captured);
    }
}

#[test]
fn left_overlay_element_rejects_close_without_all_preconditions() {
    let renderer = RecordingRenderer::default();
    let mut clipboard = iced::advanced::clipboard::Null;
    let node = node_at(Rectangle { x: 12.0, y: 55.0, width: 70.0, height: 20.0 });

    for (button, cursor, on_close) in [
        (
            mouse::Button::Right,
            mouse::Cursor::Available(Point::new(5.0, 5.0)),
            Some(TestMessage::Close),
        ),
        (mouse::Button::Left, mouse::Cursor::Unavailable, Some(TestMessage::Close)),
        (mouse::Button::Left, mouse::Cursor::Available(Point::new(5.0, 5.0)), None),
    ] {
        let mut overlay: Element<'static, TestMessage, Theme, RecordingRenderer> =
            fixed(70.0, 20.0).into();
        let mut tree = widget::Tree::new(&overlay);
        let mut element = LeftOverlayElement {
            position: Point::new(90.0, 40.0),
            target_bounds: Rectangle { x: 90.0, y: 40.0, width: 30.0, height: 50.0 },
            viewport: Rectangle::with_size(Size::new(200.0, 120.0)),
            gap: 8.0,
            snap_within_viewport: false,
            overlay: &mut overlay,
            tree: &mut tree,
            on_close,
        };
        let mut messages = Vec::new();
        let mut shell = Shell::new(&mut messages);

        overlay::Overlay::update(
            &mut element,
            &Event::Mouse(mouse::Event::ButtonPressed(button)),
            Layout::new(&node),
            cursor,
            &renderer,
            &mut clipboard,
            &mut shell,
        );

        let captured = shell.is_event_captured();
        drop(shell);
        assert!(messages.is_empty());
        assert!(captured);
    }
}

#[test]
fn left_overlay_element_delegates_draw_and_mouse_interaction() {
    let renderer = RecordingRenderer::default();
    let mut draw_renderer = RecordingRenderer::default();
    let mut overlay: Element<'static, TestMessage, Theme, RecordingRenderer> =
        fixed(70.0, 20.0).with_interaction(mouse::Interaction::Crosshair).into();
    let mut tree = widget::Tree::new(&overlay);
    let element = LeftOverlayElement {
        position: Point::new(90.0, 40.0),
        target_bounds: Rectangle { x: 90.0, y: 40.0, width: 30.0, height: 50.0 },
        viewport: Rectangle::with_size(Size::new(200.0, 120.0)),
        gap: 8.0,
        snap_within_viewport: false,
        overlay: &mut overlay,
        tree: &mut tree,
        on_close: None,
    };
    let node = node_at(Rectangle { x: 12.0, y: 55.0, width: 70.0, height: 20.0 });

    let interaction = overlay::Overlay::mouse_interaction(
        &element,
        Layout::new(&node),
        mouse::Cursor::Unavailable,
        &renderer,
    );
    overlay::Overlay::draw(
        &element,
        &mut draw_renderer,
        &Theme::Dark,
        &renderer::Style::default(),
        Layout::new(&node),
        mouse::Cursor::Unavailable,
    );

    assert_eq!(interaction, mouse::Interaction::Crosshair);
    assert_eq!(draw_renderer.fills, vec![node.bounds()]);
}

#[test]
fn point_left_overlay_element_layout_uses_anchor_and_gap() {
    let renderer = RecordingRenderer::default();
    let mut overlay: Element<'static, TestMessage, Theme, RecordingRenderer> =
        fixed(70.0, 40.0).into();
    let mut tree = widget::Tree::new(&overlay);
    let mut element = PointLeftOverlayElement {
        anchor: Point::new(100.0, 60.0),
        viewport: Rectangle::with_size(Size::new(180.0, 120.0)),
        gap: 8.0,
        snap_within_viewport: false,
        overlay: &mut overlay,
        tree: &mut tree,
        on_close: None,
    };

    let node = overlay::Overlay::layout(&mut element, &renderer, Size::new(180.0, 120.0));

    assert_eq!(node.bounds(), Rectangle { x: 22.0, y: 40.0, width: 70.0, height: 40.0 });
}

#[test]
fn point_left_overlay_element_layout_snaps_to_viewport() {
    let renderer = RecordingRenderer::default();
    let mut overlay: Element<'static, TestMessage, Theme, RecordingRenderer> =
        fixed(70.0, 40.0).into();
    let mut tree = widget::Tree::new(&overlay);
    let mut element = PointLeftOverlayElement {
        anchor: Point::new(30.0, 118.0),
        viewport: Rectangle::with_size(Size::new(180.0, 120.0)),
        gap: 8.0,
        snap_within_viewport: true,
        overlay: &mut overlay,
        tree: &mut tree,
        on_close: None,
    };

    let node = overlay::Overlay::layout(&mut element, &renderer, Size::new(180.0, 120.0));

    assert_eq!(node.bounds(), Rectangle { x: 0.0, y: 80.0, width: 70.0, height: 40.0 });
}

#[test]
fn point_left_overlay_element_closes_only_outside_overlay_bounds() {
    let renderer = RecordingRenderer::default();
    let mut clipboard = iced::advanced::clipboard::Null;
    let node = node_at(Rectangle { x: 22.0, y: 40.0, width: 70.0, height: 40.0 });

    for (cursor_position, expected_messages) in
        [(Point::new(5.0, 5.0), vec![TestMessage::Close]), (Point::new(30.0, 50.0), Vec::new())]
    {
        let mut overlay: Element<'static, TestMessage, Theme, RecordingRenderer> =
            fixed(70.0, 40.0).into();
        let mut tree = widget::Tree::new(&overlay);
        let mut element = PointLeftOverlayElement {
            anchor: Point::new(100.0, 60.0),
            viewport: Rectangle::with_size(Size::new(180.0, 120.0)),
            gap: 8.0,
            snap_within_viewport: false,
            overlay: &mut overlay,
            tree: &mut tree,
            on_close: Some(TestMessage::Close),
        };
        let mut messages = Vec::new();
        let mut shell = Shell::new(&mut messages);

        overlay::Overlay::update(
            &mut element,
            &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Layout::new(&node),
            mouse::Cursor::Available(cursor_position),
            &renderer,
            &mut clipboard,
            &mut shell,
        );

        let captured = shell.is_event_captured();
        drop(shell);
        assert_eq!(messages, expected_messages);
        assert!(captured);
    }
}

#[test]
fn point_left_overlay_element_rejects_close_without_all_preconditions() {
    let renderer = RecordingRenderer::default();
    let mut clipboard = iced::advanced::clipboard::Null;
    let node = node_at(Rectangle { x: 22.0, y: 40.0, width: 70.0, height: 40.0 });

    for (button, cursor, on_close) in [
        (
            mouse::Button::Right,
            mouse::Cursor::Available(Point::new(5.0, 5.0)),
            Some(TestMessage::Close),
        ),
        (mouse::Button::Left, mouse::Cursor::Unavailable, Some(TestMessage::Close)),
        (mouse::Button::Left, mouse::Cursor::Available(Point::new(5.0, 5.0)), None),
    ] {
        let mut overlay: Element<'static, TestMessage, Theme, RecordingRenderer> =
            fixed(70.0, 40.0).into();
        let mut tree = widget::Tree::new(&overlay);
        let mut element = PointLeftOverlayElement {
            anchor: Point::new(100.0, 60.0),
            viewport: Rectangle::with_size(Size::new(180.0, 120.0)),
            gap: 8.0,
            snap_within_viewport: false,
            overlay: &mut overlay,
            tree: &mut tree,
            on_close,
        };
        let mut messages = Vec::new();
        let mut shell = Shell::new(&mut messages);

        overlay::Overlay::update(
            &mut element,
            &Event::Mouse(mouse::Event::ButtonPressed(button)),
            Layout::new(&node),
            cursor,
            &renderer,
            &mut clipboard,
            &mut shell,
        );

        let captured = shell.is_event_captured();
        drop(shell);
        assert!(messages.is_empty());
        assert!(captured);
    }
}

#[test]
fn point_left_overlay_element_delegates_draw_and_mouse_interaction() {
    let renderer = RecordingRenderer::default();
    let mut draw_renderer = RecordingRenderer::default();
    let mut overlay: Element<'static, TestMessage, Theme, RecordingRenderer> =
        fixed(70.0, 40.0).with_interaction(mouse::Interaction::Crosshair).into();
    let mut tree = widget::Tree::new(&overlay);
    let element = PointLeftOverlayElement {
        anchor: Point::new(100.0, 60.0),
        viewport: Rectangle::with_size(Size::new(180.0, 120.0)),
        gap: 8.0,
        snap_within_viewport: false,
        overlay: &mut overlay,
        tree: &mut tree,
        on_close: None,
    };
    let node = node_at(Rectangle { x: 22.0, y: 40.0, width: 70.0, height: 40.0 });

    let interaction = overlay::Overlay::mouse_interaction(
        &element,
        Layout::new(&node),
        mouse::Cursor::Unavailable,
        &renderer,
    );
    overlay::Overlay::draw(
        &element,
        &mut draw_renderer,
        &Theme::Dark,
        &renderer::Style::default(),
        Layout::new(&node),
        mouse::Cursor::Unavailable,
    );

    assert_eq!(interaction, mouse::Interaction::Crosshair);
    assert_eq!(draw_renderer.fills, vec![node.bounds()]);
}

#[test]
fn left_overlay_element_does_not_capture_non_mouse_events() {
    let renderer = RecordingRenderer::default();
    let mut clipboard = iced::advanced::clipboard::Null;
    let mut overlay: Element<'static, TestMessage, Theme, RecordingRenderer> =
        fixed(70.0, 20.0).into();
    let mut tree = widget::Tree::new(&overlay);
    let mut element = LeftOverlayElement {
        position: Point::new(90.0, 40.0),
        target_bounds: Rectangle { x: 90.0, y: 40.0, width: 30.0, height: 50.0 },
        viewport: Rectangle::with_size(Size::new(200.0, 120.0)),
        gap: 8.0,
        snap_within_viewport: false,
        overlay: &mut overlay,
        tree: &mut tree,
        on_close: Some(TestMessage::Close),
    };
    let node = node_at(Rectangle { x: 12.0, y: 55.0, width: 70.0, height: 20.0 });
    let mut messages = Vec::new();
    let mut shell = Shell::new(&mut messages);

    overlay::Overlay::update(
        &mut element,
        &Event::Window(iced::window::Event::Resized(Size::new(300.0, 200.0))),
        Layout::new(&node),
        mouse::Cursor::Available(Point::new(5.0, 5.0)),
        &renderer,
        &mut clipboard,
        &mut shell,
    );

    let captured = shell.is_event_captured();
    drop(shell);
    assert!(messages.is_empty());
    assert!(!captured);
}

#[test]
fn point_left_overlay_element_does_not_capture_non_mouse_events() {
    let renderer = RecordingRenderer::default();
    let mut clipboard = iced::advanced::clipboard::Null;
    let mut overlay: Element<'static, TestMessage, Theme, RecordingRenderer> =
        fixed(70.0, 40.0).into();
    let mut tree = widget::Tree::new(&overlay);
    let mut element = PointLeftOverlayElement {
        anchor: Point::new(100.0, 60.0),
        viewport: Rectangle::with_size(Size::new(180.0, 120.0)),
        gap: 8.0,
        snap_within_viewport: false,
        overlay: &mut overlay,
        tree: &mut tree,
        on_close: Some(TestMessage::Close),
    };
    let node = node_at(Rectangle { x: 22.0, y: 40.0, width: 70.0, height: 40.0 });
    let mut messages = Vec::new();
    let mut shell = Shell::new(&mut messages);

    overlay::Overlay::update(
        &mut element,
        &Event::Window(iced::window::Event::Resized(Size::new(300.0, 200.0))),
        Layout::new(&node),
        mouse::Cursor::Available(Point::new(5.0, 5.0)),
        &renderer,
        &mut clipboard,
        &mut shell,
    );

    let captured = shell.is_event_captured();
    drop(shell);
    assert!(messages.is_empty());
    assert!(!captured);
}
