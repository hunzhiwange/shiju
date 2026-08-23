//! 验证行内右侧覆盖层的位置计算及组件事件委托。
//! 宽度不足时的位置回退用于保证覆盖内容仍与目标行保持可用关联。

use super::{InlineRightOverlay, InlineRightOverlayElement, compute_inline_right_position};
use iced::advanced::{
    Clipboard, Layout, Renderer as _, Shell, Widget, layout, mouse, overlay, renderer, widget,
};
use iced::widget::text;
use iced::{
    Background, Element, Event, Length, Point, Rectangle, Size, Theme, Transformation, Vector,
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

impl iced::advanced::Renderer for RecordingRenderer {
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

fn test_overlay() -> InlineRightOverlay<'static, TestMessage, Theme, RecordingRenderer> {
    InlineRightOverlay::new(TestContent::default(), TestContent::default())
}

fn tree_for_inline_right() -> widget::Tree {
    let mut tree = widget::Tree::empty();
    tree.children.push(widget::Tree::empty());
    tree.children.push(widget::Tree::empty());
    tree
}

fn layout_for(bounds: Rectangle) -> layout::Node {
    layout::Node::new(bounds.size()).move_to(Point::new(bounds.x, bounds.y))
}

fn run_overlay_element_update(
    event: Event,
    cursor: mouse::Cursor,
    on_close: Option<TestMessage>,
) -> (Vec<TestMessage>, bool) {
    let mut overlay_content: Element<'static, TestMessage, Theme, RecordingRenderer> =
        TestContent { width: 80.0, height: 32.0, ..TestContent::default() }.into();
    let mut tree = widget::Tree::empty();
    let renderer = RecordingRenderer::default();
    let mut clipboard = iced::advanced::clipboard::Null;
    let layout_node = layout_for(Rectangle { x: 136.0, y: 20.0, width: 80.0, height: 32.0 });
    let mut element = InlineRightOverlayElement {
        position: Point::new(10.0, 20.0),
        target_bounds: Rectangle { x: 10.0, y: 20.0, width: 120.0, height: 40.0 },
        viewport: Rectangle::with_size(Size::new(300.0, 200.0)),
        gap: 6.0,
        snap_within_viewport: true,
        overlay: &mut overlay_content,
        tree: &mut tree,
        on_close,
    };
    let mut messages = Vec::new();
    let captured = {
        let mut shell = Shell::new(&mut messages);
        <InlineRightOverlayElement<'_, '_, TestMessage, Theme, RecordingRenderer> as overlay::Overlay<
            TestMessage,
            Theme,
            RecordingRenderer,
        >>::update(
            &mut element,
            &event,
            Layout::new(&layout_node),
            cursor,
            &renderer,
            &mut clipboard,
            &mut shell,
        );
        shell.is_event_captured()
    };

    (messages, captured)
}

#[test]
fn inline_right_overlay_new_sets_hidden_defaults() {
    let overlay = InlineRightOverlay::new(label("content"), label("overlay"));

    assert!(!overlay.show);
    assert_eq!(overlay.gap, 0.0);
    assert!(overlay.snap_within_viewport);
    assert!(overlay.on_close.is_none());
}

#[test]
fn inline_right_overlay_builder_updates_all_flags() {
    let overlay = InlineRightOverlay::new(label("content"), label("overlay"))
        .show(true)
        .gap(12.0)
        .snap_within_viewport(false)
        .on_close(TestMessage::Close);

    assert!(overlay.show);
    assert_eq!(overlay.gap, 12.0);
    assert!(!overlay.snap_within_viewport);
    assert_eq!(overlay.on_close, Some(TestMessage::Close));
}

#[test]
fn inline_right_position_uses_right_edge_plus_gap_without_snap() {
    let position = compute_inline_right_position(
        Point::new(20.0, 40.0),
        Rectangle { x: 20.0, y: 40.0, width: 90.0, height: 30.0 },
        Rectangle::with_size(Size::new(180.0, 140.0)),
        Size::new(80.0, 70.0),
        8.0,
        false,
    );

    assert_eq!(position, Point::new(118.0, 40.0));
}

#[test]
fn inline_right_position_snaps_to_available_viewport() {
    let position = compute_inline_right_position(
        Point::new(140.0, 120.0),
        Rectangle { x: 140.0, y: 120.0, width: 70.0, height: 30.0 },
        Rectangle::with_size(Size::new(180.0, 140.0)),
        Size::new(80.0, 70.0),
        8.0,
        true,
    );

    assert_eq!(position, Point::new(100.0, 70.0));
}

#[test]
fn inline_right_position_snaps_negative_coordinates_to_origin() {
    let position = compute_inline_right_position(
        Point::new(-90.0, -20.0),
        Rectangle { x: -90.0, y: -20.0, width: 40.0, height: 30.0 },
        Rectangle::with_size(Size::new(180.0, 140.0)),
        Size::new(80.0, 70.0),
        8.0,
        true,
    );

    assert_eq!(position, Point::ORIGIN);
}

#[test]
fn inline_right_position_handles_overlay_larger_than_viewport() {
    let position = compute_inline_right_position(
        Point::new(40.0, 30.0),
        Rectangle { x: 40.0, y: 30.0, width: 90.0, height: 30.0 },
        Rectangle::with_size(Size::new(100.0, 80.0)),
        Size::new(140.0, 120.0),
        8.0,
        true,
    );

    assert_eq!(position, Point::ORIGIN);
}

#[test]
fn inline_right_widget_reports_content_and_overlay_children() {
    let overlay = test_overlay();

    let children = <InlineRightOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::children(&overlay);

    assert_eq!(children.len(), 2);
}

#[test]
fn inline_right_widget_diff_keeps_two_child_trees() {
    let overlay = test_overlay();
    let mut tree = tree_for_inline_right();

    <InlineRightOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::diff(&overlay, &mut tree);

    assert_eq!(tree.children.len(), 2);
}

#[test]
fn inline_right_widget_delegates_size_and_layout_to_content() {
    let mut overlay = test_overlay();
    let mut tree = tree_for_inline_right();
    let renderer = RecordingRenderer::default();

    let size = <InlineRightOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::size(&overlay);
    let hint = <InlineRightOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::size_hint(&overlay);
    let node = <InlineRightOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
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
fn inline_right_widget_draw_and_update_delegate_to_content() {
    let mut overlay = test_overlay();
    let mut tree = tree_for_inline_right();
    let mut renderer = RecordingRenderer::default();
    let node = layout_for(Rectangle { x: 4.0, y: 8.0, width: 120.0, height: 40.0 });

    <InlineRightOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::draw(
        &overlay,
        &tree,
        &mut renderer,
        &Theme::Dark,
        &renderer::Style::default(),
        Layout::new(&node),
        mouse::Cursor::Unavailable,
        &Rectangle::INFINITE,
    );

    let mut clipboard = iced::advanced::clipboard::Null;
    let mut messages = Vec::new();
    let mut shell = Shell::new(&mut messages);
    let renderer_ref = RecordingRenderer::default();

    <InlineRightOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::update(
        &mut overlay,
        &mut tree,
        &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        Layout::new(&node),
        mouse::Cursor::Available(Point::new(8.0, 12.0)),
        &renderer_ref,
        &mut clipboard,
        &mut shell,
        &Rectangle::INFINITE,
    );

    assert_eq!(renderer.fills, vec![Rectangle { x: 4.0, y: 8.0, width: 120.0, height: 40.0 }]);
    assert_eq!(messages, vec![TestMessage::ChildUpdated]);
}

#[test]
fn inline_right_widget_mouse_interaction_delegates_to_content() {
    let overlay = InlineRightOverlay::new(
        TestContent { interaction: mouse::Interaction::Text, ..TestContent::default() },
        TestContent::default(),
    );
    let tree = tree_for_inline_right();
    let renderer = RecordingRenderer::default();
    let node = layout_for(Rectangle { x: 0.0, y: 0.0, width: 120.0, height: 40.0 });

    let interaction = <InlineRightOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
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

    assert_eq!(interaction, mouse::Interaction::Text);
}

#[test]
fn inline_right_widget_overlay_respects_show_flag() {
    let renderer = RecordingRenderer::default();
    let node = layout_for(Rectangle { x: 10.0, y: 20.0, width: 120.0, height: 40.0 });
    let mut hidden = test_overlay();
    let mut hidden_tree = tree_for_inline_right();
    let mut shown = test_overlay().show(true).gap(6.0);
    let mut shown_tree = tree_for_inline_right();

    let hidden_overlay =
        <InlineRightOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
            TestMessage,
            Theme,
            RecordingRenderer,
        >>::overlay(
            &mut hidden,
            &mut hidden_tree,
            Layout::new(&node),
            &renderer,
            &Rectangle::with_size(Size::new(300.0, 200.0)),
            Vector::new(3.0, 4.0),
        );
    let shown_overlay = <InlineRightOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::overlay(
        &mut shown,
        &mut shown_tree,
        Layout::new(&node),
        &renderer,
        &Rectangle::with_size(Size::new(300.0, 200.0)),
        Vector::new(3.0, 4.0),
    );

    assert!(hidden_overlay.is_none());
    assert!(shown_overlay.is_some());
}

#[test]
fn inline_right_overlay_element_layout_draw_and_mouse_interaction_use_overlay_content() {
    let mut overlay_content: Element<'static, TestMessage, Theme, RecordingRenderer> =
        TestContent { width: 80.0, height: 32.0, interaction: mouse::Interaction::Grab }.into();
    let mut tree = widget::Tree::empty();
    let renderer = RecordingRenderer::default();
    let mut element = InlineRightOverlayElement {
        position: Point::new(10.0, 20.0),
        target_bounds: Rectangle { x: 10.0, y: 20.0, width: 120.0, height: 40.0 },
        viewport: Rectangle::with_size(Size::new(300.0, 200.0)),
        gap: 6.0,
        snap_within_viewport: true,
        overlay: &mut overlay_content,
        tree: &mut tree,
        on_close: None,
    };

    let node = <InlineRightOverlayElement<'_, '_, TestMessage, Theme, RecordingRenderer> as overlay::Overlay<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::layout(&mut element, &renderer, Size::new(300.0, 200.0));
    let interaction = <InlineRightOverlayElement<'_, '_, TestMessage, Theme, RecordingRenderer> as overlay::Overlay<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::mouse_interaction(&element, Layout::new(&node), mouse::Cursor::Unavailable, &renderer);

    let mut draw_renderer = RecordingRenderer::default();
    <InlineRightOverlayElement<'_, '_, TestMessage, Theme, RecordingRenderer> as overlay::Overlay<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::draw(
        &element,
        &mut draw_renderer,
        &Theme::Dark,
        &renderer::Style::default(),
        Layout::new(&node),
        mouse::Cursor::Unavailable,
    );

    assert_eq!(node.bounds(), Rectangle { x: 136.0, y: 20.0, width: 80.0, height: 32.0 });
    assert_eq!(interaction, mouse::Interaction::Grab);
    assert_eq!(draw_renderer.fills, vec![node.bounds()]);
}

#[test]
fn inline_right_overlay_element_update_closes_only_when_click_is_outside() {
    let mut overlay_content: Element<'static, TestMessage, Theme, RecordingRenderer> =
        TestContent { width: 80.0, height: 32.0, ..TestContent::default() }.into();
    let mut tree = widget::Tree::empty();
    let renderer = RecordingRenderer::default();
    let mut clipboard = iced::advanced::clipboard::Null;
    let layout_node = layout_for(Rectangle { x: 136.0, y: 20.0, width: 80.0, height: 32.0 });
    let mut element = InlineRightOverlayElement {
        position: Point::new(10.0, 20.0),
        target_bounds: Rectangle { x: 10.0, y: 20.0, width: 120.0, height: 40.0 },
        viewport: Rectangle::with_size(Size::new(300.0, 200.0)),
        gap: 6.0,
        snap_within_viewport: true,
        overlay: &mut overlay_content,
        tree: &mut tree,
        on_close: Some(TestMessage::Close),
    };

    let mut outside_messages = Vec::new();
    let mut outside_shell = Shell::new(&mut outside_messages);
    <InlineRightOverlayElement<'_, '_, TestMessage, Theme, RecordingRenderer> as overlay::Overlay<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::update(
        &mut element,
        &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        Layout::new(&layout_node),
        mouse::Cursor::Available(Point::new(250.0, 80.0)),
        &renderer,
        &mut clipboard,
        &mut outside_shell,
    );

    let mut inside_messages = Vec::new();
    let mut inside_shell = Shell::new(&mut inside_messages);
    <InlineRightOverlayElement<'_, '_, TestMessage, Theme, RecordingRenderer> as overlay::Overlay<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::update(
        &mut element,
        &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        Layout::new(&layout_node),
        mouse::Cursor::Available(Point::new(140.0, 24.0)),
        &renderer,
        &mut clipboard,
        &mut inside_shell,
    );

    assert_eq!(outside_messages, vec![TestMessage::Close, TestMessage::ChildUpdated]);
    assert_eq!(inside_messages, vec![TestMessage::ChildUpdated]);
}

#[test]
fn inline_right_overlay_element_target_click_does_not_close_and_captures_mouse() {
    let (messages, captured) = run_overlay_element_update(
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        mouse::Cursor::Available(Point::new(20.0, 30.0)),
        Some(TestMessage::Close),
    );

    assert_eq!(messages, vec![TestMessage::ChildUpdated]);
    assert!(captured);
}

#[test]
fn inline_right_overlay_element_without_close_message_only_forwards_mouse() {
    let (messages, captured) = run_overlay_element_update(
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        mouse::Cursor::Available(Point::new(250.0, 80.0)),
        None,
    );

    assert_eq!(messages, vec![TestMessage::ChildUpdated]);
    assert!(captured);
}

#[test]
fn inline_right_overlay_element_unavailable_cursor_skips_close_and_captures_mouse() {
    let (messages, captured) = run_overlay_element_update(
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        mouse::Cursor::Unavailable,
        Some(TestMessage::Close),
    );

    assert_eq!(messages, vec![TestMessage::ChildUpdated]);
    assert!(captured);
}

#[test]
fn inline_right_overlay_element_non_left_click_skips_close_and_captures_mouse() {
    let (messages, captured) = run_overlay_element_update(
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)),
        mouse::Cursor::Available(Point::new(250.0, 80.0)),
        Some(TestMessage::Close),
    );

    assert_eq!(messages, vec![TestMessage::ChildUpdated]);
    assert!(captured);
}

#[test]
fn inline_right_overlay_element_non_mouse_event_forwards_without_capture() {
    let (messages, captured) = run_overlay_element_update(
        Event::Keyboard(iced::keyboard::Event::ModifiersChanged(iced::keyboard::Modifiers::SHIFT)),
        mouse::Cursor::Available(Point::new(250.0, 80.0)),
        Some(TestMessage::Close),
    );

    assert_eq!(messages, vec![TestMessage::ChildUpdated]);
    assert!(!captured);
}

#[test]
fn inline_right_overlay_converts_to_element() {
    let _: Element<'static, TestMessage, Theme> =
        InlineRightOverlay::new(label("content"), label("overlay")).into();
}
