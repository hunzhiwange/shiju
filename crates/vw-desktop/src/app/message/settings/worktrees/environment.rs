//! Project environment commands use the existing project API and never execute on save.
use super::{SettingsMessage, WorktreeSettingsMessage};
use crate::app::{App, Message};
pub use desktop_settings_workspace::messages::EnvironmentMessage;
use iced::Task;

fn message(event: EnvironmentMessage) -> Message {
    Message::Settings(SettingsMessage::Worktrees(WorktreeSettingsMessage::Environment(event)))
}
use desktop_settings_workspace::environment::can_add;
pub(crate) fn update(app: &mut App, event: EnvironmentMessage) -> Task<Message> {
    use desktop_settings_workspace::environment::EnvironmentEffect;
    let current = crate::app::state::worktree_identity(app);
    let mut effects = Vec::new();
    let task = desktop_settings_workspace::environment::update(
        &mut app.worktree_settings.environment,
        event,
        current,
        crate::app::gateway_client(),
        app.workspace_state.recent_projects.clone(),
        can_add(&crate::app::gateway_client_endpoint()),
        &mut effects,
    )
    .map(message);
    let mut tasks = vec![task];
    for effect in effects {
        tasks.push(match effect {
            EnvironmentEffect::CommandSaved { path, command } => {
                super::super::projects::environment_command_saved(app, &path, command);
                Task::none()
            }
            EnvironmentEffect::Toast { success, message: label } => app.show_localized_toast(
                if success {
                    crate::app::state::ToastKind::Success
                } else {
                    crate::app::state::ToastKind::Error
                },
                label,
                vec![],
            ),
            EnvironmentEffect::Pick { identity, generation } => Task::perform(
                async {
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        rfd::AsyncFileDialog::new()
                            .pick_folder()
                            .await
                            .map(|f| f.path().to_string_lossy().into_owned())
                    }
                    #[cfg(target_arch = "wasm32")]
                    {
                        None
                    }
                },
                move |path| {
                    message(EnvironmentMessage::Picked {
                        identity: identity.clone(),
                        generation,
                        path,
                    })
                },
            ),
        });
    }
    Task::batch(tasks)
}
