//! Host projections and cross-domain navigation mapping.
use crate::app::{App, Message};
use crate::app::message::{ProjectMessage, ViewMessage};
use crate::app::components::system_settings::SystemTab;
use desktop_apps_ui::apps_tiles::{self as domain, Event};
use iced::Element;
fn map(event: Event) -> Message {
    match event {
        Event::None => Message::None,
        Event::OpenMostRecent => Message::View(ViewMessage::AppsOpenMostRecent),
        Event::OpenFolder => Message::Project(ProjectMessage::OpenFolderPressed),
        Event::OpenProjectsSettings => Message::View(ViewMessage::OpenSystemSettingsTab(SystemTab::Projects)),
        Event::ToggleWebLinksMenu => Message::View(ViewMessage::ToggleWebLinksMenu),
        Event::OpenWebUrlWithTitleAndSize(url, title, width, height) => Message::View(ViewMessage::OpenWebUrlWithTitleAndSize(url, title, width, height)),
        Event::OpenUrlExternal(url) => Message::View(ViewMessage::OpenUrlExternal(url)),
        Event::WebBookmarkEditStart(index) => Message::View(ViewMessage::WebBookmarkEditStart(index)),
        Event::OpenKnowledge => Message::View(ViewMessage::OpenKnowledge),
        Event::OpenWorkflowTool => Message::View(ViewMessage::OpenWorkflowTool),
        Event::OpenWorkflowManager => Message::View(ViewMessage::OpenWorkflowManager),
        Event::AppsSearchChanged(query) => Message::View(ViewMessage::AppsSearchChanged(query)),
    }
}
pub(super) fn render_tiles_grid(app: &App, blocked: bool) -> Element<'_, Message> {
    domain::render_tiles_grid(app.window_size.0, &app.navigation_tabs.apps_search_query,
        crate::app::i18n::I18n::new(app.general_settings.desktop_locale),
        app.web_navigation.web_bookmarks.iter().map(|bookmark| domain::Bookmark {
            title: &bookmark.title, url: &bookmark.url, width: bookmark.width, height: bookmark.height,
        }), blocked).map(map)
}
pub(super) fn render_header(app: &App, blocked: bool) -> Element<'static, Message> {
    domain::render_header(&app.navigation_tabs.apps_search_query, crate::app::i18n::I18n::new(app.general_settings.desktop_locale), blocked).map(map)
}
