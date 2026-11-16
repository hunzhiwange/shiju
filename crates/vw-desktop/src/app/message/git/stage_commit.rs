//! Git 暂存提交的字段投影和跨域效果装配。
use super::{GitMessage, shared};
use crate::app::{App, Message};
use desktop_git_ui::message_git::stage_commit::{self, Effect};
use iced::Task;

pub(super) fn update(app: &mut App, message: GitMessage) -> Task<Message> {
    let context_path = shared::git_context_path_for_app(app);
    let repo_path = crate::app::components::git_panel::git_repo_path_for_app(app);
    let effects = stage_commit::update(
        &mut stage_commit::UpdateInput {
            context_expansions: &mut app.context_expansions,
            expanded_files: &mut app.expanded_files,
            expanded_files_set: &mut app.expanded_files_set,
            expanded_hunks: &mut app.expanded_hunks,
            git_commit_description: &mut app.git_commit_description,
            git_commit_description_editor: &mut app.git_commit_description_editor,
            git_commit_in_progress: &mut app.git_commit_in_progress,
            git_commit_message: &mut app.git_commit_message,
            git_commit_scope: &mut app.git_commit_scope,
            git_commit_type: &mut app.git_commit_type,
            git_filter_deleted: &mut app.git_filter_deleted,
            git_filter_excluded: &mut app.git_filter_excluded,
            git_filter_included: &mut app.git_filter_included,
            git_filter_modified: &mut app.git_filter_modified,
            git_filter_new: &mut app.git_filter_new,
            git_filter_query: &mut app.git_filter_query,
            git_hovered_file_header: &mut app.git_hovered_file_header,
            git_panel_header_hovered: &mut app.git_panel_header_hovered,
            show_git_commit_help_modal: &mut app.show_git_commit_help_modal,
            show_git_filter_help_modal: &mut app.show_git_filter_help_modal,
            show_git_filter_options: &mut app.show_git_filter_options,
            staged_files_selected: &mut app.staged_files_selected,
            staged_hunks_selected: &mut app.staged_hunks_selected,
            staged_lines_selected: &mut app.staged_lines_selected,
            staged_old_lines_selected: &mut app.staged_old_lines_selected,
            remote_busy: app.git_repository.remote_busy,
            context_path,
            repo_path,
            animate: !app.file_manager.file_manager_changes_refreshing
                && !app.file_manager.file_manager_file_tree_refreshing,
        },
        message,
    );
    let tasks = effects
        .into_iter()
        .map(|effect| match effect {
            Effect::StageAll(path) => {
                Task::perform(shared::execute_stage_all_via_gateway(path), |result| {
                    Message::Git(GitMessage::StageAllFinished(result))
                })
            }
            Effect::Commit { path, request, animate } => {
                let task = Task::perform(
                    shared::execute_selected_commit_via_gateway(path, request),
                    |result| Message::Git(GitMessage::CommitSelectedFinished(result)),
                );
                if animate {
                    Task::batch([task, shared::schedule_commit_button_animation_tick()])
                } else {
                    task
                }
            }
            Effect::NotifyRaw(message) => {
                app.push_notification(message);
                Task::none()
            }
            Effect::InputError(message) => {
                app.set_localized_input_error(
                    crate::app::i18n::input::ErrorField::General,
                    message,
                    vec![],
                );
                Task::none()
            }
            Effect::Error(error) => {
                app.error_message = Some(error);
                Task::none()
            }
            Effect::Notify { message, error } => {
                app.push_localized_notification(
                    message,
                    error.into_iter().map(Into::into).collect(),
                );
                Task::none()
            }
            Effect::Toast { success, message, error } => app.show_localized_toast(
                if success {
                    crate::app::state::ToastKind::Success
                } else {
                    crate::app::state::ToastKind::Error
                },
                message,
                error.into_iter().map(Into::into).collect(),
            ),
            Effect::RefreshAfterMutation => {
                shared::refresh_git_panel_data_after_repo_mutation_task(app)
            }
            Effect::RefreshBranches => {
                app.refresh_branches();
                Task::none()
            }
        })
        .collect::<Vec<_>>();
    Task::batch(tasks)
}
