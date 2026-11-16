#[cfg(test)]
use crate::app::state::ArchivedChatsState;
// Archived metadata browsing events.

use super::SettingsMessage;

use crate::app::{App, Message};
use iced::Task;

mod delete;
mod project_delete;
use delete::{delete_candidates, sync_deleted_sessions};
use session_info::Info;

pub use desktop_settings_sessions::archived_chats::{
    ArchivedChatsMessage, Identity, accept_restore, apply, reset,
};
fn event(message: ArchivedChatsMessage) -> Message {
    Message::Settings(SettingsMessage::ArchivedChats(message))
}

pub(crate) fn update(app: &mut App, message: ArchivedChatsMessage) -> Task<Message> {
    let current = Identity::from_endpoint(&crate::app::gateway_client_endpoint());
    if project_delete::handles(&app.archived_chats, &message) {
        return project_delete::update(app, message, &current);
    }
    let candidates = app
        .archived_chats
        .delete_target
        .as_ref()
        .map(|target| delete_candidates(app, &target.row.id))
        .unwrap_or_default();
    let mut effects = Vec::new();
    let task = desktop_settings_sessions::archive_update::update(
        &mut app.archived_chats,
        message,
        current,
        crate::app::config::gateway_client(),
        candidates,
        &mut effects,
    )
    .map(event);
    let mut tasks = vec![task];
    for effect in effects {
        use desktop_settings_sessions::archive_update::Effect;
        tasks.push(match effect {
            Effect::Deleted(ids) => {
                sync_deleted_sessions(app, &ids);
                Task::none()
            }
            Effect::Restored { identity, generation, info } => {
                sync_restored_session(app, &info);
                refresh_project_task(
                    crate::app::config::gateway_client(),
                    identity,
                    generation,
                    info,
                )
            }
            Effect::ProjectLoaded(path, sessions) => crate::app::message::project::update(
                app,
                crate::app::message::project::ProjectMessage::ProjectSessionsLoaded(
                    path,
                    Ok(sessions),
                ),
            ),
            Effect::Toast { success, label } => app.show_localized_toast(
                if success {
                    crate::app::state::ToastKind::Success
                } else {
                    crate::app::state::ToastKind::Error
                },
                label,
                vec![],
            ),
        });
    }
    Task::batch(tasks)
}

fn sync_restored_session(app: &mut App, info: &Info) {
    for row in app.sessions.iter_mut().chain(app.project_sessions.values_mut().flatten()) {
        if row.id == info.id {
            *row = info.clone();
        }
    }
    app.archived_session_ids.remove(&info.id);
}

fn refresh_project_task(
    client: Result<vw_gateway_client::GatewayClient, String>,
    identity: Identity,
    generation: u64,
    info: Info,
) -> Task<Message> {
    Task::perform(
        async move {
            let client = client?;
            let path =
                crate::app::message::project::helpers::gateway_session_project_path(&client, &info)
                    .await?;
            let sessions = crate::app::message::project::helpers::list_gateway_project_sessions(
                &client, &path,
            )
            .await?;
            Ok((path, sessions))
        },
        move |result| {
            Message::Settings(SettingsMessage::ArchivedChats(
                ArchivedChatsMessage::ProjectRefreshed {
                    identity: identity.clone(),
                    generation,
                    result,
                },
            ))
        },
    )
}

pub(crate) fn gateway_changed(app: &mut App) -> Task<Message> {
    reset(&mut app.archived_chats);
    if app.navigation_panels.system_settings_tab
        == crate::app::components::system_settings::SystemTab::ArchivedChats
    {
        update(app, ArchivedChatsMessage::Refresh)
    } else {
        Task::none()
    }
}

#[cfg(test)]
#[path = "archived_chats_tests.rs"]
mod tests;
