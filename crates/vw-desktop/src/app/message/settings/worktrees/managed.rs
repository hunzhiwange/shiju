//! Desktop mapping for workspace lifecycle effects.
use crate::app::{App, Message};
use super::{SettingsMessage, WorktreeSettingsMessage};
use iced::Task;
pub use desktop_settings_workspace::messages::ManagedMessage;
#[cfg(test)]
pub(crate) use desktop_settings_workspace::managed::load_groups;

pub(super) fn finish(app: &mut App, task: Task<WorktreeSettingsMessage>, effects: Vec<desktop_settings_workspace::lifecycle::Effect>) -> Task<Message> {
    let mut tasks = Vec::new();
    for effect in effects {
        use desktop_settings_workspace::lifecycle::Effect;
        tasks.push(match effect {
            Effect::Toast { success, message } => app.show_localized_toast(
                if success { crate::app::state::ToastKind::Success } else { crate::app::state::ToastKind::Error },
                message, vec![],
            ),
            Effect::Open { project, directory } => crate::app::message::project::open_existing_checkout(app, project, directory),
        });
    }
    tasks.push(task.map(|event| Message::Settings(SettingsMessage::Worktrees(event))));
    Task::batch(tasks)
}
pub(crate) fn update(app: &mut App, event: ManagedMessage) -> Task<Message> {
    let current = crate::app::state::worktree_identity(app);
    let client = crate::app::gateway_client();
    let mut effects = Vec::new();
    let task = desktop_settings_workspace::managed::update(
        &mut app.worktree_settings, event, current,
        app.session_view_cache.navigation_generation, client, &mut effects,
    );
    super::managed::finish(app, task, effects)
}
