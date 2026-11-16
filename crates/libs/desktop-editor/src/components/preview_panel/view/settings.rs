use desktop_ui_controls::system_settings_common::help::settings_hover_help_label;
use desktop_i18n::I18n;
use desktop_ui_controls::system_settings_common::{primary_action_btn_style, settings_close_button, settings_modal_header, settings_modal_layout, settings_modal_overlay};
use iced::widget::{button, container, text};
use iced::{Element, Length};
pub fn build_settings_overlay<'a, Message: Clone + 'static>(
    i18n: I18n,
    help: Element<'a, Message>,
    body: Element<'a, Message>,
    close_message: Message,
    content_base: Element<'a, Message>,
) -> Element<'a, Message> {
    let title = i18n.__("编辑器设置");
    let header = settings_modal_header(
        settings_hover_help_label(text(title).size(20), help).wrap(),
        settings_close_button(close_message.clone()),
    );

    let footer = container(
        button(text(i18n.__("完成")).size(14))
            .on_press(close_message.clone())
            .padding([8, 18])
            .height(38)
            .style(primary_action_btn_style),
    )
    .align_right(Length::Fill);

    let card = settings_modal_layout(header, body, Some(footer.into()))
        .width(Length::Fill)
        .max_width(620.0)
        .max_height(660.0);

    let centered = container(card).center_x(Length::Fill).center_y(Length::Fill).padding(12);
    settings_modal_overlay(Some(content_base), close_message, centered)
}
