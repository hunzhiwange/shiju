use std::{cell::Cell, rc::Rc};

use iced::advanced::text::{Editor, Renderer as TextRenderer, highlighter::PlainText};
use iced::advanced::widget::Operation;
use iced::advanced::widget::operation::Scrollable as ScrollState;
use iced::advanced::widget::operation::scrollable::AbsoluteOffset;
use iced::advanced::widget::tree::{self, Tree};
use iced::advanced::{Clipboard, InputMethod, Layout, Shell, Widget, layout, overlay, renderer};
use iced::widget::{Id, Scrollable, scrollable, text, text_editor};
use iced::{Element, Event, Length, Rectangle, Renderer, Size, Theme, Vector, mouse, window};

use desktop_ui_controls::text_editor_scroll_panel::pass_wheel_to_parent;

use super::{
    CHAT_LINE_HEIGHT, CHAT_LINE_PIXELS, CHAT_MAX_HEIGHT, CHAT_MIN_HEIGHT, CHAT_PADDING,
    CHAT_TEXT_SIZE, Message,
};

pub(super) fn composer_viewport<'a>(
    content: &'a text_editor::Content,
    editor: Element<'a, Message>,
    page_lines: Rc<Cell<usize>>,
) -> Element<'a, Message> {
    let scroller = Scrollable::with_direction(
        pass_wheel_to_parent(editor),
        scrollable::Direction::Vertical(
            scrollable::Scrollbar::new().width(4).scroller_width(4).margin(7),
        ),
    )
    .width(Length::Fill)
    .style(scrollbar_style);

    Element::new(ComposerViewport { content, scroller: scroller.into(), page_lines })
}

fn scrollbar_style(theme: &Theme, status: scrollable::Status) -> scrollable::Style {
    let alpha = match status {
        scrollable::Status::Dragged { is_vertical_scrollbar_dragged: true, .. } => 0.56,
        scrollable::Status::Hovered { is_vertical_scrollbar_hovered: true, .. } => 0.40,
        _ => 0.24,
    };
    let mut style = scrollable::default(theme, status);
    style.vertical_rail.background = None;
    style.vertical_rail.border = iced::Border::default();
    style.vertical_rail.scroller.background = theme.palette().text.scale_alpha(alpha).into();
    style.vertical_rail.scroller.border = iced::border::rounded(2);
    style
}

// The editor is laid out at its full height. Keep Page Up / Down scoped to the viewport.
pub(super) fn fit_page_motion(
    binding: text_editor::Binding<Message>,
    page_lines: usize,
) -> text_editor::Binding<Message> {
    use text_editor::{Binding, Motion};
    let (motion, select) = match binding {
        Binding::Move(Motion::PageUp) => (Motion::Up, false),
        Binding::Move(Motion::PageDown) => (Motion::Down, false),
        Binding::Select(Motion::PageUp) => (Motion::Up, true),
        Binding::Select(Motion::PageDown) => (Motion::Down, true),
        _ => return binding,
    };

    Binding::Sequence(
        (0..page_lines)
            .map(|_| if select { Binding::Select(motion) } else { Binding::Move(motion) })
            .collect(),
    )
}

struct ComposerViewport<'a> {
    content: &'a text_editor::Content,
    scroller: Element<'a, Message>,
    page_lines: Rc<Cell<usize>>,
}

#[derive(Default)]
struct ViewportState {
    caret: Option<CaretLayout>,
}

#[derive(Clone, Copy, PartialEq)]
struct CaretLayout {
    cursor: text_editor::Cursor,
    document_y: f32,
    viewport: Size,
}

impl Widget<Message, Theme, Renderer> for ComposerViewport<'_> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<ViewportState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(ViewportState::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.scroller)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.scroller));
    }

    fn size(&self) -> Size<Length> {
        self.scroller.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        // Reserve room below the editor for composer controls on shorter windows.
        let cap = (limits.max().height * 0.60).clamp(CHAT_MIN_HEIGHT, CHAT_MAX_HEIGHT);
        let node = self.scroller.as_widget_mut().layout(
            &mut tree.children[0],
            renderer,
            &limits.max_height(cap),
        );
        let lines = ((node.size().height - CHAT_PADDING.y()) / CHAT_LINE_PIXELS) as usize;
        self.page_lines.set(lines.saturating_sub(1).max(1));
        node
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.scroller.as_widget_mut().operate(&mut tree.children[0], layout, renderer, operation);
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
        let scroller = self.scroller.as_widget_mut();
        let child = &mut tree.children[0];
        if !matches!(event, Event::Window(window::Event::RedrawRequested(_))) {
            scroller.update(child, event, layout, cursor, renderer, clipboard, shell, viewport);
            return;
        }

        let mut messages = Vec::new();
        let mut child_shell = Shell::new(&mut messages);
        scroller.update(
            child,
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            &mut child_shell,
            viewport,
        );

        // Iced reports the editor's caret to the native IME after applying its scroll offset.
        // Follow changes to that caret while leaving wheel and thumb movement independent.
        let state = tree.state.downcast_mut::<ViewportState>();
        if let InputMethod::Enabled { cursor: ime_caret, .. } = child_shell.input_method_mut() {
            let cursor = self.content.cursor();
            let active_end = cursor.selection.and_then(|_| {
                let content = layout.children().next()?.bounds();
                let viewport = layout.bounds().size();
                if content.height <= viewport.height {
                    return None;
                }
                state
                    .caret
                    .filter(|last| last.cursor == cursor && last.viewport == viewport)
                    .map(|last| last.document_y)
                    .or_else(|| selection_caret_y(self.content, cursor, renderer, content.width))
            });
            let mut reveal = RevealCaret {
                cursor,
                active_end,
                ime_caret,
                last: &mut state.caret,
                scrolled: false,
            };
            scroller.operate(child, layout, renderer, &mut reveal);
            if reveal.scrolled {
                child_shell.request_redraw();
            }
        } else {
            state.caret = None;
        }

        shell.merge(child_shell, |message| message);
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
        self.scroller.as_widget().draw(
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
        self.scroller.as_widget().mouse_interaction(
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
    ) -> Option<overlay::Element<'a, Message, Theme, Renderer>> {
        self.scroller.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

// The IME rectangle refers to the start of a selection. Measure the moving end separately.
fn selection_caret_y(
    content: &text_editor::Content,
    mut cursor: text_editor::Cursor,
    renderer: &Renderer,
    width: f32,
) -> Option<f32> {
    let mut editor = <Renderer as TextRenderer>::Editor::with_text(&content.text());
    cursor.selection = None;
    editor.move_to(cursor);
    editor.update(
        Size::new((width - CHAT_PADDING.x()).max(0.0), f32::INFINITY),
        renderer.default_font(),
        CHAT_TEXT_SIZE,
        CHAT_LINE_HEIGHT,
        text::Wrapping::WordOrGlyph,
        &mut PlainText,
    );
    match editor.selection() {
        iced::advanced::text::editor::Selection::Caret(point) => Some(point.y + CHAT_PADDING.top),
        iced::advanced::text::editor::Selection::Range(_) => None,
    }
}

struct RevealCaret<'a> {
    cursor: text_editor::Cursor,
    active_end: Option<f32>,
    ime_caret: &'a mut Rectangle,
    last: &'a mut Option<CaretLayout>,
    scrolled: bool,
}

impl Operation for RevealCaret<'_> {
    fn traverse(&mut self, _operate: &mut dyn FnMut(&mut dyn Operation)) {}

    fn scrollable(
        &mut self,
        _id: Option<&Id>,
        bounds: Rectangle,
        content_bounds: Rectangle,
        translation: Vector,
        state: &mut dyn ScrollState,
    ) {
        let document_y = self.active_end.unwrap_or(self.ime_caret.y - bounds.y + translation.y);
        let caret = CaretLayout { cursor: self.cursor, document_y, viewport: bounds.size() };
        if self.last.replace(caret) == Some(caret) {
            return;
        }

        let margin = CHAT_PADDING.top.min((bounds.height - CHAT_LINE_PIXELS).max(0.0) / 2.0);
        let top = document_y - margin;
        let bottom = document_y + CHAT_LINE_PIXELS + margin;
        let target = translation
            .y
            .min(top)
            .max(bottom - bounds.height)
            .clamp(0.0, (content_bounds.height - bounds.height).max(0.0));

        if target != translation.y {
            state.scroll_to(AbsoluteOffset { x: None, y: Some(target) });
            self.ime_caret.y += translation.y - target.round();
            self.scrolled = true;
        }
    }
}
