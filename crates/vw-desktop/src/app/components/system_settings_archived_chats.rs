use crate::app::{App, Message};
use crate::app::message::settings::SettingsMessage;
use crate::app::state::ArchivedChatsState;
use config_ui::DesktopLocale;
use iced::Element;
pub(crate) fn view(app: &App) -> Element<'_ , Message> { page(&app.archived_chats, app.general_settings.desktop_locale) }
pub(crate) fn page(state: &ArchivedChatsState, locale: DesktopLocale) -> Element<'_, Message> { desktop_settings_sessions::view::page(state, locale).map(|message| Message::Settings(SettingsMessage::ArchivedChats(message))) }
