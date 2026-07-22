use super::{AboveOverlay, PointAboveOverlay};
use iced::advanced::{
    Clipboard, Layout, Renderer as _, Shell, Widget, layout, mouse, overlay, renderer, widget,
};
use iced::widget::text;
use iced::{
    Background, Element, Event, Length, Point, Rectangle, Size, Theme, Transformation, Vector,
};

#[derive(Debug, Clone, PartialEq, Eq)]
enum TestMessage {
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
    has_overlay: bool,
}

impl TestContent {
    fn new(width: f32, height: f32) -> Self {
        Self {
            width,
            height,
            interaction: mouse::Interaction::None,
            publish_update: false,
            has_overlay: false,
        }
    }

    fn with_interaction(mut self, interaction: mouse::Interaction) -> Self {
        self.interaction = interaction;
        self
    }

    fn with_update(mut self) -> Self {
        self.publish_update = true;
        self
    }

    fn with_overlay(mut self) -> Self {
        self.has_overlay = true;
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
        if self.publish_update {
            shell.publish(TestMessage::ChildUpdated);
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

#[derive(Debug)]
struct TestOverlay;

impl overlay::Overlay<TestMessage, Theme, RecordingRenderer> for TestOverlay {
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

fn content(has_overlay: bool) -> TestContent {
    let content =
        TestContent::new(120.0, 40.0).with_interaction(mouse::Interaction::Pointer).with_update();

    if has_overlay { content.with_overlay() } else { content }
}

fn overlay_content() -> TestContent {
    TestContent::new(70.0, 20.0)
}

fn above_widget(
    content_has_overlay: bool,
) -> AboveOverlay<'static, TestMessage, Theme, RecordingRenderer> {
    AboveOverlay::new(content(content_has_overlay), overlay_content())
}

fn point_above_widget(
    content_has_overlay: bool,
) -> PointAboveOverlay<'static, TestMessage, Theme, RecordingRenderer> {
    PointAboveOverlay::new(content(content_has_overlay), overlay_content())
}

fn above_tree(
    overlay: &AboveOverlay<'static, TestMessage, Theme, RecordingRenderer>,
) -> widget::Tree {
    let mut tree = widget::Tree::empty();
    tree.children = <AboveOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::children(overlay);
    tree
}

fn point_above_tree(
    overlay: &PointAboveOverlay<'static, TestMessage, Theme, RecordingRenderer>,
) -> widget::Tree {
    let mut tree = widget::Tree::empty();
    tree.children = <PointAboveOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::children(overlay);
    tree
}

fn node_at(bounds: Rectangle) -> layout::Node {
    layout::Node::new(bounds.size()).move_to(Point::new(bounds.x, bounds.y))
}

fn layout_limits() -> layout::Limits {
    layout::Limits::new(Size::ZERO, Size::new(500.0, 400.0))
}

#[test]
fn above_overlay_reports_content_and_overlay_children() {
    let overlay = AboveOverlay::new(label("content"), label("overlay"));

    let children =
        <AboveOverlay<'_, TestMessage> as Widget<TestMessage, Theme, iced::Renderer>>::children(
            &overlay,
        );

    assert_eq!(children.len(), 2);
}

#[test]
fn point_above_overlay_reports_content_and_overlay_children() {
    let overlay = PointAboveOverlay::new(label("content"), label("overlay"));

    let children = <PointAboveOverlay<'_, TestMessage> as Widget<
        TestMessage,
        Theme,
        iced::Renderer,
    >>::children(&overlay);

    assert_eq!(children.len(), 2);
}

#[test]
fn above_overlay_diff_keeps_two_child_trees() {
    let mut tree = widget::Tree::empty();
    tree.children.push(widget::Tree::empty());
    tree.children.push(widget::Tree::empty());

    let replacement = AboveOverlay::new(label("new content"), label("new overlay"));
    <AboveOverlay<'_, TestMessage> as Widget<TestMessage, Theme, iced::Renderer>>::diff(
        &replacement,
        &mut tree,
    );

    assert_eq!(tree.children.len(), 2);
}

#[test]
fn above_widget_size_hint_and_layout_delegate_to_content() {
    let mut overlay = above_widget(false);
    let mut tree = above_tree(&overlay);
    let renderer = RecordingRenderer::default();

    <AboveOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::diff(&overlay, &mut tree);
    let size = <AboveOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::size(&overlay);
    let hint = <AboveOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::size_hint(&overlay);
    let node = <AboveOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::layout(&mut overlay, &mut tree, &renderer, &layout_limits());

    assert_eq!(tree.children.len(), 2);
    assert_eq!(size, Size::new(Length::Fixed(120.0), Length::Fixed(40.0)));
    assert_eq!(hint, size);
    assert_eq!(node.size(), Size::new(120.0, 40.0));
}

#[test]
fn above_widget_draw_update_and_interaction_delegate_to_content() {
    let mut overlay = above_widget(false);
    let mut tree = above_tree(&overlay);
    let renderer = RecordingRenderer::default();
    let mut draw_renderer = RecordingRenderer::default();
    let node = node_at(Rectangle { x: 10.0, y: 20.0, width: 120.0, height: 40.0 });

    <AboveOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
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

    let mut clipboard = iced::advanced::clipboard::Null;
    let mut messages = Vec::new();
    let mut shell = Shell::new(&mut messages);
    <AboveOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::update(
        &mut overlay,
        &mut tree,
        &Event::Mouse(mouse::Event::CursorMoved { position: Point::new(12.0, 24.0) }),
        Layout::new(&node),
        mouse::Cursor::Available(Point::new(12.0, 24.0)),
        &renderer,
        &mut clipboard,
        &mut shell,
        &Rectangle::INFINITE,
    );
    let interaction = <AboveOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
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

    drop(shell);
    assert_eq!(draw_renderer.fills, vec![node.bounds()]);
    assert_eq!(messages, vec![TestMessage::ChildUpdated]);
    assert_eq!(interaction, mouse::Interaction::Pointer);
}

#[test]
fn above_widget_show_flag_and_translation_control_overlay() {
    let renderer = RecordingRenderer::default();
    let viewport = Rectangle::with_size(Size::new(300.0, 200.0));
    let node = node_at(Rectangle { x: 90.0, y: 40.0, width: 120.0, height: 40.0 });
    let mut hidden = above_widget(false);
    let mut hidden_tree = above_tree(&hidden);

    let hidden_overlay = <AboveOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::overlay(
        &mut hidden,
        &mut hidden_tree,
        Layout::new(&node),
        &renderer,
        &viewport,
        Vector::new(5.0, 6.0),
    );
    assert!(hidden_overlay.is_none());

    let mut shown = above_widget(false).show(true).gap(8.0).snap_within_viewport(false);
    let mut shown_tree = above_tree(&shown);
    let mut shown_overlay = <AboveOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::overlay(
        &mut shown,
        &mut shown_tree,
        Layout::new(&node),
        &renderer,
        &viewport,
        Vector::new(5.0, 6.0),
    )
    .expect("show=true should create an above overlay");
    let overlay_node = shown_overlay.as_overlay_mut().layout(&renderer, viewport.size());

    assert_eq!(overlay_node.children().len(), 1);
    assert_eq!(
        overlay_node.children()[0].bounds(),
        Rectangle { x: 95.0, y: 18.0, width: 70.0, height: 20.0 }
    );
}

#[test]
fn above_widget_preserves_and_groups_content_overlay() {
    let renderer = RecordingRenderer::default();
    let viewport = Rectangle::with_size(Size::new(300.0, 200.0));
    let node = node_at(Rectangle { x: 90.0, y: 40.0, width: 120.0, height: 40.0 });
    let mut hidden = above_widget(true);
    let mut hidden_tree = above_tree(&hidden);
    let mut content_only = <AboveOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::overlay(
        &mut hidden,
        &mut hidden_tree,
        Layout::new(&node),
        &renderer,
        &viewport,
        Vector::ZERO,
    )
    .expect("content overlay should survive while the above overlay is hidden");
    let content_only_node = content_only.as_overlay_mut().layout(&renderer, viewport.size());
    assert_eq!(content_only_node.children().len(), 1);

    let mut shown = above_widget(true).show(true).snap_within_viewport(false);
    let mut shown_tree = above_tree(&shown);
    let mut grouped = <AboveOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::overlay(
        &mut shown,
        &mut shown_tree,
        Layout::new(&node),
        &renderer,
        &viewport,
        Vector::ZERO,
    )
    .expect("content and above overlays should be grouped");
    let grouped_node = grouped.as_overlay_mut().layout(&renderer, viewport.size());

    assert_eq!(grouped_node.children().len(), 2);
}

#[test]
fn point_above_widget_diff_size_hint_and_layout_delegate_to_content() {
    let mut overlay = point_above_widget(false);
    let mut tree = point_above_tree(&overlay);
    let renderer = RecordingRenderer::default();

    <PointAboveOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::diff(&overlay, &mut tree);
    let size = <PointAboveOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::size(&overlay);
    let hint = <PointAboveOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::size_hint(&overlay);
    let node = <PointAboveOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::layout(&mut overlay, &mut tree, &renderer, &layout_limits());

    assert_eq!(tree.children.len(), 2);
    assert_eq!(size, Size::new(Length::Fixed(120.0), Length::Fixed(40.0)));
    assert_eq!(hint, size);
    assert_eq!(node.size(), Size::new(120.0, 40.0));
}

#[test]
fn point_above_widget_draw_update_and_interaction_delegate_to_content() {
    let mut overlay = point_above_widget(false);
    let mut tree = point_above_tree(&overlay);
    let renderer = RecordingRenderer::default();
    let mut draw_renderer = RecordingRenderer::default();
    let node = node_at(Rectangle { x: 10.0, y: 20.0, width: 120.0, height: 40.0 });

    <PointAboveOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
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

    let mut clipboard = iced::advanced::clipboard::Null;
    let mut messages = Vec::new();
    let mut shell = Shell::new(&mut messages);
    <PointAboveOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::update(
        &mut overlay,
        &mut tree,
        &Event::Mouse(mouse::Event::CursorMoved { position: Point::new(12.0, 24.0) }),
        Layout::new(&node),
        mouse::Cursor::Available(Point::new(12.0, 24.0)),
        &renderer,
        &mut clipboard,
        &mut shell,
        &Rectangle::INFINITE,
    );
    let interaction = <PointAboveOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
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

    drop(shell);
    assert_eq!(draw_renderer.fills, vec![node.bounds()]);
    assert_eq!(messages, vec![TestMessage::ChildUpdated]);
    assert_eq!(interaction, mouse::Interaction::Pointer);
}

#[test]
fn point_above_widget_show_flag_anchor_and_translation_control_overlay() {
    let renderer = RecordingRenderer::default();
    let viewport = Rectangle::with_size(Size::new(300.0, 200.0));
    let node = node_at(Rectangle { x: 90.0, y: 40.0, width: 120.0, height: 40.0 });
    let mut hidden = point_above_widget(false);
    let mut hidden_tree = point_above_tree(&hidden);

    let hidden_overlay = <PointAboveOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::overlay(
        &mut hidden,
        &mut hidden_tree,
        Layout::new(&node),
        &renderer,
        &viewport,
        Vector::new(5.0, 6.0),
    );
    assert!(hidden_overlay.is_none());

    let mut shown = point_above_widget(false)
        .show(true)
        .anchor(Point::new(24.0, 80.0))
        .gap(10.0)
        .snap_within_viewport(false);
    let mut shown_tree = point_above_tree(&shown);
    let mut shown_overlay =
        <PointAboveOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
            TestMessage,
            Theme,
            RecordingRenderer,
        >>::overlay(
            &mut shown,
            &mut shown_tree,
            Layout::new(&node),
            &renderer,
            &viewport,
            Vector::new(5.0, 6.0),
        )
        .expect("show=true should create a point above overlay");
    let overlay_node = shown_overlay.as_overlay_mut().layout(&renderer, viewport.size());

    assert_eq!(overlay_node.children().len(), 1);
    assert_eq!(
        overlay_node.children()[0].bounds(),
        Rectangle { x: 84.0, y: 96.0, width: 70.0, height: 20.0 }
    );
}

#[test]
fn point_above_widget_preserves_and_groups_content_overlay() {
    let renderer = RecordingRenderer::default();
    let viewport = Rectangle::with_size(Size::new(300.0, 200.0));
    let node = node_at(Rectangle { x: 90.0, y: 40.0, width: 120.0, height: 40.0 });
    let mut hidden = point_above_widget(true);
    let mut hidden_tree = point_above_tree(&hidden);
    let mut content_only =
        <PointAboveOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
            TestMessage,
            Theme,
            RecordingRenderer,
        >>::overlay(
            &mut hidden,
            &mut hidden_tree,
            Layout::new(&node),
            &renderer,
            &viewport,
            Vector::ZERO,
        )
        .expect("content overlay should survive while the point overlay is hidden");
    let content_only_node = content_only.as_overlay_mut().layout(&renderer, viewport.size());
    assert_eq!(content_only_node.children().len(), 1);

    let mut shown = point_above_widget(true).show(true).snap_within_viewport(false);
    let mut shown_tree = point_above_tree(&shown);
    let mut grouped = <PointAboveOverlay<'_, TestMessage, Theme, RecordingRenderer> as Widget<
        TestMessage,
        Theme,
        RecordingRenderer,
    >>::overlay(
        &mut shown,
        &mut shown_tree,
        Layout::new(&node),
        &renderer,
        &viewport,
        Vector::ZERO,
    )
    .expect("content and point overlays should be grouped");
    let grouped_node = grouped.as_overlay_mut().layout(&renderer, viewport.size());

    assert_eq!(grouped_node.children().len(), 2);
}

#[test]
fn above_widgets_convert_to_elements() {
    let _: Element<'static, TestMessage> =
        AboveOverlay::new(label("content"), label("overlay")).into();
    let _: Element<'static, TestMessage> =
        PointAboveOverlay::new(label("content"), label("overlay")).into();
}
