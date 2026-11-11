//! Pi directory modal: native defaults are display-only.
use crate::app::{
    Message,
    components::system_settings_common::*,
    message::settings::aggregation_pi::{Event, Form, message},
};
use iced::{
    Element, Length,
    widget::{button, column, container, row, text, text_editor, text_input},
};

pub(crate) fn overlay<'a>(form: &'a Form, base: Element<'a, Message>) -> Element<'a, Message> {
    let ready = form.pending.is_none();
    let action = |label: &'static str, event: Event, enabled: bool| {
        button(text(label))
            .padding([8, 14])
            .style(rounded_action_btn_style)
            .on_press_maybe(enabled.then_some(message(event)))
    };
    let mut body = column![
        text("原生默认项由 Pi 管理。此处只编辑单个供应商的模型声明；不会修改账号或活跃模型。")
            .size(13)
            .style(settings_muted_text_style)
    ]
    .spacing(12)
    .width(Length::Fill);
    if let Some(error) = form.error {
        body = body.push(settings_error_banner(error));
    }
    if let Some(state) = &form.snapshot {
        body = body.push(settings_panel(
            column![
                settings_field_label("原生默认供应商（只读）"),
                text(state.default_provider_id.as_deref().unwrap_or("未配置")),
                settings_field_label("原生默认模型（只读）"),
                text(state.default_model_id.as_deref().unwrap_or("未配置")),
            ]
            .spacing(6),
        ));
        let mut members = column![settings_field_label("可管理目录条目")].spacing(8);
        for id in &state.members {
            members = members.push(
                row![
                    text(id).width(Length::Fill),
                    action("删除条目", Event::ReviewRemove(id.clone()), ready)
                ]
                .spacing(8),
            );
        }
        if state.members.is_empty() {
            members = members.push(text("没有显式模型条目。原生默认项仍可能有效。").size(13));
        }
        body = body.push(settings_panel(members));
    } else {
        body = body.push(text(if ready {
            "请刷新目录后再保存。"
        } else {
            "正在读取或更新执行主机…"
        }));
    }
    let input = |hint: &'static str, value: &'a str, event: fn(String) -> Event| {
        text_input(hint, value)
            .padding(SETTINGS_CONTROL_PADDING)
            .size(SETTINGS_CONTROL_TEXT_SIZE)
            .style(settings_text_input_style)
            .on_input_maybe(ready.then_some(move |value| message(event(value))))
    };
    let mut editor = text_editor(&form.declaration)
        .height(Length::Fixed(240.))
        .padding(SETTINGS_CONTROL_PADDING)
        .size(SETTINGS_CONTROL_TEXT_SIZE)
        .style(settings_text_editor_style_for(ready));
    if ready {
        editor = editor.on_action(|action| message(Event::Declaration(action)));
    }
    body = body.push(settings_panel(column![
        settings_field_label("供应商条目 ID"), input("供应商 ID", &form.id, Event::Id),
        settings_field_label("名称"), input("显示名称", &form.name, Event::Name),
        settings_field_label("单条模型声明（JSON）"),
        text("已有 ID 将替换该条目的公开模型声明。新增需提供 baseUrl、api、models 及模型限制。凭据和未知字段由执行端保留；不填写密钥。").size(13).style(settings_muted_text_style), editor,
    ].spacing(8)));
    if let Some(id) = &form.removing {
        body = body.push(settings_panel(
            column![
                text("确认从 Pi 目录删除以下条目？原生账号与默认项保持由 Pi 管理。"),
                text(id),
                row![
                    action("取消删除", Event::CancelRemove, ready),
                    action("确认删除", Event::ConfirmRemove, ready && form.snapshot.is_some())
                ]
                .spacing(8)
            ]
            .spacing(8),
        ));
    }
    let close = message(Event::Close);
    let header = settings_modal_header(
        text("Pi 模型目录").size(20),
        settings_close_button(if ready { close.clone() } else { Message::None }),
    );
    let footer = row![
        action("刷新", Event::Refresh, ready),
        action("关闭", Event::Close, ready),
        button(text("保存单条")).padding([8, 16]).style(primary_action_btn_style).on_press_maybe(
            (ready && form.snapshot.is_some() && form.removing.is_none())
                .then_some(message(Event::Save))
        )
    ]
    .spacing(10);
    let card = settings_modal_layout(header, body, Some(footer.into())).width(760).height(650);
    settings_modal_overlay(
        Some(base),
        if ready { close } else { Message::None },
        container(card).padding(12),
    )
}
