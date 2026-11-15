use crate::app::{App, Message, message};
pub(super) fn render(app: &App) -> iced::Element<'_, Message> {
 desktop_navigation::views::home::header::render(crate::app::i18n::I18n::new(app.desktop_locale), app.home_apps_bar_scroll_id.clone()).map(|event| {
 use desktop_navigation::views::home::header::Message as Event;
 match event {
 Event::OpenFolderPressed => Message::Project(message::ProjectMessage::OpenFolderPressed),
 Event::OpenProjectPressed => Message::Project(message::ProjectMessage::OpenProjectPressed),
 Event::OpenApps => Message::View(message::ViewMessage::OpenApps),
 Event::HomeAppsBarScrollChanged(x) => Message::View(message::ViewMessage::HomeAppsBarScrollChanged(x)),
 }
 })
}
