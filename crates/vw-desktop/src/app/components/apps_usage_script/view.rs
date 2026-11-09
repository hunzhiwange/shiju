use iced::{
    Alignment, Element, Length,
    widget::{Space, button, column, container, row, text},
};
use vw_gateway_client::vw_api_types::apps_usage::UsageSnapshotQuery;

use super::{Editor, Event, Kind, Ready, SettingsEvent, apps_usage_settings, send};
use crate::app::{
    Message,
    components::{apps_usage_details::script_package_line, system_settings_common::*},
};

pub(in crate::app::components) fn modal_view<'a>(
    editor: &'a Editor,
    selection: &UsageSnapshotQuery,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    let close = apps_usage_settings::send(SettingsEvent::Close);
    let header = settings_modal_header(
        settings_page_heading("查询脚本试运行", "在当前执行主机试运行草稿。结果仅在此弹窗显示。"),
        settings_close_button(close.clone()),
    );
    let (body, run, running): (Element<'_, Message>, _, _) = match editor {
        Editor::Loading(_) => (text("正在读取脚本模板…").into(), None, false),
        Editor::LoadFailed => (
            column![
                text("无法读取脚本模板，请检查执行主机连接和额度读取权限"),
                button(text("重试"))
                    .padding([8, 14])
                    .style(rounded_action_btn_style)
                    .on_press(apps_usage_settings::send(SettingsEvent::OpenScript)),
            ]
            .spacing(10)
            .into(),
            None,
            false,
        ),
        Editor::Ready(ready) => fields(ready, selection),
    };
    let footer = row![
        Space::new().width(Length::Fill),
        button(text(if running { "取消试运行" } else { "关闭" }))
            .padding([8, 14])
            .style(rounded_action_btn_style)
            .on_press(if running { send(Event::Cancel) } else { close.clone() }),
        button(text(if running { "正在试运行…" } else { "试运行" }))
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
    settings_modal_overlay_with_escape(base, close, container(card).padding(12))
}

fn fields<'a>(
    ready: &'a Ready,
    selection: &UsageSnapshotQuery,
) -> (Element<'a, Message>, Option<Message>, bool) {
    let draft = &ready.draft;
    let running = ready.pending.is_some();
    let template = settings_select(draft.kinds().collect::<Vec<_>>(), draft.selected, |kind| {
        send(Event::Select(kind))
    })
    .placeholder("请选择")
    .disabled(running)
    .width(Length::Fill);
    let mut timeout = settings_text_input("10", &draft.timeout).width(Length::Fill);
    if !running {
        timeout = timeout.on_input(|value| send(Event::Timeout(value)));
    }
    let mut body = column![
        column![settings_field_label("脚本模板"), template].spacing(6),
        column![settings_field_label("请求超时（秒）"), timeout].spacing(6),
    ]
    .spacing(12)
    .width(Length::Fill);
    if let Some(kind) = draft.selected {
        let content = if kind == Kind::Custom { &draft.source } else { &draft.preview };
        let mut source = settings_text_editor(content).height(200);
        let caption = if kind == Kind::Custom { "脚本源码" } else { "内置脚本预览" };
        if kind == Kind::Custom {
            if !running {
                source = source.on_action(|action| send(Event::Edit(action)));
            }
            body = body.push(
                text("可使用 {{baseUrl}} 引用供应商地址。请勿在脚本中填写密钥。")
                    .size(13)
                    .style(settings_muted_text_style),
            );
        } else {
            let mut key_ref =
                settings_text_input("引用 UUID", &draft.key_ref).secure(true).width(Length::Fill);
            if !running {
                key_ref = key_ref.on_input(|value| send(Event::KeyRef(value)));
            }
            body = body.push(column![settings_field_label("API Key 引用"), key_ref].spacing(6));
        }
        body = body.push(column![settings_field_label(caption), source].spacing(6));
    }

    let run = match draft.request(selection) {
        Ok(_) if !running => Some(send(Event::Run)),
        Ok(_) => None,
        Err(hint) => {
            body = body.push(text(hint).size(13).style(settings_muted_text_style));
            None
        }
    };
    if !ready.packages.is_empty() {
        body = body.push(settings_field_label("试运行结果"));
        for package in &ready.packages {
            body = body.push(script_package_line(package));
        }
    }
    (body.into(), run, running)
}
