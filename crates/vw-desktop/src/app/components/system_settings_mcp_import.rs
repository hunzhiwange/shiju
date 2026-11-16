//! Desktop import projection; generation checks remain in the import update layer.
use crate::app::message::settings::mcp_management_import::{self as import, ImportState};
use crate::app::{App, Message};
use iced::Element;
pub(crate) fn panel<'a>(app: &App, state: &'a ImportState) -> Element<'a, Message> {
    desktop_settings_mcp::management_import::panel(state, import::targets(app)).map(import::message)
}
