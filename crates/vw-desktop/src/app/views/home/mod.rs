//! 首页宿主字段及跨域导航消息适配。
mod common;

pub(in crate::app) use common::{parse_hex_color, project_avatar};
use crate::app::{App, Message, message};

pub fn view(app: &App) -> iced::Element<'_, Message> {
    let state = desktop_navigation::views::home::recent::State {
        desktop_locale: app.desktop_locale,
        recent_projects: &app.recent_projects,
        recent_projects_edits: &app.recent_projects_edits,
        recent_projects_meta: &app.recent_projects_meta,
        project_path: app.navigation_tabs.project_path.as_deref(),
    };
    desktop_navigation::views::home::view(&state, app.navigation_tabs.home_apps_bar_scroll_id.clone())
        .map(|event| {
            use desktop_navigation::views::home::{Message as Home, header, recent};
            match event {
                Home::Header(header::Message::OpenFolderPressed)
                | Home::Recent(recent::Message::OpenFolderPressed) =>
                    Message::Project(message::ProjectMessage::OpenFolderPressed),
                Home::Header(header::Message::OpenProjectPressed) =>
                    Message::Project(message::ProjectMessage::OpenProjectPressed),
                Home::Header(header::Message::OpenApps) =>
                    Message::View(message::ViewMessage::OpenApps),
                Home::Header(header::Message::HomeAppsBarScrollChanged(x)) =>
                    Message::View(message::ViewMessage::HomeAppsBarScrollChanged(x)),
                Home::Recent(recent::Message::OpenRecentPressed(path)) =>
                    Message::Project(message::ProjectMessage::OpenRecentPressed(path)),
            }
        })
}
