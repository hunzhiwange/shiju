use crate::app::{App, Message};
use desktop_git_ui::git_panel::{repository as domain, messages::UiMessage};
fn input(app: &App) -> domain::Input<'_, Message> {
 domain::Input { git_repository: &app.git_repository, repo_available: super::git_repo_path_for_app(app).is_some(), context_path: crate::app::message::git::git_context_path_for_app(app), git_commit_in_progress: app.git_commit_in_progress, history_file: Box::new(move |index, file| super::diff_view::view_history_diff(app, index, file).map(UiMessage::External)) }
}
pub(super) fn toolbar(app: &App) -> iced::Element<'_, Message> {
 domain::toolbar(&input(app)).map(super::view_messages::map)
}
pub(super) fn view(app: &App) -> iced::Element<'_, Message> {
 domain::view(input(app)).map(super::view_messages::map)
}
