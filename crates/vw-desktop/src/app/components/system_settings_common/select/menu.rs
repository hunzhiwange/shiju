use super::{MenuState, forward_shell, label::Label, option::MenuOption, styles};
use crate::app::components::system_settings_common::icon_svg;
use crate::app::{Message, assets::Icon};
use iced::advanced::{Clipboard, Layout, Shell, layout, mouse, overlay, renderer, widget};
use iced::widget::{Space, button, column, container, row, scrollable};
use iced::{Alignment, Element, Event, Length, Point, Rectangle, Renderer, Size, Theme};

const ROW_HEIGHT: f32 = 36.0;
const ROW_SPACING: f32 = 2.0;
const MENU_PADDING: f32 = 6.0;
const GAP: f32 = 6.0;
const EDGE: f32 = 8.0;
const SCROLL_ID: &str = "settings-select-menu";

pub(super) fn content(
    labels: &[String],
    selected: Option<usize>,
    keyboard: Option<usize>,
) -> Element<'static, usize> {
    let items = labels.iter().enumerate().map(|(index, label)| {
        let mark: Element<'static, usize> = if selected == Some(index) {
            icon_svg(Icon::Check, 16.0).style(styles::icon).into()
        } else {
            Space::new().width(16).height(16).into()
        };
        let button = button(
            row![Label::new(label.clone()), mark]
                .spacing(12)
                .height(Length::Fill)
                .align_y(Alignment::Center),
        )
        .on_press(index)
        .width(Length::Fill)
        .height(ROW_HEIGHT)
        .padding([6, 10])
        .style(|theme: &Theme, _| button::Style {
            text_color: theme.palette().text,
            ..Default::default()
        });
        Element::new(MenuOption {
            button: button.into(),
            keyboard_highlight: keyboard.map(|focused| focused == index),
        })
    });
    container(
        scrollable(container(column(items).spacing(ROW_SPACING)).padding([0, 7]))
            .id(SCROLL_ID)
            .direction(scrollable::Direction::Vertical(
                scrollable::Scrollbar::new().width(4).scroller_width(4),
            ))
            .height(Length::Shrink),
    )
    .padding([MENU_PADDING, 0.0])
    .style(styles::card)
    .into()
}

pub(super) struct SelectMenu<'a> {
    pub content: Element<'static, usize>,
    pub tree: &'a mut widget::Tree,
    pub state: &'a mut MenuState,
    pub focused: &'a mut bool,
    pub anchor: Rectangle,
    pub viewport: Rectangle,
    pub on_selected: &'a dyn Fn(usize) -> Message,
    pub option_count: usize,
}

impl SelectMenu<'_> {
    fn close(&mut self, shell: &mut Shell<'_, Message>) {
        self.state.open = false;
        shell.invalidate_layout();
        shell.request_redraw();
    }
}

impl overlay::Overlay<Message, Theme, Renderer> for SelectMenu<'_> {
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> layout::Node {
        let viewport =
            self.viewport.intersection(&Rectangle::with_size(bounds)).unwrap_or_default();
        let below =
            (viewport.y + viewport.height - EDGE - self.anchor.y - self.anchor.height - GAP)
                .max(0.0);
        let above = (self.anchor.y - GAP - viewport.y - EDGE).max(0.0);
        let wanted = (self.option_count as f32 * (ROW_HEIGHT + ROW_SPACING) - ROW_SPACING
            + MENU_PADDING * 2.0)
            .min(320.0);
        let upwards = below < wanted && above > below;
        let height = wanted.min(if upwards { above } else { below });
        let width = self.anchor.width.min((viewport.width - EDGE * 2.0).max(0.0));
        let node = self.content.as_widget_mut().layout(
            self.tree,
            renderer,
            &layout::Limits::new(Size::ZERO, Size::new(width, height)).width(Length::Fixed(width)),
        );
        let x = self.anchor.x.clamp(
            viewport.x + EDGE,
            (viewport.x + viewport.width - EDGE - width).max(viewport.x + EDGE),
        );
        let y = if upwards {
            self.anchor.y - GAP - node.size().height
        } else {
            self.anchor.y + self.anchor.height + GAP
        };
        let node = node.move_to(Point::new(x, y));
        if self.state.scroll_to_option {
            self.state.scroll_to_option = false;
            let index = self.state.keyboard_option.unwrap_or(0);
            let offset =
                (index as f32 * (ROW_HEIGHT + ROW_SPACING) - (height - ROW_HEIGHT) / 2.0).max(0.0);
            let mut operation = widget::operation::scrollable::scroll_to::<()>(
                widget::Id::from(SCROLL_ID),
                widget::operation::scrollable::AbsoluteOffset { x: None, y: Some(offset) },
            );
            self.content.as_widget_mut().operate(
                self.tree,
                Layout::new(&node),
                renderer,
                &mut operation,
            );
        }
        node
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
        use iced::keyboard::{Event as KeyEvent, Key, key::Named};
        if let Event::Keyboard(KeyEvent::KeyPressed { key, repeat, .. }) = event {
            match key {
                Key::Named(Named::Tab) => {
                    self.close(shell);
                    // Let the page's focus traversal handle Tab (and Shift-Tab).
                    return;
                }
                Key::Named(Named::Escape) => {
                    self.close(shell);
                }
                Key::Named(Named::Enter | Named::Space) => {
                    if !repeat {
                        if let Some(index) = self.state.keyboard_option {
                            shell.publish((self.on_selected)(index));
                        }
                        self.close(shell);
                    }
                }
                Key::Named(Named::ArrowDown | Named::ArrowUp | Named::Home | Named::End) => {
                    let index = self.state.keyboard_option;
                    self.state.keyboard_option = Some(match key {
                        Key::Named(Named::ArrowDown) => {
                            index.map_or(0, |i| (i + 1).min(self.option_count - 1))
                        }
                        Key::Named(Named::ArrowUp) => {
                            index.map_or(self.option_count - 1, |i| i.saturating_sub(1))
                        }
                        Key::Named(Named::Home) => 0,
                        _ => self.option_count - 1,
                    });
                    self.state.scroll_to_option = true;
                    shell.invalidate_layout();
                    shell.request_redraw();
                }
                _ => {}
            }
            shell.capture_event();
            return;
        }
        let pressed = match event {
            Event::Mouse(mouse::Event::ButtonPressed(_)) => cursor.position(),
            Event::Touch(iced::touch::Event::FingerPressed { position, .. }) => Some(*position),
            _ => None,
        };
        if let Some(point) = pressed.filter(|point| !layout.bounds().contains(*point)) {
            if !self.anchor.contains(point) {
                *self.focused = false;
            }
            self.close(shell);
            shell.capture_event();
            return;
        }
        if matches!(event, Event::Window(iced::window::Event::Unfocused)) {
            *self.focused = false;
            self.close(shell);
            return;
        }
        if matches!(
            event,
            Event::Mouse(mouse::Event::CursorMoved { .. } | mouse::Event::CursorLeft)
        ) {
            if self.state.keyboard_option.take().is_some() {
                shell.invalidate_layout();
            }
            // 每次移动都重绘；新建按钮尚无临时 status，不能靠它请求下一帧。
            shell.request_redraw();
        }
        let mut messages = Vec::new();
        let mut local = Shell::new(&mut messages);
        self.content.as_widget_mut().update(
            self.tree,
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            &mut local,
            &layout.bounds(),
        );
        forward_shell(&local, shell);
        if let Some(index) = messages.first() {
            shell.publish((self.on_selected)(*index));
            self.close(shell);
        }
        if matches!(event, Event::Mouse(_) | Event::Touch(_)) {
            shell.capture_event();
        }
    }
    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        defaults: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        self.content.as_widget().draw(
            self.tree,
            renderer,
            theme,
            defaults,
            layout,
            cursor,
            &self.viewport,
        );
    }
    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        if cursor.is_over(layout.bounds()) {
            self.content
                .as_widget()
                .mouse_interaction(self.tree, layout, cursor, &self.viewport, renderer)
                .max(mouse::Interaction::Idle)
        } else {
            mouse::Interaction::Idle
        }
    }
    fn operate(
        &mut self,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        self.content.as_widget_mut().operate(self.tree, layout, renderer, operation);
    }
}
