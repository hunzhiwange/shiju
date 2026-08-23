//! 验证预览拖拽区域与覆盖宿主的组件生命周期和事件转发。
//! 组件树差异与鼠标交互测试用于保证包装器不破坏子组件状态。

use super::widgets::{DraggableArea, PreviewOverlayHost};
use crate::app::Message;
use iced::advanced::{
    Clipboard, Layout, Renderer, Shell, Widget, layout, mouse, overlay, renderer, widget,
};
use iced::widget::{container, text};
use iced::{
    Background, Element, Event, Length, Point, Rectangle, Size, Theme, Transformation, Vector,
};

#[derive(Debug, Clone, PartialEq)]
enum TestMessage {
    ChildUpdated,
    DragStarted,
    DragEnded,
}

#[derive(Debug, Clone)]
struct TestContent<MessageT> {
    width: f32,
    height: f32,
    interaction: mouse::Interaction,
    publish: Option<MessageT>,
    has_overlay: bool,
}

impl<MessageT> TestContent<MessageT> {
    fn new(width: f32, height: f32) -> Self {
        Self {
            width,
            height,
            interaction: mouse::Interaction::Pointer,
            publish: None,
            has_overlay: false,
        }
    }

    fn publish(mut self, message: MessageT) -> Self {
        self.publish = Some(message);
        self
    }

    fn interaction(mut self, interaction: mouse::Interaction) -> Self {
        self.interaction = interaction;
        self
    }

    fn with_overlay(mut self) -> Self {
        self.has_overlay = true;
        self
    }
}

impl<MessageT: Clone> Widget<MessageT, Theme, RecordingRenderer> for TestContent<MessageT> {
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
        shell: &mut Shell<'_, MessageT>,
        _viewport: &Rectangle,
    ) {
        if let Some(message) = &self.publish {
            shell.publish(message.clone());
        }
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

    fn overlay<'a>(
        &'a mut self,
        _tree: &'a mut widget::Tree,
        _layout: Layout<'a>,
        _renderer: &RecordingRenderer,
        _viewport: &Rectangle,
        _translation: Vector,
    ) -> Option<overlay::Element<'a, MessageT, Theme, RecordingRenderer>> {
        self.has_overlay.then(|| overlay::Element::new(Box::new(TestOverlay)))
    }
}

impl<'a, MessageT: Clone + 'a> From<TestContent<MessageT>>
    for Element<'a, MessageT, Theme, RecordingRenderer>
{
    fn from(content: TestContent<MessageT>) -> Self {
        Element::new(content)
    }
}

#[derive(Debug)]
struct TestOverlay;

impl<MessageT> overlay::Overlay<MessageT, Theme, RecordingRenderer> for TestOverlay {
    fn layout(&mut self, _renderer: &RecordingRenderer, _bounds: Size) -> layout::Node {
        layout::Node::new(Size::new(10.0, 12.0))
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

#[derive(Debug, Default)]
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

type TestDraggableArea<'a> = DraggableArea<'a, TestMessage, Theme, RecordingRenderer>;
type TestPreviewOverlayHost<'a> = PreviewOverlayHost<'a, Message, Theme, RecordingRenderer>;

fn layout_for(bounds: Rectangle) -> layout::Node {
    layout::Node::new(bounds.size()).move_to(Point::new(bounds.x, bounds.y))
}

fn tree_for_area(area: &TestDraggableArea<'_>) -> widget::Tree {
    widget::Tree::new(area as &dyn Widget<TestMessage, Theme, RecordingRenderer>)
}

fn tree_for_host(host: &TestPreviewOverlayHost<'_>) -> widget::Tree {
    widget::Tree::new(host as &dyn Widget<Message, Theme, RecordingRenderer>)
}

fn draggable_area() -> TestDraggableArea<'static> {
    DraggableArea::new(
        TestContent::new(64.0, 32.0).publish(TestMessage::ChildUpdated).into(),
        TestMessage::DragStarted,
        TestMessage::DragEnded,
    )
}

fn update_draggable(
    area: &mut TestDraggableArea<'static>,
    tree: &mut widget::Tree,
    event: Event,
    cursor: mouse::Cursor,
) -> Vec<TestMessage> {
    let renderer = RecordingRenderer::default();
    let mut clipboard = iced::advanced::clipboard::Null;
    let node = layout_for(Rectangle { x: 10.0, y: 20.0, width: 64.0, height: 32.0 });
    let mut messages = Vec::new();
    let mut shell = Shell::new(&mut messages);

    <TestDraggableArea<'_> as Widget<TestMessage, Theme, RecordingRenderer>>::update(
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

fn preview_host(show: bool) -> TestPreviewOverlayHost<'static> {
    PreviewOverlayHost::new(
        TestContent::new(120.0, 80.0).publish(Message::None),
        TestContent::new(40.0, 30.0).interaction(mouse::Interaction::Text),
    )
    .show(show)
    .pos(Some((12.0, 24.0)))
    .on_close(Message::None)
}

fn preview_overlay_layout(
    host: &mut TestPreviewOverlayHost<'static>,
    tree: &mut widget::Tree,
    renderer: &RecordingRenderer,
    bounds: Size,
) -> layout::Node {
    let content_node =
        <TestPreviewOverlayHost<'_> as Widget<Message, Theme, RecordingRenderer>>::layout(
            host,
            tree,
            renderer,
            &layout::Limits::new(Size::ZERO, bounds),
        );
    let mut overlay =
        <TestPreviewOverlayHost<'_> as Widget<Message, Theme, RecordingRenderer>>::overlay(
            host,
            tree,
            Layout::new(&content_node),
            renderer,
            &Rectangle::with_size(bounds),
            Vector::new(0.0, 0.0),
        )
        .expect("shown preview host should expose an overlay");

    overlay.as_overlay_mut().layout(renderer, bounds)
}

#[test]
fn preview_overlay_host_builder_sets_children_and_size_from_content() {
    let content: Element<'_, Message> =
        container(text("content")).width(Length::Fixed(120.0)).height(Length::Fixed(80.0)).into();
    let overlay: Element<'_, Message> = container(text("overlay")).into();

    let host = PreviewOverlayHost::new(content, overlay)
        .show(true)
        .pos(Some((12.0, 24.0)))
        .on_close(Message::None);

    let children = host.children();
    assert_eq!(children.len(), 2);
    assert_eq!(host.size().width, Length::Fixed(120.0));
    assert_eq!(host.size().height, Length::Fixed(80.0));
}

#[test]
fn draggable_area_exposes_single_child_state_and_content_size() {
    let content: Element<'_, Message> =
        container(text("drag")).width(Length::Fixed(64.0)).height(Length::Fixed(32.0)).into();
    let area = DraggableArea::new(content, Message::None, Message::None);

    assert_eq!(area.children().len(), 1);
    assert_eq!(area.size().width, Length::Fixed(64.0));
    assert_eq!(area.size().height, Length::Fixed(32.0));
    assert_eq!(area.tag(), widget::tree::Tag::of::<super::widgets::DraggableAreaState>());
}

#[test]
fn draggable_area_state_starts_empty() {
    let area = draggable_area();
    let state = area.state();

    state.downcast_ref::<super::widgets::DraggableAreaState>();
}

#[test]
fn draggable_area_diff_keeps_single_wrapped_child() {
    let area = draggable_area();
    let mut tree = tree_for_area(&area);

    <TestDraggableArea<'_> as Widget<TestMessage, Theme, RecordingRenderer>>::diff(
        &area, &mut tree,
    );

    assert_eq!(tree.children.len(), 1);
}

#[test]
fn draggable_area_layout_and_draw_delegate_to_content() {
    let mut area = draggable_area();
    let mut tree = tree_for_area(&area);
    let mut renderer = RecordingRenderer::default();
    let node = <TestDraggableArea<'_> as Widget<TestMessage, Theme, RecordingRenderer>>::layout(
        &mut area,
        &mut tree,
        &renderer,
        &layout::Limits::new(Size::ZERO, Size::new(400.0, 200.0)),
    );

    assert_eq!(node.size(), Size::new(64.0, 32.0));

    <TestDraggableArea<'_> as Widget<TestMessage, Theme, RecordingRenderer>>::draw(
        &area,
        &tree,
        &mut renderer,
        &Theme::Dark,
        &renderer::Style::default(),
        Layout::new(&node),
        mouse::Cursor::Unavailable,
        &Rectangle::INFINITE,
    );

    assert_eq!(renderer.fills, vec![Rectangle { x: 0.0, y: 0.0, width: 64.0, height: 32.0 }]);
}

#[test]
fn draggable_area_mouse_interaction_delegates_to_content() {
    let area = DraggableArea::new(
        TestContent::new(64.0, 32.0).interaction(mouse::Interaction::Text).into(),
        TestMessage::DragStarted,
        TestMessage::DragEnded,
    );
    let tree = tree_for_area(&area);
    let renderer = RecordingRenderer::default();
    let node = layout_for(Rectangle { x: 0.0, y: 0.0, width: 64.0, height: 32.0 });

    let interaction =
        <TestDraggableArea<'_> as Widget<TestMessage, Theme, RecordingRenderer>>::mouse_interaction(
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
fn draggable_area_publishes_start_after_threshold_and_end_on_release() {
    let mut area = draggable_area();
    let mut tree = tree_for_area(&area);

    let pressed = update_draggable(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        mouse::Cursor::Available(Point::new(12.0, 22.0)),
    );
    let moved = update_draggable(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::CursorMoved { position: Point::new(15.0, 22.0) }),
        mouse::Cursor::Available(Point::new(15.0, 22.0)),
    );
    let released = update_draggable(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        mouse::Cursor::Available(Point::new(15.0, 22.0)),
    );

    assert_eq!(pressed, vec![TestMessage::ChildUpdated]);
    assert_eq!(moved, vec![TestMessage::DragStarted, TestMessage::ChildUpdated]);
    assert_eq!(released, vec![TestMessage::DragEnded, TestMessage::ChildUpdated]);
}

#[test]
fn draggable_area_ignores_motion_below_threshold_and_resets_on_release() {
    let mut area = draggable_area();
    let mut tree = tree_for_area(&area);

    update_draggable(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        mouse::Cursor::Available(Point::new(12.0, 22.0)),
    );
    let moved = update_draggable(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::CursorMoved { position: Point::new(14.0, 22.0) }),
        mouse::Cursor::Available(Point::new(14.0, 22.0)),
    );
    let released = update_draggable(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        mouse::Cursor::Available(Point::new(14.0, 22.0)),
    );

    assert_eq!(moved, vec![TestMessage::ChildUpdated]);
    assert_eq!(released, vec![TestMessage::ChildUpdated]);
}

#[test]
fn draggable_area_ignores_press_outside_and_non_left_buttons() {
    let mut area = draggable_area();
    let mut tree = tree_for_area(&area);

    update_draggable(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)),
        mouse::Cursor::Available(Point::new(12.0, 22.0)),
    );
    let right_moved = update_draggable(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::CursorMoved { position: Point::new(20.0, 22.0) }),
        mouse::Cursor::Available(Point::new(20.0, 22.0)),
    );
    update_draggable(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        mouse::Cursor::Available(Point::new(500.0, 22.0)),
    );
    let outside_moved = update_draggable(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::CursorMoved { position: Point::new(520.0, 22.0) }),
        mouse::Cursor::Available(Point::new(520.0, 22.0)),
    );

    assert_eq!(right_moved, vec![TestMessage::ChildUpdated]);
    assert_eq!(outside_moved, vec![TestMessage::ChildUpdated]);
}

#[test]
fn draggable_area_ignores_left_press_without_cursor_position() {
    let mut area = draggable_area();
    let mut tree = tree_for_area(&area);

    let pressed = update_draggable(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        mouse::Cursor::Unavailable,
    );
    let moved = update_draggable(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::CursorMoved { position: Point::new(25.0, 22.0) }),
        mouse::Cursor::Available(Point::new(25.0, 22.0)),
    );
    let released = update_draggable(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        mouse::Cursor::Available(Point::new(25.0, 22.0)),
    );

    assert_eq!(pressed, vec![TestMessage::ChildUpdated]);
    assert_eq!(moved, vec![TestMessage::ChildUpdated]);
    assert_eq!(released, vec![TestMessage::ChildUpdated]);
}

#[test]
fn draggable_area_starts_once_and_only_left_release_ends_drag() {
    let mut area = draggable_area();
    let mut tree = tree_for_area(&area);

    update_draggable(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        mouse::Cursor::Available(Point::new(12.0, 22.0)),
    );
    let first_move = update_draggable(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::CursorMoved { position: Point::new(15.0, 22.0) }),
        mouse::Cursor::Available(Point::new(15.0, 22.0)),
    );
    let second_move = update_draggable(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::CursorMoved { position: Point::new(25.0, 22.0) }),
        mouse::Cursor::Available(Point::new(25.0, 22.0)),
    );
    let right_release = update_draggable(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Right)),
        mouse::Cursor::Available(Point::new(25.0, 22.0)),
    );
    let left_release = update_draggable(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        mouse::Cursor::Available(Point::new(25.0, 22.0)),
    );

    assert_eq!(first_move, vec![TestMessage::DragStarted, TestMessage::ChildUpdated]);
    assert_eq!(second_move, vec![TestMessage::ChildUpdated]);
    assert_eq!(right_release, vec![TestMessage::ChildUpdated]);
    assert_eq!(left_release, vec![TestMessage::DragEnded, TestMessage::ChildUpdated]);
}

#[test]
fn preview_overlay_host_diff_size_hint_layout_draw_and_interaction_delegate_to_content() {
    let mut host = preview_host(false);
    let mut tree = tree_for_host(&host);
    let mut renderer = RecordingRenderer::default();

    <TestPreviewOverlayHost<'_> as Widget<Message, Theme, RecordingRenderer>>::diff(
        &host, &mut tree,
    );
    let size_hint =
        <TestPreviewOverlayHost<'_> as Widget<Message, Theme, RecordingRenderer>>::size_hint(&host);
    let node = <TestPreviewOverlayHost<'_> as Widget<Message, Theme, RecordingRenderer>>::layout(
        &mut host,
        &mut tree,
        &renderer,
        &layout::Limits::new(Size::ZERO, Size::new(400.0, 200.0)),
    );

    <TestPreviewOverlayHost<'_> as Widget<Message, Theme, RecordingRenderer>>::draw(
        &host,
        &tree,
        &mut renderer,
        &Theme::Dark,
        &renderer::Style::default(),
        Layout::new(&node),
        mouse::Cursor::Unavailable,
        &Rectangle::INFINITE,
    );
    let interaction =
        <TestPreviewOverlayHost<'_> as Widget<Message, Theme, RecordingRenderer>>::mouse_interaction(
            &host,
            &tree,
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            &Rectangle::INFINITE,
            &renderer,
        );

    assert_eq!(tree.children.len(), 2);
    assert_eq!(size_hint, Size::new(Length::Fixed(120.0), Length::Fixed(80.0)));
    assert_eq!(node.size(), Size::new(120.0, 80.0));
    assert_eq!(renderer.fills, vec![Rectangle { x: 0.0, y: 0.0, width: 120.0, height: 80.0 }]);
    assert_eq!(interaction, mouse::Interaction::Pointer);
}

#[test]
fn preview_overlay_host_operate_reaches_content() {
    let mut host = preview_host(false);
    let mut tree = tree_for_host(&host);
    let renderer = RecordingRenderer::default();
    let node = layout_for(Rectangle { x: 10.0, y: 20.0, width: 120.0, height: 80.0 });
    let mut operation = RecordingOperation::default();

    <TestPreviewOverlayHost<'_> as Widget<Message, Theme, RecordingRenderer>>::operate(
        &mut host,
        &mut tree,
        Layout::new(&node),
        &renderer,
        &mut operation,
    );

    assert_eq!(operation.containers, vec![node.bounds()]);
}

#[test]
fn preview_overlay_host_update_reaches_content_only() {
    let mut host = preview_host(false);
    let mut tree = tree_for_host(&host);
    let renderer = RecordingRenderer::default();
    let mut clipboard = iced::advanced::clipboard::Null;
    let node = layout_for(Rectangle { x: 10.0, y: 20.0, width: 120.0, height: 80.0 });
    let mut messages = Vec::new();
    let mut shell = Shell::new(&mut messages);

    <TestPreviewOverlayHost<'_> as Widget<Message, Theme, RecordingRenderer>>::update(
        &mut host,
        &mut tree,
        &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        Layout::new(&node),
        mouse::Cursor::Available(Point::new(12.0, 22.0)),
        &renderer,
        &mut clipboard,
        &mut shell,
        &Rectangle::INFINITE,
    );

    assert_eq!(messages.len(), 1);
}

#[test]
fn preview_overlay_host_returns_overlay_only_when_showing() {
    let renderer = RecordingRenderer::default();
    let node = layout_for(Rectangle { x: 0.0, y: 0.0, width: 120.0, height: 80.0 });
    let mut hidden = preview_host(false);
    let mut hidden_tree = tree_for_host(&hidden);
    let hidden_overlay =
        <TestPreviewOverlayHost<'_> as Widget<Message, Theme, RecordingRenderer>>::overlay(
            &mut hidden,
            &mut hidden_tree,
            Layout::new(&node),
            &renderer,
            &Rectangle::INFINITE,
            iced::Vector::new(0.0, 0.0),
        );

    let mut shown = preview_host(true);
    let mut shown_tree = tree_for_host(&shown);
    let shown_overlay =
        <TestPreviewOverlayHost<'_> as Widget<Message, Theme, RecordingRenderer>>::overlay(
            &mut shown,
            &mut shown_tree,
            Layout::new(&node),
            &renderer,
            &Rectangle::INFINITE,
            iced::Vector::new(0.0, 0.0),
        );

    assert!(hidden_overlay.is_none());
    assert!(shown_overlay.is_some());
}

#[test]
fn preview_overlay_host_preserves_content_overlay_and_groups_it_when_shown() {
    let renderer = RecordingRenderer::default();
    let bounds = Size::new(100.0, 70.0);

    let mut hidden = PreviewOverlayHost::new(
        TestContent::<Message>::new(60.0, 40.0).with_overlay(),
        TestContent::new(40.0, 30.0),
    );
    let mut hidden_tree = tree_for_host(&hidden);
    let hidden_content_node =
        <TestPreviewOverlayHost<'_> as Widget<Message, Theme, RecordingRenderer>>::layout(
            &mut hidden,
            &mut hidden_tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, bounds),
        );
    let mut hidden_overlay =
        <TestPreviewOverlayHost<'_> as Widget<Message, Theme, RecordingRenderer>>::overlay(
            &mut hidden,
            &mut hidden_tree,
            Layout::new(&hidden_content_node),
            &renderer,
            &Rectangle::with_size(bounds),
            Vector::new(0.0, 0.0),
        )
        .expect("hidden host should preserve the content overlay");
    let hidden_overlay_node = hidden_overlay.as_overlay_mut().layout(&renderer, bounds);

    assert_eq!(hidden_overlay_node.size(), Size::new(10.0, 12.0));

    let mut shown = PreviewOverlayHost::new(
        TestContent::<Message>::new(60.0, 40.0).with_overlay(),
        TestContent::new(40.0, 30.0),
    )
    .show(true);
    let mut shown_tree = tree_for_host(&shown);
    let shown_overlay_node = preview_overlay_layout(&mut shown, &mut shown_tree, &renderer, bounds);
    let child_bounds: Vec<_> =
        shown_overlay_node.children().iter().map(layout::Node::bounds).collect();

    assert_eq!(shown_overlay_node.size(), bounds);
    assert_eq!(child_bounds.len(), 2);
    assert!(child_bounds.contains(&Rectangle::with_size(Size::new(10.0, 12.0))));
    assert!(child_bounds.contains(&Rectangle::with_size(Size::new(40.0, 30.0))));
}

#[test]
fn preview_overlay_layout_defaults_to_origin_and_clamps_requested_position() {
    let renderer = RecordingRenderer::default();
    let bounds = Size::new(100.0, 70.0);

    let mut default_position = PreviewOverlayHost::new(
        TestContent::<Message>::new(60.0, 40.0),
        TestContent::new(40.0, 30.0),
    )
    .show(true);
    let mut default_tree = tree_for_host(&default_position);
    let default_node =
        preview_overlay_layout(&mut default_position, &mut default_tree, &renderer, bounds);

    let mut upper_clamped = preview_host(true).pos(Some((999.0, 999.0)));
    let mut upper_tree = tree_for_host(&upper_clamped);
    let upper_node = preview_overlay_layout(&mut upper_clamped, &mut upper_tree, &renderer, bounds);

    let mut lower_clamped = preview_host(true).pos(Some((-10.0, -8.0)));
    let mut lower_tree = tree_for_host(&lower_clamped);
    let lower_node = preview_overlay_layout(&mut lower_clamped, &mut lower_tree, &renderer, bounds);

    let mut oversized = PreviewOverlayHost::new(
        TestContent::<Message>::new(60.0, 40.0),
        TestContent::new(140.0, 90.0),
    )
    .show(true)
    .pos(Some((999.0, 999.0)));
    let mut oversized_tree = tree_for_host(&oversized);
    let oversized_node =
        preview_overlay_layout(&mut oversized, &mut oversized_tree, &renderer, bounds);

    assert_eq!(default_node.children()[0].bounds(), Rectangle::with_size(Size::new(40.0, 30.0)));
    assert_eq!(
        upper_node.children()[0].bounds(),
        Rectangle { x: 60.0, y: 40.0, width: 40.0, height: 30.0 }
    );
    assert_eq!(lower_node.children()[0].bounds(), Rectangle::with_size(Size::new(40.0, 30.0)));
    assert_eq!(oversized_node.children()[0].bounds(), Rectangle::with_size(Size::new(140.0, 90.0)));
}

#[test]
fn preview_overlay_closes_only_for_mouse_press_outside_its_bounds() {
    let renderer = RecordingRenderer::default();
    let bounds = Size::new(100.0, 70.0);
    let mut host = preview_host(true);
    let mut tree = tree_for_host(&host);
    let content_node =
        <TestPreviewOverlayHost<'_> as Widget<Message, Theme, RecordingRenderer>>::layout(
            &mut host,
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, bounds),
        );
    let mut overlay =
        <TestPreviewOverlayHost<'_> as Widget<Message, Theme, RecordingRenderer>>::overlay(
            &mut host,
            &mut tree,
            Layout::new(&content_node),
            &renderer,
            &Rectangle::with_size(bounds),
            Vector::new(0.0, 0.0),
        )
        .expect("shown host should expose an overlay");
    let overlay_node = overlay.as_overlay_mut().layout(&renderer, bounds);
    let mut messages = Vec::new();
    let mut shell = Shell::new(&mut messages);
    let mut clipboard = iced::advanced::clipboard::Null;

    overlay.as_overlay_mut().update(
        &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        Layout::new(&overlay_node),
        mouse::Cursor::Available(Point::new(13.0, 25.0)),
        &renderer,
        &mut clipboard,
        &mut shell,
    );
    overlay.as_overlay_mut().update(
        &Event::Mouse(mouse::Event::CursorMoved { position: Point::new(99.0, 69.0) }),
        Layout::new(&overlay_node),
        mouse::Cursor::Available(Point::new(99.0, 69.0)),
        &renderer,
        &mut clipboard,
        &mut shell,
    );
    overlay.as_overlay_mut().update(
        &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        Layout::new(&overlay_node),
        mouse::Cursor::Unavailable,
        &renderer,
        &mut clipboard,
        &mut shell,
    );
    overlay.as_overlay_mut().update(
        &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)),
        Layout::new(&overlay_node),
        mouse::Cursor::Available(Point::new(99.0, 69.0)),
        &renderer,
        &mut clipboard,
        &mut shell,
    );

    drop(shell);
    assert_eq!(messages.len(), 1);
}

#[test]
fn preview_overlay_delegates_update_draw_and_mouse_interaction() {
    let mut renderer = RecordingRenderer::default();
    let bounds = Size::new(100.0, 70.0);
    let mut host = PreviewOverlayHost::new(
        TestContent::<Message>::new(60.0, 40.0),
        TestContent::new(40.0, 30.0).publish(Message::None).interaction(mouse::Interaction::Text),
    )
    .show(true)
    .pos(Some((12.0, 24.0)));
    let mut tree = tree_for_host(&host);
    let content_node =
        <TestPreviewOverlayHost<'_> as Widget<Message, Theme, RecordingRenderer>>::layout(
            &mut host,
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, bounds),
        );
    let mut overlay =
        <TestPreviewOverlayHost<'_> as Widget<Message, Theme, RecordingRenderer>>::overlay(
            &mut host,
            &mut tree,
            Layout::new(&content_node),
            &renderer,
            &Rectangle::with_size(bounds),
            Vector::new(0.0, 0.0),
        )
        .expect("shown host should expose an overlay");
    let overlay_node = overlay.as_overlay_mut().layout(&renderer, bounds);
    let interaction = overlay.as_overlay().mouse_interaction(
        Layout::new(&overlay_node),
        mouse::Cursor::Available(Point::new(13.0, 25.0)),
        &renderer,
    );

    overlay.as_overlay().draw(
        &mut renderer,
        &Theme::Dark,
        &renderer::Style::default(),
        Layout::new(&overlay_node),
        mouse::Cursor::Available(Point::new(13.0, 25.0)),
    );

    let mut messages = Vec::new();
    let mut shell = Shell::new(&mut messages);
    let mut clipboard = iced::advanced::clipboard::Null;
    overlay.as_overlay_mut().update(
        &Event::Mouse(mouse::Event::CursorMoved { position: Point::new(13.0, 25.0) }),
        Layout::new(&overlay_node),
        mouse::Cursor::Available(Point::new(13.0, 25.0)),
        &renderer,
        &mut clipboard,
        &mut shell,
    );

    drop(shell);
    assert_eq!(interaction, mouse::Interaction::Text);
    assert_eq!(renderer.fills, vec![Rectangle { x: 12.0, y: 24.0, width: 40.0, height: 30.0 }]);
    assert_eq!(messages.len(), 1);
}

#[test]
fn preview_overlay_host_converts_into_element() {
    let _element: Element<'_, Message, Theme, RecordingRenderer> = preview_host(false).into();
}
