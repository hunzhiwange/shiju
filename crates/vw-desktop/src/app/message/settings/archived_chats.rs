//! Archived metadata browsing events.

use super::SettingsMessage;
use crate::app::state::{ArchivedChatsMenu, ArchivedChatsSort, ArchivedChatsState};
use crate::app::{App, Message};
use iced::Task;
use vw_shared::session::info::Info;

pub(crate) use crate::app::state::ArchivedChatsIdentity as Identity;

#[derive(Debug, Clone)]
pub enum ArchivedChatsMessage {
    Refresh,
    Loaded { identity: Identity, generation: u64, result: Result<Vec<Info>, String> },
    SearchChanged(String),
    ProjectSelected(Option<String>),
    SortSelected(ArchivedChatsSort),
    MenuClosed,
    TypeMenuToggled,
    ProjectMenuToggled,
}

pub(crate) fn apply(
    state: &mut ArchivedChatsState,
    message: ArchivedChatsMessage,
    current: &Identity,
) {
    match message {
        ArchivedChatsMessage::Refresh => {
            if state.identity.as_ref().is_some_and(|identity| identity != current) {
                reset(state);
            }
            state.identity = Some(current.clone());
            state.request_generation += 1;
            state.loading = true;
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
                    if state.selected_project_id.as_ref().is_some_and(|selected| {
                        !state.project_options().iter().any(|project| &project.id == selected)
                    }) {
                        state.selected_project_id = None;
                    }
                }
                Err(error) => state.error = Some(error),
            }
        }
        ArchivedChatsMessage::SearchChanged(search) => state.search = search,
        ArchivedChatsMessage::ProjectSelected(project) => {
            state.selected_project_id = project;
            state.menu = None;
        }
        ArchivedChatsMessage::SortSelected(sort) => {
            state.sort = sort;
            state.menu = None;
        }
        ArchivedChatsMessage::MenuClosed => state.menu = None,
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
    if matches!(message, ArchivedChatsMessage::Refresh) {
        apply(&mut app.archived_chats, message, &current);
        let generation = app.archived_chats.request_generation;
        let client = crate::app::config::gateway_client();
        return load_task(client, current, generation);
    }
    apply(&mut app.archived_chats, message, &current);
    Task::none()
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
