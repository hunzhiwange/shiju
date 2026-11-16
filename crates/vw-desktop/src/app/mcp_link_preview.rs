//! Untrusted launch payloads only become inert catalog drafts after explicit handoff.
use crate::app::{
    App, Message, deep_links,
    message::settings::{GeneralSettingsMessage, SettingsMessage, mcp_management_import},
};
use iced::{Element, Task};

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
    app.navigation_panels.system_settings_tab =
        crate::app::components::system_settings::SystemTab::Agents;
    app.agents_settings.active_config_tab = crate::app::state::AgentConfigTab::Mcp;
    let _ = deep_links::acknowledge(id);
    Task::none()
}

pub(crate) fn overlay<'a>(app: &App, base: Element<'a, Message>) -> Element<'a, Message> {
    let Some(link) = pending() else { return base };
    let close = Message::Settings(SettingsMessage::General(
        GeneralSettingsMessage::McpLinkRejected(link.id),
    ));
    let accept = Message::Settings(SettingsMessage::General(
        GeneralSettingsMessage::McpLinkPreview(link.id),
    ));
    desktop_navigation::view_layout::link_preview::mcp(
        base,
        &link.url,
        close,
        &app.apps_catalog.host,
        accept,
    )
}
