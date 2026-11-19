#[derive(Clone)]
pub enum OverlayEvent<E> {
    Local(Event),
    External(E),
}

#[derive(Clone)]
pub enum Event {
    Open,
    Close,
    Export,
}
fn message<E>(event: Event) -> OverlayEvent<E> {
    OverlayEvent::Local(event)
}
use desktop_ui_controls::system_settings_common::*;
use iced::widget::{button, column, row, text};
use iced::{Alignment, Element, Length};

pub fn view(available: bool) -> Element<'static, Event> {
    button(text("导出脱敏配置"))
        .padding([8, 14])
        .style(rounded_action_btn_style)
        .on_press_maybe(available.then_some(Event::Open))
        .into()
}

pub fn overlay<'a, E: Clone + 'static>(
    busy: bool,
    visible: bool,
    base: Element<'a, E>,
) -> Element<'a, OverlayEvent<E>> {
    if !visible {
        return base.map(OverlayEvent::External);
    }
    let close = message(Event::Close);
    let header =
        settings_modal_header(text("导出脱敏配置").size(18), settings_close_button(close.clone()));
    let body = settings_group_card(
        column![
            text("应用偏好").size(14),
            text("导出应用排序与显示设置。").size(13).style(settings_muted_text_style),
        ]
        .spacing(5),
    )
    .padding([12, 14]);
    let footer = row![
        button(text("取消"))
            .padding([8, 14])
            .style(rounded_action_btn_style)
            .on_press_maybe((!busy).then_some(close.clone())),
        button(text(if busy { "正在导出或保存…" } else { "导出并选择保存位置" }))
            .padding([8, 14])
            .style(primary_action_btn_style)
            .on_press_maybe((!busy).then_some(message(Event::Export))),
    ]
    .spacing(8);
    let card = settings_modal_card(
        column![header, body, footer].spacing(16).align_x(Alignment::End).width(Length::Fill),
    )
    .width(480);
    settings_modal_overlay(Some(base.map(OverlayEvent::External)), close, card)
}
