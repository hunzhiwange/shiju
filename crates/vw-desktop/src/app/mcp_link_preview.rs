//! Untrusted launch payloads only become inert catalog drafts after explicit handoff.
use crate::app::{
    App, Message, deep_links,
    message::settings::{GeneralSettingsMessage, SettingsMessage, mcp_management_import},
};
use iced::{Element, Task};

use desktop_platform::mcp_link_preview::{parse, pending};

pub(crate) fn handoff(app: &mut App, id: u64) -> Task<Message> {
    let Some(link) = pending().filter(|l| l.id == id) else { return Task::none() };
    let Some(context) = crate::app::message::settings::mcp_management_list::context(app) else {
        return app.show_localized_toast(
            crate::app::state::ToastKind::Error,
            "请先连接目标网关",
            vec![],
        );
    };
    let drafts = match parse(&link.url, &context.host) {
        Ok(drafts) => drafts,
        Err(error) => {
            return app.show_error_toast(
                desktop_i18n::I18n::new(app.general_settings.desktop_locale).__(&error),
            );
        }
    };
    if app.mcp_settings.import.is_some() {
        return app.show_localized_toast(
            crate::app::state::ToastKind::Error,
            "请先关闭已有 MCP 导入弹窗",
            vec![],
        );
    }
    let navigation = crate::app::components::third_party_settings::navigate(
        app,
        desktop_ui_controls::settings_navigation::ThirdPartyPage::Mcp,
    );
    if app.apps_catalog.third_party_page
        != desktop_ui_controls::settings_navigation::ThirdPartyPage::Mcp
    {
        return navigation;
    }
    if !mcp_management_import::open_link(app, drafts) {
        return app.show_localized_toast(
            crate::app::state::ToastKind::Error,
            "请检查网关连接并关闭已有 MCP 导入弹窗",
            vec![],
        );
    }
    app.navigation_tabs.screen = crate::app::Screen::ThirdPartyAgents;
    app.navigation_panels.show_system_settings = false;
    let _ = deep_links::acknowledge(id);
    navigation
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
        desktop_i18n::I18n::new(app.general_settings.desktop_locale),
        close,
        local_app_auth::HOST_ID,
        accept,
    )
}
