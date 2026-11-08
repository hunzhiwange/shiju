use super::super::system_settings_common::*;
use super::list::Help;
use crate::app::{
    Message,
    message::app_providers::env::{Event, message},
    state::app_providers::env::{
        Action, Form, MAX_SELECTIONS, Operation, SCROLL_ID, cleanup_selection,
    },
};
use iced::{
    Element, Length,
    widget::{button, checkbox, column, row, text, text_input},
};
use vw_gateway_client::vw_api_types::app_providers::{
    env_diagnose::EnvSource, env_restore::EnvRestoreStatus,
};

pub(super) fn overlay<'a>(
    form: &'a Form,
    help: Help<'_>,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    let ready = form.busy.is_none() && form.confirmation.is_none();
    let close = if form.writing() { Message::None } else { message(Event::Close) };
    let header = settings_modal_header(
        help.heading("环境诊断", "检查执行主机上的静态 Shell 定义和继承环境。", 28),
        settings_close_button(close.clone()),
    );

    let mut body = column![text("清理前会创建备份。").size(13).style(settings_muted_text_style)]
        .spacing(12)
        .width(Length::Fill);
    if let Some(error) = form.error {
        body = body.push(settings_error_banner(error));
    }
    if let Some(operation) = form.busy {
        let label = match operation {
            Operation::Diagnose => "正在读取环境诊断…",
            Operation::Write(Action::Clean) => "正在备份并清理所选定义…",
            Operation::Write(Action::Restore) => "正在恢复环境备份…",
        };
        body = body.push(settings_panel(text(label)));
    }
    if let Some(action) = form.confirmation {
        let notice = match action {
            Action::Clean => format!("确认备份并清理所选的 {} 项定义？", form.selected.len()),
            Action::Restore => "确认恢复此备份中的定义？文件有外部改动时会保留当前内容。".into(),
        };
        body = body.push(settings_panel(text(notice)));
    }

    let mut backup = column![
        help.field("备份编号", "清理后会自动填入编号。可保留编号供日后恢复。"),
        text_input("输入备份编号", &form.backup_id)
            .padding(SETTINGS_CONTROL_PADDING)
            .size(SETTINGS_CONTROL_TEXT_SIZE)
            .style(settings_text_input_style)
            .on_input_maybe(ready.then_some(|id| message(Event::BackupId(id)))),
    ]
    .spacing(6);
    for item in &form.restored {
        let status = match item.status {
            EnvRestoreStatus::Restored => "已恢复",
            EnvRestoreStatus::AlreadyRestored => "已恢复过",
            EnvRestoreStatus::NotCleaned => "未被清理",
            EnvRestoreStatus::Conflict => "有外部改动，已保留当前内容",
            EnvRestoreStatus::Unavailable => "无法读取文件",
        };
        backup = backup.push(text(format!("{} · {status}", item.file.filename())).size(14));
    }
    body = body.push(settings_panel(backup));

    let mut entries = column![settings_field_label("环境定义")].spacing(10);
    if form.entries.is_empty() && form.busy.is_none() && form.error.is_none() {
        entries = entries.push(text("没有发现相关的环境定义。").size(14));
    }
    if form.selected.len() > MAX_SELECTIONS {
        entries = entries.push(text(format!("每次最多可清理 {MAX_SELECTIONS} 项，请减少选择。")));
    }
    for (index, entry) in form.entries.iter().enumerate() {
        let source = match entry.source {
            EnvSource::UserShell { file, line } => {
                format!("用户文件 · {} 第 {line} 行", file.filename())
            }
            EnvSource::SystemShell { file, line } => {
                format!("系统文件 · {} 第 {line} 行", file.filename())
            }
            EnvSource::ProcessInherited => "执行进程继承的环境".into(),
        };
        let details = if cleanup_selection(entry).is_some() {
            column![
                checkbox(form.selected.contains(&index))
                    .label(entry.variable.as_str())
                    .style(settings_checkbox_style)
                    .on_toggle_maybe(
                        ready.then_some(move |value| message(Event::Toggle(index, value)))
                    ),
                text(source).size(13).style(settings_muted_text_style),
            ]
        } else {
            column![
                text(&entry.variable).size(14),
                text(format!("{source} · 只读")).size(13).style(settings_muted_text_style),
            ]
        };
        entries = entries.push(settings_panel(details.spacing(4)));
    }
    body = body.push(entries);

    let footer = if let Some(action) = form.confirmation {
        row![
            action_button("取消", Event::Cancel, true, false),
            action_button(
                match action {
                    Action::Clean => "确认清理",
                    Action::Restore => "确认恢复",
                },
                Event::Confirm,
                true,
                true,
            )
        ]
    } else {
        row![
            action_button("关闭", Event::Close, !form.writing(), false),
            action_button("重新诊断", Event::Refresh, ready, false),
            action_button(
                "恢复备份",
                Event::Review(Action::Restore),
                ready && !form.backup_id.trim().is_empty(),
                false,
            ),
            action_button(
                "清理所选项",
                Event::Review(Action::Clean),
                ready && form.can_clean(),
                true
            ),
        ]
    };
    let card = settings_modal_layout_with_scroll_id(
        header,
        body,
        Some(footer.spacing(10).into()),
        Some(SCROLL_ID),
    )
    .width(660)
    .height(610);
    settings_modal_overlay(Some(base), close, card)
}

fn action_button(
    label: &'static str,
    event: Event,
    enabled: bool,
    primary: bool,
) -> Element<'static, Message> {
    button(text(label))
        .padding([8, 14])
        .style(if primary { primary_action_btn_style } else { rounded_action_btn_style })
        .on_press_maybe(enabled.then(|| message(event)))
        .into()
}
