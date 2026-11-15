use crate::state::*;
use iced::Task;
use session_info::Info;
pub use crate::state::ArchivedChatsIdentity as Identity;
mod delete;
pub use delete::{accept_delete, valid_delete_target, delete_task, delete_candidates};
pub mod project_delete;
#[derive(Debug, Clone)]
pub enum ArchivedChatsMessage {
    Noop,
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

pub fn apply(
    state: &mut ArchivedChatsState,
    message: ArchivedChatsMessage,
    current: &Identity,
) {
    if project_delete::apply(state, &message, current) {
        return;
    }
    match message {
        ArchivedChatsMessage::Noop => {},
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
                state.delete_target = Some(crate::state::ArchivedChatDeleteTarget {
                    identity: current.clone(),
                    row: row.clone(),
                });
                state.delete_confirmation =
                    Some(crate::state::ArchivedChatsDeleteConfirmation::Chat(id));
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
                Some(crate::state::ArchivedChatsDeleteConfirmation::Chat(target)) => {
                    target == &id
                }
                Some(crate::state::ArchivedChatsDeleteConfirmation::Project(project)) => {
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
        }
        ArchivedChatsMessage::SortSelected(sort) => {
            state.sort = sort;
        }
    }
}

pub fn reset(state: &mut ArchivedChatsState) {
    let generation = state.request_generation + 1;
    *state = ArchivedChatsState { request_generation: generation, ..Default::default() };
}

pub fn accept_restore(
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

pub fn restore_task(
    client: Result<vw_gateway_client::GatewayClient, String>,
    identity: Identity,
    generation: u64,
    row: Info,
) -> Task<ArchivedChatsMessage> {
    let id = row.id.clone();
    Task::perform(
        async move {
            gateway_session::GatewaySessionClient::new(client?.transport().clone())
                .session_update::<Info>(
                    &row.id,
                    Some(&row.directory),
                    &vw_api_session::GatewaySessionPatchBody {
                        title: None,
                        time: Some(vw_api_session::GatewaySessionPatchTime {
                            archived: Some(0),
                        }),
                    },
                )
                .await
        },
        move |result| {
            ArchivedChatsMessage::Restored {
                identity: identity.clone(),
                generation,
                id: id.clone(),
                result,
            }
        },
    )
}

pub fn load_task(
    client: Result<vw_gateway_client::GatewayClient, String>,
    identity: Identity,
    generation: u64,
) -> Task<ArchivedChatsMessage> {
    Task::perform(
        async move {
            let client = client?;
            gateway_session::GatewaySessionClient::new(client.transport().clone()).session_list::<Vec<Info>>(None).await.map_err(|error| error.to_string())
        },
        move |result| {
            ArchivedChatsMessage::Loaded {
                identity: identity.clone(),
                generation,
                result,
            }
        },
    )
}


#[cfg(test)]
#[path = "update_tests.rs"]
mod tests;
