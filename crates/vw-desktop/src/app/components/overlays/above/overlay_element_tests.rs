use super::overlay_element::{AboveOverlayElement, PointAboveOverlayElement};
use super::overlay_element::{compute_above_position, compute_point_above_position};
use iced::advanced::{
    Clipboard, Layout, Renderer, Shell, Widget, layout, mouse, overlay, renderer, widget,
};
use iced::{
    Background, Element, Event, Length, Point, Rectangle, Size, Theme, Transformation, Vector,
};
use std::cell::Cell;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq, Eq)]
enum TestMessage {
    ChildUpdated,
    Close,
}

#[derive(Debug, Clone)]
struct TestContent {
    width: f32,
    height: f32,
    interaction: mouse::Interaction,
    operated: Rc<Cell<bool>>,
    has_overlay: bool,
}

impl Default for TestContent {
    fn default() -> Self {
        Self {
            width: 70.0,
            height: 40.0,
            interaction: mouse::Interaction::Pointer,
            operated: Rc::new(Cell::new(false)),
            has_overlay: false,
        }
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

    fn operate(
        &mut self,
        _tree: &mut widget::Tree,
        _layout: Layout<'_>,
        _renderer: &RecordingRenderer,
        _operation: &mut dyn widget::Operation,
    ) {
        self.operated.set(true);
    }

    fn overlay<'a>(
        &'a mut self,
        _tree: &'a mut widget::Tree,
        _layout: Layout<'a>,
        _renderer: &RecordingRenderer,
        _viewport: &Rectangle,
        _translation: Vector,
    ) -> Option<overlay::Element<'a, TestMessage, Theme, RecordingRenderer>> {
        self.has_overlay.then(|| overlay::Element::new(Box::new(TestOverlay)))
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

struct TestOverlay;

impl overlay::Overlay<TestMessage, Theme, RecordingRenderer> for TestOverlay {
    fn layout(&mut self, _renderer: &RecordingRenderer, _bounds: Size) -> layout::Node {
        layout::Node::new(Size::new(18.0, 12.0))
    }

    fn draw(
        &self,
        _renderer: &mut RecordingRenderer,
        _theme: &Theme,
        _style: &renderer::Style,
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
    ) {
    }
}

fn overlay_content(
    width: f32,
    height: f32,
    interaction: mouse::Interaction,
) -> Element<'static, TestMessage, Theme, RecordingRenderer> {
    TestContent {
        width,
        height,
        interaction,
        operated: Rc::new(Cell::new(false)),
        has_overlay: false,
    }
    .into()
}

fn layout_for(bounds: Rectangle) -> layout::Node {
    layout::Node::new(bounds.size()).move_to(Point::new(bounds.x, bounds.y))
}

struct NoopOperation;

impl widget::Operation for NoopOperation {
    fn traverse(&mut self, _operate: &mut dyn FnMut(&mut dyn widget::Operation)) {}
}

#[test]
fn above_position_keeps_raw_coordinates_without_snap() {
    let position = compute_above_position(
        Point::new(260.0, 40.0),
        Rectangle { x: 260.0, y: 40.0, width: 32.0, height: 20.0 },
        Rectangle::with_size(Size::new(300.0, 200.0)),
        Size::new(120.0, 80.0),
        6.0,
        false,
    );

    assert_eq!(position, Point::new(260.0, -46.0));
}

#[test]
fn above_position_snaps_to_viewport_edges() {
    let position = compute_above_position(
        Point::new(260.0, 30.0),
        Rectangle { x: 260.0, y: 30.0, width: 32.0, height: 20.0 },
        Rectangle { x: 10.0, y: 12.0, width: 300.0, height: 200.0 },
        Size::new(120.0, 80.0),
        6.0,
        true,
    );

    assert_eq!(position, Point::new(190.0, 12.0));
}

#[test]
fn point_above_position_centers_on_anchor_without_snap() {
    let position = compute_point_above_position(
        Point::new(80.0, 90.0),
        Rectangle::with_size(Size::new(300.0, 200.0)),
        Size::new(60.0, 40.0),
        10.0,
        false,
    );

    assert_eq!(position, Point::new(50.0, 40.0));
}

#[test]
fn point_above_position_snaps_when_anchor_is_near_edges() {
    let position = compute_point_above_position(
        Point::new(12.0, 8.0),
        Rectangle::with_size(Size::new(100.0, 80.0)),
        Size::new(40.0, 30.0),
        6.0,
        true,
    );

    assert_eq!(position, Point::new(0.0, 0.0));
}

#[test]
fn above_position_snaps_to_viewport_origin_when_overlay_is_larger() {
    let position = compute_above_position(
        Point::new(-40.0, 12.0),
        Rectangle { x: -40.0, y: 12.0, width: 20.0, height: 10.0 },
        Rectangle { x: 30.0, y: 40.0, width: 60.0, height: 50.0 },
        Size::new(120.0, 100.0),
        8.0,
        true,
    );

    assert_eq!(position, Point::new(30.0, 40.0));
}

#[test]
fn point_above_position_snaps_to_max_edges() {
    let position = compute_point_above_position(
        Point::new(120.0, 110.0),
        Rectangle::with_size(Size::new(100.0, 90.0)),
        Size::new(30.0, 20.0),
        5.0,
        true,
    );

    assert_eq!(position, Point::new(70.0, 70.0));
}

#[test]
fn above_overlay_element_layout_draws_and_reports_interaction() {
    let mut overlay = overlay_content(70.0, 40.0, mouse::Interaction::Text);
    let mut tree = widget::Tree::empty();
    let renderer = RecordingRenderer::default();
    let mut element = AboveOverlayElement {
        position: Point::new(170.0, 110.0),
        target_bounds: Rectangle { x: 170.0, y: 110.0, width: 40.0, height: 20.0 },
        viewport: Rectangle::with_size(Size::new(220.0, 160.0)),
        gap: 6.0,
        snap_within_viewport: true,
        overlay: &mut overlay,
        tree: &mut tree,
        on_close: None,
        block_background_events: false,
    };

    let node = overlay::Overlay::layout(&mut element, &renderer, Size::new(220.0, 160.0));
    let interaction = overlay::Overlay::mouse_interaction(
        &element,
        Layout::new(&node),
        mouse::Cursor::Unavailable,
        &renderer,
    );
    let mut draw_renderer = RecordingRenderer::default();
    overlay::Overlay::draw(
        &element,
        &mut draw_renderer,
        &Theme::Dark,
        &renderer::Style::default(),
        Layout::new(&node),
        mouse::Cursor::Unavailable,
    );

    assert_eq!(node.bounds(), Rectangle { x: 150.0, y: 64.0, width: 70.0, height: 40.0 });
    assert_eq!(interaction, mouse::Interaction::Text);
    assert_eq!(draw_renderer.fills, vec![node.bounds()]);
}

#[test]
fn above_overlay_element_update_closes_outside_and_captures_inside() {
    let mut overlay = overlay_content(70.0, 40.0, mouse::Interaction::Pointer);
    let mut tree = widget::Tree::empty();
    let renderer = RecordingRenderer::default();
    let mut clipboard = iced::advanced::clipboard::Null;
    let node = layout_for(Rectangle { x: 150.0, y: 64.0, width: 70.0, height: 40.0 });
    let mut element = AboveOverlayElement {
        position: Point::new(170.0, 110.0),
        target_bounds: Rectangle { x: 170.0, y: 110.0, width: 40.0, height: 20.0 },
        viewport: Rectangle::with_size(Size::new(220.0, 160.0)),
        gap: 6.0,
        snap_within_viewport: true,
        overlay: &mut overlay,
        tree: &mut tree,
        on_close: Some(TestMessage::Close),
        block_background_events: false,
    };

    let mut outside_messages = Vec::new();
    let outside_captured = {
        let mut shell = Shell::new(&mut outside_messages);
        overlay::Overlay::update(
            &mut element,
            &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Layout::new(&node),
            mouse::Cursor::Available(Point::new(20.0, 20.0)),
            &renderer,
            &mut clipboard,
            &mut shell,
        );
        shell.is_event_captured()
    };

    let mut inside_messages = Vec::new();
    let inside_captured = {
        let mut shell = Shell::new(&mut inside_messages);
        overlay::Overlay::update(
            &mut element,
            &Event::Mouse(mouse::Event::CursorMoved { position: Point::new(160.0, 80.0) }),
            Layout::new(&node),
            mouse::Cursor::Available(Point::new(160.0, 80.0)),
            &renderer,
            &mut clipboard,
            &mut shell,
        );
        shell.is_event_captured()
    };

    assert_eq!(outside_messages, vec![TestMessage::Close, TestMessage::ChildUpdated]);
    assert!(outside_captured);
    assert_eq!(inside_messages, vec![TestMessage::ChildUpdated]);
    assert!(inside_captured);
}

#[test]
fn blocking_above_overlay_captures_full_outside_click_before_closing() {
    let mut overlay = overlay_content(70.0, 40.0, mouse::Interaction::Pointer);
    let mut tree = widget::Tree::empty();
    let renderer = RecordingRenderer::default();
    let mut clipboard = iced::advanced::clipboard::Null;
    let node = layout_for(Rectangle { x: 150.0, y: 64.0, width: 70.0, height: 40.0 });
    let mut element = AboveOverlayElement {
        position: Point::new(170.0, 110.0),
        target_bounds: Rectangle { x: 170.0, y: 110.0, width: 40.0, height: 20.0 },
        viewport: Rectangle::with_size(Size::new(220.0, 160.0)),
        gap: 6.0,
        snap_within_viewport: true,
        overlay: &mut overlay,
        tree: &mut tree,
        on_close: Some(TestMessage::Close),
        block_background_events: true,
    };

    let mut pressed_messages = Vec::new();
    let pressed_captured = {
        let mut shell = Shell::new(&mut pressed_messages);
        overlay::Overlay::update(
            &mut element,
            &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Layout::new(&node),
            mouse::Cursor::Available(Point::new(20.0, 20.0)),
            &renderer,
            &mut clipboard,
            &mut shell,
        );
        shell.is_event_captured()
    };

    let mut released_messages = Vec::new();
    let released_captured = {
        let mut shell = Shell::new(&mut released_messages);
        overlay::Overlay::update(
            &mut element,
            &Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            Layout::new(&node),
            mouse::Cursor::Available(Point::new(20.0, 20.0)),
            &renderer,
            &mut clipboard,
            &mut shell,
        );
        shell.is_event_captured()
    };

    assert_eq!(pressed_messages, vec![TestMessage::ChildUpdated]);
    assert!(pressed_captured);
    assert_eq!(released_messages, vec![TestMessage::Close, TestMessage::ChildUpdated]);
    assert!(released_captured);
}

#[test]
fn above_overlay_element_update_skips_close_without_cursor_or_callback() {
    let mut overlay = overlay_content(70.0, 40.0, mouse::Interaction::Pointer);
    let mut tree = widget::Tree::empty();
    let renderer = RecordingRenderer::default();
    let mut clipboard = iced::advanced::clipboard::Null;
    let node = layout_for(Rectangle { x: 150.0, y: 64.0, width: 70.0, height: 40.0 });
    let mut element = AboveOverlayElement {
        position: Point::new(170.0, 110.0),
        target_bounds: Rectangle { x: 170.0, y: 110.0, width: 40.0, height: 20.0 },
        viewport: Rectangle::with_size(Size::new(220.0, 160.0)),
        gap: 6.0,
        snap_within_viewport: true,
        overlay: &mut overlay,
        tree: &mut tree,
        on_close: None,
        block_background_events: false,
    };
    let mut messages = Vec::new();
    let captured = {
        let mut shell = Shell::new(&mut messages);
        overlay::Overlay::update(
            &mut element,
            &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            &renderer,
            &mut clipboard,
            &mut shell,
        );
        shell.is_event_captured()
    };

    assert_eq!(messages, vec![TestMessage::ChildUpdated]);
    assert!(!captured);
}

#[test]
fn point_above_overlay_element_layout_draws_and_reports_interaction() {
    let mut overlay = overlay_content(60.0, 30.0, mouse::Interaction::Grab);
    let mut tree = widget::Tree::empty();
    let renderer = RecordingRenderer::default();
    let mut element = PointAboveOverlayElement {
        anchor: Point::new(90.0, 80.0),
        target_bounds: Rectangle { x: 70.0, y: 70.0, width: 40.0, height: 20.0 },
        viewport: Rectangle::with_size(Size::new(180.0, 140.0)),
        gap: 8.0,
        snap_within_viewport: false,
        overlay: &mut overlay,
        tree: &mut tree,
        on_close: None,
    };

    let node = overlay::Overlay::layout(&mut element, &renderer, Size::new(180.0, 140.0));
    let interaction = overlay::Overlay::mouse_interaction(
        &element,
        Layout::new(&node),
        mouse::Cursor::Unavailable,
        &renderer,
    );
    let mut draw_renderer = RecordingRenderer::default();
    overlay::Overlay::draw(
        &element,
        &mut draw_renderer,
        &Theme::Dark,
        &renderer::Style::default(),
        Layout::new(&node),
        mouse::Cursor::Unavailable,
    );

    assert_eq!(node.bounds(), Rectangle { x: 60.0, y: 42.0, width: 60.0, height: 30.0 });
    assert_eq!(interaction, mouse::Interaction::Grab);
    assert_eq!(draw_renderer.fills, vec![node.bounds()]);
}

#[test]
fn point_above_overlay_element_update_closes_outside_and_captures_inside() {
    let mut overlay = overlay_content(60.0, 30.0, mouse::Interaction::Pointer);
    let mut tree = widget::Tree::empty();
    let renderer = RecordingRenderer::default();
    let mut clipboard = iced::advanced::clipboard::Null;
    let node = layout_for(Rectangle { x: 60.0, y: 42.0, width: 60.0, height: 30.0 });
    let mut element = PointAboveOverlayElement {
        anchor: Point::new(90.0, 80.0),
        target_bounds: Rectangle { x: 70.0, y: 70.0, width: 40.0, height: 20.0 },
        viewport: Rectangle::with_size(Size::new(180.0, 140.0)),
        gap: 8.0,
        snap_within_viewport: false,
        overlay: &mut overlay,
        tree: &mut tree,
        on_close: Some(TestMessage::Close),
    };

    let mut outside_messages = Vec::new();
    let outside_captured = {
        let mut shell = Shell::new(&mut outside_messages);
        overlay::Overlay::update(
            &mut element,
            &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Layout::new(&node),
            mouse::Cursor::Available(Point::new(150.0, 100.0)),
            &renderer,
            &mut clipboard,
            &mut shell,
        );
        shell.is_event_captured()
    };

    let mut inside_messages = Vec::new();
    let inside_captured = {
        let mut shell = Shell::new(&mut inside_messages);
        overlay::Overlay::update(
            &mut element,
            &Event::Mouse(mouse::Event::CursorMoved { position: Point::new(90.0, 60.0) }),
            Layout::new(&node),
            mouse::Cursor::Available(Point::new(90.0, 60.0)),
            &renderer,
            &mut clipboard,
            &mut shell,
        );
        shell.is_event_captured()
    };

    assert_eq!(outside_messages, vec![TestMessage::Close, TestMessage::ChildUpdated]);
    assert!(outside_captured);
    assert_eq!(inside_messages, vec![TestMessage::ChildUpdated]);
    assert!(inside_captured);
}

#[test]
fn point_above_overlay_element_update_skips_non_left_click_close() {
    let mut overlay = overlay_content(60.0, 30.0, mouse::Interaction::Pointer);
    let mut tree = widget::Tree::empty();
    let renderer = RecordingRenderer::default();
    let mut clipboard = iced::advanced::clipboard::Null;
    let node = layout_for(Rectangle { x: 60.0, y: 42.0, width: 60.0, height: 30.0 });
    let mut element = PointAboveOverlayElement {
        anchor: Point::new(90.0, 80.0),
        target_bounds: Rectangle { x: 70.0, y: 70.0, width: 40.0, height: 20.0 },
        viewport: Rectangle::with_size(Size::new(180.0, 140.0)),
        gap: 8.0,
        snap_within_viewport: false,
        overlay: &mut overlay,
        tree: &mut tree,
        on_close: Some(TestMessage::Close),
    };
    let mut messages = Vec::new();
    let captured = {
        let mut shell = Shell::new(&mut messages);
        overlay::Overlay::update(
            &mut element,
            &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)),
            Layout::new(&node),
            mouse::Cursor::Available(Point::new(150.0, 100.0)),
            &renderer,
            &mut clipboard,
            &mut shell,
        );
        shell.is_event_captured()
    };

    assert_eq!(messages, vec![TestMessage::ChildUpdated]);
    assert!(!captured);
}

#[test]
fn above_overlay_element_operate_forwards_to_overlay_content() {
    let operated = Rc::new(Cell::new(false));
    let mut overlay: Element<'_, TestMessage, Theme, RecordingRenderer> = TestContent {
        width: 70.0,
        height: 40.0,
        interaction: mouse::Interaction::Pointer,
        operated: Rc::clone(&operated),
        has_overlay: false,
    }
    .into();
    let mut tree = widget::Tree::empty();
    let renderer = RecordingRenderer::default();
    let mut element = AboveOverlayElement {
        position: Point::new(10.0, 20.0),
        target_bounds: Rectangle { x: 10.0, y: 20.0, width: 40.0, height: 20.0 },
        viewport: Rectangle::with_size(Size::new(220.0, 160.0)),
        gap: 6.0,
        snap_within_viewport: false,
        overlay: &mut overlay,
        tree: &mut tree,
        on_close: None,
        block_background_events: false,
    };
    let node = overlay::Overlay::layout(&mut element, &renderer, Size::new(220.0, 160.0));
    let mut noop = NoopOperation;
    overlay::Overlay::operate(&mut element, Layout::new(&node), &renderer, &mut noop);
    assert!(operated.get());
}

#[test]
fn above_overlay_element_forwards_nested_overlay_from_content() {
    let mut overlay: Element<'_, TestMessage, Theme, RecordingRenderer> = TestContent {
        has_overlay: true,
        ..TestContent::default()
    }
    .into();
    let mut tree = widget::Tree::empty();
    let renderer = RecordingRenderer::default();
    let mut element = AboveOverlayElement {
        position: Point::new(10.0, 20.0),
        target_bounds: Rectangle { x: 10.0, y: 20.0, width: 40.0, height: 20.0 },
        viewport: Rectangle::with_size(Size::new(220.0, 160.0)),
        gap: 6.0,
        snap_within_viewport: false,
        overlay: &mut overlay,
        tree: &mut tree,
        on_close: None,
        block_background_events: false,
    };
    let node = overlay::Overlay::layout(&mut element, &renderer, Size::new(220.0, 160.0));
    let mut nested = overlay::Overlay::overlay(&mut element, Layout::new(&node), &renderer)
        .expect("nested overlay should be forwarded");

    assert_eq!(
        nested.as_overlay_mut().layout(&renderer, Size::new(220.0, 160.0)).size(),
        Size::new(18.0, 12.0)
    );
}

#[test]
fn point_above_overlay_element_operate_forwards_to_overlay_content() {
    let operated = Rc::new(Cell::new(false));
    let mut overlay: Element<'_, TestMessage, Theme, RecordingRenderer> = TestContent {
        width: 60.0,
        height: 30.0,
        interaction: mouse::Interaction::Pointer,
        operated: Rc::clone(&operated),
        has_overlay: false,
    }
    .into();
    let mut tree = widget::Tree::empty();
    let renderer = RecordingRenderer::default();
    let mut element = PointAboveOverlayElement {
        anchor: Point::new(90.0, 80.0),
        target_bounds: Rectangle { x: 70.0, y: 70.0, width: 40.0, height: 20.0 },
        viewport: Rectangle::with_size(Size::new(180.0, 140.0)),
        gap: 8.0,
        snap_within_viewport: false,
        overlay: &mut overlay,
        tree: &mut tree,
        on_close: None,
    };
    let node = overlay::Overlay::layout(&mut element, &renderer, Size::new(180.0, 140.0));
    let mut noop = NoopOperation;
    overlay::Overlay::operate(&mut element, Layout::new(&node), &renderer, &mut noop);
    assert!(operated.get());
}
