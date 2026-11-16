use crate::app::{App, Message, message};
use desktop_workspace::views::project::components::projects_list as domain;
fn map(event: domain::Message) -> Message {
 match event {
 domain::Message::OpenRecentPressed(path) => Message::Project(message::ProjectMessage::OpenRecentPressed(path)),
 domain::Message::RecentHovered(path) => Message::Project(message::ProjectMessage::RecentHovered(path)),
 domain::Message::CodeProjectsMenuChanged(open) => Message::Project(message::ProjectMessage::CodeProjectsMenuChanged(open)),
 domain::Message::OpenFolderPressed => Message::Project(message::ProjectMessage::OpenFolderPressed),
 #[cfg(not(target_arch = "wasm32"))]
 domain::Message::ExpertHub => Message::SkillHub(message::SkillHubMessage::Open),
 #[cfg(not(target_arch = "wasm32"))]
 domain::Message::ShijuSkills => Message::SkillHub(message::SkillHubMessage::OpenShijuSkills),
 #[cfg(not(target_arch = "wasm32"))]
 domain::Message::ShijuApps => Message::ShijuApps(message::ShijuAppsMessage::Open),
 }
}
pub fn projects_list<'a>(app: &App, collapsed_hover_mode: bool) -> iced::Element<'a, Message> {
 let attention = |sessions: &[session_info::Info]| sessions.iter().any(|session| app.session_runtime_states.get(&session.id).is_some_and(|runtime| runtime.request_lifecycle.is_requesting || !runtime.request_lifecycle.queue.is_empty() || runtime.has_unseen_success));
 let input = domain::Input {
 recent_projects: &app.recent_projects, recent_projects_edits: &app.recent_projects_edits,
 recent_projects_meta: &app.recent_projects_meta, project_path: &app.project_path,
 hovered_recent_project: &app.hovered_recent_project, code_projects_menu_open: app.navigation_mode.code_projects_menu_open,
 desktop_locale: app.desktop_locale,
 project_attention: app.recent_projects_edits.iter().enumerate().map(|(index, _)| {
 let path = app.recent_projects.get(index).map(String::as_str).unwrap_or("");
 (path, app.project_sessions.get(path).map(|sessions| attention(sessions)).unwrap_or_else(|| app.project_path.as_deref() == Some(path) && attention(&app.sessions)))
 }).collect(),
 };
 domain::projects_list(&input, collapsed_hover_mode).map(map)
}
pub(super) fn title_tooltip<'a>(content: impl Into<iced::Element<'a, Message>>, title: impl Into<String>, position: iced::widget::tooltip::Position) -> iced::Element<'a, Message> {
 // Tooltip has no events of its own; keep the caller's message type.
 desktop_workspace::views::project::components::projects_list::title_tooltip(content, title, position)
}
#[cfg(all(test, not(target_arch = "wasm32")))]
pub(crate) use domain::project_list_action_order;
#[cfg(test)]
#[path = "projects_list_tests.rs"]
mod projects_list_tests;
