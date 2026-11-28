//! Render the shared skills help modal with the desktop close action.
use crate::app::{App, Message, message};
use iced::Element;
pub(super) fn view_overlays<'a>(
    app: &'a App,
    dialog: Element<'a, Message>,
) -> Element<'a, Message> {
    desktop_settings_skills::help::view_overlays(
        app.skills_settings.show_help_modal,
        app.general_settings.desktop_locale,
        app.clipboard_state.last_copied_code_hash,
        dialog,
        Message::Settings(message::SettingsMessage::SkillsHelpClose),
    )
}
