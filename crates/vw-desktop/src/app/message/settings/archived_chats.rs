//! Archived metadata browsing events.

use super::SettingsMessage;
use crate::app::state::{ArchivedChatsMenu, ArchivedChatsSort, ArchivedChatsState};
use crate::app::{App, Message};
use iced::Task;

mod delete;
mod project_delete;
use delete::{
    accept_delete, delete_candidates, delete_task, sync_deleted_sessions, valid_delete_target,
};
use vw_shared::session::info::Info;

pub(crate) use crate::app::state::ArchivedChatsIdentity as Identity;

#[derive(Debug, Clone)]
pub enum ArchivedChatsMessage {
    ProjectActionsToggled(String),
    ProjectDeleteRequested(String),
    ProjectDeleteRetry,
    ProjectDeleted {
        identity: Identity,
        generation: u64,
        id: String,
        result: Result<Option<Vec<String>>, String>,
    },
    DeleteRequested(String),
    DeleteCancelled,
    DeleteConfirmed,
    Deleted {
        identity: Identity,
        generation: u64,
        id: String,
        result: Result<Vec<String>, String>,
    },
    Refresh,
    Loaded {
        identity: Identity,
        generation: u64,
        result: Result<Vec<Info>, String>,
    },
    SearchChanged(String),
    ProjectSelected(Option<String>),
    SortSelected(ArchivedChatsSort),
    MenuClosed,
    TypeMenuToggled,
    ProjectMenuToggled,
    Restore(String),
    Restored {
        identity: Identity,
        generation: u64,
        id: String,
        result: Result<Info, String>,
    },
    ProjectRefreshed {
        identity: Identity,
        generation: u64,
        result: Result<(String, Vec<Info>), String>,
    },
}

pub(crate) fn apply(
    state: &mut ArchivedChatsState,
    message: ArchivedChatsMessage,
    current: &Identity,
) {
    if project_delete::apply(state, &message, current) {
        return;
    }
    match message {
        ArchivedChatsMessage::ProjectActionsToggled(_)
        | ArchivedChatsMessage::ProjectDeleteRequested(_)
        | ArchivedChatsMessage::ProjectDeleteRetry
        | ArchivedChatsMessage::ProjectDeleted { .. } => unreachable!(),
        ArchivedChatsMessage::DeleteRequested(id) => {
            if state.identity.as_ref() != Some(current)
                || state.loading
                || !state.operating_ids.is_empty()
                || state.delete_confirmation.is_some()
            {
                return;
            }
            if let Some(row) = state
                .sessions
                .iter()
                .find(|row| row.id == id && row.time.archived.is_some() && row.is_user_navigable())
            {
                state.delete_target = Some(crate::app::state::ArchivedChatDeleteTarget {
                    identity: current.clone(),
                    row: row.clone(),
                });
                state.delete_confirmation =
                    Some(crate::app::state::ArchivedChatsDeleteConfirmation::Chat(id));
                state.menu = None;
            }
        }
        ArchivedChatsMessage::DeleteCancelled => {
            if state.delete_generation.is_none() {
                state.delete_confirmation = None;
                state.delete_target = None;
                state.project_delete = None;
            }
        }
        ArchivedChatsMessage::DeleteConfirmed => {
            if state.delete_generation.is_some() {
                return;
            }
            if !valid_delete_target(state, current) {
                state.delete_confirmation = None;
                state.delete_target = None;
                return;
            }
            let id = state.delete_target.as_ref().unwrap().row.id.clone();
            state.request_generation += 1;
            state.delete_generation = Some(state.request_generation);
            state.operating_ids.insert(id);
        }
        ArchivedChatsMessage::Deleted { identity, generation, id, result } => {
            if !accept_delete(state, current, &identity, generation, &id) {
                return;
            }
            state.delete_generation = None;
            state.operating_ids.remove(&id);
            if let Ok(ids) = result {
                state.error = None;
                state.sessions.retain(|row| !ids.contains(&row.id));
                state.delete_confirmation = None;
                state.delete_target = None;
                state.request_generation += 1;
                if state.selected_project_id.as_ref().is_some_and(|selected| {
                    !state.project_options().iter().any(|project| &project.id == selected)
                }) {
                    state.selected_project_id = None;
                }
            }
        }
        ArchivedChatsMessage::Restore(id) => {
            let Some(row) = state.sessions.iter().find(|row| row.id == id) else {
                return;
            };
            let conflicting_delete = match &state.delete_confirmation {
                Some(crate::app::state::ArchivedChatsDeleteConfirmation::Chat(target)) => {
                    target == &id
                }
                Some(crate::app::state::ArchivedChatsDeleteConfirmation::Project(project)) => {
                    project == &row.project_id
                }
                None => false,
            };
            if state.identity.as_ref() != Some(current)
                || state.loading
                || row.time.archived.is_none()
                || !row.is_user_navigable()
                || state.delete_generation.is_some()
                || conflicting_delete
                || !state.operating_ids.insert(id.clone())
            {
                return;
            }
            state.request_generation += 1;
            state.restore_requests.insert(id, state.request_generation);
        }
        ArchivedChatsMessage::Restored { identity, generation, id, result } => {
            if !accept_restore(state, current, &identity, generation, &id) {
                return;
            }
            state.operating_ids.remove(&id);
            state.restore_requests.remove(&id);
            if result.is_ok_and(|info| info.id == id && info.time.archived.is_none()) {
                // A project snapshot taken before this commit must not re-archive the row.
                state.request_generation += 1;
                state.sessions.retain(|row| row.id != id);
                if state.selected_project_id.as_ref().is_some_and(|selected| {
                    !state.project_options().iter().any(|project| &project.id == selected)
                }) {
                    state.selected_project_id = None;
                }
            }
        }
        ArchivedChatsMessage::ProjectRefreshed { .. } => {}
        ArchivedChatsMessage::Refresh => {
            if !state.operating_ids.is_empty() {
                return;
            }
            if state.identity.as_ref().is_some_and(|identity| identity != current) {
                reset(state);
            }
            state.identity = Some(current.clone());
            state.request_generation += 1;
            state.loading = true;
            state.project_action_menu = None;
            state.error = None;
            state.menu = None;
        }
        ArchivedChatsMessage::Loaded { identity, generation, result } => {
            if &identity != current || generation != state.request_generation || !state.loading {
                return;
            }
            state.loading = false;
            match result {
                Ok(sessions) => {
                    state.sessions = sessions;
                    state.error = None;
                    if state.delete_target.is_some() && !valid_delete_target(state, current) {
                        state.delete_confirmation = None;
                        state.delete_target = None;
                    }
                    if state.selected_project_id.as_ref().is_some_and(|selected| {
                        !state.project_options().iter().any(|project| &project.id == selected)
                    }) {
                        state.selected_project_id = None;
                    }
                }
                Err(error) => state.error = Some(error),
            }
        }
        ArchivedChatsMessage::SearchChanged(search) => {
            state.search = search;
            state.project_action_menu = None;
        }
        ArchivedChatsMessage::ProjectSelected(project) => {
            state.selected_project_id = project;
            state.project_action_menu = None;
            state.menu = None;
        }
        ArchivedChatsMessage::SortSelected(sort) => {
            state.sort = sort;
            state.menu = None;
        }
        ArchivedChatsMessage::MenuClosed => {
            state.menu = None;
            state.project_action_menu = None;
        }
        ArchivedChatsMessage::TypeMenuToggled => {
            state.menu =
                (state.menu != Some(ArchivedChatsMenu::Type)).then_some(ArchivedChatsMenu::Type);
        }
        ArchivedChatsMessage::ProjectMenuToggled => {
            state.menu = (state.menu != Some(ArchivedChatsMenu::Project))
                .then_some(ArchivedChatsMenu::Project);
        }
    }
}

pub(crate) fn reset(state: &mut ArchivedChatsState) {
    let generation = state.request_generation + 1;
    *state = ArchivedChatsState { request_generation: generation, ..Default::default() };
}

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
            return delete_task(crate::app::config::gateway_client(), current, generation, target);
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

fn accept_restore(
    state: &ArchivedChatsState,
    current: &Identity,
    identity: &Identity,
    generation: u64,
    id: &str,
) -> bool {
    identity == current
        && state.identity.as_ref() == Some(identity)
        && state.operating_ids.contains(id)
        && state.restore_requests.get(id) == Some(&generation)
}

fn sync_restored_session(app: &mut App, info: &Info) {
    for row in app.sessions.iter_mut().chain(app.project_sessions.values_mut().flatten()) {
        if row.id == info.id {
            *row = info.clone();
        }
    }
    app.archived_session_ids.remove(&info.id);
}

pub(crate) fn restore_task(
    client: Result<vw_gateway_client::GatewayClient, String>,
    identity: Identity,
    generation: u64,
    row: Info,
) -> Task<Message> {
    let id = row.id.clone();
    Task::perform(
        async move {
            client?
                .session_update::<Info>(
                    &row.id,
                    Some(&row.directory),
                    &vw_gateway_client::GatewaySessionPatchBody {
                        title: None,
                        time: Some(vw_gateway_client::GatewaySessionPatchTime {
                            archived: Some(0),
                        }),
                    },
                )
                .await
        },
        move |result| {
            Message::Settings(SettingsMessage::ArchivedChats(ArchivedChatsMessage::Restored {
                identity: identity.clone(),
                generation,
                id: id.clone(),
                result,
            }))
        },
    )
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

pub(crate) fn load_task(
    client: Result<vw_gateway_client::GatewayClient, String>,
    identity: Identity,
    generation: u64,
) -> Task<Message> {
    Task::perform(
        async move {
            let client = client?;
            client.session_list::<Vec<Info>>(None).await.map_err(|error| error.to_string())
        },
        move |result| {
            Message::Settings(SettingsMessage::ArchivedChats(ArchivedChatsMessage::Loaded {
                identity: identity.clone(),
                generation,
                result,
            }))
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
