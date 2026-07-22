use super::{PointBelowOverlay, compute_point_below_position};
use iced::advanced::{
    Clipboard, Layout, Renderer, Shell, Widget, layout, mouse, overlay, renderer, widget,
};
use iced::widget::text;
use iced::{Background, Element, Event, Length, Point, Rectangle, Size, Theme, Transformation};

#[derive(Debug, Clone, PartialEq)]
enum TestMessage {
    ContentUpdated,
    Close,
}

fn label(value: &'static str) -> Element<'static, TestMessage> {
    text(value).into()
}

#[derive(Debug, Clone, Copy)]
struct TestContent {
    width: f32,
    height: f32,
    interaction: mouse::Interaction,
}

impl Default for TestContent {
    fn default() -> Self {
        Self { width: 100.0, height: 30.0, interaction: mouse::Interaction::Pointer }
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
        shell.publish(TestMessage::ContentUpdated);
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

fn test_overlay() -> PointBelowOverlay<'static, TestMessage, Theme, RecordingRenderer> {
    PointBelowOverlay::new(
        TestContent::default(),
        TestContent { width: 70.0, height: 40.0, ..TestContent::default() },
    )
}

fn tree_for_overlay() -> widget::Tree {
    let mut tree = widget::Tree::empty();
    tree.children.push(widget::Tree::empty());
    tree.children.push(widget::Tree::empty());
    tree
}

fn layout_for(bounds: Rectangle) -> layout::Node {
    layout::Node::new(bounds.size()).move_to(Point::new(bounds.x, bounds.y))
}

fn overlay_element<'a>(
    overlay: &'a mut Element<'static, TestMessage, Theme, RecordingRenderer>,
    tree: &'a mut widget::Tree,
    capture_outside_click: bool,
) -> super::PointBelowOverlayElement<'static, 'a, TestMessage, Theme, RecordingRenderer> {
    overlay_element_with(overlay, tree, capture_outside_click, false)
}

fn overlay_element_with<'a>(
    overlay: &'a mut Element<'static, TestMessage, Theme, RecordingRenderer>,
    tree: &'a mut widget::Tree,
    capture_outside_click: bool,
    close_on_target_click: bool,
) -> super::PointBelowOverlayElement<'static, 'a, TestMessage, Theme, RecordingRenderer> {
    super::PointBelowOverlayElement {
        anchor: Point::new(80.0, 40.0),
        target_bounds: Rectangle { x: 20.0, y: 20.0, width: 100.0, height: 30.0 },
        viewport: Rectangle::with_size(Size::new(180.0, 140.0)),
        gap: 6.0,
        snap_within_viewport: true,
        snap_within_target_bounds: false,
        overlay,
        tree,
        on_close: Some(TestMessage::Close),
        capture_outside_click,
        close_on_target_click,
    }
}

#[test]
fn point_below_overlay_new_sets_hidden_defaults() {
    let overlay = PointBelowOverlay::new(label("content"), label("overlay"));

    assert!(!overlay.show);
    assert_eq!(overlay.anchor, Point::ORIGIN);
    assert_eq!(overlay.gap, 0.0);
    assert!(overlay.snap_within_viewport);
    assert!(!overlay.snap_within_target_bounds);
    assert!(overlay.on_close.is_none());
    assert!(overlay.capture_outside_click);
    assert!(!overlay.close_on_target_click);
}

#[test]
fn point_below_overlay_builder_updates_all_flags() {
    let overlay = PointBelowOverlay::new(label("content"), label("overlay"))
        .show(true)
        .anchor(Point::new(18.0, 24.0))
        .gap(5.0)
        .snap_within_viewport(false)
        .snap_within_target_bounds(true)
        .capture_outside_click(false)
        .close_on_target_click(true)
        .on_close(TestMessage::Close);

    assert!(overlay.show);
    assert_eq!(overlay.anchor, Point::new(18.0, 24.0));
    assert_eq!(overlay.gap, 5.0);
    assert!(!overlay.snap_within_viewport);
    assert!(overlay.snap_within_target_bounds);
    assert!(!overlay.capture_outside_click);
    assert!(overlay.close_on_target_click);
    assert_eq!(overlay.on_close, Some(TestMessage::Close));
}

#[test]
fn point_below_position_uses_anchor_plus_gap_without_snap() {
    let position = compute_point_below_position(
        Point::new(90.0, 40.0),
        Rectangle { x: 30.0, y: 20.0, width: 120.0, height: 80.0 },
        Rectangle::with_size(Size::new(200.0, 160.0)),
        Size::new(70.0, 40.0),
        8.0,
        false,
        false,
    );

    assert_eq!(position, Point::new(90.0, 48.0));
}

#[test]
fn point_below_position_snaps_to_viewport() {
    let position = compute_point_below_position(
        Point::new(190.0, 150.0),
        Rectangle { x: 30.0, y: 20.0, width: 120.0, height: 80.0 },
        Rectangle::with_size(Size::new(200.0, 160.0)),
        Size::new(70.0, 40.0),
        8.0,
        true,
        false,
    );

    assert_eq!(position, Point::new(130.0, 120.0));
}

#[test]
fn point_below_position_target_snap_takes_precedence() {
    let position = compute_point_below_position(
        Point::new(190.0, 150.0),
        Rectangle { x: 30.0, y: 20.0, width: 120.0, height: 80.0 },
        Rectangle::with_size(Size::new(200.0, 160.0)),
        Size::new(70.0, 40.0),
        8.0,
        true,
        true,
    );

    assert_eq!(position, Point::new(80.0, 60.0));
}

#[test]
fn point_below_position_clamps_to_offset_viewport_minimum() {
    let position = compute_point_below_position(
        Point::new(-50.0, -40.0),
        Rectangle { x: 30.0, y: 20.0, width: 120.0, height: 80.0 },
        Rectangle { x: 10.0, y: 15.0, width: 200.0, height: 160.0 },
        Size::new(70.0, 40.0),
        5.0,
        true,
        false,
    );

    assert_eq!(position, Point::new(10.0, 15.0));
}

#[test]
fn point_below_position_oversized_overlay_uses_viewport_origin() {
    let position = compute_point_below_position(
        Point::new(100.0, 100.0),
        Rectangle { x: 30.0, y: 20.0, width: 120.0, height: 80.0 },
        Rectangle { x: 10.0, y: 15.0, width: 40.0, height: 30.0 },
        Size::new(70.0, 40.0),
        5.0,
        true,
        false,
    );

    assert_eq!(position, Point::new(10.0, 15.0));
}

#[test]
fn point_below_position_oversized_overlay_uses_target_origin() {
    let position = compute_point_below_position(
        Point::new(100.0, 100.0),
        Rectangle { x: 30.0, y: 20.0, width: 40.0, height: 30.0 },
        Rectangle::with_size(Size::new(200.0, 160.0)),
        Size::new(70.0, 40.0),
        5.0,
        false,
        true,
    );

    assert_eq!(position, Point::new(30.0, 20.0));
}

#[test]
fn point_below_overlay_converts_to_element() {
    let element: Element<'static, TestMessage, Theme> =
        PointBelowOverlay::new(label("content"), label("overlay")).into();

    assert_eq!(element.as_widget().children().len(), 2);
}

#[test]
fn point_below_widget_children_and_diff_keep_content_and_overlay() {
    let overlay = test_overlay();
    let mut tree = tree_for_overlay();

    let children = <PointBelowOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::children(&overlay);
    <PointBelowOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::diff(&overlay, &mut tree);

    assert_eq!(children.len(), 2);
    assert_eq!(tree.children.len(), 2);
}

#[test]
fn point_below_widget_size_layout_draw_update_and_mouse_delegate_to_content() {
    let mut overlay = test_overlay();
    let mut tree = tree_for_overlay();
    let renderer = RecordingRenderer::default();
    let mut draw_renderer = RecordingRenderer::default();
    let node = layout_for(Rectangle { x: 4.0, y: 8.0, width: 100.0, height: 30.0 });
    let mut clipboard = iced::advanced::clipboard::Null;
    let mut messages = Vec::new();
    let mut shell = Shell::new(&mut messages);

    let size = <PointBelowOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::size(&overlay);
    let hint = <PointBelowOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::size_hint(&overlay);
    let laid_out = <PointBelowOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::layout(
        &mut overlay,
        &mut tree,
        &renderer,
        &layout::Limits::new(Size::ZERO, Size::new(400.0, 200.0)),
    );
    <PointBelowOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::draw(
        &overlay,
        &tree,
        &mut draw_renderer,
        &Theme::Dark,
        &renderer::Style::default(),
        Layout::new(&node),
        mouse::Cursor::Unavailable,
        &Rectangle::INFINITE,
    );
    <PointBelowOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::update(
        &mut overlay,
        &mut tree,
        &Event::Mouse(mouse::Event::CursorEntered),
        Layout::new(&node),
        mouse::Cursor::Unavailable,
        &renderer,
        &mut clipboard,
        &mut shell,
        &Rectangle::INFINITE,
    );
    let interaction = <PointBelowOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
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

    assert_eq!(size, Size::new(Length::Fixed(100.0), Length::Fixed(30.0)));
    assert_eq!(hint, size);
    assert_eq!(laid_out.size(), Size::new(100.0, 30.0));
    assert_eq!(draw_renderer.fills, vec![Rectangle { x: 4.0, y: 8.0, width: 100.0, height: 30.0 }]);
    assert_eq!(messages, vec![TestMessage::ContentUpdated]);
    assert_eq!(interaction, mouse::Interaction::Pointer);
}

#[test]
fn point_below_widget_overlay_respects_show_flag() {
    let mut hidden = test_overlay();
    let mut visible = test_overlay().show(true);
    let mut hidden_tree = tree_for_overlay();
    let mut visible_tree = tree_for_overlay();
    let renderer = RecordingRenderer::default();
    let node = layout_for(Rectangle { x: 10.0, y: 20.0, width: 100.0, height: 30.0 });

    let hidden_overlay = <PointBelowOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::overlay(
        &mut hidden,
        &mut hidden_tree,
        Layout::new(&node),
        &renderer,
        &Rectangle::INFINITE,
        iced::Vector::new(0.0, 0.0),
    );
    let visible_overlay =
        <PointBelowOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
            TestMessage,
            Theme,
            RecordingRenderer,
        >>::overlay(
            &mut visible,
            &mut visible_tree,
            Layout::new(&node),
            &renderer,
            &Rectangle::INFINITE,
            iced::Vector::new(3.0, 4.0),
        );

    assert!(hidden_overlay.is_none());
    assert!(visible_overlay.is_some());
}

#[test]
fn point_below_overlay_element_layout_draws_and_reports_interaction() {
    let mut overlay_content: Element<'static, TestMessage, Theme, RecordingRenderer> =
        TestContent { width: 70.0, height: 40.0, interaction: mouse::Interaction::Text }.into();
    let mut tree = widget::Tree::empty();
    let mut element = overlay_element(&mut overlay_content, &mut tree, true);
    let renderer = RecordingRenderer::default();
    let mut draw_renderer = RecordingRenderer::default();

    let layout = <super::PointBelowOverlayElement<'_, '_, TestMessage, Theme, RecordingRenderer> as overlay::Overlay<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::layout(&mut element, &renderer, Size::new(180.0, 140.0));
    <super::PointBelowOverlayElement<'_, '_, TestMessage, Theme, RecordingRenderer> as overlay::Overlay<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::draw(
        &element,
        &mut draw_renderer,
        &Theme::Dark,
        &renderer::Style::default(),
        Layout::new(&layout),
        mouse::Cursor::Unavailable,
    );
    let interaction = <super::PointBelowOverlayElement<
        '_,
        '_,
        TestMessage,
        Theme,
        RecordingRenderer,
    > as overlay::Overlay<TestMessage, Theme, RecordingRenderer>>::mouse_interaction(
        &element,
        Layout::new(&layout),
        mouse::Cursor::Unavailable,
        &renderer,
    );

    assert_eq!(layout.bounds(), Rectangle { x: 80.0, y: 46.0, width: 70.0, height: 40.0 });
    assert_eq!(
        draw_renderer.fills,
        vec![Rectangle { x: 80.0, y: 46.0, width: 70.0, height: 40.0 }]
    );
    assert_eq!(interaction, mouse::Interaction::Text);
}

#[test]
fn point_below_overlay_element_outside_click_publishes_close_and_can_capture() {
    let mut overlay_content: Element<'static, TestMessage, Theme, RecordingRenderer> =
        TestContent { width: 70.0, height: 40.0, ..TestContent::default() }.into();
    let mut tree = widget::Tree::empty();
    let mut element = overlay_element(&mut overlay_content, &mut tree, true);
    let renderer = RecordingRenderer::default();
    let node = layout_for(Rectangle { x: 80.0, y: 46.0, width: 70.0, height: 40.0 });
    let mut clipboard = iced::advanced::clipboard::Null;
    let mut messages = Vec::new();
    let captured = {
        let mut shell = Shell::new(&mut messages);
        <super::PointBelowOverlayElement<'_, '_, TestMessage, Theme, RecordingRenderer> as overlay::Overlay<
            TestMessage,
            Theme,
            RecordingRenderer,
        >>::update(
            &mut element,
            &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Layout::new(&node),
            mouse::Cursor::Available(Point::new(170.0, 120.0)),
            &renderer,
            &mut clipboard,
            &mut shell,
        );
        shell.is_event_captured()
    };

    assert_eq!(messages, vec![TestMessage::Close, TestMessage::ContentUpdated]);
    assert!(captured);
}

#[test]
fn point_below_overlay_element_outside_click_can_skip_capture() {
    let mut overlay_content: Element<'static, TestMessage, Theme, RecordingRenderer> =
        TestContent { width: 70.0, height: 40.0, ..TestContent::default() }.into();
    let mut tree = widget::Tree::empty();
    let mut element = overlay_element(&mut overlay_content, &mut tree, false);
    let renderer = RecordingRenderer::default();
    let node = layout_for(Rectangle { x: 80.0, y: 46.0, width: 70.0, height: 40.0 });
    let mut clipboard = iced::advanced::clipboard::Null;
    let mut messages = Vec::new();
    let captured = {
        let mut shell = Shell::new(&mut messages);
        <super::PointBelowOverlayElement<'_, '_, TestMessage, Theme, RecordingRenderer> as overlay::Overlay<
            TestMessage,
            Theme,
            RecordingRenderer,
        >>::update(
            &mut element,
            &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Layout::new(&node),
            mouse::Cursor::Available(Point::new(170.0, 120.0)),
            &renderer,
            &mut clipboard,
            &mut shell,
        );
        shell.is_event_captured()
    };

    assert_eq!(messages, vec![TestMessage::Close, TestMessage::ContentUpdated]);
    assert!(!captured);
}

#[test]
fn point_below_overlay_element_without_close_message_ignores_outside_click() {
    let mut overlay_content: Element<'static, TestMessage, Theme, RecordingRenderer> =
        TestContent { width: 70.0, height: 40.0, ..TestContent::default() }.into();
    let mut tree = widget::Tree::empty();
    let mut element = overlay_element(&mut overlay_content, &mut tree, true);
    element.on_close = None;
    let renderer = RecordingRenderer::default();
    let node = layout_for(Rectangle { x: 80.0, y: 46.0, width: 70.0, height: 40.0 });
    let mut clipboard = iced::advanced::clipboard::Null;
    let mut messages = Vec::new();
    let captured = {
        let mut shell = Shell::new(&mut messages);
        <super::PointBelowOverlayElement<'_, '_, TestMessage, Theme, RecordingRenderer> as overlay::Overlay<
            TestMessage,
            Theme,
            RecordingRenderer,
        >>::update(
            &mut element,
            &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Layout::new(&node),
            mouse::Cursor::Available(Point::new(170.0, 120.0)),
            &renderer,
            &mut clipboard,
            &mut shell,
        );
        shell.is_event_captured()
    };

    assert_eq!(messages, vec![TestMessage::ContentUpdated]);
    assert!(!captured);
}

#[test]
fn point_below_overlay_element_non_left_click_does_not_close_or_capture() {
    let mut overlay_content: Element<'static, TestMessage, Theme, RecordingRenderer> =
        TestContent { width: 70.0, height: 40.0, ..TestContent::default() }.into();
    let mut tree = widget::Tree::empty();
    let mut element = overlay_element(&mut overlay_content, &mut tree, true);
    let renderer = RecordingRenderer::default();
    let node = layout_for(Rectangle { x: 80.0, y: 46.0, width: 70.0, height: 40.0 });
    let mut clipboard = iced::advanced::clipboard::Null;
    let mut messages = Vec::new();
    let captured = {
        let mut shell = Shell::new(&mut messages);
        <super::PointBelowOverlayElement<'_, '_, TestMessage, Theme, RecordingRenderer> as overlay::Overlay<
            TestMessage,
            Theme,
            RecordingRenderer,
        >>::update(
            &mut element,
            &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)),
            Layout::new(&node),
            mouse::Cursor::Available(Point::new(170.0, 120.0)),
            &renderer,
            &mut clipboard,
            &mut shell,
        );
        shell.is_event_captured()
    };

    assert_eq!(messages, vec![TestMessage::ContentUpdated]);
    assert!(!captured);
}

#[test]
fn point_below_overlay_element_unavailable_cursor_does_not_close_or_capture() {
    let mut overlay_content: Element<'static, TestMessage, Theme, RecordingRenderer> =
        TestContent { width: 70.0, height: 40.0, ..TestContent::default() }.into();
    let mut tree = widget::Tree::empty();
    let mut element = overlay_element(&mut overlay_content, &mut tree, true);
    let renderer = RecordingRenderer::default();
    let node = layout_for(Rectangle { x: 80.0, y: 46.0, width: 70.0, height: 40.0 });
    let mut clipboard = iced::advanced::clipboard::Null;
    let mut messages = Vec::new();
    let captured = {
        let mut shell = Shell::new(&mut messages);
        <super::PointBelowOverlayElement<'_, '_, TestMessage, Theme, RecordingRenderer> as overlay::Overlay<
            TestMessage,
            Theme,
            RecordingRenderer,
        >>::update(
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

    assert_eq!(messages, vec![TestMessage::ContentUpdated]);
    assert!(!captured);
}

#[test]
fn point_below_overlay_element_inside_target_or_overlay_does_not_close() {
    let mut overlay_content: Element<'static, TestMessage, Theme, RecordingRenderer> =
        TestContent { width: 70.0, height: 40.0, ..TestContent::default() }.into();
    let mut tree = widget::Tree::empty();
    let renderer = RecordingRenderer::default();
    let node = layout_for(Rectangle { x: 80.0, y: 46.0, width: 70.0, height: 40.0 });
    let mut clipboard = iced::advanced::clipboard::Null;

    let mut target_messages = Vec::new();
    let target_captured = {
        let mut element = overlay_element(&mut overlay_content, &mut tree, true);
        let mut shell = Shell::new(&mut target_messages);
        <super::PointBelowOverlayElement<'_, '_, TestMessage, Theme, RecordingRenderer> as overlay::Overlay<
            TestMessage,
            Theme,
            RecordingRenderer,
        >>::update(
            &mut element,
            &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Layout::new(&node),
            mouse::Cursor::Available(Point::new(40.0, 30.0)),
            &renderer,
            &mut clipboard,
            &mut shell,
        );
        shell.is_event_captured()
    };

    let mut overlay_messages = Vec::new();
    let overlay_captured = {
        let mut element = overlay_element(&mut overlay_content, &mut tree, true);
        let mut shell = Shell::new(&mut overlay_messages);
        <super::PointBelowOverlayElement<'_, '_, TestMessage, Theme, RecordingRenderer> as overlay::Overlay<
            TestMessage,
            Theme,
            RecordingRenderer,
        >>::update(
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

    assert_eq!(target_messages, vec![TestMessage::ContentUpdated]);
    assert!(!target_captured);
    assert_eq!(overlay_messages, vec![TestMessage::ContentUpdated]);
    assert!(overlay_captured);
}

#[test]
fn point_below_overlay_element_inside_overlay_left_click_captures_without_close() {
    let mut overlay_content: Element<'static, TestMessage, Theme, RecordingRenderer> =
        TestContent { width: 70.0, height: 40.0, ..TestContent::default() }.into();
    let mut tree = widget::Tree::empty();
    let mut element = overlay_element(&mut overlay_content, &mut tree, true);
    let renderer = RecordingRenderer::default();
    let node = layout_for(Rectangle { x: 80.0, y: 46.0, width: 70.0, height: 40.0 });
    let mut clipboard = iced::advanced::clipboard::Null;
    let mut messages = Vec::new();
    let captured = {
        let mut shell = Shell::new(&mut messages);
        <super::PointBelowOverlayElement<'_, '_, TestMessage, Theme, RecordingRenderer> as overlay::Overlay<
            TestMessage,
            Theme,
            RecordingRenderer,
        >>::update(
            &mut element,
            &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Layout::new(&node),
            mouse::Cursor::Available(Point::new(90.0, 60.0)),
            &renderer,
            &mut clipboard,
            &mut shell,
        );
        shell.is_event_captured()
    };

    assert_eq!(messages, vec![TestMessage::ContentUpdated]);
    assert!(captured);
}

#[test]
fn point_below_overlay_element_closes_on_target_click_when_enabled() {
    let mut overlay_content: Element<'static, TestMessage, Theme, RecordingRenderer> =
        TestContent { width: 70.0, height: 40.0, ..TestContent::default() }.into();
    let mut tree = widget::Tree::empty();
    let mut element = overlay_element_with(&mut overlay_content, &mut tree, true, true);
    let renderer = RecordingRenderer::default();
    let node = layout_for(Rectangle { x: 80.0, y: 46.0, width: 70.0, height: 40.0 });
    let mut clipboard = iced::advanced::clipboard::Null;
    let mut messages = Vec::new();
    {
        let mut shell = Shell::new(&mut messages);
        // 光标落在锚点 target_bounds 内、但在浮层菜单之外。
        <super::PointBelowOverlayElement<'_, '_, TestMessage, Theme, RecordingRenderer> as overlay::Overlay<
            TestMessage,
            Theme,
            RecordingRenderer,
        >>::update(
            &mut element,
            &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Layout::new(&node),
            mouse::Cursor::Available(Point::new(40.0, 30.0)),
            &renderer,
            &mut clipboard,
            &mut shell,
        );
    }

    // 开启 close_on_target_click 后，锚点区域内点击也应触发关闭。
    assert_eq!(messages, vec![TestMessage::Close, TestMessage::ContentUpdated]);
}
