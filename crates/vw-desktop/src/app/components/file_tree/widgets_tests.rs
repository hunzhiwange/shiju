use super::widgets::RightClickArea;
use iced::advanced::{Clipboard, Layout, Renderer, Shell, Widget, layout, mouse, renderer, widget};
use iced::{Background, Element, Event, Length, Point, Rectangle, Size, Theme, Transformation};

#[derive(Debug, Clone, PartialEq)]
enum TestMessage {
    ChildUpdated,
    RightClicked(Point),
    DragStarted,
    DragEnded,
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
    assert_eq!(released, vec![TestMessage::DragEnded, TestMessage::ChildUpdated]);
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
