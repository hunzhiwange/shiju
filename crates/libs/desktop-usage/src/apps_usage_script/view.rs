use iced::{
    Alignment, Element, Length,
    widget::{Space, button, column, container, row, text},
};
use vw_api_apps_usage::apps_usage::UsageSnapshotQuery;

use super::{Editor, Event, Kind, Ready, send};
use super::Message;
use desktop_ui_controls::system_settings_common::*;
use crate::apps_usage_details;

pub fn modal_view<'a>(i18n: desktop_i18n::I18n,
    editor: &'a Editor,
    selection: &UsageSnapshotQuery,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    modal_view_with_base(i18n, editor, selection, base, |event| event)
}
pub fn modal_view_with_base<'a, M: Clone + 'static>(i18n: desktop_i18n::I18n, editor: &'a Editor, selection: &UsageSnapshotQuery, base: Element<'a, M>, map: impl Fn(Message) -> M + Copy + 'a) -> Element<'a, M> {
    let close = Message::Close;
    let header = settings_modal_header(
        settings_page_heading(i18n.__("查询脚本试运行"), i18n.__("试运行当前脚本草稿。")),
        settings_close_button(close.clone()),
    );
    let (body, run, running): (Element<'_, Message>, _, _) = match editor {
        Editor::Loading(_) => (text(i18n.__("正在读取脚本模板…")).into(), None, false),
        Editor::LoadFailed => (
            column![
                text(i18n.__("无法读取脚本模板，请检查网关连接和额度读取权限")),
                button(text(i18n.__("重试")))
                    .padding([8, 14])
                    .style(rounded_action_btn_style)
                    .on_press(Message::OpenScript),
            ]
            .spacing(10)
            .into(),
            None,
            false,
        ),
        Editor::Ready(ready) => fields(i18n, ready, selection),
    };
    let footer = row![
        Space::new().width(Length::Fill),
        button(text(if running { i18n.__("取消试运行") } else { i18n.__("关闭") }))
            .padding([8, 14])
            .style(rounded_action_btn_style)
            .on_press(if running { send(Event::Cancel) } else { close.clone() }),
        button(text(if running { i18n.__("正在试运行…") } else { i18n.__("试运行") }))
            .padding([8, 14])
            .style(primary_action_btn_style)
            .on_press_maybe(run),
    ]
    .spacing(10)
    .align_y(Alignment::Center);
    let card = settings_modal_layout(header, body, Some(footer.into()))
        .width(Length::Fill)
        .max_width(680)
        .height(570);
    settings_modal_overlay_with_escape(base, map(close), container(iced::Element::from(card).map(map)).padding(12))
}

fn fields<'a>(i18n: desktop_i18n::I18n,
    ready: &'a Ready,
    selection: &UsageSnapshotQuery,
) -> (Element<'a, Message>, Option<Message>, bool) {
    let draft = &ready.draft;
    let running = ready.pending.is_some();
    let template = settings_select(draft.kinds().collect::<Vec<_>>(), draft.selected, |kind| {
        send(Event::Select(kind))
    })
    .format_labels(|kind| i18n.__(&kind.to_string()).to_owned())
    .placeholder(i18n.__("请选择"))
    .disabled(running)
    .width(Length::Fill);
    let mut timeout = settings_text_input("10", &draft.timeout).width(Length::Fill);
    if !running {
        timeout = timeout.on_input(|value| send(Event::Timeout(value)));
    }
    let mut body = column![
        column![settings_field_label(i18n.__("脚本模板")), template].spacing(6),
        column![settings_field_label(i18n.__("请求超时（秒）")), timeout].spacing(6),
    ]
    .spacing(12)
    .width(Length::Fill);
    if let Some(kind) = draft.selected {
        let content = if kind == Kind::Custom { &draft.source } else { &draft.preview };
        let mut source = settings_text_editor(content).height(200);
        let caption = if kind == Kind::Custom { i18n.__("脚本源码") } else { i18n.__("内置脚本预览") };
        if kind == Kind::Custom {
            if !running {
                source = source.on_action(|action| send(Event::Edit(action)));
            }
            body = body.push(
                text(i18n.__("可使用 {{baseUrl}} 引用供应商地址。请勿在脚本中填写密钥。"))
                    .size(13)
                    .style(settings_muted_text_style),
            );
        } else {
            let mut key_ref =
                settings_text_input(i18n.__("引用 UUID"), &draft.key_ref).secure(true).width(Length::Fill);
            if !running {
                key_ref = key_ref.on_input(|value| send(Event::KeyRef(value)));
            }
            body = body.push(column![settings_field_label(i18n.__("API Key 引用")), key_ref].spacing(6));
        }
        body = body.push(column![settings_field_label(i18n.__(caption)), source].spacing(6));
    }

    let run = match draft.request(selection) {
        Ok(_) if !running => Some(send(Event::Run)),
        Ok(_) => None,
        Err(hint) => {
            body = body.push(text(i18n.__(hint)).size(13).style(settings_muted_text_style));
            None
        }
    };
    if !ready.packages.is_empty() {
        body = body.push(settings_field_label(i18n.__("试运行结果")));
        for package in &ready.packages {
            body = body.push(apps_usage_details::script_package_line(i18n, package).map(|_| unreachable!("package lines have no actions")));
        }
    }
    (body.into(), run, running)
}
