//! 验证消息组件测量、绘制裁剪与高度缓存更新的边界行为。
//! 自定义渲染器只记录布局副作用，用于隔离验证无效视口和未变化高度的短路路径。

use super::message_measure::{
    ChatMessageMeasurement, MeasuredChatMessage, collect_visible_chat_message_measurements,
};
use super::virtualizer::ChatVirtualizer;
use crate::app::App;
use crate::app::message::chat::input::apply_chat_message_measurements;
use crate::app::models::{ChatMessage, ChatRole};
use iced::advanced::widget::operation::{Operation, black_box};
use iced::advanced::{Clipboard, Layout, Renderer, Shell, Widget, layout, renderer, widget};
use iced::{
    Background, Color, Element, Event, Length, Point, Rectangle, Size, Theme, Transformation, mouse,
};

#[derive(Debug, Clone, PartialEq)]
enum TestMessage {}

// 固定尺寸内容将布局输入与真实消息排版解耦，使测试只观察测量协议。
#[derive(Debug, Clone, Copy)]
struct TestContent {
    width: f32,
    height: f32,
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

    fn update(
        &mut self,
        _tree: &mut widget::Tree,
        _event: &Event,
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _renderer: &RecordingRenderer,
        _clipboard: &mut dyn Clipboard,
        _shell: &mut Shell<'_, TestMessage>,
        _viewport: &Rectangle,
    ) {
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
}

impl<'a> From<TestContent> for Element<'a, TestMessage, Theme, RecordingRenderer> {
    fn from(content: TestContent) -> Self {
        Element::new(content)
    }
}

// 仅记录填充区域和重置边界；其它渲染能力故意保持最小实现。
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

fn layout_for(bounds: Rectangle) -> layout::Node {
    layout::Node::new(bounds.size()).move_to(Point::new(bounds.x, bounds.y))
}

fn test_app() -> App {
    App::new().0
}

fn chat_message(content: &str) -> ChatMessage {
    ChatMessage {
        role: ChatRole::Assistant,
        content: content.to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }
}

#[test]
fn chat_message_measure_operation_collects_bounds() {
    let mut widget = MeasuredChatMessage::new(7, TestContent { width: 120.0, height: 44.0 });
    let mut tree = widget::Tree::empty();
    tree.children.push(widget::Tree::empty());
    let layout = layout_for(Rectangle { x: 16.0, y: 24.0, width: 120.0, height: 44.0 });
    let renderer = RecordingRenderer::default();
    let mut operation = collect_visible_chat_message_measurements();
    {
        let mut operation_ref = black_box(&mut operation);
        widget.operate(&mut tree, Layout::new(&layout), &renderer, &mut operation_ref);
    }

    let iced::advanced::widget::operation::Outcome::Some(measurements) = operation.finish() else {
        panic!("expected chat message measurements");
    };
    assert_eq!(
        measurements,
        vec![ChatMessageMeasurement { message_idx: 7, y: 24.0, height: 44.0, target: None }]
    );
}

#[test]
fn chat_message_measure_operation_ignores_foreign_custom_state() {
    let mut operation = collect_visible_chat_message_measurements();
    let mut foreign_state = 7usize;

    operation.custom(
        None,
        Rectangle { x: 4.0, y: 8.0, width: 12.0, height: 16.0 },
        &mut foreign_state,
    );

    let iced::advanced::widget::operation::Outcome::Some(measurements) = operation.finish() else {
        panic!("expected measurement outcome");
    };
    assert!(measurements.is_empty());
}

#[test]
fn chat_message_measure_operation_accumulates_multiple_widgets_in_order() {
    let renderer = RecordingRenderer::default();
    let mut operation = collect_visible_chat_message_measurements();

    for (message_idx, bounds) in [
        (2, Rectangle { x: 0.0, y: 12.0, width: 80.0, height: 24.0 }),
        (5, Rectangle { x: 0.0, y: 48.0, width: 80.0, height: 36.0 }),
    ] {
        let mut measured = MeasuredChatMessage::new(
            message_idx,
            TestContent { width: 80.0, height: bounds.height },
        );
        let mut tree = widget::Tree::empty();
        tree.children.push(widget::Tree::empty());
        let node = layout_for(bounds);
        let mut operation_ref = black_box(&mut operation);
        measured.operate(&mut tree, Layout::new(&node), &renderer, &mut operation_ref);
    }

    let iced::advanced::widget::operation::Outcome::Some(measurements) = operation.finish() else {
        panic!("expected measurement outcome");
    };
    assert_eq!(
        measurements,
        vec![
            ChatMessageMeasurement { message_idx: 2, y: 12.0, height: 24.0, target: None },
            ChatMessageMeasurement { message_idx: 5, y: 48.0, height: 36.0, target: None },
        ]
    );
}

#[test]
fn chat_message_draw_skips_non_finite_viewport() {
    let mut measured = MeasuredChatMessage::new(0, TestContent { width: 80.0, height: 24.0 });
    let mut tree = widget::Tree::empty();
    tree.children.push(widget::Tree::empty());
    let renderer = RecordingRenderer::default();
    let node = measured.layout(&mut tree, &renderer, &layout::Limits::NONE);
    let mut renderer = RecordingRenderer::default();

    measured.draw(
        &tree,
        &mut renderer,
        &Theme::Dark,
        &renderer::Style::default(),
        Layout::new(&node),
        mouse::Cursor::Unavailable,
        &Rectangle { y: f32::INFINITY, ..Rectangle::with_size(Size::new(800.0, 600.0)) },
    );

    assert!(renderer.fills.is_empty());
}

#[test]
fn chat_message_draw_skips_finite_viewport_outside_text_coordinate_range() {
    let mut measured = MeasuredChatMessage::new(0, TestContent { width: 80.0, height: 24.0 });
    let mut tree = widget::Tree::empty();
    tree.children.push(widget::Tree::empty());
    let renderer = RecordingRenderer::default();
    let node = measured.layout(&mut tree, &renderer, &layout::Limits::NONE);
    let mut renderer = RecordingRenderer::default();

    measured.draw(
        &tree,
        &mut renderer,
        &Theme::Dark,
        &renderer::Style::default(),
        Layout::new(&node),
        mouse::Cursor::Unavailable,
        &Rectangle { y: f32::MAX, ..Rectangle::with_size(Size::new(800.0, 600.0)) },
    );

    assert!(renderer.fills.is_empty());
}

#[test]
fn chat_message_measure_ignores_unchanged_heights() {
    let mut app = test_app();
    app.chat = vec![chat_message("answer")];
    app.chat_virtualizer = ChatVirtualizer::from_estimated_heights(&[100.0]);
    app.sync_chat_virtualizer_compat_fields();
    app.chat_message_measured_heights.insert(0, 100.0);
    app.chat_render_cache.insert(0, Default::default());
    app.chat_visible_text_cache = vec![Some("copy".to_string())];
    app.chat_copy_hash_cache = vec![Some(42)];

    let changed = apply_chat_message_measurements(
        &mut app,
        vec![ChatMessageMeasurement { message_idx: 0, y: 10.0, height: 100.4, target: None }],
    );

    assert!(!changed);
    assert_eq!(app.chat_message_estimated_heights, vec![100.0]);
    assert_eq!(app.chat_message_measured_heights.get(&0), Some(&100.0));
    assert!(app.chat_render_cache.contains_key(&0));
    assert_eq!(app.chat_visible_text_cache, vec![Some("copy".to_string())]);
    assert_eq!(app.chat_copy_hash_cache, vec![Some(42)]);
}

#[test]
fn chat_message_measure_updates_changed_heights_without_touching_copy_caches() {
    let mut app = test_app();
    app.chat = vec![chat_message("answer")];
    app.chat_virtualizer = ChatVirtualizer::from_estimated_heights(&[100.0]);
    app.sync_chat_virtualizer_compat_fields();
    app.chat_render_cache.insert(0, Default::default());
    app.chat_visible_text_cache = vec![Some("copy".to_string())];
    app.chat_copy_hash_cache = vec![Some(42)];

    let changed = apply_chat_message_measurements(
        &mut app,
        vec![ChatMessageMeasurement { message_idx: 0, y: 10.0, height: 124.0, target: None }],
    );

    assert!(changed);
    assert_eq!(app.chat_message_estimated_heights, vec![124.0]);
    assert_eq!(app.chat_message_measured_heights.get(&0), Some(&124.0));
    assert_eq!(app.chat_height_index.scroll_top_for_item(0), 0.0);
    assert!(app.chat_render_cache.contains_key(&0));
    assert_eq!(app.chat_visible_text_cache, vec![Some("copy".to_string())]);
    assert_eq!(app.chat_copy_hash_cache, vec![Some(42)]);
}
