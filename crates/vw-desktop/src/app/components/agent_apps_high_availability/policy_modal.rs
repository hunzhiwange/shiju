use crate::app::{Message, components::system_settings_common::*, message::agent_apps_high_availability_policy::{self as policy, Event, State}};
use iced::{Element, Length, widget::{button, column, container, row, text}};

pub(crate) const LABELS: [&str; 9] = [
    "最大重试次数", "首字节超时（秒）", "流空闲超时（秒）", "非流式总超时（秒）",
    "连续失败阈值", "恢复成功阈值", "熔断冷却（秒）", "错误率（%）", "最小请求数",
];
pub(crate) fn overlay<'a>(state: &'a State, base: Element<'a, Message>) -> Element<'a, Message> {
    if !state.open { return base; }
    let close = policy::message(Event::Close);
    let header = settings_modal_header(text("高可用策略").size(20), Some(settings_close_button(close.clone())));
    let mut body = column![
        text("重试共享同一总尝试预算（首次发送 + 最大重试次数），不会逐层叠加。"),
        text("流空闲超时为 0 时仅关闭空闲计时器。保存后对新请求生效；范围由服务端校验。"),
    ].spacing(12);
    if state.busy { body = body.push(text("正在请求服务端…")); }
    if state.loaded {
        for (index, label) in LABELS.iter().enumerate() {
            body = body.push(column![
                settings_field_label(*label),
                settings_text_input("", &state.fields[index])
                    .on_input_maybe((!state.busy).then_some(move |value| policy::message(Event::Edit(index, value))))
            ].spacing(6));
        }
    }
    if let Some(error) = &state.error { body = body.push(settings_error_banner(error)); }
    let save = button(text("保存")).padding([8, 16]).style(primary_action_btn_style)
        .on_press_maybe((state.loaded && !state.busy).then(|| policy::message(Event::Save)));
    let reload = button(text("重新读取")).padding([8, 16]).style(rounded_action_btn_style)
        .on_press_maybe((!state.busy).then(|| policy::message(Event::Reload)));
    let cancel = button(text("关闭")).padding([8, 16]).style(rounded_action_btn_style).on_press(close.clone());
    let card = settings_modal_layout(header, body, Some(row![reload, cancel, save].spacing(10).into()))
        .width(620).height(660);
    settings_modal_overlay(Some(base), close, container(card).padding(12).center(Length::Fill))
}
