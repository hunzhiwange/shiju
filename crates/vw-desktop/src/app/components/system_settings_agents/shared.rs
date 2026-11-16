//! Shared tab style at the cross-domain settings navigation boundary.
use crate::app::Message;
use iced::widget::Button;
pub(super) fn agent_tab_button(
    label: &str,
    selected: bool,
    message: Message,
) -> Button<'_, Message> {
    desktop_ui_controls::system_settings_common::settings_tab_button(label, selected, Some(message))
}
