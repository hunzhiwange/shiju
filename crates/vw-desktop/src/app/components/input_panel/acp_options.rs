//! Backend-advertised session options, using the shared settings modal controls.
use crate::app::components::system_settings_common::{
    primary_action_btn_style, rounded_action_btn_style, settings_close_button, settings_modal_card,
    settings_modal_overlay,
};
use crate::app::{App, Message, message::ChatMessage};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{Space, button as action, column, container, row, scrollable, text};
use iced::{Alignment, Element, Length};
use vw_gateway_client::vw_api_types::acp_session::AcpSessionSelection;

pub(super) fn button(app: &App) -> Element<'_, Message> {
    action(text("会话选项").size(12))
        .padding([4, 10])
        .style(rounded_action_btn_style)
        .on_press_maybe(
            app.active_session_id.as_ref().map(|_| Message::Chat(ChatMessage::AcpOptionsOpen)),
        )
        .into()
}

pub(crate) fn with_modal<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    let Some(runtime) =
        app.current_session_runtime_ref().filter(|r| r.acp_options_open && r.acp_agent.is_some())
    else {
        return base;
    };
    let close = Message::Chat(ChatMessage::AcpOptionsClose);
    let mut fields = column![].spacing(16);
    if !runtime.acp_options.connected {
        fields = fields.push(text("连接会话后可读取后端支持的选项").size(13));
    } else if runtime.acp_options.options.is_empty() {
        fields = fields.push(text("后端未提供可调整的会话选项").size(13));
    }
    for option in &runtime.acp_options.options {
        let mut choices = column![text(&option.name).size(14)].spacing(6);
        for choice in &option.choices {
            let selected = option.current == choice.id;
            let control = action(
                text(format!("{}{}", if selected { "✓ " } else { "" }, choice.name)).size(13),
            )
            .width(Length::Fill)
            .padding([8, 12])
            .style(if selected { primary_action_btn_style } else { rounded_action_btn_style })
            .on_press_maybe(
                (!selected && !runtime.acp_options_pending && !runtime.is_requesting).then(|| {
                    Message::Chat(ChatMessage::AcpOptionSelected(AcpSessionSelection {
                        kind: option.kind.clone(),
                        value: choice.id.clone(),
                    }))
                }),
            );
            choices = choices.push(control);
        }
        fields = fields.push(choices);
    }
    let contents = scrollable(container(fields).padding([0, 7]))
        .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
        .height(Length::Fixed(320.0));
    let card = settings_modal_card(
        column![
            row![
                text("ACP 会话选项").size(20),
                Space::new().width(Length::Fill),
                settings_close_button(close.clone())
            ]
            .align_y(Alignment::Center),
            text("选择后即时保存；失败时保留原值").size(12),
            contents,
            row![
                action(text(if runtime.acp_options_pending { "处理中…" } else { "刷新" }))
                    .padding([8, 16])
                    .style(rounded_action_btn_style)
                    .on_press_maybe(
                        (!runtime.acp_options_pending)
                            .then_some(Message::Chat(ChatMessage::AcpOptionsOpen))
                    ),
                Space::new().width(Length::Fill),
                action(text("关闭"))
                    .padding([8, 16])
                    .style(rounded_action_btn_style)
                    .on_press(close.clone())
            ]
            .spacing(8)
        ]
        .spacing(16),
    )
    .width(Length::Fixed(480.0));
    settings_modal_overlay(Some(base), close, card)
}
