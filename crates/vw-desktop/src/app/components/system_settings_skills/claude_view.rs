//! Credential-free Claude configuration controls using the shared modal layout.
use crate::app::{App, Message};
use crate::app::components::system_settings_common::*;
use crate::app::message::settings::skills::claude_view::{Event, message};
use iced::{Element, widget::{button, column, container, row, text}};
use vw_gateway_client::vw_api_types::agent_apps_skills::claude_api::*;

pub(super) fn view<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    let s = &app.skills_settings.claude_config;
    if !s.open { return base; }
    let status = match &s.status {
        None => "尚未读取配置",
        Some(ClaudeConfigStatus::Missing) => "配置文件不存在",
        Some(ClaudeConfigStatus::Invalid) => "配置无效，请修复后刷新",
        Some(ClaudeConfigStatus::Valid { managed: true, .. }) => "已启用受管标记",
        Some(ClaudeConfigStatus::Valid { primary_api_key_present: true, .. }) =>
            "存在外部凭据，标记控制已禁用；请先在配置来源处理冲突",
        Some(ClaudeConfigStatus::Valid { .. }) => "未启用受管标记",
    };
    let editable = !s.busy && matches!(s.status,
        Some(ClaudeConfigStatus::Missing) | Some(ClaudeConfigStatus::Valid { primary_api_key_present: false, .. })
        | Some(ClaudeConfigStatus::Valid { managed: true, .. }));
    let control = |label, event| {
        let enabled = if matches!(event, Event::Onboarding(_)) { !s.busy } else { editable };
        button(label).padding([8, 10]).style(rounded_action_btn_style)
            .on_press_maybe(enabled.then_some(message(event)))
    };
    let mut body = column![
        settings_field_label("配置标记"), text(status),
        text("仅显示凭据是否存在；受管标记不代表插件已安装或启用。"),
        row![control("启用受管标记", Event::Marker(ClaudeConfigAction::Enable)),
            control("清除受管标记", Event::Marker(ClaudeConfigAction::Disable))].spacing(8),
        settings_field_label("首次启动确认"),
        text("当前确认标记未读取。跳过确认和恢复确认仅在点击对应按钮后写入。"),
        row![control("跳过首次确认", Event::Onboarding(ClaudeConfigAction::Enable)),
            control("恢复首次确认", Event::Onboarding(ClaudeConfigAction::Disable))].spacing(8),
        text("外部配置或并发事务冲突时操作会拒绝，请刷新后处理来源配置。"),
    ].spacing(12);
    if s.busy { body = body.push(text("正在同步配置…")); }
    if let Some(notice) = &s.notice { body = body.push(text(notice)); }
    if let Some(error) = &s.error { body = body.push(settings_error_banner(error.clone())); }
    let close = message(Event::Close);
    let header = settings_modal_header(text("Claude 配置控制").size(20), settings_close_button(close.clone()));
    let footer = row![
        button("刷新").padding([8, 10]).style(rounded_action_btn_style)
            .on_press_maybe((!s.busy).then_some(message(Event::Refresh))),
        button("关闭").padding([8, 10]).style(rounded_action_btn_style)
            .on_press_maybe((!s.busy).then_some(close.clone())),
    ].spacing(8);
    let card = settings_modal_layout(header, body, Some(footer.into()))
        .style(super::modal_card_style).width(760).height(580);
    settings_modal_overlay(Some(base), close, container(card).padding(12))
}
