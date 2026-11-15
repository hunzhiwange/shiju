//! Restricted archived deletion and post-commit cache cleanup.
use super::*;

pub(super) fn valid_delete_target(state: &ArchivedChatsState, current: &Identity) -> bool {
    !state.loading
        && state.operating_ids.is_empty()
        && state.identity.as_ref() == Some(current)
        && state.delete_target.as_ref().is_some_and(|target| {
            target.identity == *current
                && matches!(&state.delete_confirmation,
                    Some(crate::app::state::ArchivedChatsDeleteConfirmation::Chat(id)) if id == &target.row.id)
                && state.sessions.iter().any(|row| row.id == target.row.id
                    && row.project_id == target.row.project_id && row.directory == target.row.directory
                    && row.time.archived.is_some() && row.is_user_navigable())
        })
}

pub(super) fn accept_delete(
    state: &ArchivedChatsState,
    current: &Identity,
    identity: &Identity,
    generation: u64,
    id: &str,
) -> bool {
    identity == current
        && state.identity.as_ref() == Some(identity)
        && state.delete_generation == Some(generation)
        && state.operating_ids.contains(id)
        && state
            .delete_target
            .as_ref()
            .is_some_and(|target| target.row.id == id && target.identity == *identity)
}

pub(super) fn delete_task(
    client: Result<vw_gateway_client::GatewayClient, String>,
    identity: Identity,
    generation: u64,
    row: Info,
) -> Task<Message> {
    let id = row.id.clone();
    Task::perform(
        async move {
            client?
                .session_delete_archived(&row.id, Some(&row.directory))
                .await
                .map(|response| response.deleted_ids)
        },
        move |result| {
            Message::Settings(SettingsMessage::ArchivedChats(ArchivedChatsMessage::Deleted {
                identity: identity.clone(),
                generation,
                id: id.clone(),
                result,
            }))
        },
    )
}

pub(super) fn sync_deleted_sessions(app: &mut App, ids: &[String]) {
    if !ids.is_empty() {
        // A pending navigation may target an inactive deleted root or child.
        app.session_view_cache.navigation_generation += 1;
    }
    app.sessions.retain(|row| !ids.contains(&row.id));
    for rows in app.project_sessions.values_mut() {
        rows.retain(|row| !ids.contains(&row.id));
    }
    for id in ids {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(runtime) = app.session_runtime_states.get(id) {
            for panel in &runtime.mcp_app_panels {
                panel.cancel();
            }
        }
        app.session_runtime_states.remove(id);
        app.archived_session_ids.remove(id);
        app.session_chat_cache.remove(id);
        app.session_chat_message_id_cache.remove(id);
        app.session_view_cache.remove(id);
        app.session_previews.remove(id);
    }
    if app.active_session_id.as_ref().is_some_and(|id| ids.contains(id)) {
        // Invalidate pending navigation and history replies before clearing the deleted view.
        app.agent_stream_id = app.agent_stream_id.wrapping_add(1);
        app.active_session_id = None;
        app.chat.clear();
        app.chat_message_ids.clear();
        app.active_session_view_state = Default::default();
        app.usage = Default::default();
        app.close_session_transient_ui();
        app.invalidate_chat_ui_state();
    }
}

// A failed tree delete can have committed some descendants. Reconcile only this
// tree against the next successful full gateway snapshot, never against a filter.
pub(super) fn delete_candidates(app: &App, root: &str) -> Vec<String> {
    let rows: Vec<_> = app
        .archived_chats
        .sessions
        .iter()
        .chain(app.sessions.iter())
        .chain(app.project_sessions.values().flatten())
        .collect();
    let mut ids = vec![root.to_owned()];
    loop {
        let before = ids.len();
        for row in &rows {
            if row.parent_id.as_ref().is_some_and(|parent| ids.contains(parent))
                && !ids.contains(&row.id)
            {
                ids.push(row.id.clone());
            }
        }
        if ids.len() == before {
            return ids;
        }
    }
}
