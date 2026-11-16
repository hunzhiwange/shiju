//! Map shared help copy and close actions into the desktop message envelope.
use crate::app::{App, Message, message};
pub(super) use desktop_settings_skills::help::help_text;
use iced::Element;
pub(super) fn view_overlays<'a>(
    app: &'a App,
    dialog: Element<'a, Message>,
) -> Element<'a, Message> {
    desktop_settings_skills::help::view_overlays(
        app.skills_settings.show_help_modal,
        app.desktop_locale,
        app.last_copied_code_hash,
        dialog,
        Message::CopyCode,
        Message::Settings(message::SettingsMessage::SkillsHelpClose),
    )
}
