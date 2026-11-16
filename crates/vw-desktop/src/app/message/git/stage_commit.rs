//! 提交领域效果的宿主装配。
use super::{GitMessage, shared};
use crate::app::{App, Message};
use desktop_git_ui::message_git::stage_commit::Effect;
use iced::Task;
pub(super) fn apply(app: &mut App, effect: Effect) -> Task<Message> {
    match effect {
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
            app.input_feedback.error_message = Some(error);
            Task::none()
        }
        Effect::Notify { message, error } => {
            app.push_localized_notification(message, error.into_iter().map(Into::into).collect());
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
    }
}
