//! 验证拖放区域向子组件委托布局、绘制、事件与鼠标交互。
//! 伪渲染器记录绘制四边形，使测试无需真实窗口即可检查覆盖层反馈。

use iced::advanced::{
    Clipboard, Layout, Renderer as _, Shell, Widget, layout, mouse, renderer, widget,
};
use iced::widget::text;
use iced::{Background, Element, Event, Length, Point, Rectangle, Size, Theme, Transformation};

use crate::app::Message;

use super::drop_area::{DropArea, DropAreaElement};

#[derive(Debug, Clone, PartialEq)]
enum TestMessage {
    Dropped,
    Hovered,
    NotHovered,
    ChildUpdated,
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
}

impl iced::advanced::Renderer for RecordingRenderer {
    fn start_layer(&mut self, _bounds: Rectangle) {}

    fn end_layer(&mut self) {}

    fn start_transformation(&mut self, _transformation: Transformation) {}

    fn end_transformation(&mut self) {}

    fn fill_quad(&mut self, quad: renderer::Quad, _background: impl Into<Background>) {
        self.fills.push(quad.bounds);
    }

    fn reset(&mut self, _new_bounds: Rectangle) {
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

fn test_area(
    track_hover: bool,
    hover_messages: Option<(TestMessage, TestMessage)>,
) -> DropArea<'static, TestMessage, Theme, RecordingRenderer> {
    DropArea::new(TestContent::default().into(), TestMessage::Dropped, hover_messages, track_hover)
}

fn tree_for_drop_area() -> widget::Tree {
    let mut tree = widget::Tree::empty();
    tree.children.push(widget::Tree::empty());
    tree
}

fn layout_for(bounds: Rectangle) -> layout::Node {
    layout::Node::new(bounds.size()).move_to(Point::new(bounds.x, bounds.y))
}

fn update_with(
    area: &mut DropArea<'static, TestMessage, Theme, RecordingRenderer>,
    tree: &mut widget::Tree,
    event: Event,
    cursor: mouse::Cursor,
) -> Vec<TestMessage> {
    let renderer = RecordingRenderer::default();
    let mut clipboard = iced::advanced::clipboard::Null;
    let node = layout_for(Rectangle { x: 10.0, y: 20.0, width: 120.0, height: 40.0 });
    let mut messages = Vec::new();
    {
        let mut shell = Shell::new(&mut messages);

        <DropArea<'_, TestMessage, Theme, RecordingRenderer> as Widget<
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
    }

    messages
}

#[test]
fn drop_area_new_wraps_content_and_exposes_child_tree() {
    let content: Element<'static, u8> = text("drop").into();
    let area = DropArea::new(content, 7, Some((1, 2)), true);

    let children = area.children();
    assert_eq!(children.len(), 1);
}

#[test]
fn drop_area_size_delegates_to_content() {
    let content: Element<'static, u8> =
        iced::widget::container(text("sized")).width(Length::Fixed(123.0)).into();
    let area = DropArea::new(content, 9, None, false);

    let size = area.size();
    assert!(matches!(size.width, Length::Fixed(width) if width == 123.0));
}

#[test]
fn drop_area_can_be_wrapped_as_iced_element_and_app_alias() {
    let content: Element<'static, u8> = text("element").into();
    let _: Element<'static, u8> = Element::new(DropArea::new(content, 1, None, false));

    let app_content: Element<'static, Message> = text("app element").into();
    let _: DropAreaElement<'static> = DropArea::new(app_content, Message::None, None, false);
}

#[test]
fn drop_area_diff_keeps_single_child_tree() {
    let area = test_area(false, None);
    let mut tree = tree_for_drop_area();

    <DropArea<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::diff(&area, &mut tree);

    assert_eq!(tree.children.len(), 1);
}

#[test]
fn drop_area_layout_draw_and_mouse_interaction_delegate_to_content() {
    let mut area = DropArea::new(
        TestContent { width: 150.0, height: 32.0, interaction: mouse::Interaction::Grabbing }
            .into(),
        TestMessage::Dropped,
        None,
        false,
    );
    let mut tree = tree_for_drop_area();
    let mut renderer = RecordingRenderer::default();
    let limits = layout::Limits::new(Size::ZERO, Size::new(300.0, 200.0));

    let node = <DropArea<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::layout(&mut area, &mut tree, &renderer, &limits);
    let layout = Layout::new(&node);

    assert_eq!(node.size(), Size::new(150.0, 32.0));

    <DropArea<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::draw(
        &area,
        &tree,
        &mut renderer,
        &Theme::Dark,
        &renderer::Style::default(),
        layout,
        mouse::Cursor::Unavailable,
        &Rectangle::INFINITE,
    );

    let interaction = <DropArea<'_, TestMessage, Theme, RecordingRenderer> as Widget<
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

    assert_eq!(renderer.fills, vec![Rectangle::with_size(Size::new(150.0, 32.0))]);
    assert_eq!(interaction, mouse::Interaction::Grabbing);
}

#[test]
fn drop_area_update_publishes_hover_drop_and_delegates_to_content() {
    let mut area = test_area(true, Some((TestMessage::Hovered, TestMessage::NotHovered)));
    let mut tree = tree_for_drop_area();

    let messages = update_with(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        mouse::Cursor::Available(Point::new(20.0, 30.0)),
    );

    assert_eq!(
        messages,
        vec![TestMessage::Hovered, TestMessage::Dropped, TestMessage::ChildUpdated]
    );
}

#[test]
fn drop_area_update_reports_not_hovered_without_drop_when_cursor_is_outside() {
    let mut area = test_area(true, Some((TestMessage::Hovered, TestMessage::NotHovered)));
    let mut tree = tree_for_drop_area();

    let messages = update_with(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::CursorMoved { position: Point::new(500.0, 500.0) }),
        mouse::Cursor::Available(Point::new(500.0, 500.0)),
    );

    assert_eq!(messages, vec![TestMessage::NotHovered, TestMessage::ChildUpdated]);
}

#[test]
fn drop_area_update_reports_hover_on_button_press_without_drop() {
    let mut area = test_area(true, Some((TestMessage::Hovered, TestMessage::NotHovered)));
    let mut tree = tree_for_drop_area();

    let messages = update_with(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        mouse::Cursor::Available(Point::new(20.0, 30.0)),
    );

    assert_eq!(messages, vec![TestMessage::Hovered, TestMessage::ChildUpdated]);
}

#[test]
fn drop_area_update_reports_hover_but_rejects_drop_for_non_left_release() {
    let mut area = test_area(true, Some((TestMessage::Hovered, TestMessage::NotHovered)));
    let mut tree = tree_for_drop_area();

    let messages = update_with(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Right)),
        mouse::Cursor::Available(Point::new(20.0, 30.0)),
    );

    assert_eq!(messages, vec![TestMessage::Hovered, TestMessage::ChildUpdated]);
}

#[test]
fn drop_area_update_ignores_hover_tracking_when_disabled() {
    let mut area = test_area(false, Some((TestMessage::Hovered, TestMessage::NotHovered)));
    let mut tree = tree_for_drop_area();

    let messages = update_with(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        mouse::Cursor::Available(Point::new(20.0, 30.0)),
    );

    assert_eq!(messages, vec![TestMessage::Dropped, TestMessage::ChildUpdated]);
}

#[test]
fn drop_area_update_handles_file_hover_inside_bounds() {
    let mut area = test_area(true, Some((TestMessage::Hovered, TestMessage::NotHovered)));
    let mut tree = tree_for_drop_area();

    let messages = update_with(
        &mut area,
        &mut tree,
        Event::Window(iced::window::Event::FileHovered("hover.txt".into())),
        mouse::Cursor::Available(Point::new(20.0, 30.0)),
    );

    assert_eq!(messages, vec![TestMessage::Hovered, TestMessage::ChildUpdated]);
}

#[test]
fn drop_area_update_handles_file_drop_inside_bounds() {
    let mut area = test_area(true, Some((TestMessage::Hovered, TestMessage::NotHovered)));
    let mut tree = tree_for_drop_area();

    let messages = update_with(
        &mut area,
        &mut tree,
        Event::Window(iced::window::Event::FileDropped("drop.txt".into())),
        mouse::Cursor::Available(Point::new(20.0, 30.0)),
    );

    assert_eq!(
        messages,
        vec![TestMessage::Hovered, TestMessage::Dropped, TestMessage::ChildUpdated]
    );
}

#[test]
fn drop_area_update_reports_not_hovered_when_files_leave_without_cursor() {
    let mut area = test_area(true, Some((TestMessage::Hovered, TestMessage::NotHovered)));
    let mut tree = tree_for_drop_area();

    let messages = update_with(
        &mut area,
        &mut tree,
        Event::Window(iced::window::Event::FilesHoveredLeft),
        mouse::Cursor::Unavailable,
    );

    assert_eq!(messages, vec![TestMessage::NotHovered, TestMessage::ChildUpdated]);
}

#[test]
fn drop_area_overlay_delegates_to_content() {
    let mut area = test_area(false, None);
    let mut tree = tree_for_drop_area();
    let renderer = RecordingRenderer::default();
    let node = layout_for(Rectangle { x: 0.0, y: 0.0, width: 120.0, height: 40.0 });

    let overlay = <DropArea<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::overlay(
        &mut area,
        &mut tree,
        Layout::new(&node),
        &renderer,
        &Rectangle::INFINITE,
        iced::Vector::default(),
    );

    assert!(overlay.is_none());
}
