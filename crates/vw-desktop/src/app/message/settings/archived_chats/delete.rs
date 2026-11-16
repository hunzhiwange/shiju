//! Restricted archived deletion and post-commit cache cleanup.
use super::*;

pub(super) fn sync_deleted_sessions(app: &mut App, ids: &[String]) {
    if !ids.is_empty() {
        // A pending navigation may target an inactive deleted root or child.
        app.session_state.session_view_cache.navigation_generation += 1;
    }
    app.session_state.sessions.retain(|row| !ids.contains(&row.id));
    for rows in app.workspace_state.project_sessions.values_mut() {
        rows.retain(|row| !ids.contains(&row.id));
    }
    for id in ids {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(runtime) = app.session_state.session_runtime_states.get(id) {
            for panel in &runtime.mcp_app_panels {
                panel.cancel();
            }
        }
        app.session_state.session_runtime_states.remove(id);
        app.session_state.archived_session_ids.remove(id);
        app.session_state.session_chat_cache.remove(id);
        app.session_state.session_chat_message_id_cache.remove(id);
        app.session_state.session_view_cache.remove(id);
        app.session_state.session_previews.remove(id);
    }
    if app.session_state.active_session_id.as_ref().is_some_and(|id| ids.contains(id)) {
        // Invalidate pending navigation and history replies before clearing the deleted view.
        app.session_state.agent_stream_id = app.session_state.agent_stream_id.wrapping_add(1);
        app.session_state.active_session_id = None;
        app.chat_view.chat.clear();
        app.chat_view.chat_message_ids.clear();
        app.chat_view.active_session_view_state = Default::default();
        app.chat_view.usage = Default::default();
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
        .chain(app.session_state.sessions.iter())
        .chain(app.workspace_state.project_sessions.values().flatten())
        .collect();
    desktop_settings_sessions::archived_chats::delete_candidates(&rows, root)
}
