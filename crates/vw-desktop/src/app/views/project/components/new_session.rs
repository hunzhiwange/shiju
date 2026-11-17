use crate::app::{App, Message};
use desktop_workspace::views::project::components::new_session as domain;
fn input(app: &App) -> domain::Input {
 domain::Input {
 new_session_picker_project: app.workspace_state.new_session_picker_project.clone(),
 new_session_creating: app.workspace_state.new_session_creating.clone(),
 new_session_picker_options: app.workspace_state.new_session_picker_options.clone(),
 new_session_worktree_name: app.workspace_state.new_session_worktree_name.clone(),
 new_session_confirm_delete_directory: app.workspace_state.new_session_confirm_delete_directory.clone(),
 new_session_force_delete_directory: app.workspace_state.new_session_force_delete_directory.clone(),
 new_session_delete_error: app.workspace_state.new_session_delete_error.as_ref().map(|err| app.render_input_error(crate::app::i18n::input::ErrorField::WorktreeDelete, err)),
 new_session_confirm_reset_directory: app.workspace_state.new_session_confirm_reset_directory.clone(),
 new_session_reset_error: app.workspace_state.new_session_reset_error.as_ref().map(|err| app.render_input_error(crate::app::i18n::input::ErrorField::WorktreeReset, err)),
 recent_projects: app.workspace_state.recent_projects.clone(),
 recent_projects_edits: app.workspace_state.recent_projects_edits.clone(),
 window_size: app.window_size.clone(),
 desktop_locale: app.general_settings.desktop_locale.clone(),
 }
}
pub(super) fn map(message: domain::Message) -> Message {
 use crate::app::message::ProjectMessage;
 match message {
 domain::Message::None => Message::None,
 domain::Message::ProjectCreateSession(value) => Message::Project(ProjectMessage::ProjectCreateSession(value)),
 domain::Message::ProjectCreateSessionDeleteWorktree(value) => Message::Project(ProjectMessage::ProjectCreateSessionDeleteWorktree(value)),
 domain::Message::ProjectCreateSessionDeleteWorktreeCancel => Message::Project(ProjectMessage::ProjectCreateSessionDeleteWorktreeCancel),
 domain::Message::ProjectCreateSessionDeleteWorktreeConfirmed => Message::Project(ProjectMessage::ProjectCreateSessionDeleteWorktreeConfirmed),
 domain::Message::ProjectCreateSessionDeleteWorktreeForceConfirmed => Message::Project(ProjectMessage::ProjectCreateSessionDeleteWorktreeForceConfirmed),
 domain::Message::ProjectCreateSessionPicked{ project_path, directory } => Message::Project(ProjectMessage::ProjectCreateSessionPicked{ project_path, directory }),
 domain::Message::ProjectCreateSessionPickerClose => Message::Project(ProjectMessage::ProjectCreateSessionPickerClose),
 domain::Message::ProjectCreateSessionResetWorktree(value) => Message::Project(ProjectMessage::ProjectCreateSessionResetWorktree(value)),
 domain::Message::ProjectCreateSessionResetWorktreeCancel => Message::Project(ProjectMessage::ProjectCreateSessionResetWorktreeCancel),
 domain::Message::ProjectCreateSessionResetWorktreeConfirmed => Message::Project(ProjectMessage::ProjectCreateSessionResetWorktreeConfirmed),
 domain::Message::ProjectCreateSessionWorktree(value) => Message::Project(ProjectMessage::ProjectCreateSessionWorktree(value)),
 domain::Message::ProjectCreateSessionWorktreeNameChanged(value) => Message::Project(ProjectMessage::ProjectCreateSessionWorktreeNameChanged(value)),
 }
}
#[cfg(test)]
pub fn new_session_button<'a>(app: &App, path: String) -> iced::Element<'a, Message> { domain::new_session_button(&domain::Input { new_session_picker_project: app.workspace_state.new_session_picker_project.clone(), new_session_creating: app.workspace_state.new_session_creating, desktop_locale: app.general_settings.desktop_locale, ..Default::default() }, path).map(map) }
pub fn new_session_picker_layer<'a>(app: &App) -> iced::Element<'a, Message> { domain::new_session_picker_layer(&if app.workspace_state.new_session_picker_project.is_some() { input(app) } else { domain::Input::default() }).map(map) }

pub(super) fn button_input(app: &App) -> domain::Input { domain::Input { new_session_picker_project: app.workspace_state.new_session_picker_project.clone(), new_session_creating: app.workspace_state.new_session_creating, desktop_locale: app.general_settings.desktop_locale, ..Default::default() } }
