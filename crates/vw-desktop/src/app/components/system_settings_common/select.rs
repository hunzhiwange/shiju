//! 设置选择框：值由调用方拥有，控件管理焦点和悬浮菜单。

#[cfg(test)]
mod alignment_tests;
mod label;
mod menu;
mod option;
mod styles;
#[cfg(test)]
mod tests;

use super::{SETTINGS_CONTROL_TEXT_SIZE, icon_svg};
use crate::app::{Message, assets::Icon};
use iced::advanced::widget::operation::focusable::Focusable;
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer, widget};
use iced::keyboard::{Event as KeyEvent, Key, key::Named};
use iced::widget::{button, row};
use iced::{Alignment, Element, Event, Length, Rectangle, Renderer, Size, Theme, Vector};
use std::rc::Rc;

/// 选项的相等性表示同一个值，显示标签不参与消息转换。
pub fn settings_select<'a, T: Clone + PartialEq + ToString + 'static>(
    options: impl IntoIterator<Item = T>,
    selected: Option<T>,
    on_selected: impl Fn(T) -> Message + 'a,
) -> SettingsSelect<'a, T> {
    let options: Rc<[T]> = options.into_iter().collect();
    let selected_index =
        selected.as_ref().and_then(|value| options.iter().position(|v| v == value));
    let labels = options.iter().map(ToString::to_string).collect();
    let label = selected.map(|value| value.to_string());
    let enabled = !options.is_empty();
    let trigger = trigger(label.clone().unwrap_or_default(), label.is_none(), enabled);
    let values = Rc::clone(&options);
    SettingsSelect {
        trigger,
        options,
        labels,
        label,
        placeholder: String::new(),
        selected: selected_index,
        enabled,
        on_selected: Box::new(move |index| on_selected(values[index].clone())),
        on_open: None,
        width: Length::Fixed(280.0),
    }
}

pub struct SettingsSelect<'a, T> {
    trigger: Element<'static, ()>,
    options: Rc<[T]>,
    labels: Vec<String>,
    label: Option<String>,
    placeholder: String,
    selected: Option<usize>,
    enabled: bool,
    on_selected: Box<dyn Fn(usize) -> Message + 'a>,
    on_open: Option<(String, Message)>,
    width: Length,
}

impl<T> SettingsSelect<'_, T> {
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Only shown when no value is supplied. An unavailable saved value stays visible.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self.rebuild_trigger();
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.enabled = !disabled && !self.options.is_empty();
        self.rebuild_trigger();
        self
    }

    /// Reloads options when opened. The menu stays open as results arrive for the same key.
    pub fn reload_on_open(mut self, key: String, message: Message) -> Self {
        self.on_open = Some((key, message));
        self
    }

    fn rebuild_trigger(&mut self) {
        let label = self.label.as_ref().unwrap_or(&self.placeholder);
        self.trigger = trigger(label.clone(), self.label.is_none(), self.enabled);
    }
}

fn trigger(value: String, placeholder: bool, enabled: bool) -> Element<'static, ()> {
    button(
        row![
            label::Label::new(value),
            icon_svg(Icon::ChevronDown, 12.0).style(move |theme, status| {
                if enabled {
                    styles::icon(theme, status)
                } else {
                    iced::widget::svg::Style { color: Some(styles::label(theme, false, false)) }
                }
            }),
        ]
        .spacing(12)
        .height(Length::Fill)
        .align_y(Alignment::Center),
    )
    .on_press_maybe(enabled.then_some(()))
    .padding([9, 12])
    .width(Length::Fill)
    .height(38)
    .style(move |theme: &Theme, _| button::Style {
        text_color: styles::label(theme, placeholder, enabled),
        ..Default::default()
    })
    .into()
}

#[derive(Default)]
struct MenuState {
    open: bool,
    keyboard_option: Option<usize>,
    scroll_to_option: bool,
}

struct State<T> {
    menu: MenuState,
    focused: bool,
    options: Rc<[T]>,
    selected: Option<usize>,
    reload_key: Option<String>,
}

impl<T> Focusable for State<T> {
    fn is_focused(&self) -> bool {
        self.focused
    }

    fn focus(&mut self) {
        self.focused = true;
    }

    fn unfocus(&mut self) {
        self.focused = false;
        self.menu.open = false;
    }
}

impl<T: PartialEq + 'static> Widget<Message, Theme, Renderer> for SettingsSelect<'_, T> {
    fn tag(&self) -> widget::tree::Tag {
        widget::tree::Tag::of::<State<T>>()
    }

    fn state(&self) -> widget::tree::State {
        widget::tree::State::new(State {
            menu: MenuState::default(),
            focused: false,
            options: Rc::clone(&self.options),
            selected: self.selected,
            reload_key: self.on_open.as_ref().map(|(key, _)| key.clone()),
        })
    }

    fn children(&self) -> Vec<widget::Tree> {
        vec![widget::Tree::new(self.trigger.as_widget()), widget::Tree::empty()]
    }

    fn diff(&self, tree: &mut widget::Tree) {
        let state = tree.state.downcast_mut::<State<T>>();
        let reload_key = self.on_open.as_ref().map(|(key, _)| key);
        let same_source = state.reload_key.as_ref() == reload_key;
        if !same_source
            || state.options != self.options
            || state.selected != self.selected
            || !self.enabled
        {
            // An in-flight press or keyboard index must never select a newly supplied value.
            let keep_open = self.enabled && same_source && reload_key.is_some() && state.menu.open;
            state.menu = MenuState { open: keep_open, ..MenuState::default() };
            state.options = Rc::clone(&self.options);
            state.selected = self.selected;
            state.reload_key = reload_key.cloned();
            tree.children[0] = widget::Tree::new(self.trigger.as_widget());
            tree.children[1] = widget::Tree::empty();
        } else {
            tree.children[0].diff(self.trigger.as_widget());
        }
        if !self.enabled || !same_source {
            state.unfocus();
        }
    }

    fn size(&self) -> Size<Length> {
        Size::new(self.width, Length::Shrink)
    }

    fn layout(
        &mut self,
        tree: &mut widget::Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.trigger.as_widget_mut().layout(
            &mut tree.children[0],
            renderer,
            &limits.width(self.width),
        )
    }

    fn operate(
        &mut self,
        tree: &mut widget::Tree,
        layout: Layout<'_>,
        _renderer: &Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        if self.enabled {
            operation.focusable(None, layout.bounds(), tree.state.downcast_mut::<State<T>>());
        }
    }

    fn update(
        &mut self,
        tree: &mut widget::Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<State<T>>();
        if !self.enabled || state.menu.open {
            return;
        }

        let pointer_over = cursor.is_over(layout.bounds()) && cursor.is_over(*viewport);
        if matches!(
            event,
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                | Event::Touch(iced::touch::Event::FingerPressed { .. })
        ) {
            let focused = pointer_over && !shell.is_event_captured();
            if state.focused != focused {
                state.focused = focused;
                shell.request_redraw();
            }
        } else if matches!(event, Event::Window(iced::window::Event::Unfocused)) && state.focused {
            state.unfocus();
            shell.request_redraw();
        }

        if shell.is_event_captured() {
            return;
        }
        let mut messages = Vec::new();
        let mut local = Shell::new(&mut messages);
        self.trigger.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            if cursor.is_over(*viewport) { cursor } else { mouse::Cursor::Unavailable },
            renderer,
            clipboard,
            &mut local,
            viewport,
        );
        forward_shell(&local, shell);

        let keyboard_start = match event {
            Event::Keyboard(KeyEvent::KeyPressed { key, modifiers, repeat: false, .. })
                if state.focused && modifiers.is_empty() =>
            {
                match key {
                    Key::Named(Named::Enter | Named::Space | Named::ArrowDown) => {
                        self.selected.or(Some(0))
                    }
                    Key::Named(Named::ArrowUp) => self.selected.or(Some(self.options.len() - 1)),
                    _ => None,
                }
            }
            _ => None,
        };
        if !messages.is_empty() || keyboard_start.is_some() {
            state.menu = MenuState {
                open: true,
                keyboard_option: keyboard_start.or(self.selected),
                scroll_to_option: true,
            };
            // Discard an unfinished press and scroll offset from the previous menu.
            tree.children[1] = widget::Tree::empty();
            if let Some((_, message)) = &self.on_open {
                shell.publish(message.clone());
            }
            shell.capture_event();
            shell.invalidate_layout();
            shell.request_redraw();
        }
    }

    fn draw(
        &self,
        tree: &widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        defaults: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        use iced::advanced::Renderer as _;
        let state = tree.state.downcast_ref::<State<T>>();
        let style = styles::trigger(
            theme,
            self.enabled,
            state.menu.open,
            state.focused,
            cursor.is_over(layout.bounds()) && cursor.is_over(*viewport),
        );
        renderer.fill_quad(
            renderer::Quad { bounds: layout.bounds(), border: style.border, ..Default::default() },
            style.background,
        );
        self.trigger.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            defaults,
            layout,
            cursor,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &widget::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.trigger.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut widget::Tree,
        layout: Layout<'_>,
        _renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let state = tree.state.downcast_mut::<State<T>>();
        if !state.menu.open {
            return None;
        }
        let anchor = layout.bounds() + translation;
        let content = menu::content(&self.labels, self.selected, state.menu.keyboard_option);
        tree.children[1].diff(content.as_widget());
        Some(overlay::Element::new(Box::new(menu::SelectMenu {
            content,
            tree: &mut tree.children[1],
            state: &mut state.menu,
            focused: &mut state.focused,
            anchor,
            viewport: *viewport,
            on_selected: self.on_selected.as_ref(),
            option_count: self.labels.len(),
        })))
    }
}

impl<'a, T: PartialEq + 'static> From<SettingsSelect<'a, T>> for Element<'a, Message> {
    fn from(select: SettingsSelect<'a, T>) -> Self {
        Element::new(select)
    }
}

fn forward_shell(local: &Shell<'_, impl Sized>, shell: &mut Shell<'_, Message>) {
    shell.request_redraw_at(local.redraw_request());
    if local.is_event_captured() {
        shell.capture_event();
    }
    if local.is_layout_invalid() {
        shell.invalidate_layout();
    }
    if local.are_widgets_invalid() {
        shell.invalidate_widgets();
    }
}
