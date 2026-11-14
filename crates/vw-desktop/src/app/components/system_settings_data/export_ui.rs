use crate::app::{App, Message};
use crate::app::message::settings::data_management::export_ui::{self, Event, message};
use crate::app::components::system_settings_common::*;
use iced::{Element};
use iced::widget::{button, checkbox, column, row, text};
use vw_gateway_client::vw_api_types::agent_apps::data_management::DataCategory;

pub(crate) fn view(app: &App) -> Element<'_, Message> {
    button(text("导出脱敏配置")).padding([8, 14]).style(rounded_action_btn_style)
        .on_press_maybe(export_ui::identity(app).is_some().then_some(message(Event::Open))).into()
}

pub(crate) fn overlay<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    if !export_ui::visible(app) { return base; }
    let state = &app.apps_catalog.data_export;
    let close = message(Event::Close);
    let header = settings_modal_header(text("导出脱敏配置").size(18), settings_close_button(close.clone()));
    let mut body = column![
        text("可迁移导出排除凭据、主机绝对路径与系统集成；它不等同于完整本机回退备份。"),
        text("授权主体 ID（由网关重新鉴权）"),
        settings_text_input("填写当前授权主体 ID", &state.principal)
            .on_input_maybe((!state.busy).then_some(|value| message(Event::Principal(value)))),
        text("选择类别；执行主机不支持的类别会明确报错。"),
    ].spacing(12);
    for (category, label) in [
        (DataCategory::Providers, "供应商"), (DataCategory::Mcp, "MCP"),
        (DataCategory::Skills, "技能"), (DataCategory::Rules, "规则"),
        (DataCategory::Profiles, "组合"), (DataCategory::Preferences, "偏好"),
        (DataCategory::Usage, "用量"),
    ] {
        body = body.push(checkbox(state.categories.contains(&category)).label(label)
            .text_size(SETTINGS_CONTROL_TEXT_SIZE).style(settings_checkbox_style)
            .on_toggle_maybe((!state.busy).then_some(move |selected| message(Event::Select(category, selected)))));
    }
    let footer = row![
        button(text("取消")).padding([8, 14]).style(rounded_action_btn_style)
            .on_press_maybe((!state.busy).then_some(close.clone())),
        button(text(if state.busy { "正在导出或保存…" } else { "导出并选择保存位置" }))
            .padding([8, 14]).style(primary_action_btn_style)
            .on_press_maybe((!state.busy && !state.categories.is_empty() && !state.principal.trim().is_empty()).then_some(message(Event::Export))),
    ].spacing(10);
    settings_modal_overlay(Some(base), close, settings_modal_layout(header, body, Some(footer.into())).width(600).height(620))
}
