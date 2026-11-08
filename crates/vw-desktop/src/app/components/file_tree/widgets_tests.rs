//! 验证文件树右键区域对布局绘制、子组件事件和拖动生命周期的委托。
//! 记录型内容与渲染器区分包装器副作用和子组件行为，并覆盖阈值前后的拖动状态。

use super::widgets::RightClickArea;
use iced::advanced::{Clipboard, Layout, Renderer, Shell, Widget, layout, mouse, renderer, widget};
use iced::{Background, Element, Event, Length, Point, Rectangle, Size, Theme, Transformation};

#[derive(Debug, Clone, PartialEq)]
enum TestMessage {
    ChildUpdated,
    RightClicked(Point),
    DragStarted,
    DragEnded,
    TargetChanged(u8),
    Requested(u8),
    Cancelled,
}

// 可观测子组件始终发布更新消息，便于区分包装器消费事件与继续向下传播。
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

// 渲染记录只保留包装器测试所需的边界、层和变换信息。
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

type TestArea<'a> = RightClickArea<'a, TestMessage, Theme, RecordingRenderer>;

fn area_with_drag_messages(
    on_left_press: Option<TestMessage>,
    on_left_release: Option<TestMessage>,
) -> TestArea<'static> {
    RightClickArea::new(
        TestContent::default().into(),
        Box::new(TestMessage::RightClicked),
        on_left_press,
        on_left_release,
    )
}

fn area() -> TestArea<'static> {
    area_with_drag_messages(Some(TestMessage::DragStarted), Some(TestMessage::DragEnded))
}

fn tree_for_area(area: &TestArea<'_>) -> widget::Tree {
    widget::Tree::new(area as &dyn Widget<TestMessage, Theme, RecordingRenderer>)
}

fn layout_for(bounds: Rectangle) -> layout::Node {
    layout::Node::new(bounds.size()).move_to(Point::new(bounds.x, bounds.y))
}

// 所有指针事件经同一固定布局派发，避免各测试自行组装时产生坐标差异。
fn update_area(
    area: &mut TestArea<'static>,
    tree: &mut widget::Tree,
    event: Event,
    cursor: mouse::Cursor,
) -> Vec<TestMessage> {
    let renderer = RecordingRenderer::default();
    let mut clipboard = iced::advanced::clipboard::Null;
    let node = layout_for(Rectangle { x: 10.0, y: 20.0, width: 120.0, height: 40.0 });
    let mut messages = Vec::new();
    let mut shell = Shell::new(&mut messages);

    <TestArea<'_> as Widget<TestMessage, Theme, RecordingRenderer>>::update(
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
fn file_tree_surface_preserves_nested_context_menu_overlay() {
    use crate::app::components::overlays::PointBelowOverlay;

    let menu = PointBelowOverlay::new(Element::new(area()), TestContent::default())
        .show(true)
        .anchor(Point::new(12.0, 8.0));
    let mut surface =
        RightClickArea::new(menu.into(), Box::new(TestMessage::RightClicked), None, None);
    let mut tree = tree_for_area(&surface);
    let renderer = RecordingRenderer::default();
    let node = surface.layout(
        &mut tree,
        &renderer,
        &layout::Limits::new(Size::ZERO, Size::new(400.0, 200.0)),
    );

    assert!(
        surface
            .overlay(
                &mut tree,
                Layout::new(&node),
                &renderer,
                &Rectangle::INFINITE,
                iced::Vector::ZERO,
            )
            .is_some()
    );
}

#[test]
fn right_click_area_exposes_single_child_state_and_content_size() {
    let area = area();
    let state = area.state();
    let children = area.children();
    let tree = tree_for_area(&area);

    let _ = state;
    assert_eq!(children.len(), 1);
    assert_eq!(tree.children.len(), 1);
    assert_eq!(area.size(), Size::new(Length::Fixed(120.0), Length::Fixed(40.0)));
}

#[test]
fn right_click_area_diff_keeps_single_wrapped_child() {
    let area = area();
    let mut tree = tree_for_area(&area);

    <TestArea<'_> as Widget<TestMessage, Theme, RecordingRenderer>>::diff(&area, &mut tree);

    assert_eq!(tree.children.len(), 1);
}

#[test]
fn right_click_area_layout_and_draw_delegate_to_content() {
    let mut area = area();
    let mut tree = tree_for_area(&area);
    let mut renderer = RecordingRenderer::default();
    let node = <TestArea<'_> as Widget<TestMessage, Theme, RecordingRenderer>>::layout(
        &mut area,
        &mut tree,
        &renderer,
        &layout::Limits::new(Size::ZERO, Size::new(400.0, 200.0)),
    );

    assert_eq!(node.size(), Size::new(120.0, 40.0));

    <TestArea<'_> as Widget<TestMessage, Theme, RecordingRenderer>>::draw(
        &area,
        &tree,
        &mut renderer,
        &Theme::Dark,
        &renderer::Style::default(),
        Layout::new(&node),
        mouse::Cursor::Unavailable,
        &Rectangle::INFINITE,
    );

    assert_eq!(renderer.fills, vec![Rectangle { x: 0.0, y: 0.0, width: 120.0, height: 40.0 }]);
}

#[test]
fn right_click_area_mouse_interaction_delegates_to_content() {
    let area = RightClickArea::new(
        TestContent { interaction: mouse::Interaction::Text, ..TestContent::default() }.into(),
        Box::new(TestMessage::RightClicked),
        None,
        None,
    );
    let tree = tree_for_area(&area);
    let renderer = RecordingRenderer::default();
    let node = layout_for(Rectangle { x: 0.0, y: 0.0, width: 120.0, height: 40.0 });

    let interaction =
        <TestArea<'_> as Widget<TestMessage, Theme, RecordingRenderer>>::mouse_interaction(
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
fn right_click_inside_publishes_local_position_before_child_update() {
    let mut area = area();
    let mut tree = tree_for_area(&area);

    let messages = update_area(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)),
        mouse::Cursor::Available(Point::new(34.0, 47.0)),
    );

    assert_eq!(
        messages,
        vec![TestMessage::RightClicked(Point::new(24.0, 27.0)), TestMessage::ChildUpdated]
    );
}

#[test]
fn right_click_outside_or_without_cursor_only_reaches_child() {
    let mut area = area();
    let mut tree = tree_for_area(&area);

    let outside = update_area(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)),
        mouse::Cursor::Available(Point::new(500.0, 47.0)),
    );
    let unavailable = update_area(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)),
        mouse::Cursor::Unavailable,
    );

    assert_eq!(outside, vec![TestMessage::ChildUpdated]);
    assert_eq!(unavailable, vec![TestMessage::ChildUpdated]);
}

#[test]
fn drag_starts_at_threshold_once_and_release_ends_and_resets_it() {
    let mut area = area();
    let mut tree = tree_for_area(&area);

    let pressed = update_area(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        mouse::Cursor::Available(Point::new(12.0, 22.0)),
    );
    let threshold = update_area(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::CursorMoved { position: Point::new(15.0, 22.0) }),
        mouse::Cursor::Available(Point::new(15.0, 22.0)),
    );
    let continued = update_area(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::CursorMoved { position: Point::new(18.0, 22.0) }),
        mouse::Cursor::Available(Point::new(18.0, 22.0)),
    );
    let released = update_area(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        mouse::Cursor::Available(Point::new(18.0, 22.0)),
    );
    let after_release = update_area(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::CursorMoved { position: Point::new(30.0, 22.0) }),
        mouse::Cursor::Available(Point::new(30.0, 22.0)),
    );

    assert_eq!(pressed, vec![TestMessage::ChildUpdated]);
    assert_eq!(threshold, vec![TestMessage::DragStarted, TestMessage::ChildUpdated]);
    assert_eq!(continued, vec![TestMessage::ChildUpdated]);
    assert_eq!(released, vec![TestMessage::DragEnded]);
    assert_eq!(after_release, vec![TestMessage::ChildUpdated]);
}

#[test]
fn motion_below_drag_threshold_does_not_publish_drag_messages() {
    let mut area = area();
    let mut tree = tree_for_area(&area);

    update_area(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        mouse::Cursor::Available(Point::new(12.0, 22.0)),
    );
    let moved = update_area(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::CursorMoved { position: Point::new(14.0, 22.0) }),
        mouse::Cursor::Available(Point::new(14.0, 22.0)),
    );
    let released = update_area(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        mouse::Cursor::Available(Point::new(14.0, 22.0)),
    );

    assert_eq!(moved, vec![TestMessage::ChildUpdated]);
    assert_eq!(released, vec![TestMessage::ChildUpdated]);
}

#[test]
fn non_left_or_outside_or_cursorless_press_cannot_start_drag() {
    let mut area = area();
    let mut tree = tree_for_area(&area);

    update_area(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)),
        mouse::Cursor::Available(Point::new(12.0, 22.0)),
    );
    let after_right = update_area(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::CursorMoved { position: Point::new(20.0, 22.0) }),
        mouse::Cursor::Available(Point::new(20.0, 22.0)),
    );
    update_area(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::CursorMoved { position: Point::new(500.0, 22.0) }),
        mouse::Cursor::Available(Point::new(500.0, 22.0)),
    );
    update_area(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        mouse::Cursor::Available(Point::new(500.0, 22.0)),
    );
    let after_outside = update_area(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::CursorMoved { position: Point::new(520.0, 22.0) }),
        mouse::Cursor::Available(Point::new(520.0, 22.0)),
    );
    update_area(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::CursorLeft),
        mouse::Cursor::Unavailable,
    );
    update_area(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        mouse::Cursor::Unavailable,
    );
    let after_cursorless = update_area(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::CursorMoved { position: Point::new(20.0, 22.0) }),
        mouse::Cursor::Available(Point::new(20.0, 22.0)),
    );

    assert_eq!(after_right, vec![TestMessage::ChildUpdated]);
    assert_eq!(after_outside, vec![TestMessage::ChildUpdated]);
    assert_eq!(after_cursorless, vec![TestMessage::ChildUpdated]);
}

#[test]
fn missing_drag_callbacks_still_tracks_drag_without_extra_messages() {
    let mut area = area_with_drag_messages(None, None);
    let mut tree = tree_for_area(&area);

    update_area(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        mouse::Cursor::Available(Point::new(12.0, 22.0)),
    );
    let moved = update_area(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::CursorMoved { position: Point::new(15.0, 22.0) }),
        mouse::Cursor::Available(Point::new(15.0, 22.0)),
    );
    let released = update_area(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        mouse::Cursor::Available(Point::new(15.0, 22.0)),
    );

    assert_eq!(moved, vec![TestMessage::ChildUpdated]);
    assert_eq!(released, vec![TestMessage::ChildUpdated]);
}

#[test]
fn file_tree_drop_drag_release_does_not_reach_source_button() {
    let mut area = area();
    let mut tree = tree_for_area(&area);
    let cursor = mouse::Cursor::Available(Point::new(12.0, 22.0));
    update_area(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        cursor,
    );
    update_area(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::CursorMoved { position: Point::new(15.0, 22.0) }),
        mouse::Cursor::Available(Point::new(15.0, 22.0)),
    );
    assert_eq!(
        update_area(
            &mut area,
            &mut tree,
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            cursor
        ),
        vec![TestMessage::DragEnded]
    );
}

fn drop_area(target: u8) -> TestArea<'static> {
    RightClickArea::new(
        TestContent::default().into(),
        Box::new(TestMessage::RightClicked),
        None,
        None,
    )
    .file_tree_drop(TestMessage::TargetChanged(target), TestMessage::Requested(target))
}

fn update_drop(
    area: &mut TestArea<'static>,
    tree: &mut widget::Tree,
    event: Event,
    point: Point,
    bounds: Rectangle,
    viewport: Rectangle,
) -> Vec<TestMessage> {
    let renderer = RecordingRenderer::default();
    let mut clipboard = iced::advanced::clipboard::Null;
    let node = layout_for(bounds);
    let mut messages = Vec::new();
    area.update(
        tree,
        &event,
        Layout::new(&node),
        mouse::Cursor::Available(point),
        &renderer,
        &mut clipboard,
        &mut Shell::new(&mut messages),
        &viewport,
    );
    messages
}

#[test]
fn file_tree_drop_scroll_viewport_clips_target_and_scrollbar() {
    let mut area = drop_area(1);
    let mut tree = tree_for_area(&area);
    let bounds = Rectangle::new(Point::ORIGIN, Size::new(120.0, 40.0));
    let viewport = Rectangle::new(Point::new(7.0, 20.0), Size::new(106.0, 20.0));
    for point in [Point::new(10.0, 10.0), Point::new(116.0, 30.0)] {
        let messages = update_drop(
            &mut area,
            &mut tree,
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            point,
            bounds,
            viewport,
        );
        assert!(!messages.iter().any(|message| matches!(message, TestMessage::Requested(_))));
    }
    let messages = update_drop(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        Point::new(10.0, 30.0),
        bounds,
        viewport,
    );
    assert_eq!(messages, vec![TestMessage::ChildUpdated, TestMessage::Requested(1)]);
}

#[test]
fn file_tree_drop_nested_child_precedes_parent_on_release_and_hover() {
    let child = drop_area(2);
    let mut parent =
        RightClickArea::new(Element::new(child), Box::new(TestMessage::RightClicked), None, None)
            .file_tree_drop(TestMessage::TargetChanged(1), TestMessage::Requested(1));
    let mut tree = tree_for_area(&parent);
    let messages = update_area(
        &mut parent,
        &mut tree,
        Event::Mouse(mouse::Event::CursorMoved { position: Point::new(20.0, 30.0) }),
        mouse::Cursor::Available(Point::new(20.0, 30.0)),
    );
    assert_eq!(messages, vec![TestMessage::ChildUpdated, TestMessage::TargetChanged(2)]);
    let messages = update_area(
        &mut parent,
        &mut tree,
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        mouse::Cursor::Available(Point::new(20.0, 30.0)),
    );
    assert_eq!(messages, vec![TestMessage::ChildUpdated, TestMessage::Requested(2)]);
}

#[test]
fn file_tree_drop_root_strip_does_not_accept_file_row_or_blank_below_it() {
    let mut area = drop_area(0);
    let mut tree = tree_for_area(&area);
    let bounds = Rectangle::new(Point::new(7.0, 0.0), Size::new(106.0, 24.0));
    let messages = update_drop(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        Point::new(12.0, 25.0),
        bounds,
        Rectangle::INFINITE,
    );
    assert_eq!(messages, vec![TestMessage::ChildUpdated]);
    let messages = update_drop(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        Point::new(12.0, 12.0),
        bounds,
        Rectangle::INFINITE,
    );
    assert_eq!(messages, vec![TestMessage::ChildUpdated, TestMessage::Requested(0)]);
}

#[test]
fn file_tree_drop_surface_clears_hover_before_child_hit() {
    let mut area = RightClickArea::new(
        Element::new(drop_area(2)),
        Box::new(TestMessage::RightClicked),
        None,
        None,
    )
    .file_tree_surface(TestMessage::TargetChanged(0), TestMessage::Cancelled);
    let mut tree = tree_for_area(&area);
    assert_eq!(
        update_area(
            &mut area,
            &mut tree,
            Event::Mouse(mouse::Event::CursorMoved { position: Point::new(20.0, 30.0) }),
            mouse::Cursor::Available(Point::new(20.0, 30.0))
        ),
        vec![
            TestMessage::TargetChanged(0),
            TestMessage::ChildUpdated,
            TestMessage::TargetChanged(2)
        ]
    );
}

#[test]
fn file_tree_drop_unfocus_cancels_drag_without_later_release() {
    let mut area = area().on_drag_cancel(TestMessage::Cancelled);
    let mut tree = tree_for_area(&area);
    let cursor = mouse::Cursor::Available(Point::new(12.0, 22.0));
    update_area(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        cursor,
    );
    update_area(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::CursorMoved { position: Point::new(15.0, 22.0) }),
        mouse::Cursor::Available(Point::new(15.0, 22.0)),
    );
    assert_eq!(
        update_area(&mut area, &mut tree, Event::Window(iced::window::Event::Unfocused), cursor),
        vec![TestMessage::Cancelled, TestMessage::ChildUpdated]
    );
    assert_eq!(
        update_area(
            &mut area,
            &mut tree,
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            cursor
        ),
        vec![TestMessage::ChildUpdated]
    );
}

#[test]
fn file_tree_drop_three_pixel_threshold_preserves_ordinary_click() {
    let mut area = area();
    let mut tree = tree_for_area(&area);
    let cursor = mouse::Cursor::Available(Point::new(12.0, 22.0));
    update_area(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        cursor,
    );
    assert_eq!(
        update_area(
            &mut area,
            &mut tree,
            Event::Mouse(mouse::Event::CursorMoved { position: Point::new(14.9, 22.0) }),
            mouse::Cursor::Available(Point::new(14.9, 22.0))
        ),
        vec![TestMessage::ChildUpdated]
    );
    assert_eq!(
        update_area(
            &mut area,
            &mut tree,
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            cursor
        ),
        vec![TestMessage::ChildUpdated]
    );
    update_area(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::CursorMoved { position: Point::new(12.0, 22.0) }),
        cursor,
    );
    update_area(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        cursor,
    );
    assert!(
        update_area(
            &mut area,
            &mut tree,
            Event::Mouse(mouse::Event::CursorMoved { position: Point::new(15.0, 22.0) }),
            mouse::Cursor::Available(Point::new(15.0, 22.0))
        )
        .contains(&TestMessage::DragStarted)
    );
}

#[test]
fn file_tree_drop_release_is_consumed_once() {
    let mut area = drop_area(1);
    let mut tree = tree_for_area(&area);
    let cursor = mouse::Cursor::Available(Point::new(20.0, 30.0));
    let release = Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left));
    assert!(
        update_area(&mut area, &mut tree, release.clone(), cursor)
            .contains(&TestMessage::Requested(1))
    );
    assert_eq!(update_area(&mut area, &mut tree, release, cursor), vec![TestMessage::ChildUpdated]);
}

#[test]
fn file_tree_drop_offscreen_press_cannot_start_drag() {
    let mut area = area();
    let mut tree = tree_for_area(&area);
    let bounds = Rectangle::new(Point::ORIGIN, Size::new(120.0, 40.0));
    let viewport = Rectangle::new(Point::new(0.0, 20.0), Size::new(120.0, 20.0));
    update_drop(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        Point::new(10.0, 10.0),
        bounds,
        viewport,
    );
    let messages = update_drop(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::CursorMoved { position: Point::new(20.0, 25.0) }),
        Point::new(20.0, 25.0),
        bounds,
        viewport,
    );
    assert_eq!(messages, vec![TestMessage::ChildUpdated]);
}

#[test]
fn file_tree_external_drop_each_native_callback_hits_visible_directory_without_hover() {
    let mut area = drop_area(1).external_file_drop(|path| {
        assert!(path.is_absolute());
        TestMessage::Requested(8)
    });
    let mut tree = tree_for_area(&area);
    let cursor = mouse::Cursor::Available(Point::new(20.0, 30.0));
    for path in ["/tmp/文件夹", "/tmp/second"] {
        let messages = update_area(
            &mut area,
            &mut tree,
            Event::Window(iced::window::Event::FileDropped(path.into())),
            cursor,
        );
        assert_eq!(messages, vec![TestMessage::ChildUpdated, TestMessage::Requested(8)]);
    }
    for cursor in [mouse::Cursor::Unavailable, mouse::Cursor::Available(Point::new(500.0, 500.0))] {
        assert_eq!(
            update_area(
                &mut area,
                &mut tree,
                Event::Window(iced::window::Event::FileDropped("/tmp/outside".into())),
                cursor
            ),
            vec![TestMessage::ChildUpdated]
        );
    }
}

#[test]
fn file_tree_external_drop_hover_does_not_submit_and_clipped_rows_do_not_receive() {
    let mut area = drop_area(1).external_file_drop(|_| TestMessage::Requested(8));
    let mut tree = tree_for_area(&area);
    assert_eq!(
        update_area(
            &mut area,
            &mut tree,
            Event::Window(iced::window::Event::FileHovered("/tmp/source".into())),
            mouse::Cursor::Available(Point::new(20.0, 30.0))
        ),
        vec![TestMessage::ChildUpdated]
    );
    let messages = update_drop(
        &mut area,
        &mut tree,
        Event::Window(iced::window::Event::FileDropped("/tmp/source".into())),
        Point::new(10.0, 10.0),
        Rectangle::new(Point::ORIGIN, Size::new(120.0, 40.0)),
        Rectangle::new(Point::new(0.0, 20.0), Size::new(120.0, 20.0)),
    );
    assert_eq!(messages, vec![TestMessage::ChildUpdated]);
}

#[test]
fn file_tree_external_drop_nested_target_consumes_once() {
    let child = drop_area(2).external_file_drop(|_| TestMessage::Requested(2));
    let mut parent: TestArea<'_> =
        RightClickArea::new(Element::new(child), Box::new(TestMessage::RightClicked), None, None)
            .external_file_drop(|_| TestMessage::Requested(1));
    let mut tree = tree_for_area(&parent);
    let messages = update_area(
        &mut parent,
        &mut tree,
        Event::Window(iced::window::Event::FileDropped("/tmp/source".into())),
        mouse::Cursor::Available(Point::new(20.0, 30.0)),
    );
    assert_eq!(messages, vec![TestMessage::ChildUpdated, TestMessage::Requested(2)]);
}

#[test]
fn file_tree_drop_batched_events_use_press_position_before_final_cursor() {
    let mut area =
        RightClickArea::new(Element::new(area()), Box::new(TestMessage::RightClicked), None, None)
            .track_window_cursor();
    let mut tree = tree_for_area(&area);
    let start = Point::new(20.0, 30.0);
    let end = Point::new(200.0, 90.0);
    let cursor = mouse::Cursor::Available(end);
    update_area(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::CursorMoved { position: start }),
        cursor,
    );
    update_area(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        cursor,
    );
    let moved = update_area(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::CursorMoved { position: end }),
        cursor,
    );
    assert!(moved.contains(&TestMessage::DragStarted));
    let released = update_area(
        &mut area,
        &mut tree,
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        cursor,
    );
    assert!(released.contains(&TestMessage::DragEnded));
}

#[test]
fn file_tree_click_after_scrolling_hits_the_visible_directory() {
    use iced::widget::{Space, button, column, scrollable};

    let clickable_row = |id| {
        Element::new(RightClickArea::new(
            button(Space::new().width(120).height(20))
                .padding(0)
                .on_press(TestMessage::Requested(id))
                .into(),
            Box::new(TestMessage::RightClicked),
            Some(TestMessage::DragStarted),
            Some(TestMessage::DragEnded),
        ))
    };
    let rows = column![clickable_row(1), clickable_row(2), clickable_row(3), clickable_row(4)];
    let scroll = scrollable(rows).height(40).anchor_bottom();
    let mut surface =
        RightClickArea::new(scroll.into(), Box::new(TestMessage::RightClicked), None, None)
            .track_window_cursor();
    let renderer = ();
    let mut tree = widget::Tree::new(&surface as &dyn Widget<TestMessage, Theme, ()>);
    let node = surface.layout(
        &mut tree,
        &renderer,
        &layout::Limits::new(Size::ZERO, Size::new(120.0, 40.0)),
    );
    let mut clipboard = iced::advanced::clipboard::Null;
    let mut messages = Vec::new();
    // Only rows 3 (directory) and 4 are visible. The final cursor is over row 4.
    for event in [
        mouse::Event::CursorMoved { position: Point::new(10.0, 10.0) },
        mouse::Event::ButtonPressed(mouse::Button::Left),
        mouse::Event::ButtonReleased(mouse::Button::Left),
        mouse::Event::CursorMoved { position: Point::new(10.0, 30.0) },
    ] {
        surface.update(
            &mut tree,
            &Event::Mouse(event),
            Layout::new(&node),
            mouse::Cursor::Available(Point::new(10.0, 30.0)),
            &renderer,
            &mut clipboard,
            &mut Shell::new(&mut messages),
            &Rectangle::INFINITE,
        );
    }
    assert_eq!(messages, vec![TestMessage::Requested(3)]);
}

#[test]
fn file_tree_refresh_discards_the_replaced_rows_press() {
    let mut old_row = area().for_row("tree-dir", "/project/old".into());
    let mut tree = tree_for_area(&old_row);
    update_area(
        &mut old_row,
        &mut tree,
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        mouse::Cursor::Available(Point::new(12.0, 22.0)),
    );

    let mut new_row = area().for_row("tree-file", "/project/new".into());
    tree.diff(&new_row as &dyn Widget<TestMessage, Theme, RecordingRenderer>);
    let moved = update_area(
        &mut new_row,
        &mut tree,
        Event::Mouse(mouse::Event::CursorMoved { position: Point::new(20.0, 22.0) }),
        mouse::Cursor::Available(Point::new(20.0, 22.0)),
    );
    assert_eq!(moved, vec![TestMessage::ChildUpdated]);
}
