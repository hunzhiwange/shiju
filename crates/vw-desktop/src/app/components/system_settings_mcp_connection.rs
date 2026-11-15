//! Desktop connection-draft projection and message mapping.
use crate::app::Message;
use crate::app::message::settings::mcp_management_form::{Draft, FormMessage, message};
use desktop_settings_mcp::connection as ui;
use iced::Element;
pub(crate) fn connection_form_panel(draft: &Draft) -> Element<'_, Message> {
    ui::connection_form_panel(ui::ConnectionDraft {
        editing: draft.record.is_some(), json: draft.json, saving: draft.saving,
        id: &draft.id,
        name: &draft.name,
        transport: &draft.transport,
        command: &draft.command,
        args: &draft.args,
        values: &draft.values,
        cwd: &draft.cwd,
        url: &draft.url,
        editor: &draft.editor,
        error: &draft.error,
        presets: &draft.presets,
    }).map(|event| message(match event {
        ui::FormMessage::Close => FormMessage::Close,
        ui::FormMessage::Field(field, value) => FormMessage::Field(field, value),
        ui::FormMessage::Json(value) => FormMessage::Json(value),
        ui::FormMessage::Editor(value) => FormMessage::Editor(value),
        ui::FormMessage::Preset(value) => FormMessage::Preset(value),
        ui::FormMessage::ValueAdd => FormMessage::ValueAdd,
        ui::FormMessage::ValueRemove(value) => FormMessage::ValueRemove(value),
        ui::FormMessage::ValueName(key, value) => FormMessage::ValueName(key, value),
        ui::FormMessage::ValueKind(key, value) => FormMessage::ValueKind(key, value),
        ui::FormMessage::ValueText(key, value) => FormMessage::ValueText(key, value),
        ui::FormMessage::ValueService(key, value) => FormMessage::ValueService(key, value),
        ui::FormMessage::Save => FormMessage::Save,
    }))
}
