use super::*;
use iced::advanced::{Clipboard, Layout, Renderer, Shell, Widget, layout, renderer, widget};
use iced::{Background, Length, Point, Transformation};

#[derive(Debug, Clone, PartialEq)]
enum TestMessage {
    ChildUpdated,
    WheelScrolled(mouse::ScrollDelta),
    PanelWheelScrolled(mouse::ScrollDelta, f32),
    ScrollbarChanged(f32, f32),
}

#[derive(Debug, Clone, Copy)]
struct TestContent {
    width: f32,
    height: f32,
    interaction: mouse::Interaction,
}

impl Default for TestContent {
    fn default() -> Self {
        Self { width: 120.0, height: 40.0, interaction: mouse::Interaction::Text }
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

fn interceptor() -> WheelInterceptor<'static, TestMessage, Theme, RecordingRenderer> {
    WheelInterceptor {
        content: TestContent::default().into(),
        on_scroll: Some(Box::new(TestMessage::WheelScrolled)),
    }
}

fn passthrough_interceptor() -> WheelInterceptor<'static, TestMessage, Theme, RecordingRenderer> {
    WheelInterceptor { content: TestContent::default().into(), on_scroll: None }
}

fn tree_for(
    _widget: &WheelInterceptor<'static, TestMessage, Theme, RecordingRenderer>,
) -> widget::Tree {
    let mut tree = widget::Tree::empty();
    tree.children.push(widget::Tree::empty());
    tree
}

fn layout_for(bounds: Rectangle) -> layout::Node {
    layout::Node::new(bounds.size()).move_to(Point::new(bounds.x, bounds.y))
}

fn update_with(
    widget: &mut WheelInterceptor<'static, TestMessage, Theme, RecordingRenderer>,
    tree: &mut widget::Tree,
    event: Event,
    cursor: mouse::Cursor,
) -> (Vec<TestMessage>, bool) {
    let renderer = RecordingRenderer::default();
    let mut clipboard = iced::advanced::clipboard::Null;
    let node = layout_for(Rectangle { x: 10.0, y: 20.0, width: 120.0, height: 40.0 });
    let mut messages = Vec::new();
    let mut shell = Shell::new(&mut messages);

    <WheelInterceptor<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::update(
        widget,
        tree,
        &event,
        Layout::new(&node),
        cursor,
        &renderer,
        &mut clipboard,
        &mut shell,
        &Rectangle::INFINITE,
    );

    let captured = shell.is_event_captured();
    drop(shell);
    (messages, captured)
}

#[test]
fn scroll_panel_state_clamps_viewport_lines_and_scroll() {
    let state = scroll_panel_state(
        Size::new(300.0, 100.0),
        TextEditorScrollPanelMetrics {
            viewport_padding: 20.0,
            line_height: 10.0,
            line_count: 20,
            scroll_top_line: 50.0,
        },
    );

    assert_eq!(
        state,
        ScrollPanelState { viewport_height: 80.0, max_scroll: 12.0, scroll_top_line: 12.0 }
    );
}

#[test]
fn scroll_panel_state_keeps_degenerate_values_interactive() {
    let state = scroll_panel_state(
        Size::new(300.0, 0.0),
        TextEditorScrollPanelMetrics {
            viewport_padding: 20.0,
            line_height: 0.0,
            line_count: 0,
            scroll_top_line: -10.0,
        },
    );

    assert_eq!(
        state,
        ScrollPanelState { viewport_height: 1.0, max_scroll: 0.0, scroll_top_line: 0.0 }
    );
}

#[test]
fn scrollbar_style_uses_four_pixel_rail_and_dark_theme_colors() {
    let active = scrollbar_style(&Theme::Dark, iced::widget::vertical_slider::Status::Active);
    let hovered = scrollbar_style(&Theme::Dark, iced::widget::vertical_slider::Status::Hovered);
    let dragged = scrollbar_style(&Theme::Dark, iced::widget::vertical_slider::Status::Dragged);

    assert_eq!(active.rail.width, 4.0);
    assert_eq!(active.handle.border_width, 0.0);
    assert_ne!(active.handle.background, hovered.handle.background);
    assert_ne!(hovered.handle.background, dragged.handle.background);
}

#[test]
fn panel_style_uses_theme_background_and_border() {
    let style = panel_style(&Theme::Dark);

    assert!(style.background.is_some());
    assert_eq!(style.border.width, 1.0);
    assert_eq!(style.border.radius.top_left, 10.0);
}

#[test]
fn text_editor_scroll_panel_builds_without_scrollbar_when_content_fits() {
    let element = text_editor_scroll_panel(
        iced::widget::text("content"),
        Size::new(240.0, 100.0),
        TextEditorScrollPanelMetrics {
            viewport_padding: 0.0,
            line_height: 20.0,
            line_count: 2,
            scroll_top_line: 0.0,
        },
        TestMessage::PanelWheelScrolled,
        TestMessage::ScrollbarChanged,
    );

    assert_eq!(element.as_widget().size(), Size::new(Length::Fill, Length::Fill));
}

#[test]
fn text_editor_scroll_panel_builds_with_scrollbar_when_content_overflows() {
    let element = text_editor_scroll_panel(
        iced::widget::text("content"),
        Size::new(240.0, 100.0),
        TextEditorScrollPanelMetrics {
            viewport_padding: 20.0,
            line_height: 10.0,
            line_count: 30,
            scroll_top_line: 3.0,
        },
        TestMessage::PanelWheelScrolled,
        TestMessage::ScrollbarChanged,
    );

    assert_eq!(element.as_widget().size(), Size::new(Length::Fill, Length::Fill));
}

#[test]
fn wheel_interceptor_converts_into_element() {
    let element = wheel_interceptor(TestContent::default(), TestMessage::WheelScrolled);

    assert_eq!(element.as_widget().size(), Size::new(Length::Fixed(120.0), Length::Fixed(40.0)));
}

#[test]
fn wheel_passthrough_converts_into_element() {
    let element = pass_wheel_to_parent(TestContent::default());

    assert_eq!(element.as_widget().size(), Size::new(Length::Fixed(120.0), Length::Fixed(40.0)));
}

#[test]
fn children_and_diff_keep_single_child() {
    let widget = interceptor();
    let children = <WheelInterceptor<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::children(&widget);
    let mut tree = tree_for(&widget);

    <WheelInterceptor<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::diff(&widget, &mut tree);

    assert_eq!(children.len(), 1);
    assert_eq!(tree.children.len(), 1);
}

#[test]
fn size_delegates_to_content() {
    let widget = interceptor();

    let size = <WheelInterceptor<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::size(&widget);

    assert_eq!(size, Size::new(Length::Fixed(120.0), Length::Fixed(40.0)));
}

#[test]
fn layout_delegates_to_content() {
    let mut widget = interceptor();
    let mut tree = tree_for(&widget);
    let renderer = RecordingRenderer::default();

    let node = <WheelInterceptor<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::layout(
        &mut widget,
        &mut tree,
        &renderer,
        &layout::Limits::new(Size::ZERO, Size::new(400.0, 200.0)),
    );

    assert_eq!(node.size(), Size::new(120.0, 40.0));
}

#[test]
fn operate_delegates_to_content() {
    let mut widget = interceptor();
    let mut tree = tree_for(&widget);
    let renderer = RecordingRenderer::default();
    let node = layout_for(Rectangle { x: 4.0, y: 8.0, width: 120.0, height: 40.0 });
    let mut operation = RecordingOperation::default();

    <WheelInterceptor<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::operate(&mut widget, &mut tree, Layout::new(&node), &renderer, &mut operation);

    assert_eq!(
        operation.containers,
        vec![Rectangle { x: 4.0, y: 8.0, width: 120.0, height: 40.0 }]
    );
}

#[test]
fn wheel_inside_publishes_scroll_message_and_captures_event() {
    let mut widget = interceptor();
    let mut tree = tree_for(&widget);
    let delta = mouse::ScrollDelta::Lines { x: 1.0, y: -2.0 };

    let (messages, captured) = update_with(
        &mut widget,
        &mut tree,
        Event::Mouse(mouse::Event::WheelScrolled { delta }),
        mouse::Cursor::Available(Point::new(34.0, 47.0)),
    );

    assert_eq!(messages, vec![TestMessage::WheelScrolled(delta)]);
    assert!(captured);
}

#[test]
fn wheel_passthrough_leaves_pixel_scroll_for_parent() {
    let mut widget = passthrough_interceptor();
    let mut tree = tree_for(&widget);

    let (messages, captured) = update_with(
        &mut widget,
        &mut tree,
        Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Pixels { x: 0.0, y: 8.0 },
        }),
        mouse::Cursor::Available(Point::new(34.0, 47.0)),
    );

    assert!(messages.is_empty());
    assert!(!captured);
}

#[test]
fn wheel_outside_delegates_to_content() {
    let mut widget = interceptor();
    let mut tree = tree_for(&widget);

    let (messages, captured) = update_with(
        &mut widget,
        &mut tree,
        Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Pixels { x: 0.0, y: 8.0 },
        }),
        mouse::Cursor::Available(Point::new(500.0, 47.0)),
    );

    assert_eq!(messages, vec![TestMessage::ChildUpdated]);
    assert!(!captured);
}

#[test]
fn non_wheel_event_delegates_to_content() {
    let mut widget = interceptor();
    let mut tree = tree_for(&widget);

    let (messages, captured) = update_with(
        &mut widget,
        &mut tree,
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        mouse::Cursor::Available(Point::new(34.0, 47.0)),
    );

    assert_eq!(messages, vec![TestMessage::ChildUpdated]);
    assert!(!captured);
}

#[test]
fn draw_delegates_to_content() {
    let widget = interceptor();
    let tree = tree_for(&widget);
    let mut renderer = RecordingRenderer::default();
    let node = layout_for(Rectangle { x: 4.0, y: 8.0, width: 120.0, height: 40.0 });

    <WheelInterceptor<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::draw(
        &widget,
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
fn mouse_interaction_delegates_to_content() {
    let widget = WheelInterceptor {
        content: TestContent { interaction: mouse::Interaction::Pointer, ..TestContent::default() }
            .into(),
        on_scroll: Some(Box::new(TestMessage::WheelScrolled)),
    };
    let tree = tree_for(&widget);
    let renderer = RecordingRenderer::default();
    let node = layout_for(Rectangle { x: 0.0, y: 0.0, width: 120.0, height: 40.0 });

    let interaction = <WheelInterceptor<'_, TestMessage, Theme, RecordingRenderer> as Widget<
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

    assert_eq!(interaction, mouse::Interaction::Pointer);
}

#[test]
fn overlay_delegates_to_content_none() {
    let mut widget = interceptor();
    let mut tree = tree_for(&widget);
    let renderer = RecordingRenderer::default();
    let node = layout_for(Rectangle { x: 0.0, y: 0.0, width: 120.0, height: 40.0 });

    let overlay = <WheelInterceptor<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::overlay(
        &mut widget,
        &mut tree,
        Layout::new(&node),
        &renderer,
        &Rectangle::INFINITE,
        Vector::new(0.0, 0.0),
    );

    assert!(overlay.is_none());
}
