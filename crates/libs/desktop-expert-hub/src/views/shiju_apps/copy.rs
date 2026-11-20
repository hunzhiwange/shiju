use desktop_apps_ui::shiju_apps::operations::{DetailAction, valid_copy_id};
use desktop_i18n::I18n;
use desktop_ui_controls::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, settings_close_button,
    settings_field_label, settings_modal_header, settings_modal_layout, settings_modal_overlay,
    settings_text_input_style,
};
use iced::widget::{Space, column, container, row, text_input};
use iced::{Element, Length};

use super::{Event, Message, ViewInput, action, muted, styles, title};

pub(super) fn with_overlay<'a, M: Clone + 'static>(
    app: &ViewInput<'_>,
    base: Element<'a, M>,
    map: fn(Message) -> M,
) -> Element<'a, M> {
    let Some(form) = &app.shiju_apps.copy_form else { return base };
    let i18n = I18n::new(app.desktop_locale);
    let busy = app.shiju_apps.action.is_some();
    let close = Event::DetailAction(DetailAction::CopyCancel);
    let input = |value: &str, change: fn(String) -> DetailAction| {
        text_input("", value)
            .padding(SETTINGS_CONTROL_PADDING)
            .size(SETTINGS_CONTROL_TEXT_SIZE)
            .style(settings_text_input_style)
            .on_input_maybe(
                (!busy).then_some(move |value| Event::DetailAction(change(value)).into()),
            )
    };
    let content = column![
        settings_field_label(i18n.__("应用名称")),
        input(&form.title, DetailAction::CopyTitle),
        settings_field_label(i18n.__("应用 ID")),
        input(&form.app_id, DetailAction::CopyId),
        muted(i18n.__("应用 ID 使用小写英文字母、数字和连字符，以字母开头。"), 12),
    ]
    .spacing(10);
    let valid = !busy
        && valid_copy_id(&form.app_id)
        && !form.title.trim().is_empty()
        && form.title.chars().count() <= 128
        && !form.expected_version.is_empty();
    let header = settings_modal_header(
        title(i18n.__("复制应用"), 20),
        settings_close_button(close.clone().into()),
    );
    let footer = row![
        Space::new().width(Length::Fill),
        action(i18n.__("取消"), (!busy).then_some(close.clone())),
        action(
            i18n.__(if busy { "正在复制…" } else { "复制应用" }),
            valid.then_some(Event::DetailAction(DetailAction::CopySubmit))
        )
        .style(styles::primary_button_style),
    ]
    .spacing(8);
    let card: Element<'static, Message> =
        settings_modal_layout(header, content, Some(footer.into())).width(460).height(340).into();
    settings_modal_overlay(Some(base), map(close.into()), container(card.map(map)).padding(12))
}
