//! Desktop connection-draft projection and message mapping.
use crate::app::Message;
use crate::app::message::settings::mcp_management_form::{Draft, FormMessage, message};
use desktop_settings_mcp::connection as ui;
use iced::Element;
pub(crate) fn connection_form_panel(
    i18n: desktop_i18n::I18n,
    draft: &Draft,
) -> Element<'_, Message> {
    desktop_settings_mcp::management_form::panel(i18n, draft).map(|event| {
        message(match event {
            ui::FormMessage::IncludeGlobal(value) => FormMessage::IncludeGlobal(value),
            ui::FormMessage::GlobalEnabled(value) => FormMessage::GlobalEnabled(value),
            ui::FormMessage::GlobalRequired(value) => FormMessage::GlobalRequired(value),
            ui::FormMessage::Replace(value) => FormMessage::Replace(value),
            ui::FormMessage::ArgAdd => FormMessage::ArgAdd,
            ui::FormMessage::ArgRemove(index) => FormMessage::ArgRemove(index),
            ui::FormMessage::ArgSet(index, value) => FormMessage::ArgSet(index, value),
            ui::FormMessage::PasteEditor(value) => FormMessage::PasteEditor(value),
            ui::FormMessage::ParsePaste => FormMessage::ParsePaste,
            ui::FormMessage::ChoosePaste(value) => FormMessage::ChoosePaste(value),
            ui::FormMessage::SkipPaste(value) => FormMessage::SkipPaste(value),
            ui::FormMessage::Details(value) => FormMessage::Details(value),
            ui::FormMessage::Target(index, value) => FormMessage::Target(index, value),
            ui::FormMessage::Retry => FormMessage::Retry,
            ui::FormMessage::Close => FormMessage::Close,
            ui::FormMessage::Field(field, value) => FormMessage::Field(field, value),
            ui::FormMessage::Json(value) => FormMessage::Json(value),
            ui::FormMessage::Editor(value) => FormMessage::Editor(value),
            ui::FormMessage::Description(value) => FormMessage::Description(value),
            ui::FormMessage::Preset(value) => FormMessage::Preset(value),
            ui::FormMessage::ValueAdd => FormMessage::ValueAdd,
            ui::FormMessage::ValueRemove(value) => FormMessage::ValueRemove(value),
            ui::FormMessage::ValueName(key, value) => FormMessage::ValueName(key, value),
            ui::FormMessage::ValueKind(key, value) => FormMessage::ValueKind(key, value),
            ui::FormMessage::ValueText(key, value) => FormMessage::ValueText(key, value),
            ui::FormMessage::ValueService(key, value) => FormMessage::ValueService(key, value),
            ui::FormMessage::Save => FormMessage::Save,
        })
    })
}
