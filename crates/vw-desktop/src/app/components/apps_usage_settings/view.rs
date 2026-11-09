use iced::{
    Alignment, Element, Length,
    widget::{Space, button, checkbox, column, container, row, text},
};

use super::{Draft, Event, Field, Kind, Phase, live_scope, send};
use crate::app::{App, Message, components::system_settings_common::*};
use vw_gateway_client::AppsUsageDisplay;

pub(crate) fn overlay<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    let Some(modal) = app
        .app_providers
        .usage
        .settings
        .modal
        .as_ref()
        .filter(|modal| live_scope(app).as_ref() == Some(&modal.scope))
    else {
        return base;
    };
    modal_view(&modal.phase, &modal.selection, base)
}

pub(super) fn modal_view<'a>(
    phase: &'a Phase,
    selection: &vw_gateway_client::vw_api_types::apps_usage::UsageSnapshotQuery,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    if let Phase::Script(script) = phase {
        return super::apps_usage_script::modal_view(script, selection, base);
    }
    let close = send(Event::Close);
    let (content, save, saving): (Element<'_, Message>, _, _) = match phase {
        Phase::Script(_) => unreachable!(),
        Phase::Loading => (text("正在读取额度设置…").into(), None, false),
        Phase::LoadFailed => (
            column![
                text("额度设置读取失败"),
                button(text("重试"))
                    .on_press(send(Event::Open(selection.app_id, selection.provider_id.clone())))
                    .padding([8, 14])
                    .style(rounded_action_btn_style),
            ]
            .spacing(10)
            .into(),
            None,
            false,
        ),
        Phase::Editing { draft, saving } => {
            let validation = draft.settings();
            let mut body = fields(draft, *saving);
            if let Err(hint) = validation {
                body = body.push(text(hint).size(13).style(settings_muted_text_style));
            }
            (body.into(), (validation.is_ok() && !saving).then(|| send(Event::Save)), *saving)
        }
    };
    let close_button =
        if saving { Space::new().into() } else { settings_close_button(close.clone()) };
    let header = settings_modal_header(
        settings_page_heading("原生额度查询设置", "为当前供应商选择查询类型、自动刷新和显示方式。"),
        close_button,
    );
    let footer = row![
        button(text("试运行脚本")).padding([8, 14]).style(rounded_action_btn_style).on_press_maybe(
            (!saving && !matches!(phase, Phase::Loading)).then(|| send(Event::OpenScript)),
        ),
        Space::new().width(Length::Fill),
        button(text("取消"))
            .padding([8, 14])
            .style(rounded_action_btn_style)
            .on_press_maybe((!saving).then_some(close.clone())),
        button(text(if saving { "正在保存…" } else { "保存" }))
            .padding([8, 14])
            .style(primary_action_btn_style)
            .on_press_maybe(save),
    ]
    .spacing(10)
    .align_y(Alignment::Center);
    let card = settings_modal_layout(header, content, Some(footer.into()))
        .width(Length::Fill)
        .max_width(560)
        .height(550);
    settings_modal_overlay(Some(base), close, container(card).padding(12))
}

fn fields(draft: &Draft, saving: bool) -> iced::widget::Column<'_, Message> {
    let input = |label, placeholder, field, value| {
        let mut input = settings_text_input(placeholder, value)
            .secure(matches!(field, Field::AccessKey | Field::SecretKey))
            .width(Length::Fill);
        if !saving {
            input = input.on_input(move |value| send(Event::Edit(field, value)));
        }
        column![settings_field_label(label), input].spacing(6)
    };
    let mut enabled = checkbox(draft.enabled).label("启用额度查询").style(settings_checkbox_style);
    let mut display = checkbox(draft.display == AppsUsageDisplay::Used)
        .label("显示已用额度")
        .style(settings_checkbox_style);
    if !saving {
        enabled = enabled.on_toggle(|value| send(Event::Enabled(value)));
        display = display.on_toggle(|value| send(Event::DisplayUsed(value)));
    }
    let mut fields = column![
        column![
            settings_field_label("查询类型"),
            settings_select(
                [
                    Kind::Balance,
                    Kind::CodingPlan,
                    Kind::ZhipuTeam,
                    Kind::Volcengine,
                    Kind::OfficialSubscription
                ],
                draft.kind,
                |kind| send(Event::Select(kind)),
            )
            .placeholder("请选择")
            .disabled(saving)
            .width(Length::Fill),
        ]
        .spacing(6),
        enabled,
    ]
    .spacing(12);
    match draft.kind {
        Some(Kind::ZhipuTeam) => {
            fields = fields
                .push(input("团队组织 ID", "组织 ID", Field::Organization, &draft.organization))
                .push(input("团队项目 ID", "项目 ID", Field::Project, &draft.project));
        }
        Some(Kind::Volcengine) => {
            fields = fields
                .push(
                    text("请使用管理员登记的不透明引用。")
                        .size(13)
                        .style(settings_muted_text_style),
                )
                .push(input("AK 引用", "引用 UUID", Field::AccessKey, &draft.access_key))
                .push(input("SK 引用", "引用 UUID", Field::SecretKey, &draft.secret_key));
        }
        _ => {}
    }
    fields
        .push(input("自动刷新间隔（分钟）", "0 表示关闭", Field::Interval, &draft.interval))
        .push(display)
}
