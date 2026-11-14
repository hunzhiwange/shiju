use super::super::system_settings_common::*;
use super::list::Help;
use crate::app::{
    Message,
    message::app_providers::paths::{Event, message},
    state::app_providers::paths::Form,
};
use iced::{
    Element, Length,
    widget::{button, column, row, text, text_input},
};
use vw_gateway_client::vw_api_types::app_providers::paths::ConfigPathState;

pub(super) fn overlay<'a>(
    form: &'a Form,
    help: Help<'_>,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    let close = if form.saving { Message::None } else { message(Event::Close) };
    let header = settings_modal_header(
        help.heading("配置目录", "查看 Claude Code 在执行主机上的配置文件。", 28),
        settings_close_button(close.clone()),
    );
    let ready = form.pending.is_none();
    let editable = ready;
    let mut body = column![].spacing(12).width(Length::Fill);
    if let Some(error) = form.error {
        body = body.push(settings_error_banner(error));
    }
    if !ready {
        body = body.push(text(if form.saving {
            "正在保存目录…"
        } else {
            "正在读取目录…"
        }));
    }
    if let Some(paths) = &form.paths {
        body = body.push(settings_panel(
            column![settings_field_label("当前配置目录"), text(&paths.directory)].spacing(8),
        ));
    }
    let input = text_input("留空使用默认目录", &form.draft)
        .padding(SETTINGS_CONTROL_PADDING)
        .size(SETTINGS_CONTROL_TEXT_SIZE)
        .style(settings_text_input_style);
    body = body.push(settings_panel(column![
        settings_field_label("覆盖目录"),
        if editable { input.on_input(|value| message(Event::Draft(value))) } else { input },
        action("选择本机目录…", Event::PickFolder, editable && form.local_open, false),
        text(if form.local_open {
            "取消选择会保留当前输入；选择后点击保存，由网关验证目录。"
        } else {
            "请填写执行主机上的目录，保存时仅由网关验证；远端主机不会打开本机选择器。"
        }).size(13).style(settings_muted_text_style),
        text("目录须位于执行主机的授权用户目录内，且已经存在。存在生效供应商或未完成事务时不可更改。")
            .size(13).style(settings_muted_text_style),
    ].spacing(8)));
    if let Some(paths) = &form.paths {
        let mut files = column![settings_field_label("配置文件")].spacing(8);
        for file in &paths.files {
            let state = match file.state {
                ConfigPathState::Present => "存在",
                ConfigPathState::Absent => "尚未创建",
                ConfigPathState::Inaccessible => "无法访问",
            };
            files = files.push(text(format!("{state}　{}", file.path)));
        }
        body = body.push(settings_panel(files));
    }
    let footer = row![
        action("取消", Event::Close, !form.saving, false),
        action("重新读取", Event::Refresh, ready, false),
        action(
            "在本机打开",
            Event::OpenFolder,
            editable && form.local_open && form.paths.is_some(),
            false
        ),
        action("保存", Event::Save, editable, true),
    ]
    .spacing(10);
    let card = settings_modal_layout(header, body, Some(footer.into())).width(650).height(560);
    settings_modal_overlay(Some(base), close, card)
}

fn action<'a>(
    label: &'static str,
    event: Event,
    enabled: bool,
    primary: bool,
) -> Element<'a, Message> {
    let button = button(text(label)).padding([8, 13]).style(if primary {
        primary_action_btn_style
    } else {
        rounded_action_btn_style
    });
    if enabled { button.on_press(message(event)) } else { button }.into()
}
