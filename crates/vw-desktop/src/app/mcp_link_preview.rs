//! Untrusted launch payloads only become inert global configuration drafts after explicit handoff.
use crate::app::{
    App, Message, deep_links,
    message::settings::{GeneralSettingsMessage, SettingsMessage, mcp_management_form},
};
use iced::{Element, Task};

use desktop_platform::mcp_link_preview::{parse, pending};

pub(crate) fn handoff(app: &mut App, id: u64) -> Task<Message> {
    let Some(link) = pending().filter(|l| l.id == id) else { return Task::none() };
    let Some(context) = mcp_management_form::context() else {
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
    if app.mcp_settings.modal_open() {
        return app.show_localized_toast(
            crate::app::state::ToastKind::Error,
            "请先关闭已有 MCP 导入弹窗",
            vec![],
        );
    }
    let navigation = crate::app::components::system_settings_mcp::open(app);
    if !crate::app::components::system_settings_mcp::is_visible(app) {
        return navigation;
    }
    let invalid = desktop_i18n::I18n::new(app.general_settings.desktop_locale).__("无效项");
    let candidates = drafts
        .into_iter()
        .map(|item| {
            let (id, connection) = match item {
                Ok(draft) => (draft.id, Ok(draft.connection)),
                Err(error) => (invalid.to_owned(), Err(error)),
            };
            desktop_settings_mcp::management_form::paste::Candidate { id, connection }
        })
        .collect();
    let draft =
        mcp_management_form::update(app, mcp_management_form::FormMessage::OpenLink(candidates));
    if app.mcp_settings.connection_form.is_some() {
        let _ = deep_links::acknowledge(id);
    }
    Task::batch([navigation, draft])
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
