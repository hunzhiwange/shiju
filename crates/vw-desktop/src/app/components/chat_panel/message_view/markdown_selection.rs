//! One mouse selection across the text fragments in a rendered Markdown body.

use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;

use iced::advanced::graphics::text::{Paragraph, cosmic_text::Cursor as TextCursor};
use iced::advanced::text::{self, Paragraph as _, Span};
use iced::advanced::widget::{Operation, Tree, operation, tree};
use iced::advanced::{Clipboard, Layout, Renderer as _, Shell, Widget, layout, overlay, renderer};
use iced::widget::{column, span};
use iced::{
    Color, Element, Event, Font, Length, Point, Rectangle, Renderer, Size, Theme, Vector, font,
    keyboard, mouse,
};

use crate::app::components::text_editor_context_menu::TextContextAction;
use crate::app::i18n::I18n;
use crate::app::{App, Message};

use super::super::tool_text_support::chat_text_selection_color;
use super::super::utils::text_context_menu;
use super::markdown_links::LinkTarget;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Position {
    fragment: FragmentId,
    byte: usize,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct FragmentId {
    pub section: usize,
    pub row: usize,
    pub column: usize,
}

#[derive(Clone, Default)]
pub(super) struct Selection(Rc<Cell<Gesture>>);

#[derive(Clone, Copy, Default)]
struct Gesture {
    range: Option<(Position, Position)>,
    dragged: bool,
}

#[derive(Default)]
struct AreaState {
    owner: (Option<String>, usize, usize),
    gesture: Gesture,
    press: Option<Point>,
    selected: bool,
    text_menu: TextMenuState,
}

#[derive(Default)]
struct TextMenuState {
    tree: Option<Tree>,
    request: Option<(Vector, String)>,
    modifiers: keyboard::Modifiers,
}

struct TextMenu<'a> {
    full_text: std::borrow::Cow<'a, str>,
    i18n: I18n,
    content: Option<Element<'static, TextContextAction>>,
}

pub(super) fn area<'a>(
    content: Element<'a, Message>,
    selection: Selection,
    app: &'a App,
    message_idx: usize,
    text_idx: usize,
) -> Element<'a, Message> {
    Element::new(Area {
        content,
        selection,
        chat: Some(ChatOwner { app, message_idx, text_idx }),
        text_menu: None,
    })
}

/// Reuses Markdown hit testing and highlights for plain text with a bold title.
pub(in crate::app::components) fn selectable_titled_text<'a>(
    title: &'a str,
    body: impl Into<std::borrow::Cow<'a, str>>,
    color: Color,
    i18n: I18n,
) -> Element<'a, Message> {
    let selection = Selection::default();
    let render = |row, content, size, font| {
        fragment(
            selection.clone(),
            FragmentId { section: 0, row, column: 0 },
            vec![span(content)],
            FragmentStyle { size, color, font, width: Length::Fill, align: text::Alignment::Left },
            |_| Message::None,
        )
    };
    let title = render(0, title.into(), 14.0, Font { weight: font::Weight::Bold, ..Font::DEFAULT });
    let full_text = body.into();
    let body = render(1, full_text.clone(), 13.0, Font::DEFAULT);
    Element::new(Area {
        content: column![title, body].spacing(3).into(),
        selection,
        chat: None,
        text_menu: Some(TextMenu { full_text, i18n, content: None }),
    })
}

struct Area<'a> {
    content: Element<'a, Message>,
    selection: Selection,
    chat: Option<ChatOwner<'a>>,
    text_menu: Option<TextMenu<'a>>,
}

struct ChatOwner<'a> {
    app: &'a App,
    message_idx: usize,
    text_idx: usize,
}

impl Widget<Message, Theme, Renderer> for Area<'_> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<AreaState>()
    }

    fn state(&self) -> tree::State {
        let owner = self.chat.as_ref().map(ChatOwner::key).unwrap_or_default();
        tree::State::new(AreaState { owner, ..AreaState::default() })
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let state = tree.state.downcast_mut::<AreaState>();
        if let Some(chat) = &self.chat {
            if state.owner.0.as_deref() != chat.app.active_session_id.as_deref()
                || state.owner.1 != chat.message_idx
                || state.owner.2 != chat.text_idx
            {
                *state = AreaState { owner: chat.key(), ..AreaState::default() };
            }
            if state.press.is_none() && !chat.has_selection() {
                state.gesture = Gesture::default();
                state.selected = false;
            }
        }
        self.selection.0.set(state.gesture);
        self.content.as_widget_mut().layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content.as_widget_mut().operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        use mouse::{Button::Left, Event::*};

        let state = tree.state.downcast_mut::<AreaState>();
        let child = &mut tree.children[0];
        let position = cursor.position();
        let mut fragments = Fragments::new(*viewport);
        if let Some(menu) = &self.text_menu {
            let menu_state = &mut state.text_menu;
            if let Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) = event {
                menu_state.modifiers = *modifiers;
            }
            let secondary_click =
                matches!(event, Event::Mouse(ButtonPressed(mouse::Button::Right)))
                    || (cfg!(target_os = "macos")
                        && menu_state.modifiers.control()
                        && matches!(event, Event::Mouse(ButtonPressed(Left))));
            let menu_position =
                if secondary_click { cursor.position_over(*viewport) } else { None };
            let copy_shortcut = matches!(event, Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. })
                if modifiers.command() && !modifiers.alt()
                    && matches!(key.as_ref(), keyboard::Key::Character("c" | "C")));
            if menu_position.is_some() || (copy_shortcut && state.selected) {
                let selected = state.gesture.range.and_then(|(a, b)| {
                    self.content.as_widget_mut().operate(child, layout, renderer, &mut fragments);
                    fragments.selected(a, b)
                });
                if let Some(position) = menu_position {
                    let text = selected.unwrap_or_else(|| menu.full_text.to_string());
                    menu_state.request = Some((position - layout.position(), text));
                    menu_state.tree = None;
                } else if let Some(text) = selected {
                    menu_state.request = None;
                    shell.publish(Message::CopyCode(text));
                }
                shell.invalidate_layout();
                shell.request_redraw();
                shell.capture_event();
                return;
            }
            if matches!(event, Event::Mouse(ButtonPressed(Left)))
                && menu_state.request.take().is_some()
            {
                shell.invalidate_layout();
                shell.request_redraw();
                shell.capture_event();
                return;
            }
        }
        let relevant = match event {
            Event::Mouse(ButtonPressed(Left)) => {
                position.is_some_and(|point| layout.bounds().contains(point))
            }
            Event::Mouse(CursorMoved { .. } | ButtonReleased(Left)) => state.press.is_some(),
            _ => false,
        };
        if relevant {
            self.content.as_widget_mut().operate(child, layout, renderer, &mut fragments);
        }

        match event {
            Event::Mouse(ButtonPressed(Left)) => {
                if state.selected {
                    if let Some(chat) = &self.chat {
                        chat.publish_selection(shell, None);
                    }
                    shell.request_redraw();
                }
                state.gesture = Gesture::default();
                state.press = None;
                state.selected = false;
                if let Some(point) = position
                    && let Some(position) = fragments.position(point, true)
                {
                    state.press = Some(point);
                    state.gesture.range = Some((position, position));
                }
            }
            Event::Mouse(CursorMoved { .. } | ButtonReleased(Left))
                if let (Some(press), Some(point), Some((anchor, _))) =
                    (state.press, position, state.gesture.range) =>
            {
                if state.gesture.dragged || press.distance(point) > 3.0 {
                    state.gesture.dragged = true;
                    if let Some(end) = fragments.position(point, false) {
                        state.gesture.range = Some((anchor, end));
                    }
                    shell.request_redraw();
                }
            }
            _ => {}
        }
        self.selection.0.set(state.gesture);

        self.content
            .as_widget_mut()
            .update(child, event, layout, cursor, renderer, clipboard, shell, viewport);

        if matches!(event, Event::Mouse(ButtonReleased(Left))) && state.press.take().is_some() {
            let selected = state.gesture.range.and_then(|(a, b)| fragments.selected(a, b));
            state.selected = selected.is_some();
            if state.selected {
                if let Some(chat) = &self.chat {
                    chat.publish_selection(shell, selected);
                }
                shell.capture_event();
            }
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn overlay<'a>(
        &'a mut self,
        tree: &'a mut Tree,
        layout: Layout<'a>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<iced::advanced::overlay::Element<'a, Message, Theme, Renderer>> {
        if let Some(config) = &mut self.text_menu {
            let state = &mut tree.state.downcast_mut::<AreaState>().text_menu;
            let offset = state.request.as_ref()?.0;
            let content = config
                .content
                .get_or_insert_with(|| text_context_menu(config.i18n, |action| action));
            let tree = state.tree.get_or_insert_with(|| Tree::new(&*content));
            return Some(overlay::Element::new(Box::new(TextMenuOverlay {
                content,
                tree,
                anchor: layout.position() + offset + translation,
                request: &mut state.request,
            })));
        }
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

struct TextMenuOverlay<'a> {
    content: &'a mut Element<'static, TextContextAction>,
    tree: &'a mut Tree,
    anchor: Point,
    request: &'a mut Option<(Vector, String)>,
}

impl overlay::Overlay<Message, Theme, Renderer> for TextMenuOverlay<'_> {
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> layout::Node {
        let node = self.content.as_widget_mut().layout(
            self.tree,
            renderer,
            &layout::Limits::new(Size::ZERO, bounds),
        );
        let size = node.size();
        let x = self.anchor.x.clamp(0.0, (bounds.width - size.width).max(0.0));
        let y = if self.anchor.y + size.height <= bounds.height {
            self.anchor.y
        } else {
            (self.anchor.y - size.height).max(0.0)
        };
        node.move_to(Point::new(x, y))
    }

    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) {
        if matches!(
            event,
            Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(keyboard::key::Named::Escape),
                ..
            })
        ) {
            *self.request = None;
            shell.invalidate_layout();
            shell.request_redraw();
            shell.capture_event();
            return;
        }

        let bounds = layout.bounds();
        let mut actions = Vec::new();
        let mut menu_shell = Shell::new(&mut actions);
        self.content.as_widget_mut().update(
            self.tree,
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            &mut menu_shell,
            &bounds,
        );
        shell.request_redraw_at(menu_shell.redraw_request());
        if matches!(event, Event::Mouse(_)) && cursor.is_over(bounds) {
            shell.capture_event();
        }
        if let Some(action) = actions.into_iter().next()
            && let Some((_, text)) = self.request.take()
        {
            if let Some(message) = action.message(text) {
                shell.publish(message);
            }
            shell.invalidate_layout();
            shell.request_redraw();
        }
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        self.content.as_widget().draw(
            self.tree,
            renderer,
            theme,
            style,
            layout,
            cursor,
            &layout.bounds(),
        );
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        if !cursor.is_over(layout.bounds()) {
            return mouse::Interaction::None;
        }
        self.content
            .as_widget()
            .mouse_interaction(self.tree, layout, cursor, &layout.bounds(), renderer)
            .max(mouse::Interaction::Idle)
    }
}

impl ChatOwner<'_> {
    fn key(&self) -> (Option<String>, usize, usize) {
        (self.app.active_session_id.clone(), self.message_idx, self.text_idx)
    }

    fn has_selection(&self) -> bool {
        self.app.chat_markdown_selection.as_ref().is_some_and(|selection| {
            selection.message_idx == self.message_idx
                && selection.text_idx == self.text_idx
                && self.app.active_session_id.as_deref() == Some(selection.session_id.as_str())
        })
    }

    fn publish_selection(&self, shell: &mut Shell<'_, Message>, text: Option<String>) {
        if let Some(session_id) = self.app.active_session_id.clone() {
            shell.publish(Message::ChatMarkdownSelectionChanged {
                session_id,
                message_idx: self.message_idx,
                text_idx: self.text_idx,
                text,
            });
        }
    }
}

#[derive(Clone)]
struct Geometry {
    id: FragmentId,
    text: Arc<str>,
    line_starts: Arc<[usize]>,
    paragraph: Paragraph,
}

impl Geometry {
    fn byte_at(&self, point: Point) -> usize {
        if point.y < 0.0 {
            return 0;
        }
        if point.y >= self.paragraph.min_height() {
            return self.text.len();
        }
        self.paragraph
            .buffer()
            .hit(point.x, point.y)
            .and_then(|cursor| self.line_starts.get(cursor.line).map(|start| start + cursor.index))
            .unwrap_or(self.text.len())
            .min(self.text.len())
    }

    fn cursor_at(&self, byte: usize) -> TextCursor {
        let line = self.line_starts.partition_point(|start| *start <= byte).saturating_sub(1);
        TextCursor::new(line, byte - self.line_starts[line])
    }
}

struct Fragment {
    geometry: Geometry,
    bounds: Rectangle,
    visible: Option<Rectangle>,
}

struct Fragments {
    items: Vec<Fragment>,
    clip: Rectangle,
    translation: Vector,
    scroll: Option<(Rectangle, Vector)>,
}

impl Fragments {
    fn new(clip: Rectangle) -> Self {
        Self { items: Vec::new(), clip, translation: Vector::ZERO, scroll: None }
    }

    fn position(&self, point: Point, inside: bool) -> Option<Position> {
        let fragment = self
            .items
            .iter()
            .filter_map(|fragment| {
                let visible = fragment.visible?;
                let dx = (visible.x - point.x).max(point.x - visible.x - visible.width).max(0.0);
                let dy = (visible.y - point.y).max(point.y - visible.y - visible.height).max(0.0);
                (!inside || dx + dy == 0.0).then_some((fragment, dy * 1000.0 + dx))
            })
            .min_by(|(_, a), (_, b)| a.total_cmp(b))?
            .0;
        let byte = fragment
            .geometry
            .byte_at(Point::new(point.x - fragment.bounds.x, point.y - fragment.bounds.y));
        Some(Position { fragment: fragment.geometry.id, byte })
    }

    fn selected(&mut self, a: Position, b: Position) -> Option<String> {
        let (start, end) = ordered(a, b);
        if start == end {
            return None;
        }
        self.items.sort_by_key(|fragment| fragment.geometry.id);
        let mut output = String::new();
        let mut previous: Option<FragmentId> = None;
        for fragment in &self.items {
            let geometry = &fragment.geometry;
            let Some((from, to)) = fragment_range(geometry, start, end) else { continue };
            let Some(text) = geometry.text.get(from..to) else { continue };
            if let Some(previous) = previous {
                output.push_str(if previous.section != geometry.id.section {
                    "\n\n"
                } else if previous.row != geometry.id.row {
                    "\n"
                } else {
                    "\t"
                });
            }
            output.push_str(text);
            previous = Some(geometry.id);
        }
        (!output.is_empty()).then_some(output)
    }
}

impl Operation for Fragments {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        if let Some((bounds, translation)) = self.scroll.take() {
            let previous = (self.clip, self.translation);
            self.clip = self.clip.intersection(&(bounds - self.translation)).unwrap_or_default();
            self.translation = self.translation + translation;
            operate(self);
            (self.clip, self.translation) = previous;
        } else {
            operate(self);
        }
    }

    fn scrollable(
        &mut self,
        _id: Option<&iced::advanced::widget::Id>,
        bounds: Rectangle,
        _content_bounds: Rectangle,
        translation: Vector,
        _state: &mut dyn operation::Scrollable,
    ) {
        self.scroll = Some((bounds, translation));
    }

    fn custom(
        &mut self,
        _id: Option<&iced::advanced::widget::Id>,
        bounds: Rectangle,
        state: &mut dyn std::any::Any,
    ) {
        if let Some(geometry) = state.downcast_ref::<Geometry>() {
            let bounds = bounds - self.translation;
            self.items.push(Fragment {
                geometry: geometry.clone(),
                bounds,
                visible: bounds.intersection(&self.clip),
            });
        }
    }
}

fn ordered(a: Position, b: Position) -> (Position, Position) {
    (a.min(b), a.max(b))
}

fn fragment_range(geometry: &Geometry, start: Position, end: Position) -> Option<(usize, usize)> {
    let id = geometry.id;
    if id < start.fragment || id > end.fragment {
        return None;
    }
    let from = if id == start.fragment { start.byte } else { 0 };
    let to = if id == end.fragment { end.byte } else { geometry.text.len() };
    Some((from.min(geometry.text.len()), to.min(geometry.text.len())))
}

pub(super) struct FragmentStyle {
    pub size: f32,
    pub color: iced::Color,
    pub font: Font,
    pub width: Length,
    pub align: text::Alignment,
}

pub(super) fn fragment<'a>(
    selection: Selection,
    id: FragmentId,
    spans: Vec<Span<'a, LinkTarget>>,
    style: FragmentStyle,
    on_link: impl Fn(LinkTarget) -> Message + 'a,
) -> Element<'a, Message> {
    let content = iced::widget::rich_text(spans.clone())
        .size(style.size)
        .font(style.font)
        .color(style.color)
        .width(style.width)
        .align_x(style.align)
        .line_height(text::LineHeight::Relative(1.6))
        .wrapping(text::Wrapping::WordOrGlyph);
    Element::new(FragmentView {
        selection,
        id,
        spans,
        style,
        content: content.into(),
        on_link: Box::new(on_link),
    })
}

struct FragmentView<'a> {
    selection: Selection,
    id: FragmentId,
    spans: Vec<Span<'a, LinkTarget>>,
    style: FragmentStyle,
    content: Element<'a, Message>,
    on_link: Box<dyn Fn(LinkTarget) -> Message + 'a>,
}

#[derive(Default)]
struct FragmentState {
    geometry: Option<Geometry>,
    spans: Vec<Span<'static, LinkTarget>>,
    pressed: Option<usize>,
}

impl Widget<Message, Theme, Renderer> for FragmentView<'_> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<FragmentState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(FragmentState::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let state = tree.state.downcast_mut::<FragmentState>();
        let shape = text::Text {
            content: self.spans.as_slice(),
            bounds: limits.width(self.style.width).max(),
            size: self.style.size.into(),
            line_height: text::LineHeight::Relative(1.6),
            font: self.style.font,
            align_x: self.style.align,
            align_y: iced::alignment::Vertical::Top,
            shaping: text::Shaping::Advanced,
            wrapping: text::Wrapping::WordOrGlyph,
        };
        if state.geometry.is_none() || state.spans != self.spans {
            let text: String = self.spans.iter().map(|span| span.text.as_ref()).collect();
            let starts = std::iter::once(0)
                .chain(text.match_indices('\n').map(|(index, _)| index + 1))
                .collect::<Vec<_>>();
            state.geometry = Some(Geometry {
                id: self.id,
                text: text.into(),
                line_starts: starts.into(),
                paragraph: Paragraph::with_spans(shape),
            });
            state.spans = self.spans.iter().cloned().map(Span::to_static).collect();
        } else if let Some(geometry) = &mut state.geometry {
            geometry.id = self.id;
            match geometry.paragraph.compare(shape.with_content(())) {
                text::Difference::None => {}
                text::Difference::Bounds => geometry.paragraph.resize(shape.bounds),
                text::Difference::Shape => geometry.paragraph = Paragraph::with_spans(shape),
            }
        }
        self.content.as_widget_mut().layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        _renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        if let Some(geometry) = tree.state.downcast_mut::<FragmentState>().geometry.as_mut() {
            operation.custom(None, layout.bounds(), geometry);
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
        let state = tree.state.downcast_ref::<FragmentState>();
        if let (Some(geometry), Some((a, b))) = (&state.geometry, self.selection.0.get().range) {
            let (start, end) = ordered(a, b);
            if let Some((from, to)) = fragment_range(geometry, start, end)
                && from < to
            {
                let from = geometry.cursor_at(from);
                let to = geometry.cursor_at(to);
                for run in geometry.paragraph.buffer().layout_runs() {
                    if let Some((x, width)) = run.highlight(from, to) {
                        let bounds = Rectangle::new(
                            Point::new(layout.position().x + x, layout.position().y + run.line_top),
                            Size::new(width, run.line_height),
                        );
                        if let Some(bounds) = bounds.intersection(viewport) {
                            renderer.fill_quad(
                                renderer::Quad { bounds, ..Default::default() },
                                chat_text_selection_color(theme).scale_alpha(0.72),
                            );
                        }
                    }
                }
            }
        }
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<FragmentState>();
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                state.pressed = self.link_at(state, layout, cursor, viewport);
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if let Some(span) = state.pressed.take()
                    && !self.selection.0.get().dragged
                    && self.link_at(state, layout, cursor, viewport) == Some(span)
                    && let Some(link) = self.spans[span].link.clone()
                {
                    shell.publish((self.on_link)(link));
                    shell.capture_event();
                }
            }
            _ => {}
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        if self
            .link_at(tree.state.downcast_ref::<FragmentState>(), layout, cursor, viewport)
            .is_some()
        {
            mouse::Interaction::Pointer
        } else if cursor.is_over(layout.bounds()) && cursor.is_over(*viewport) {
            mouse::Interaction::Text
        } else {
            mouse::Interaction::None
        }
    }
}

impl FragmentView<'_> {
    fn link_at(
        &self,
        state: &FragmentState,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) -> Option<usize> {
        if !cursor.is_over(*viewport) {
            return None;
        }
        let point = cursor.position_in(layout.bounds())?;
        let paragraph = &state.geometry.as_ref()?.paragraph;
        let index = paragraph.hit_span(point)?;
        self.spans.get(index)?.link.as_ref()?;
        paragraph.span_bounds(index).iter().any(|bounds| bounds.contains(point)).then_some(index)
    }
}

#[cfg(test)]
#[path = "markdown_selection_tests.rs"]
mod tests;
