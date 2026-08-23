//! 验证应用通用组件的构造、主题样式和边界输入。
//!
//! 组件夹具关注点击、右键、工具提示和主题样式，覆盖空内容与禁用边界。

use crate::app::components::widgets::RightClickArea;
use iced::advanced::{Clipboard, Layout, Renderer, Shell, Widget, layout, mouse, renderer, widget};
use iced::{Background, Element, Event, Length, Point, Rectangle, Size, Theme, Transformation};

#[derive(Debug, Clone, PartialEq)]
enum TestMessage {
    ChildUpdated,
    RightClicked(Point),
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

    fn operate(
        &mut self,
        _tree: &mut widget::Tree,
        layout: Layout<'_>,
        _renderer: &RecordingRenderer,
        operation: &mut dyn widget::Operation,
    ) {
        operation.container(None, layout.bounds());
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

#[derive(Default)]
struct RecordingOperation {
    containers: Vec<Rectangle>,
}

impl widget::Operation for RecordingOperation {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn widget::Operation)) {
        operate(self);
    }

    fn container(&mut self, _id: Option<&widget::Id>, bounds: Rectangle) {
        self.containers.push(bounds);
    }
}

fn area() -> RightClickArea<'static, TestMessage, Theme, RecordingRenderer> {
    RightClickArea::new(TestContent::default().into(), Box::new(TestMessage::RightClicked))
}

fn tree_for(
    widget: &RightClickArea<'static, TestMessage, Theme, RecordingRenderer>,
) -> widget::Tree {
    let widget: &dyn Widget<TestMessage, Theme, RecordingRenderer> = widget;
    widget::Tree::new(widget)
}

fn layout_for(bounds: Rectangle) -> layout::Node {
    layout::Node::new(bounds.size()).move_to(Point::new(bounds.x, bounds.y))
}

fn update_with(
    area: &mut RightClickArea<'static, TestMessage, Theme, RecordingRenderer>,
    tree: &mut widget::Tree,
    event: Event,
    cursor: mouse::Cursor,
) -> Vec<TestMessage> {
    update_with_bounds(
        area,
        tree,
        event,
        cursor,
        Rectangle { x: 10.0, y: 20.0, width: 120.0, height: 40.0 },
    )
}

fn update_with_bounds(
    area: &mut RightClickArea<'static, TestMessage, Theme, RecordingRenderer>,
    tree: &mut widget::Tree,
    event: Event,
    cursor: mouse::Cursor,
    bounds: Rectangle,
) -> Vec<TestMessage> {
    let renderer = RecordingRenderer::default();
    let mut clipboard = iced::advanced::clipboard::Null;
    let node = layout_for(bounds);
    let mut messages = Vec::new();
    let mut shell = Shell::new(&mut messages);

    <RightClickArea<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::update(
        area,
        tree,
        &event,
        Layout::new(&node),
        cursor,
        &renderer,
        &mut clipboard,
        &mut shell,
        &Rectangle::INFINITE,
    );

    messages
}

#[test]
fn new_wraps_content_and_preserves_child_tree() {
    let area = area();
    let children = <RightClickArea<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::children(&area);

    assert_eq!(children.len(), 1);
}

#[test]
fn diff_keeps_single_wrapped_child() {
    let area = area();
    let mut tree = tree_for(&area);

    <RightClickArea<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::diff(&area, &mut tree);

    assert_eq!(tree.children.len(), 1);
}

#[test]
fn size_and_size_hint_delegate_to_content() {
    let area = area();

    let size = <RightClickArea<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::size(&area);
    let hint = <RightClickArea<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::size_hint(&area);

    assert_eq!(size, Size::new(Length::Fixed(120.0), Length::Fixed(40.0)));
    assert_eq!(hint, size);
}

#[test]
fn layout_delegates_to_content() {
    let mut area = area();
    let mut tree = tree_for(&area);
    let renderer = RecordingRenderer::default();
    let node = <RightClickArea<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::layout(
        &mut area,
        &mut tree,
        &renderer,
        &layout::Limits::new(Size::ZERO, Size::new(400.0, 200.0)),
    );

    assert_eq!(node.size(), Size::new(120.0, 40.0));
}

#[test]
fn operate_delegates_to_content() {
    let mut area = area();
    let mut tree = tree_for(&area);
    let renderer = RecordingRenderer::default();
    let bounds = Rectangle { x: 4.0, y: 8.0, width: 120.0, height: 40.0 };
    let node = layout_for(bounds);
    let mut operation = RecordingOperation::default();

    <RightClickArea<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::operate(&mut area, &mut tree, Layout::new(&node), &renderer, &mut operation);

    assert_eq!(operation.containers, vec![bounds]);
}

#[test]
fn draw_delegates_to_content() {
    let area = area();
    let tree = tree_for(&area);
    let mut renderer = RecordingRenderer::default();
    let node = layout_for(Rectangle { x: 4.0, y: 8.0, width: 120.0, height: 40.0 });

    <RightClickArea<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::draw(
        &area,
        &tree,
        &mut renderer,
        &Theme::Dark,
        &renderer::Style::default(),
        Layout::new(&node),
        mouse::Cursor::Unavailable,
        &Rectangle::INFINITE,
    );

    assert_eq!(renderer.fills, vec![Rectangle { x: 4.0, y: 8.0, width: 120.0, height: 40.0 }]);
}

#[test]
fn right_click_inside_publishes_local_position_after_child_update() {
    let mut area = area();
    let mut tree = tree_for(&area);

    let messages = update_with(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)),
        mouse::Cursor::Available(Point::new(34.0, 47.0)),
    );

    assert_eq!(
        messages,
        vec![TestMessage::ChildUpdated, TestMessage::RightClicked(Point::new(24.0, 27.0)),]
    );
}

#[test]
fn global_position_mode_publishes_window_position_after_child_update() {
    let mut area = area().use_global_position();
    let mut tree = tree_for(&area);

    let messages = update_with(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)),
        mouse::Cursor::Available(Point::new(34.0, 47.0)),
    );

    assert_eq!(
        messages,
        vec![TestMessage::ChildUpdated, TestMessage::RightClicked(Point::new(34.0, 47.0)),]
    );
}

#[test]
fn global_position_mode_uses_raw_window_position_inside_scrolling_content() {
    let mut area = area().use_global_position();
    let mut tree = tree_for(&area);

    let moved_messages = update_with(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::CursorMoved { position: Point::new(34.0, 47.0) }),
        mouse::Cursor::Available(Point::new(34.0, 47.0)),
    );
    assert_eq!(moved_messages, vec![TestMessage::ChildUpdated]);

    let messages = update_with_bounds(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)),
        mouse::Cursor::Available(Point::new(34.0, 147.0)),
        Rectangle { x: 10.0, y: 120.0, width: 120.0, height: 40.0 },
    );

    assert_eq!(
        messages,
        vec![TestMessage::ChildUpdated, TestMessage::RightClicked(Point::new(34.0, 47.0)),]
    );
}

#[test]
fn preserve_on_right_click_skips_child_update_for_right_click_inside() {
    let mut area = area().preserve_on_right_click();
    let mut tree = tree_for(&area);

    let messages = update_with(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)),
        mouse::Cursor::Available(Point::new(34.0, 47.0)),
    );

    assert_eq!(messages, vec![TestMessage::RightClicked(Point::new(24.0, 27.0))]);
}

#[test]
fn non_right_click_only_reaches_child() {
    let mut area = area();
    let mut tree = tree_for(&area);

    let messages = update_with(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        mouse::Cursor::Available(Point::new(34.0, 47.0)),
    );

    assert_eq!(messages, vec![TestMessage::ChildUpdated]);
}

#[test]
fn right_click_outside_only_reaches_child() {
    let mut area = area();
    let mut tree = tree_for(&area);

    let messages = update_with(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)),
        mouse::Cursor::Available(Point::new(500.0, 47.0)),
    );

    assert_eq!(messages, vec![TestMessage::ChildUpdated]);
}

#[test]
fn right_click_without_cursor_only_reaches_child() {
    let mut area = area();
    let mut tree = tree_for(&area);

    let messages = update_with(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)),
        mouse::Cursor::Unavailable,
    );

    assert_eq!(messages, vec![TestMessage::ChildUpdated]);
}

#[test]
fn preserved_area_forwards_non_right_click_to_child() {
    let mut area = area().preserve_on_right_click();
    let mut tree = tree_for(&area);

    let messages = update_with(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        mouse::Cursor::Available(Point::new(34.0, 47.0)),
    );

    assert_eq!(messages, vec![TestMessage::ChildUpdated]);
}

#[test]
fn preserved_area_forwards_right_click_outside_to_child() {
    let mut area = area().preserve_on_right_click();
    let mut tree = tree_for(&area);

    let messages = update_with(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)),
        mouse::Cursor::Available(Point::new(500.0, 47.0)),
    );

    assert_eq!(messages, vec![TestMessage::ChildUpdated]);
}

#[test]
fn non_button_mouse_event_only_reaches_child() {
    let mut area = area();
    let mut tree = tree_for(&area);

    let messages = update_with(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::CursorMoved { position: Point::new(34.0, 47.0) }),
        mouse::Cursor::Available(Point::new(34.0, 47.0)),
    );

    assert_eq!(messages, vec![TestMessage::ChildUpdated]);
}

#[test]
fn right_click_at_top_left_publishes_zero_local_position() {
    let mut area = area();
    let mut tree = tree_for(&area);

    let messages = update_with(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)),
        mouse::Cursor::Available(Point::new(10.0, 20.0)),
    );

    assert_eq!(
        messages,
        vec![TestMessage::ChildUpdated, TestMessage::RightClicked(Point::new(0.0, 0.0)),]
    );
}

#[test]
fn mouse_interaction_delegates_to_content() {
    let area = RightClickArea::new(
        TestContent { interaction: mouse::Interaction::Text, ..TestContent::default() }.into(),
        Box::new(TestMessage::RightClicked),
    );
    let tree = tree_for(&area);
    let renderer = RecordingRenderer::default();
    let node = layout_for(Rectangle { x: 0.0, y: 0.0, width: 120.0, height: 40.0 });

    let interaction = <RightClickArea<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::mouse_interaction(
        &area,
        &tree,
        Layout::new(&node),
        mouse::Cursor::Unavailable,
        &Rectangle::INFINITE,
        &renderer,
    );

    assert_eq!(interaction, mouse::Interaction::Text);
}

#[test]
fn overlay_delegates_to_content_none() {
    let mut area = area();
    let mut tree = tree_for(&area);
    let renderer = RecordingRenderer::default();
    let node = layout_for(Rectangle { x: 0.0, y: 0.0, width: 120.0, height: 40.0 });

    let overlay = <RightClickArea<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::overlay(
        &mut area,
        &mut tree,
        Layout::new(&node),
        &renderer,
        &Rectangle::INFINITE,
        iced::Vector::new(0.0, 0.0),
    );

    assert!(overlay.is_none());
}

#[test]
fn right_click_area_converts_into_element() {
    let _element: Element<'_, TestMessage, Theme, RecordingRenderer> = area().into();
}
