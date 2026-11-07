//! 图标改名只维护输入和提交状态，最终名称校验由网关负责。
use crate::app::components::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, primary_action_btn_style,
    rounded_action_btn_style, settings_close_button, settings_control_with_hint,
    settings_error_banner, settings_field_label, settings_modal_header, settings_modal_layout,
    settings_modal_overlay, settings_muted_text_style, settings_text_input_style,
};
use crate::app::{
    Message,
    message::{SettingsMessage, settings::AgentAppsMessage},
    state::IconRenameState,
};
use iced::widget::{Space, button, column, container, row, text, text_input};
use iced::{Alignment, Element, Length};

fn message(value: AgentAppsMessage) -> Message {
    Message::Settings(SettingsMessage::AgentApps(value))
}

pub(super) fn view<'a>(
    form: &'a IconRenameState,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    let close = message(AgentAppsMessage::RenameClose);
    let mut input = text_input("请输入图标名称", &form.name)
        .id("custom-icon-rename")
        .padding(SETTINGS_CONTROL_PADDING)
        .size(SETTINGS_CONTROL_TEXT_SIZE)
        .style(settings_text_input_style);
    if !form.saving {
        input = input
            .on_input(|name| message(AgentAppsMessage::RenameInput(name)))
            .on_submit(message(AgentAppsMessage::RenameSubmit));
    }
    let mut body = column![
        settings_field_label("名称"),
        settings_control_with_hint(input, "最多 64 个字符，名称由网关校验。"),
    ]
    .spacing(8);
    if let Some(error) = &form.error {
        body = body.push(settings_error_banner(error.clone()));
    }
    let cancel = button(text("取消").size(13))
        .padding([8, 14])
        .style(rounded_action_btn_style)
        .on_press_maybe((!form.saving).then_some(close.clone()));
    let save = button(text(if form.saving { "保存中…" } else { "保存" }).size(13))
        .padding([8, 14])
        .style(primary_action_btn_style)
        .on_press_maybe((!form.saving).then_some(message(AgentAppsMessage::RenameSubmit)));
    let header = settings_modal_header(
        column![
            text("图标改名").size(20),
            text("修改当前公司图标的名称。").size(13).style(settings_muted_text_style),
        ]
        .spacing(4),
        settings_close_button(if form.saving { Message::None } else { close.clone() }),
    );
    let actions =
        row![Space::new().width(Length::Fill), cancel, save].spacing(10).align_y(Alignment::Center);
    let card = settings_modal_layout(header, body, Some(actions.into())).width(520).height(340);
    settings_modal_overlay(Some(base), close, container(card).padding(12))
}
