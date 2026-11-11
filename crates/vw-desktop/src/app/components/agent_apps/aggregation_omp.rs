//! OMP native directory modal; all defaults and profile information are read-only.
use crate::app::components::system_settings_common::*;
use crate::app::{
    Message,
    message::settings::aggregation_omp::{Event, Form, message},
};
use iced::{
    Element, Length,
    widget::{button, column, container, row, text, text_editor, text_input},
};
fn action(label: &str, event: Event, enabled: bool) -> Element<'_, Message> {
    button(text(label))
        .padding([8, 14])
        .style(rounded_action_btn_style)
        .on_press_maybe(enabled.then_some(message(event)))
        .into()
}
pub(crate) fn overlay<'a>(form: &'a Form, base: Element<'a, Message>) -> Element<'a, Message> {
    let ready = form.pending.is_none();
    let close = message(Event::Close);
    let header = settings_modal_header(
        text("OMP 独立模型条目").size(20),
        settings_close_button(close.clone()),
    );
    let mut body=column![text("活动 profile 与默认模型只读。条目按执行主机当前 profile 保存。登录和 MCP 请使用各自入口。修改已有条目时请提供完整供应商声明。")].spacing(12);
    if let Some(error) = &form.error {
        body = body.push(settings_error_banner(error.as_str()));
    }
    if let Some(state) = &form.state {
        body = body.push(settings_panel(
            column![
                settings_field_label("活动 profile"),
                text(state.profile.as_deref().unwrap_or("默认 profile")),
                settings_field_label("默认供应商（只读）"),
                text(state.default_provider_id.as_deref().unwrap_or("未设置")),
                settings_field_label("默认模型（只读）"),
                text(state.default_model_id.as_deref().unwrap_or("未设置")),
            ]
            .spacing(8),
        ));
        if state.members.is_empty() {
            body = body.push(text("暂无独立条目；内置默认模型仍可生效。"));
        }
        for id in &state.members {
            body = body.push(settings_panel(
                row![
                    text(id).width(Length::Fill),
                    action("移除", Event::ReviewRemove(id.clone()), ready)
                ]
                .spacing(10)
                .align_y(iced::Alignment::Center),
            ));
        }
    } else {
        body = body.push(text(if ready {
            "目录未就绪，请刷新。"
        } else {
            "正在读取目录…"
        }));
    }
    if let Some(id) = &form.removing {
        body = body.push(settings_panel(
            column![
                text("确认移除以下独立条目？默认模型保持只读。"),
                text(id),
                row![
                    action("取消移除", Event::CancelRemove, ready),
                    action("确认移除", Event::ConfirmRemove, ready)
                ]
                .spacing(10)
            ]
            .spacing(8),
        ));
    }
    let input = |hint: &'static str, value: &'a str, event: fn(String) -> Event| {
        text_input(hint, value)
            .padding(SETTINGS_CONTROL_PADDING)
            .size(SETTINGS_CONTROL_TEXT_SIZE)
            .style(settings_text_input_style)
            .on_input_maybe(ready.then_some(move |s| message(event(s))))
    };
    let mut editor = text_editor(&form.declaration)
        .height(260)
        .padding(SETTINGS_CONTROL_PADDING)
        .size(SETTINGS_CONTROL_TEXT_SIZE)
        .style(settings_text_editor_style_for(ready));
    if ready {
        editor = editor.on_action(|a| message(Event::Declaration(a)));
    }
    body = body.push(settings_panel(
        column![
            settings_field_label("条目 ID"),
            input("供应商 ID", &form.id, Event::Id),
            settings_field_label("名称"),
            input("显示名称", &form.name, Event::Name),
            settings_field_label("供应商模型声明（YAML）"),
            editor
        ]
        .spacing(8),
    ));
    let actions = row![
        action("刷新目录", Event::Refresh, ready),
        action("取消", Event::Close, ready),
        button(text(if ready { "保存条目" } else { "处理中…" }))
            .padding([8, 14])
            .style(primary_action_btn_style)
            .on_press_maybe((ready && form.state.is_some()).then_some(message(Event::Save)))
    ]
    .spacing(10);
    let card = settings_modal_layout(header, body, Some(actions.into())).width(800).height(720);
    settings_modal_overlay(Some(base), close, container(card).padding(12))
}

#[cfg(test)]
#[path = "aggregation_omp_tests.rs"]
mod tests;
