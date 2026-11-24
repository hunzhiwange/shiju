use crate::app::message::settings::{
    mcp_management_import as import, mcp_management_resync as resync,
};
use crate::app::{App, Message};
use iced::Element;
pub(crate) fn panel(app: &App) -> Element<'_, Message> {
    desktop_settings_mcp::management_resync::panel(
        desktop_i18n::I18n::new(app.general_settings.desktop_locale),
        resync::visible(app),
    )
    .map(resync::message)
}
pub(crate) fn overlay<'a>(app: &'a App, dialog: Element<'a, Message>) -> Element<'a, Message> {
    let Some(state) = resync::visible(app).filter(|s| s.open) else {
        return dialog;
    };
    desktop_ui_controls::system_settings_common::settings_modal_overlay(
        Some(dialog),
        resync::message(resync::ResyncMessage::Close),
        desktop_settings_mcp::management_resync::card(
            desktop_i18n::I18n::new(app.general_settings.desktop_locale),
            state,
            &app.mcp_settings.bindings.selected,
            import::targets(app),
        )
        .map(resync::message),
    )
}

/// Third-party matrix uses the same ownership-aware confirmation and progress UI.
pub(crate) fn rewrite_panel(app: &App) -> Element<'_, Message> { panel(app) }
