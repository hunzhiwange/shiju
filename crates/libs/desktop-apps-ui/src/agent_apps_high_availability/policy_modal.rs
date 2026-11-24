use desktop_ui_controls::system_settings_common::*;
type Message = Event;
#[derive(Clone, Debug)]
pub enum Event {
    Close,
    Edit(usize, String),
    Save,
    Reload,
}
pub struct State<'a> {
    pub open: bool,
    pub busy: bool,
    pub loaded: bool,
    pub fields: &'a [String; 9],
    pub error: &'a Option<String>,
}
fn message(event: Event) -> Event {
    event
}
use iced::{
    Alignment, Element, Length,
    widget::{button, column, container, responsive, row, text},
};

pub const LABELS: [&str; 9] = [
    "最大重试次数",
    "首字节超时（秒）",
    "流空闲超时（秒）",
    "非流式总超时（秒）",
    "连续失败阈值",
    "恢复成功阈值",
    "熔断冷却（秒）",
    "错误率（%）",
    "最小请求数",
];
pub fn inline(i18n: desktop_i18n::I18n, state: State<'_>) -> Element<'_, Message> {
    let mut body = column![
        text(i18n.__("队列有多个供应商时，请求失败后按队列顺序尝试。达到熔断阈值的供应商会被暂时跳过。"))
            .size(13)
            .style(settings_muted_text_style)
    ]
    .spacing(14);
    if state.loaded {
        let fields = state.fields;
        let editable = !state.busy;
        body = body
            .push(policy_group(i18n, i18n.__("重试设置"), &[0, 4], fields, editable))
            .push(policy_group(i18n, i18n.__("超时配置"), &[1, 2, 3], fields, editable))
            .push(policy_group(i18n, i18n.__("熔断器设置"), &[5, 6, 7, 8], fields, editable));
    }
    if let Some(error) = state.error {
        body = body.push(settings_error_banner(i18n.__(error).to_owned()));
    }
    if state.busy {
        return body.push(text(i18n.__("正在请求服务端…")).size(13).style(settings_muted_text_style)).into();
    }
    if !state.open {
        return body.into();
    }
    let reset = button(
        text(if state.loaded { i18n.__("重置") } else { i18n.__("重新读取") }).size(SETTINGS_CONTROL_TEXT_SIZE),
    )
    .padding([8, 14])
    .style(rounded_action_btn_style)
    .on_press(Event::Reload);
    let save = button(text(i18n.__("保存")).size(SETTINGS_CONTROL_TEXT_SIZE))
        .padding([8, 16])
        .style(primary_action_btn_style)
        .on_press_maybe(state.loaded.then_some(Event::Save));
    body.push(container(row![reset, save].spacing(8)).align_right(Length::Fill)).into()
}

fn policy_group<'a>(i18n: desktop_i18n::I18n,
    title: &'static str,
    indices: &'static [usize],
    fields: &'a [String; 9],
    editable: bool,
) -> Element<'a, Message> {
    let hints: [&str; 9] = [
        i18n.__("0–10 次"),
        i18n.__("1–120 秒"),
        i18n.__("0–600 秒，0 表示禁用"),
        i18n.__("60–1200 秒"),
        i18n.__("连续失败 1–20 次"),
        i18n.__("半开后成功 1–10 次"),
        i18n.__("0–300 秒"),
        "0–100%",
        i18n.__("5–100 次"),
    ];
    settings_panel(
        column![
            text(title).size(15),
            responsive(move |size| {
                let columns = (size.width as usize / 140).clamp(1, indices.len());
                let mut grid = column![].spacing(12);
                for chunk in indices.chunks(columns) {
                    let mut inputs = row![].spacing(12).align_y(Alignment::Start);
                    for &index in chunk {
                        inputs = inputs.push(
                            column![
                                settings_field_label(i18n.__(LABELS[index])),
                                settings_text_input("", &fields[index]).on_input_maybe(
                                    editable.then_some(move |value| Event::Edit(index, value))
                                ),
                                text(hints[index]).size(12).style(settings_muted_text_style),
                            ]
                            .spacing(5)
                            .width(Length::Fill),
                        );
                    }
                    grid = grid.push(inputs);
                }
                grid.into()
            })
            .height(Length::Shrink),
        ]
        .spacing(12),
    )
    .into()
}

pub fn overlay<'a>(i18n: desktop_i18n::I18n, state: State<'a>, base: Element<'a, Message>) -> Element<'a, Message> {
    if !state.open {
        return base;
    }
    let close = message(Event::Close);
    let header = settings_modal_header(
        text(i18n.__("高可用策略")).size(20),
        Some(settings_close_button(close.clone())),
    );
    let mut body = column![
        text(i18n.__("重试共享同一总尝试预算（首次发送 + 最大重试次数），不会逐层叠加。")),
        text(i18n.__("流空闲超时为 0 时仅关闭空闲计时器。保存后对新请求生效；范围由服务端校验。")),
    ]
    .spacing(12);
    if state.busy {
        body = body.push(text(i18n.__("正在请求服务端…")));
    }
    if state.loaded {
        for (index, label) in LABELS.iter().enumerate() {
            body = body.push(
                column![
                    settings_field_label(*label),
                    settings_text_input("", &state.fields[index]).on_input_maybe(
                        (!state.busy).then_some(move |value| message(Event::Edit(index, value)))
                    )
                ]
                .spacing(6),
            );
        }
    }
    if let Some(error) = state.error {
        body = body.push(settings_error_banner(i18n.__(error).to_owned()));
    }
    let save = button(text(i18n.__("保存")))
        .padding([8, 16])
        .style(primary_action_btn_style)
        .on_press_maybe((state.loaded && !state.busy).then(|| message(Event::Save)));
    let reload = button(text(i18n.__("重新读取")))
        .padding([8, 16])
        .style(rounded_action_btn_style)
        .on_press_maybe((!state.busy).then(|| message(Event::Reload)));
    let cancel = button(text(i18n.__("关闭")))
        .padding([8, 16])
        .style(rounded_action_btn_style)
        .on_press(close.clone());
    let card =
        settings_modal_layout(header, body, Some(row![reload, cancel, save].spacing(10).into()))
            .width(620)
            .height(660);
    settings_modal_overlay(Some(base), close, container(card).padding(12).center(Length::Fill))
}
