//! Untrusted launch payloads only become inert catalog drafts after explicit handoff.
use crate::app::{App, Message, deep_links, message::settings::{GeneralSettingsMessage, SettingsMessage, mcp_management_import}};
use iced::{Element, Task, widget::{button, row, text}};

use desktop_platform::mcp_link_preview::{parse, pending};

pub(crate) fn handoff(app: &mut App, id: u64) -> Task<Message> {
    let Some(link) = pending().filter(|l| l.id == id) else { return Task::none() };
    let drafts = match parse(&link.url, &app.apps_catalog.host) {
        Ok(drafts) => drafts,
        Err(error) => return app.show_error_toast(error),
    };
    if !mcp_management_import::open_link(app, drafts) {
        return app.show_error_toast("请先连接目标主机并关闭已有 MCP 导入弹窗");
    }
    app.system_settings_tab = crate::app::components::system_settings::SystemTab::Agents;
    app.agents_settings.active_config_tab = crate::app::state::AgentConfigTab::Mcp;
    let _ = deep_links::acknowledge(id);
    Task::none()
}

pub(crate) fn overlay<'a>(app: &App, base: Element<'a, Message>) -> Element<'a, Message> {
    use crate::app::components::system_settings_common::*;
    let Some(link) = pending() else { return base };
    let close = Message::Settings(SettingsMessage::General(GeneralSettingsMessage::McpLinkRejected(link.id)));
    let parsed = parse(&link.url, &app.apps_catalog.host);
    let valid = parsed.is_ok();
    let body = desktop_platform::mcp_link_preview::body(parsed).map(|_| unreachable!());
    let footer = row![button("继续预览").padding([8, 10]).style(primary_action_btn_style)
        .on_press_maybe(valid.then_some(Message::Settings(SettingsMessage::General(GeneralSettingsMessage::McpLinkPreview(link.id))))),
        button("拒绝链接").padding([8, 10]).style(rounded_action_btn_style).on_press(close.clone())].spacing(8);
    settings_modal_overlay(Some(base), close.clone(), settings_modal_layout(
        settings_modal_header(text("MCP 链接预览"), settings_close_button(close)), body, Some(footer.into())
    ).style(settings_modal_card_style).width(640).height(360))
}
