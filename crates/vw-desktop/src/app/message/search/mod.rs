//! 搜索效果的桌面跨域适配。
use crate::app::{App, Message, components::search_panel};
use iced::{Task, widget::operation};
pub use desktop_search::SearchMessage;

#[cfg(test)]
mod tests;

pub fn update(app: &mut App, message: SearchMessage) -> Task<Message> {
    match desktop_search::update(&mut app.search, message) {
        desktop_search::Effect::None => Task::none(),
        desktop_search::Effect::RefreshFiles { focus } => {
            app.refresh_search_panel_file_cache();
            if focus { operation::focus(search_panel::input_id()) } else { Task::none() }
        }
        desktop_search::Effect::OpenProject(path) => app.open_project_and_index(path),
        desktop_search::Effect::SelectFile(path) => {
            app.project_input.file_url_input = path;
            Task::none()
        }
        desktop_search::Effect::OpenSession(id) => {
            let path = app.known_session_directory(&id).unwrap_or_default();
            Task::done(Message::Project(
                crate::app::message::project::ProjectMessage::OpenProjectSessionPressed(path, id),
            ))
        }
    }
}
