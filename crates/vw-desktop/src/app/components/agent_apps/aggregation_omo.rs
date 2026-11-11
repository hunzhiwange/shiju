use crate::app::{
    Message,
    components::system_settings_common::*,
    message::settings::aggregation_omo::{Event, message},
    state::aggregation_omo::{Form, Variant},
};
use iced::{
    Element, Length,
    widget::{button, column, container, row, text, text_input},
};
pub(crate) fn overlay<'a>(form: &'a Form, base: Element<'a, Message>) -> Element<'a, Message> {
    let active = !form.busy;
    let action = |label: &'static str, event: Event, enabled: bool| {
        button(text(label))
            .padding([8, 14])
            .style(rounded_action_btn_style)
            .on_press_maybe(enabled.then_some(message(event)))
    };
    let title = match form.variant {
        Variant::Standard => "OMO 标准目录",
        Variant::Slim => "OMO Slim 目录",
    };
    let close = message(Event::Close);
    let header = settings_modal_header(text(title).size(20), settings_close_button(close.clone()));
    let mut body = column![
        text("应用已保存的本变体档案：标准分类为 omo，Slim 分类为 omo-slim。切换变体前请先显式停用当前变体。配置段只读，编辑请使用供应商档案。"),
        settings_field_label("供应商标识"),
        text_input("已保存的供应商标识", &form.provider).padding(SETTINGS_CONTROL_PADDING).size(SETTINGS_CONTROL_TEXT_SIZE).style(settings_text_input_style).on_input_maybe(active.then_some(|v| message(Event::Provider(v)))),
    ].spacing(12).width(Length::Fill);
    if form.busy {
        body = body.push(text("正在请求执行主机…").style(settings_muted_text_style));
    }
    if let Some(error) = &form.error {
        body = body.push(settings_error_banner(error));
    }
    if let Some(snapshot) = &form.snapshot {
        body = body
            .push(settings_field_label("当前项"))
            .push(text(snapshot.current().unwrap_or("已停用")));
        for (label, value) in snapshot.sections() {
            body = body
                .push(settings_panel(column![settings_field_label(label), text(value)].spacing(8)));
        }
    }
    let writable = active && form.snapshot.is_some();
    let footer = row![
        action("关闭", Event::Close, true),
        action("刷新后重试", Event::Refresh, active),
        action("停用此变体", Event::Disable, writable),
        button(text("应用供应商")).padding([8, 14]).style(primary_action_btn_style).on_press_maybe(
            (writable && !form.provider.trim().is_empty()).then_some(message(Event::Apply))
        ),
    ]
    .spacing(8)
    .wrap();
    let card = settings_modal_layout(header, body, Some(footer.into())).width(720).height(620);
    settings_modal_overlay(Some(base), close, container(card).padding(12))
}
