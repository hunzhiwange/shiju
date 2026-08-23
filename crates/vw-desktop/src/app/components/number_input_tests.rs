//! 验证数值输入的解析、范围约束、步进消息与显示格式。
//! 非法文本、上下界和带后缀显示用于区分编辑值与提交值。

use super::{
    NumberInput, NumberInputStyleVariant, State, clamp, fmt_value, prop_text_input_style,
    round_precision, settings_text_input_style,
};
use crate::app::Message;
use iced::advanced::text::editor::{Action, Cursor, Line, LineEnding, Position, Selection};
use iced::advanced::text::highlighter::Highlighter;
use iced::advanced::text::{self, Difference, LineHeight, Text, Wrapping};
use iced::advanced::{Layout, Shell, Widget, layout, mouse, renderer, widget};
use iced::keyboard::key::{Named, NativeCode, Physical};
use iced::{Background, Color, Event, Font, Length, Pixels, Point, Rectangle, Size, Theme};
use std::borrow::Cow;

#[derive(Debug, Default)]
struct RecordingRenderer {
    fills: Vec<Rectangle>,
    reset_bounds: Option<Rectangle>,
}

impl iced::advanced::Renderer for RecordingRenderer {
    fn start_layer(&mut self, _bounds: Rectangle) {}

    fn end_layer(&mut self) {}

    fn start_transformation(&mut self, _transformation: iced::Transformation) {}

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

#[derive(Debug, Clone)]
struct TestParagraph {
    text: String,
    bounds: Size,
    size: Pixels,
    font: Font,
    line_height: LineHeight,
    wrapping: Wrapping,
}

impl Default for TestParagraph {
    fn default() -> Self {
        Self {
            text: String::new(),
            bounds: Size::ZERO,
            size: Pixels(16.0),
            font: Font::default(),
            line_height: LineHeight::default(),
            wrapping: Wrapping::default(),
        }
    }
}

impl text::Paragraph for TestParagraph {
    type Font = Font;

    fn with_text(text: Text<&str, Self::Font>) -> Self {
        Self {
            text: text.content.to_string(),
            bounds: text.bounds,
            size: text.size,
            font: text.font,
            line_height: text.line_height,
            wrapping: text.wrapping,
        }
    }

    fn with_spans<Link>(text: Text<&[text::Span<'_, Link, Self::Font>], Self::Font>) -> Self {
        Self {
            bounds: text.bounds,
            size: text.size,
            font: text.font,
            line_height: text.line_height,
            wrapping: text.wrapping,
            ..Default::default()
        }
    }

    fn resize(&mut self, new_bounds: Size) {
        self.bounds = new_bounds;
    }

    fn compare(&self, text: Text<(), Self::Font>) -> Difference {
        if self.bounds != text.bounds { Difference::Bounds } else { Difference::None }
    }

    fn size(&self) -> Pixels {
        self.size
    }

    fn font(&self) -> Self::Font {
        self.font
    }

    fn line_height(&self) -> LineHeight {
        self.line_height
    }

    fn align_x(&self) -> text::Alignment {
        text::Alignment::Default
    }

    fn align_y(&self) -> iced::alignment::Vertical {
        iced::alignment::Vertical::Top
    }

    fn wrapping(&self) -> Wrapping {
        self.wrapping
    }

    fn shaping(&self) -> text::Shaping {
        text::Shaping::Basic
    }

    fn bounds(&self) -> Size {
        self.bounds
    }

    fn min_bounds(&self) -> Size {
        Size::new(self.text.len() as f32, self.size.0)
    }

    fn hit_test(&self, _point: Point) -> Option<text::Hit> {
        Some(text::Hit::CharOffset(0))
    }

    fn hit_span(&self, _point: Point) -> Option<usize> {
        Some(0)
    }

    fn span_bounds(&self, _index: usize) -> Vec<Rectangle> {
        vec![Rectangle::with_size(self.bounds)]
    }

    fn grapheme_position(&self, _line: usize, index: usize) -> Option<Point> {
        Some(Point::new(index as f32, 0.0))
    }
}

#[derive(Debug, Clone, Default)]
struct TestEditor {
    text: String,
    bounds: Size,
}

impl text::Editor for TestEditor {
    type Font = Font;

    fn with_text(text: &str) -> Self {
        Self { text: text.to_string(), bounds: Size::ZERO }
    }

    fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    fn cursor(&self) -> Cursor {
        Cursor { position: Position { line: 0, column: 0 }, selection: None }
    }

    fn selection(&self) -> Selection {
        Selection::Caret(Point::ORIGIN)
    }

    fn copy(&self) -> Option<String> {
        Some(self.text.clone())
    }

    fn line(&self, index: usize) -> Option<Line<'_>> {
        (index == 0)
            .then(|| Line { text: Cow::Borrowed(self.text.as_str()), ending: LineEnding::None })
    }

    fn line_count(&self) -> usize {
        1
    }

    fn perform(&mut self, _action: Action) {}

    fn move_to(&mut self, _cursor: Cursor) {}

    fn bounds(&self) -> Size {
        self.bounds
    }

    fn min_bounds(&self) -> Size {
        Size::new(self.text.len() as f32, 16.0)
    }

    fn update(
        &mut self,
        new_bounds: Size,
        _new_font: Self::Font,
        _new_size: Pixels,
        _new_line_height: LineHeight,
        _new_wrapping: Wrapping,
        _new_highlighter: &mut impl Highlighter,
    ) {
        self.bounds = new_bounds;
    }

    fn highlight<H: Highlighter>(
        &mut self,
        _font: Self::Font,
        _highlighter: &mut H,
        _format_highlight: impl Fn(&H::Highlight) -> text::highlighter::Format<Self::Font>,
    ) {
    }
}

impl text::Renderer for RecordingRenderer {
    type Font = Font;
    type Paragraph = TestParagraph;
    type Editor = TestEditor;

    const ICON_FONT: Self::Font = Font::DEFAULT;
    const CHECKMARK_ICON: char = 'x';
    const ARROW_DOWN_ICON: char = 'v';
    const SCROLL_UP_ICON: char = '^';
    const SCROLL_DOWN_ICON: char = 'v';
    const SCROLL_LEFT_ICON: char = '<';
    const SCROLL_RIGHT_ICON: char = '>';
    const ICED_LOGO: char = 'I';

    fn default_font(&self) -> Self::Font {
        Font::default()
    }

    fn default_size(&self) -> Pixels {
        Pixels(16.0)
    }

    fn fill_paragraph(
        &mut self,
        _text: &Self::Paragraph,
        _position: Point,
        _color: Color,
        _clip_bounds: Rectangle,
    ) {
    }

    fn fill_editor(
        &mut self,
        _editor: &Self::Editor,
        _position: Point,
        _color: Color,
        _clip_bounds: Rectangle,
    ) {
    }

    fn fill_text(
        &mut self,
        _text: Text<String, Self::Font>,
        _position: Point,
        _color: Color,
        _clip_bounds: Rectangle,
    ) {
    }
}

fn input(value: f32) -> NumberInput<'static> {
    NumberInput::new(value, -10.0, 10.0, 0.5, 2, 0.25, |v| Message::CopyCode(format!("{v:.2}")))
}

fn tree_for(widget: &NumberInput<'static>) -> widget::Tree {
    widget::Tree::new(widget as &dyn Widget<Message, Theme, RecordingRenderer>)
}

fn layout_for(bounds: Rectangle) -> layout::Node {
    layout::Node::new(bounds.size()).move_to(Point::new(bounds.x, bounds.y))
}

fn key_event(key: iced::keyboard::Key, text: Option<&str>) -> Event {
    Event::Keyboard(iced::keyboard::Event::KeyPressed {
        modified_key: key.clone(),
        key,
        physical_key: Physical::Unidentified(NativeCode::Unidentified),
        location: iced::keyboard::Location::Standard,
        modifiers: iced::keyboard::Modifiers::empty(),
        text: text.map(Into::into),
        repeat: false,
    })
}

fn update_with(
    widget: &mut NumberInput<'static>,
    tree: &mut widget::Tree,
    event: Event,
    cursor: mouse::Cursor,
) -> Vec<Message> {
    let renderer = RecordingRenderer::default();
    let mut clipboard = iced::advanced::clipboard::Null;
    let node = layout_for(Rectangle { x: 10.0, y: 20.0, width: 120.0, height: 40.0 });
    let mut messages = Vec::new();
    let mut shell = Shell::new(&mut messages);

    <NumberInput<'_> as Widget<Message, Theme, RecordingRenderer>>::update(
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

    messages
}

fn assert_copy_code_value(message: &Message, expected: &str) {
    match message {
        Message::CopyCode(value) => assert_eq!(value, expected),
        other => panic!("expected CopyCode message, got {other:?}"),
    }
}

#[test]
fn clamp_limits_values_to_bounds() {
    assert_eq!(clamp(-12.0, -10.0, 10.0), -10.0);
    assert_eq!(clamp(12.0, -10.0, 10.0), 10.0);
    assert_eq!(clamp(2.5, -10.0, 10.0), 2.5);
}

#[test]
fn round_precision_rounds_to_requested_decimal_places() {
    assert_eq!(round_precision(3.14159, 0), 3.0);
    assert_eq!(round_precision(3.14159, 2), 3.14);
    assert_eq!(round_precision(-3.146, 2), -3.15);
    assert_eq!(round_precision(1.23456, 4), 1.2346);
}

#[test]
fn fmt_value_trims_fractional_suffixes() {
    assert_eq!(fmt_value(42.9, 0), "43");
    assert_eq!(fmt_value(3.140, 4), "3.14");
    assert_eq!(fmt_value(10.0, 2), "10");
    assert_eq!(fmt_value(-2.556, 2), "-2.56");
}

#[test]
fn style_variant_height_matches_variant() {
    assert_eq!(NumberInputStyleVariant::PropertyPanel.height(), 26.0);
    assert_eq!(NumberInputStyleVariant::Settings.height(), 40.0);
}

#[test]
fn new_clamps_precision_and_uses_property_style_by_default() {
    let widget = NumberInput::new(1.0, 0.0, 2.0, 0.1, 9, 0.5, |_| Message::None);

    assert_eq!(widget.value, 1.0);
    assert_eq!(widget.min, 0.0);
    assert_eq!(widget.max, 2.0);
    assert_eq!(widget.step, 0.1);
    assert_eq!(widget.precision, 4);
    assert_eq!(widget.sensitivity, 0.5);
    assert_eq!(widget.style_variant, NumberInputStyleVariant::PropertyPanel);
}

#[test]
fn settings_style_switches_variant() {
    let widget = input(1.0).settings_style();

    assert_eq!(widget.style_variant, NumberInputStyleVariant::Settings);
}

#[test]
fn default_state_starts_idle() {
    let state = State::default();

    assert!(!state.is_dragging);
    assert!(!state.is_focused);
    assert!(!state.has_dragged);
    assert_eq!(state.drag_anchor_x, 0.0);
    assert_eq!(state.drag_start_value, 0.0);
    assert_eq!(state.edit_buffer, None);
    assert_eq!(state.caret, 0);
}

#[test]
fn settings_text_input_style_distinguishes_statuses() {
    let active = settings_text_input_style(&Theme::Dark, iced::widget::text_input::Status::Active);
    let hovered =
        settings_text_input_style(&Theme::Dark, iced::widget::text_input::Status::Hovered);
    let focused = settings_text_input_style(
        &Theme::Light,
        iced::widget::text_input::Status::Focused { is_hovered: true },
    );
    let disabled =
        settings_text_input_style(&Theme::Light, iced::widget::text_input::Status::Disabled);

    assert_ne!(active.background, hovered.background);
    assert_ne!(hovered.border.color, focused.border.color);
    assert!(disabled.value.a < 1.0);
}

#[test]
fn property_text_input_style_distinguishes_active_and_focused_statuses() {
    let active = prop_text_input_style(&Theme::Dark, iced::widget::text_input::Status::Active);
    let focused = prop_text_input_style(
        &Theme::Dark,
        iced::widget::text_input::Status::Focused { is_hovered: false },
    );

    assert_ne!(active.background, focused.background);
    assert_ne!(active.border.color, focused.border.color);
}

#[test]
fn widget_reports_state_size_and_idle_interaction() {
    let mut widget = input(2.0).settings_style();
    let mut tree = tree_for(&widget);
    let renderer = RecordingRenderer::default();
    let size = <NumberInput<'_> as Widget<Message, Theme, RecordingRenderer>>::size(&widget);
    let node = <NumberInput<'_> as Widget<Message, Theme, RecordingRenderer>>::layout(
        &mut widget,
        &mut tree,
        &renderer,
        &layout::Limits::new(Size::ZERO, Size::new(300.0, 90.0)),
    );
    let interaction =
        <NumberInput<'_> as Widget<Message, Theme, RecordingRenderer>>::mouse_interaction(
            &widget,
            &tree,
            Layout::new(&node),
            mouse::Cursor::Available(Point::new(20.0, 30.0)),
            &Rectangle::INFINITE,
            &renderer,
        );

    assert_eq!(size.width, Length::Fill);
    assert_eq!(size.height, Length::Fixed(40.0));
    assert_eq!(node.size(), Size::new(300.0, 40.0));
    assert_eq!(interaction, mouse::Interaction::Idle);
}

#[test]
fn widget_draw_does_not_emit_renderer_commands() {
    let widget = input(2.0);
    let tree = tree_for(&widget);
    let mut renderer = RecordingRenderer::default();
    let node = layout_for(Rectangle { x: 0.0, y: 0.0, width: 120.0, height: 26.0 });

    <NumberInput<'_> as Widget<Message, Theme, RecordingRenderer>>::draw(
        &widget,
        &tree,
        &mut renderer,
        &Theme::Dark,
        &renderer::Style::default(),
        Layout::new(&node),
        mouse::Cursor::Unavailable,
        &Rectangle::INFINITE,
    );

    assert!(renderer.fills.is_empty());
}

#[test]
fn click_without_drag_enters_edit_mode() {
    let mut widget = input(3.5);
    let mut tree = tree_for(&widget);

    update_with(
        &mut widget,
        &mut tree,
        Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)),
        mouse::Cursor::Available(Point::new(20.0, 30.0)),
    );
    update_with(
        &mut widget,
        &mut tree,
        Event::Mouse(iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left)),
        mouse::Cursor::Available(Point::new(20.0, 30.0)),
    );
    let state = tree.state.downcast_ref::<State>();

    assert!(!state.is_dragging);
    assert_eq!(state.edit_buffer.as_deref(), Some("3.5"));
    assert_eq!(state.caret, 3);
}

#[test]
fn drag_publishes_rounded_clamped_value() {
    let mut widget = input(9.0);
    let mut tree = tree_for(&widget);

    update_with(
        &mut widget,
        &mut tree,
        Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)),
        mouse::Cursor::Available(Point::new(20.0, 30.0)),
    );
    let messages = update_with(
        &mut widget,
        &mut tree,
        Event::Mouse(iced::mouse::Event::CursorMoved { position: Point::new(50.0, 30.0) }),
        mouse::Cursor::Available(Point::new(50.0, 30.0)),
    );
    let state = tree.state.downcast_ref::<State>();

    assert_copy_code_value(&messages[0], "10.00");
    assert!(state.has_dragged);
    assert_eq!(state.edit_buffer, None);
}

#[test]
fn small_drag_does_not_publish_message() {
    let mut widget = input(4.0);
    let mut tree = tree_for(&widget);

    update_with(
        &mut widget,
        &mut tree,
        Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)),
        mouse::Cursor::Available(Point::new(20.0, 30.0)),
    );
    let messages = update_with(
        &mut widget,
        &mut tree,
        Event::Mouse(iced::mouse::Event::CursorMoved { position: Point::new(21.5, 30.0) }),
        mouse::Cursor::Available(Point::new(21.5, 30.0)),
    );

    assert!(messages.is_empty());
}

#[test]
fn drag_release_after_real_motion_does_not_enter_edit_mode() {
    let mut widget = input(4.0);
    let mut tree = tree_for(&widget);

    update_with(
        &mut widget,
        &mut tree,
        Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)),
        mouse::Cursor::Available(Point::new(20.0, 30.0)),
    );
    update_with(
        &mut widget,
        &mut tree,
        Event::Mouse(iced::mouse::Event::CursorMoved { position: Point::new(40.0, 30.0) }),
        mouse::Cursor::Available(Point::new(40.0, 30.0)),
    );
    update_with(
        &mut widget,
        &mut tree,
        Event::Mouse(iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left)),
        mouse::Cursor::Available(Point::new(40.0, 30.0)),
    );
    let state = tree.state.downcast_ref::<State>();

    assert!(!state.is_dragging);
    assert!(state.has_dragged);
    assert_eq!(state.edit_buffer, None);
}

#[test]
fn mouse_press_inside_existing_edit_buffer_keeps_edit_mode_without_dragging() {
    let mut widget = input(4.0);
    let mut tree = tree_for(&widget);
    {
        let state = tree.state.downcast_mut::<State>();
        state.is_focused = true;
        state.edit_buffer = Some("4".to_string());
        state.caret = 1;
    }

    let messages = update_with(
        &mut widget,
        &mut tree,
        Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)),
        mouse::Cursor::Available(Point::new(20.0, 30.0)),
    );
    let state = tree.state.downcast_ref::<State>();

    assert!(messages.is_empty());
    assert!(state.is_focused);
    assert!(!state.is_dragging);
    assert_eq!(state.edit_buffer.as_deref(), Some("4"));
}

#[test]
fn outside_click_commits_valid_buffer_and_clears_focus() {
    let mut widget = input(1.0);
    let mut tree = tree_for(&widget);
    {
        let state = tree.state.downcast_mut::<State>();
        state.is_focused = true;
        state.edit_buffer = Some("42".to_string());
        state.caret = 2;
    }

    let messages = update_with(
        &mut widget,
        &mut tree,
        Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)),
        mouse::Cursor::Available(Point::new(500.0, 30.0)),
    );
    let state = tree.state.downcast_ref::<State>();

    assert_copy_code_value(&messages[0], "10.00");
    assert!(!state.is_focused);
    assert!(!state.is_dragging);
    assert_eq!(state.edit_buffer, None);
}

#[test]
fn outside_click_rejects_invalid_buffer_and_clears_focus() {
    let mut widget = input(1.0);
    let mut tree = tree_for(&widget);
    {
        let state = tree.state.downcast_mut::<State>();
        state.is_focused = true;
        state.edit_buffer = Some("not-a-number".to_string());
        state.caret = 12;
    }

    let messages = update_with(
        &mut widget,
        &mut tree,
        Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)),
        mouse::Cursor::Available(Point::new(500.0, 30.0)),
    );
    let state = tree.state.downcast_ref::<State>();

    assert!(messages.is_empty());
    assert!(!state.is_focused);
    assert!(!state.is_dragging);
    assert_eq!(state.edit_buffer, None);
}

#[test]
fn unavailable_cursor_clears_focus_and_edit_buffer() {
    let mut widget = input(1.0);
    let mut tree = tree_for(&widget);
    {
        let state = tree.state.downcast_mut::<State>();
        state.is_focused = true;
        state.edit_buffer = Some("5".to_string());
    }

    let messages = update_with(
        &mut widget,
        &mut tree,
        Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)),
        mouse::Cursor::Unavailable,
    );
    let state = tree.state.downcast_ref::<State>();

    assert!(messages.is_empty());
    assert!(!state.is_focused);
    assert_eq!(state.edit_buffer, None);
}

#[test]
fn enter_commits_buffer_and_escape_cancels_buffer() {
    let mut widget = input(1.0);
    let mut tree = tree_for(&widget);
    {
        let state = tree.state.downcast_mut::<State>();
        state.is_focused = true;
        state.edit_buffer = Some("-20".to_string());
        state.caret = 3;
    }

    let messages = update_with(
        &mut widget,
        &mut tree,
        key_event(iced::keyboard::Key::Named(Named::Enter), None),
        mouse::Cursor::Unavailable,
    );
    let state = tree.state.downcast_ref::<State>();

    assert_copy_code_value(&messages[0], "-10.00");
    assert!(!state.is_focused);
    assert_eq!(state.edit_buffer, None);

    let mut tree = tree_for(&widget);
    {
        let state = tree.state.downcast_mut::<State>();
        state.is_focused = true;
        state.edit_buffer = Some("7".to_string());
    }
    let messages = update_with(
        &mut widget,
        &mut tree,
        key_event(iced::keyboard::Key::Named(Named::Escape), None),
        mouse::Cursor::Unavailable,
    );
    let state = tree.state.downcast_ref::<State>();

    assert!(messages.is_empty());
    assert!(!state.is_focused);
    assert_eq!(state.edit_buffer, None);
}

#[test]
fn arrow_keys_adjust_value_by_step() {
    let mut widget = input(9.75);
    let mut tree = tree_for(&widget);
    tree.state.downcast_mut::<State>().is_focused = true;

    let messages = update_with(
        &mut widget,
        &mut tree,
        key_event(iced::keyboard::Key::Named(Named::ArrowUp), None),
        mouse::Cursor::Unavailable,
    );

    assert_copy_code_value(&messages[0], "10.00");

    let mut widget = input(-9.75);
    let mut tree = tree_for(&widget);
    tree.state.downcast_mut::<State>().is_focused = true;
    let messages = update_with(
        &mut widget,
        &mut tree,
        key_event(iced::keyboard::Key::Named(Named::ArrowDown), None),
        mouse::Cursor::Unavailable,
    );

    assert_copy_code_value(&messages[0], "-10.00");
}

#[test]
fn backspace_and_delete_edit_existing_buffers() {
    let mut widget = input(123.45);
    let mut tree = tree_for(&widget);
    {
        let state = tree.state.downcast_mut::<State>();
        state.is_focused = true;
        state.edit_buffer = Some("123".to_string());
        state.caret = 2;
    }

    update_with(
        &mut widget,
        &mut tree,
        key_event(iced::keyboard::Key::Named(Named::Backspace), None),
        mouse::Cursor::Unavailable,
    );
    {
        let state = tree.state.downcast_ref::<State>();
        assert_eq!(state.edit_buffer.as_deref(), Some("13"));
        assert_eq!(state.caret, 1);
    }

    update_with(
        &mut widget,
        &mut tree,
        key_event(iced::keyboard::Key::Named(Named::Delete), None),
        mouse::Cursor::Unavailable,
    );
    let state = tree.state.downcast_ref::<State>();

    assert_eq!(state.edit_buffer.as_deref(), Some("1"));
    assert_eq!(state.caret, 1);
}

#[test]
fn backspace_and_delete_start_editing_when_buffer_is_empty() {
    let mut widget = input(12.3);
    let mut tree = tree_for(&widget);
    tree.state.downcast_mut::<State>().is_focused = true;

    update_with(
        &mut widget,
        &mut tree,
        key_event(iced::keyboard::Key::Named(Named::Backspace), None),
        mouse::Cursor::Unavailable,
    );
    {
        let state = tree.state.downcast_ref::<State>();
        assert_eq!(state.edit_buffer.as_deref(), Some("12."));
        assert_eq!(state.caret, 3);
    }

    let mut tree = tree_for(&widget);
    tree.state.downcast_mut::<State>().is_focused = true;
    update_with(
        &mut widget,
        &mut tree,
        key_event(iced::keyboard::Key::Named(Named::Delete), None),
        mouse::Cursor::Unavailable,
    );
    let state = tree.state.downcast_ref::<State>();

    assert_eq!(state.edit_buffer.as_deref(), Some("12.3"));
    assert_eq!(state.caret, 0);
}

#[test]
fn navigation_keys_move_caret_within_bounds() {
    let mut widget = input(12.3);
    let mut tree = tree_for(&widget);
    {
        let state = tree.state.downcast_mut::<State>();
        state.is_focused = true;
        state.edit_buffer = Some("123".to_string());
        state.caret = 1;
    }

    update_with(
        &mut widget,
        &mut tree,
        key_event(iced::keyboard::Key::Named(Named::ArrowLeft), None),
        mouse::Cursor::Unavailable,
    );
    update_with(
        &mut widget,
        &mut tree,
        key_event(iced::keyboard::Key::Named(Named::ArrowRight), None),
        mouse::Cursor::Unavailable,
    );
    update_with(
        &mut widget,
        &mut tree,
        key_event(iced::keyboard::Key::Named(Named::Home), None),
        mouse::Cursor::Unavailable,
    );
    update_with(
        &mut widget,
        &mut tree,
        key_event(iced::keyboard::Key::Named(Named::End), None),
        mouse::Cursor::Unavailable,
    );
    let state = tree.state.downcast_ref::<State>();

    assert_eq!(state.caret, 3);
}

#[test]
fn character_key_inserts_only_numeric_characters() {
    let mut widget = input(0.0);
    let mut tree = tree_for(&widget);
    tree.state.downcast_mut::<State>().is_focused = true;

    update_with(
        &mut widget,
        &mut tree,
        key_event(iced::keyboard::Key::Character("1a.-".into()), None),
        mouse::Cursor::Unavailable,
    );
    let state = tree.state.downcast_ref::<State>();

    assert_eq!(state.edit_buffer.as_deref(), Some("1.-"));
    assert_eq!(state.caret, 3);
}

#[test]
fn text_payload_inserts_numeric_characters_for_other_keys() {
    let mut widget = input(0.0);
    let mut tree = tree_for(&widget);
    tree.state.downcast_mut::<State>().is_focused = true;

    update_with(
        &mut widget,
        &mut tree,
        key_event(iced::keyboard::Key::Unidentified, Some("x-4.2")),
        mouse::Cursor::Unavailable,
    );
    let state = tree.state.downcast_ref::<State>();

    assert_eq!(state.edit_buffer.as_deref(), Some("-4.2"));
    assert_eq!(state.caret, 4);
}

#[test]
fn keyboard_input_is_ignored_without_focus() {
    let mut widget = input(0.0);
    let mut tree = tree_for(&widget);

    let messages = update_with(
        &mut widget,
        &mut tree,
        key_event(iced::keyboard::Key::Character("123".into()), None),
        mouse::Cursor::Unavailable,
    );
    let state = tree.state.downcast_ref::<State>();

    assert!(messages.is_empty());
    assert_eq!(state.edit_buffer, None);
    assert_eq!(state.caret, 0);
}

#[test]
fn element_conversion_accepts_both_styles() {
    let _property: iced::Element<'_, Message> = input(2.5).into();
    let _settings: iced::Element<'_, Message> = input(2.5).settings_style().into();
}
