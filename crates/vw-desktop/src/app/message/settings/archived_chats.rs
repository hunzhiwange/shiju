#[cfg(test)]
use crate::app::state::ArchivedChatsState;
// Archived metadata browsing events.

use super::SettingsMessage;

use crate::app::{App, Message};
use iced::Task;

mod delete;
mod project_delete;
use delete::{
    delete_candidates, sync_deleted_sessions,
};
use session_info::Info;

pub use desktop_settings_sessions::archived_chats::{ArchivedChatsMessage, Identity, apply, reset, accept_restore};
use desktop_settings_sessions::archived_chats::{accept_delete, delete_task};
fn event(message: ArchivedChatsMessage) -> Message { Message::Settings(SettingsMessage::ArchivedChats(message)) }
pub(crate) fn load_task(client: Result<vw_gateway_client::GatewayClient, String>, identity: Identity, generation: u64) -> Task<Message> { desktop_settings_sessions::archived_chats::load_task(client, identity, generation).map(event) }
pub(crate) fn restore_task(client: Result<vw_gateway_client::GatewayClient, String>, identity: Identity, generation: u64, row: Info) -> Task<Message> { desktop_settings_sessions::archived_chats::restore_task(client, identity, generation, row).map(event) }

pub(crate) fn update(app: &mut App, message: ArchivedChatsMessage) -> Task<Message> {
    let current = Identity::from_endpoint(&crate::app::gateway_client_endpoint());
    if project_delete::handles(&app.archived_chats, &message) {
        return project_delete::update(app, message, &current);
    }
    match message {
        ArchivedChatsMessage::DeleteConfirmed => {
            if app.archived_chats.delete_generation.is_some()
                || app.archived_chats.delete_target.is_none()
            {
                return Task::none();
            }
            apply(&mut app.archived_chats, message, &current);
            let Some(generation) = app.archived_chats.delete_generation else {
                return update(app, ArchivedChatsMessage::Refresh);
            };
            let target = app.archived_chats.delete_target.as_ref().unwrap().row.clone();
            app.archived_chats.delete_reconcile_ids = delete_candidates(app, &target.id);
            return delete_task(crate::app::config::gateway_client(), current, generation, target).map(event);
        }
        ArchivedChatsMessage::Deleted { identity, generation, id, result } => {
            if !accept_delete(&app.archived_chats, &current, &identity, generation, &id) {
                return Task::none();
            }
            let result = result.and_then(|ids| {
                if ids.contains(&id) {
                    Ok(ids)
                } else {
                    Err("Incomplete archived deletion response".into())
                }
            });
            apply(
                &mut app.archived_chats,
                ArchivedChatsMessage::Deleted { identity, generation, id, result: result.clone() },
                &current,
            );
            return match result {
                Ok(ids) => {
                    app.archived_chats.delete_reconcile_ids.retain(|id| !ids.contains(id));
                    sync_deleted_sessions(app, &ids);
                    let toast = app.show_localized_toast(
                        crate::app::state::ToastKind::Success,
                        "已删除归档聊天。",
                        vec![],
                    );
                    if app.archived_chats.delete_reconcile_ids.is_empty() {
                        toast
                    } else {
                        Task::batch([toast, update(app, ArchivedChatsMessage::Refresh)])
                    }
                }
                Err(_) => {
                    let toast = app.show_localized_toast(
                        crate::app::state::ToastKind::Error,
                        "删除归档聊天失败，请重试。",
                        vec![],
                    );
                    Task::batch([toast, update(app, ArchivedChatsMessage::Refresh)])
                }
            };
        }
        ArchivedChatsMessage::Loaded { ref identity, generation, ref result }
            if identity == &current
                && app.archived_chats.identity.as_ref() == Some(identity)
                && generation == app.archived_chats.request_generation
                && app.archived_chats.loading =>
        {
            if let Ok(rows) = result {
                let removed: Vec<_> = app
                    .archived_chats
                    .delete_reconcile_ids
                    .iter()
                    .filter(|id| !rows.iter().any(|row| &row.id == *id))
                    .cloned()
                    .collect();
                app.archived_chats.delete_reconcile_ids.clear();
                sync_deleted_sessions(app, &removed);
            }
        }
        ArchivedChatsMessage::Restore(ref id) => {
            if app.archived_chats.operating_ids.contains(id) {
                return Task::none();
            }
            let id = id.clone();
            apply(&mut app.archived_chats, message, &current);
            let Some(&generation) = app.archived_chats.restore_requests.get(&id) else {
                return Task::none();
            };
            let row = app.archived_chats.sessions.iter().find(|row| row.id == id).unwrap().clone();
            return restore_task(crate::app::config::gateway_client(), current, generation, row);
        }
        ArchivedChatsMessage::Restored { identity, generation, id, result } => {
            if !accept_restore(&app.archived_chats, &current, &identity, generation, &id) {
                return Task::none();
            }
            let result = result.and_then(|info| {
                if info.id != id || info.time.archived.is_some() {
                    Err("Gateway did not return the restored session".into())
                } else {
                    Ok(info)
                }
            });
            apply(
                &mut app.archived_chats,
                ArchivedChatsMessage::Restored {
                    identity: identity.clone(),
                    generation,
                    id,
                    result: result.clone(),
                },
                &current,
            );
            return match result {
                Ok(info) => {
                    sync_restored_session(app, &info);
                    refresh_project_task(
                        crate::app::config::gateway_client(),
                        identity,
                        app.archived_chats.request_generation,
                        info,
                    )
                }
                Err(_) => app.show_localized_toast(
                    crate::app::state::ToastKind::Error,
                    "取消归档失败，请重试。",
                    vec![],
                ),
            };
        }
        ArchivedChatsMessage::ProjectRefreshed { identity, generation, result } => {
            if identity != current
                || app.archived_chats.identity.as_ref() != Some(&identity)
                || generation != app.archived_chats.request_generation
            {
                return Task::none();
            }
            return match result {
                Ok((path, sessions)) => crate::app::message::project::update(
                    app,
                    crate::app::message::project::ProjectMessage::ProjectSessionsLoaded(
                        path,
                        Ok(sessions),
                    ),
                ),
                Err(_) => Task::none(),
            };
        }
        _ => {}
    }
    if matches!(message, ArchivedChatsMessage::Refresh) {
        if !app.archived_chats.operating_ids.is_empty() {
            return Task::none();
        }
        apply(&mut app.archived_chats, message, &current);
        let generation = app.archived_chats.request_generation;
        let client = crate::app::config::gateway_client();
        return load_task(client, current, generation);
    }
    apply(&mut app.archived_chats, message, &current);
    Task::none()
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
    if app.system_settings_tab == crate::app::components::system_settings::SystemTab::ArchivedChats
    {
        update(app, ArchivedChatsMessage::Refresh)
    } else {
        Task::none()
    }
}

#[cfg(test)]
#[path = "archived_chats_tests.rs"]
mod tests;
