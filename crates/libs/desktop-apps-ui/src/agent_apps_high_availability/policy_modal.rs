use desktop_ui_controls::system_settings_common::{*, help::settings_hover_help_label};
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
const FIELD_HELP: [&str; 9] = [
    "首次发送之外允许的重试次数，范围 0–10 次。填 0 只发送一次。",
    "等待首个有效流式输出的最长时间，范围 1–600 秒，内置默认 300 秒。",
    "两次流式输出之间允许的最长间隔，范围 0–600 秒，内置默认 120 秒。填 0 关闭空闲计时。",
    "等待非流式请求完整返回的最长时间，范围 60–1200 秒，内置默认 600 秒。",
    "供应商连续失败达到此次数时触发熔断，范围 1–20 次。",
    "冷却结束后，试探请求连续成功达到此次数时恢复供应商，范围 1–10 次。",
    "供应商熔断后，重新允许试探请求前的等待时间，范围 0–300 秒。",
    "达到最小请求数后，失败比例达到此值时触发熔断，范围 0–100%。",
    "启用错误率判断前，至少要记录的请求数，范围 5–100 次。",
];

pub fn overview(i18n: desktop_i18n::I18n) -> &'static str {
    i18n.__("启用故障转移后，确认请求尚未发出时，可按队列顺序尝试其他可用供应商。供应商熔断后会暂时跳过，冷却结束后允许试探请求。保存后的设置用于新请求。")
}

pub fn inline(i18n: desktop_i18n::I18n, state: State<'_>) -> Element<'_, Message> {
    let mut body = column![].spacing(14);
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
    settings_section_content(
        column![
            settings_group_header(text(title).size(15)),
            responsive(move |size| {
                let columns = (size.width as usize / 140).clamp(1, indices.len());
                let mut grid = column![].spacing(12);
                for chunk in indices.chunks(columns) {
                    let mut inputs = row![].spacing(12).align_y(Alignment::Start);
                    for &index in chunk {
                        inputs = inputs.push(policy_field(i18n, index, &fields[index], editable));
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

fn policy_field(
    i18n: desktop_i18n::I18n,
    index: usize,
    value: &str,
    editable: bool,
) -> Element<'_, Message> {
    column![
        settings_hover_help_label(
            settings_field_label(i18n.__(LABELS[index])),
            text(i18n.__(FIELD_HELP[index])).size(13).into(),
        ),
        settings_text_input("", value)
            .on_input_maybe(editable.then_some(move |value| Event::Edit(index, value))),
    ]
    .spacing(5)
    .width(Length::Fill)
    .into()
}

pub fn overlay<'a>(i18n: desktop_i18n::I18n, state: State<'a>, base: Element<'a, Message>) -> Element<'a, Message> {
    if !state.open {
        return base;
    }
    let close = message(Event::Close);
    let header = settings_modal_header(
        settings_hover_help_label(
            text(i18n.__("高可用策略")).size(20),
            text(overview(i18n)).size(13).into(),
        ),
        Some(settings_close_button(close.clone())),
    );
    let mut body = column![].spacing(12);
    if state.busy {
        body = body.push(text(i18n.__("正在请求服务端…")));
    }
    if state.loaded {
        for (index, value) in state.fields.iter().enumerate() {
            body = body.push(policy_field(i18n, index, value, !state.busy));
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
