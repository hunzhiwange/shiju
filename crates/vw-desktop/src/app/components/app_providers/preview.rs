use super::super::system_settings_common::*;
use crate::app::{
    Message,
    message::app_providers::preview::{Event, message},
    state::app_providers::preview::{Display, Form, SCROLL_ID},
};
use iced::{
    Element, Length,
    widget::{button, column, row, text, text_editor},
};
use vw_gateway_client::vw_api_types::app_providers::{
    editor_merge::EditorChoice, operations::ProviderOperationPhase, preview::PreviewChangeKind,
};

fn action(
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

pub(super) fn overlay<'a>(form: &'a Form, base: Element<'a, Message>) -> Element<'a, Message> {
    let ready = form.reading.is_none() && form.save.is_none();
    let can_close = form.save.is_none();
    let close = if can_close { message(Event::Close) } else { Message::None };
    let header = settings_modal_header(
        column![
            settings_page_intro("配置预览与冲突确认", "预览由执行主机生成，显示内容已脱敏。"),
            text(&form.name).size(14).style(settings_muted_text_style),
        ]
        .spacing(4),
        settings_close_button(close.clone()),
    );
    let mut body = column![].spacing(12).width(Length::Fill);
    if form.reading.is_some() {
        body = body.push(settings_panel(text("正在读取配置预览…")));
    }
    if let Some(notice) = &form.notice {
        body = body.push(settings_panel(text(notice).size(14)));
    }
    if let Some(save) = &form.save {
        let summary = match save.progress.as_ref() {
            Some(progress) => {
                let phase = match progress.phase {
                    ProviderOperationPhase::Preparing => "正在检查配置",
                    ProviderOperationPhase::Publishing => "正在提交配置",
                    ProviderOperationPhase::Settling => "正在完成已开始的提交",
                    ProviderOperationPhase::Terminal => "正在确认结果",
                };
                if progress.total == 0 {
                    phase.into()
                } else {
                    format!("{phase} · {}/{}", progress.completed, progress.total)
                }
            }
            None => "正在提交保存请求".into(),
        };
        body = body.push(settings_panel(column![
            text(summary),
            text(if save.cancel_requested {
                "已请求取消，等待执行主机确认。"
            } else {
                ""
            })
            .size(13),
        ]));
    }
    if let Some(conflict) = &form.confirmation.conflict {
        let mut paths = column![
            text("配置冲突").size(17),
            text("请选择这些设置采用哪一方的改动。其他改动仍会保存。").size(14),
        ]
        .spacing(6);
        for path in &conflict.paths {
            paths = paths.push(text(path.join(" › ")).size(14));
        }
        body = body.push(settings_panel(paths));
    }
    if let Some(editor) = &form.confirmation.editor {
        let editing = ready && form.confirmation.conflict.is_none();
        let mut draft = text_editor(&form.draft)
            .height(Length::Fixed(120.))
            .padding(SETTINGS_CONTROL_PADDING)
            .size(SETTINGS_CONTROL_TEXT_SIZE)
            .style(settings_text_editor_style_for(editing));
        if editing {
            draft = draft.on_action(|action| message(Event::Edit(action)));
        }
        body = body.push(settings_panel(
            column![
                text(if editor.current {
                    "编辑 Claude 全局配置"
                } else {
                    "编辑已保存的 Claude 配置"
                })
                .size(16),
                text("填写 JSON 改动列表。每项用 path 指定键路径，用 value 指定设置或删除。")
                    .size(13),
                text(r#"[{"path":["alwaysThinkingEnabled"],"value":{"kind":"set","value":true}}]"#)
                    .size(13),
                text(r#"删除示例  {"path":["alwaysThinkingEnabled"],"value":{"kind":"remove"}}"#)
                    .size(13),
                draft,
            ]
            .spacing(6),
        ));
    }
    if let Some(view) = &form.view {
        match view {
            Display::Native(preview) => {
                let mut changes = column![text("预览差异").size(16)].spacing(6);
                if preview.changes.is_empty() {
                    changes = changes.push(text("没有配置差异。").size(14));
                }
                for change in &preview.changes {
                    let kind = match change.kind {
                        PreviewChangeKind::Added => "新增",
                        PreviewChangeKind::Removed => "删除",
                        PreviewChangeKind::Changed => "修改",
                    };
                    changes = changes
                        .push(text(format!("{kind} · {}", change.path.join(" › "))).size(14));
                }
                for path in &preview.inactive {
                    changes = changes.push(text(format!("未生效 · {}", path.join(" › "))).size(14));
                }
                body = body.push(settings_panel(changes));
                body = body.push(settings_panel(
                    column![text("投影配置").size(16), text(pretty(&preview.settings)).size(14),]
                        .spacing(6),
                ));
            }
            Display::Saved(settings) => {
                body = body.push(settings_panel(
                    column![text("已保存的供应商配置").size(16), text(pretty(settings)).size(14),]
                        .spacing(6),
                ));
            }
        }
    }
    if let Some(editor) = &form.confirmation.editor
        && !editor.current
    {
        body = body.push(settings_panel(
            column![text("可编辑的已保存配置").size(16), text(pretty(&editor.settings)).size(14),]
                .spacing(6),
        ));
    }

    let footer = if let Some(save) = &form.save {
        row![
            action("取消保存", Event::CancelSave, !save.cancel_requested, false),
            action("重试查询", Event::Check, !save.waiting, false),
        ]
    } else if form.confirmation.conflict.is_some() {
        row![
            action("取消", Event::Close, true, false),
            action("保留外部修改", Event::Save(EditorChoice::KeepTheirs), ready, false),
            action("采用我的修改", Event::Save(EditorChoice::KeepMine), ready, true),
        ]
    } else {
        row![
            action("关闭", Event::Close, true, false),
            action("重新预览", Event::Refresh, ready, false),
            action(
                "保存改动",
                Event::Save(EditorChoice::Refuse),
                ready && form.confirmation.editor.is_some(),
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
    .width(730)
    .height(655);
    settings_modal_overlay(Some(base), close, card)
}

fn pretty(value: &serde_json::Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_default()
}
