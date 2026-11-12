//! Confirmation layout and its initially focused, keyboard-safe cancel action.
use super::*;
use crate::app::components::system_settings_common::{
    danger_action_btn_style, settings_close_button, settings_modal_card,
    settings_modal_overlay_with_escape,
};
use iced::advanced::{
    Layout, Widget, layout, renderer,
    widget::{Operation, Tree, tree},
};
use iced::widget::column;
use iced::{Rectangle, Renderer, Size, Theme, mouse};

pub(super) fn overlay<'a>(
    base: Element<'a, Message>,
    state: &'a ArchivedChatsState,
    i18n: I18n,
) -> Element<'a, Message> {
    let Some(target) = &state.delete_target else {
        return base;
    };
    let pending = state.delete_generation.is_some();
    let close = event(ArchivedChatsMessage::DeleteCancelled);
    let cancel = optional_action(
        i18n.__("取消"),
        (!pending).then_some(ArchivedChatsMessage::DeleteCancelled),
    );
    let cancel = Element::new(CancelFocus { content: cancel, enabled: !pending });
    let confirm = button(text(i18n.__(if pending { "正在删除…" } else { "删除" })).size(13))
        .padding([8, 12])
        .style(danger_action_btn_style)
        .on_press_maybe(
            (!pending && !state.loading).then(|| event(ArchivedChatsMessage::DeleteConfirmed)),
        );
    let title = if target.row.title.trim().is_empty() {
        i18n.__("未命名聊天")
    } else {
        &target.row.title
    };
    let card = settings_modal_card(
        column![
            row![
                text(i18n.__("删除已归档的聊天？")).size(20).width(Length::Fill),
                settings_close_button(if pending { Message::None } else { close.clone() })
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            container(
                scrollable(container(text(title).size(14)).padding([0, 7]))
                    .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
            )
            .max_height(120),
            text(i18n.__("这将永久删除该聊天及关联子聊天，无法撤销。"))
                .size(13)
                .style(settings_muted_text_style),
            row![Space::new().width(Length::Fill), cancel, confirm].spacing(8),
        ]
        .spacing(16),
    )
    .width(Length::Fill)
    .max_width(480);
    settings_modal_overlay_with_escape(base, close, card)
}

// Iced's stock button has no keyboard focus. Give cancellation initial focus;
// Enter/Space remain safe and the modal background cannot receive key events.
struct CancelFocus<'a> {
    content: Element<'a, Message>,
    enabled: bool,
}
struct Focus(bool);
impl iced::advanced::widget::operation::focusable::Focusable for Focus {
    fn is_focused(&self) -> bool {
        self.0
    }
    fn focus(&mut self) {
        self.0 = true;
    }
    fn unfocus(&mut self) {
        self.0 = false;
    }
}
impl Widget<Message, Theme, Renderer> for CancelFocus<'_> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Focus>()
    }
    fn state(&self) -> tree::State {
        tree::State::new(Focus(true))
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
        self.content.as_widget_mut().layout(&mut tree.children[0], renderer, limits)
    }
    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        if self.enabled {
            operation.focusable(None, layout.bounds(), tree.state.downcast_mut::<Focus>());
        }
        self.content.as_widget_mut().operate(&mut tree.children[0], layout, renderer, operation);
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
        if self.enabled && tree.state.downcast_ref::<Focus>().0 {
            use iced::advanced::Renderer as _;
            renderer.fill_quad(
                renderer::Quad {
                    bounds: layout.bounds(),
                    border: iced::Border {
                        color: theme.palette().primary,
                        width: 1.0,
                        radius: 6.0.into(),
                    },
                    ..Default::default()
                },
                iced::Color::TRANSPARENT,
            );
        }
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
    fn update(
        &mut self,
        tree: &mut Tree,
        event: &iced::Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn iced::advanced::Clipboard,
        shell: &mut iced::advanced::Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        use iced::keyboard::{Event, Key, key::Named};
        if self.enabled
            && tree.state.downcast_ref::<Focus>().0
            && let iced::Event::Keyboard(Event::KeyPressed {
                key: Key::Named(Named::Enter | Named::Space),
                repeat: false,
                ..
            }) = event
        {
            shell.publish(super::event(ArchivedChatsMessage::DeleteCancelled));
            shell.capture_event();
            return;
        }
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }
}
