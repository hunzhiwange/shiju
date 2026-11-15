//! Map picker domain effects to desktop session, gateway and preference operations.
use crate::app::message::project::ProjectMessage;
use crate::app::message::project::helpers::{create_gateway_session_worktree, delete_gateway_worktree, reset_gateway_worktree};
use crate::app::{App, Message};
use desktop_workspace::project_message::session_worktree::{self, Effect, ErrorField, Event, Picker};

fn error_field(field: ErrorField) -> crate::app::i18n::input::ErrorField {
    use crate::app::i18n::input::ErrorField as Host;
    match field { ErrorField::General => Host::General, ErrorField::Delete => Host::WorktreeDelete, ErrorField::Reset => Host::WorktreeReset }
}
pub(crate) fn handle(app: &mut App, message: ProjectMessage) -> Option<iced::Task<Message>> {
    let event = match message {
        ProjectMessage::ProjectCreateSessionPickerLoaded { project_path, options } => Event::Loaded { project_path, options },
        ProjectMessage::ProjectCreateSessionPickerClose => Event::Close,
        ProjectMessage::ProjectCreateSessionWorktreeNameChanged(name) => Event::Name(name),
        ProjectMessage::ProjectCreateSessionPicked { project_path, directory } => Event::Picked { project_path, directory },
        ProjectMessage::ProjectCreateSessionWorktree(path) => Event::Create(path),
        ProjectMessage::SessionWorktreeCreated { project_path, generation, result } => Event::Created { project_path, generation, result },
        ProjectMessage::ProjectCreateSessionDeleteWorktree(path) => Event::Delete(path),
        ProjectMessage::ProjectCreateSessionDeleteWorktreeCancel => Event::DeleteCancel,
        ProjectMessage::ProjectCreateSessionDeleteWorktreeConfirmed => Event::DeleteConfirmed,
        ProjectMessage::ProjectCreateSessionDeleteWorktreeForceConfirmed => Event::ForceDeleteConfirmed,
        ProjectMessage::ProjectCreateSessionDeleteWorktreeResult { project_path, directory, result } => Event::Deleted { project_path, directory, result },
        ProjectMessage::ProjectCreateSessionResetWorktree(path) => Event::Reset(path),
        ProjectMessage::ProjectCreateSessionResetWorktreeCancel => Event::ResetCancel,
        ProjectMessage::ProjectCreateSessionResetWorktreeConfirmed => Event::ResetConfirmed,
        ProjectMessage::ProjectCreateSessionResetWorktreeResult { project_path, directory: _, result } => Event::ResetDone { project_path, result },
        _ => return None,
    };
    Some(update_picker(app, event))
}
pub(super) fn begin_picker(app: &mut App, project_path: String) -> iced::Task<Message> {
    update_picker(app, Event::Begin(project_path))
}
fn update_picker(app: &mut App, event: Event) -> iced::Task<Message> {
    let effects = session_worktree::update(&mut Picker {
        project: &mut app.new_session_picker_project, options: &mut app.new_session_picker_options,
        last_directory: &mut app.new_session_last_directory, name: &mut app.new_session_worktree_name,
        creating: &mut app.new_session_creating, confirm_delete: &mut app.new_session_confirm_delete_directory,
        force_delete: &mut app.new_session_force_delete_directory, delete_error: &mut app.new_session_delete_error,
        confirm_reset: &mut app.new_session_confirm_reset_directory, reset_error: &mut app.new_session_reset_error,
    }, app.project_path.as_deref(), app.agent_stream_id, event);
    let mut tasks = Vec::new();
    for effect in effects {
        match effect {
            Effect::InputError(field, text, args) => app.set_localized_input_error(error_field(field), text, args.into_iter().map(Into::into).collect()),
            Effect::ClearError(field) => { app.localized_input_errors.remove(&error_field(field)); }
            Effect::ClearHover => app.hovered_recent_project = None,
            Effect::OpenDraft { project_path, directory } => tasks.push(super::open::open_session_draft(app, project_path, directory)),
            Effect::Create { project_path, name, generation } => {
                let path = project_path.clone();
                tasks.push(iced::Task::perform(async move { create_gateway_session_worktree(path, name).await }, move |result| {
                    Message::Project(ProjectMessage::SessionWorktreeCreated { project_path, generation, result })
                }));
            }
            Effect::CreateFailed(error) => tasks.push(iced::Task::done(Message::Project(ProjectMessage::SessionCreateFailed(error)))),
            Effect::Delete { project_path, directory, force } => {
                let path = project_path.clone();
                let target = directory.clone();
                tasks.push(iced::Task::perform(async move { delete_gateway_worktree(&path, &target, force).await }, move |result| {
                    Message::Project(ProjectMessage::ProjectCreateSessionDeleteWorktreeResult { project_path, directory: directory.clone(), result })
                }));
            }
            Effect::Reload { project_path, clear_last_directory } => {
                if clear_last_directory { tasks.push(app.save_desktop_preference("new_session_last_directory", serde_json::Value::Null)); }
                tasks.push(iced::Task::done(Message::Project(ProjectMessage::ProjectCreateSession(project_path))));
            }
            Effect::Reset { project_path, directory } => {
                let path = project_path.clone();
                let target = directory.clone();
                tasks.push(iced::Task::perform(async move { reset_gateway_worktree(&path, &target).await }, move |result| {
                    Message::Project(ProjectMessage::ProjectCreateSessionResetWorktreeResult { project_path, directory: directory.clone(), result })
                }));
            }
            Effect::ResetNotification => app.push_localized_notification("工作区已重置", vec![]),
        }
    }
    iced::Task::batch(tasks)
}
#[cfg(test)]
#[path = "worktree_tests.rs"]
mod worktree_tests;
