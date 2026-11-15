//! Desktop import projection; generation checks remain in the import update layer.
use crate::app::{App, Message};
use crate::app::message::settings::mcp_management_import::{self as import, ImportState, ImportMessage};
use desktop_settings_mcp::import as ui;
use iced::Element;
pub(crate) fn panel<'a>(app: &App, state: &'a ImportState) -> Element<'a, Message> {
    ui::panel(ui::ImportView {
        host: &state.context.host, importing: state.importing, loading: state.loading,
        link_drafts: &state.link_drafts, draft: &state.draft, source: &state.source,
        account: &state.account, has_preview: state.preview.is_some(), can_confirm: state.can_confirm(),
        result: &state.result,
    }, import::targets(app), state.preview.as_ref().map(import::preview_labels).unwrap_or_default())
    .map(|event| import::message(match event {
        ui::ImportMessage::Close => ImportMessage::Close,
        ui::ImportMessage::Select(v) => ImportMessage::Select(v),
        ui::ImportMessage::Account(v) => ImportMessage::Account(v),
        ui::ImportMessage::Preview => ImportMessage::Preview,
        ui::ImportMessage::Paste => ImportMessage::Paste,
        ui::ImportMessage::Confirm => ImportMessage::Confirm,
    }))
}
