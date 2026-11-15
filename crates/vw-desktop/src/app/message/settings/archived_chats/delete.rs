//! Restricted archived deletion and post-commit cache cleanup.
use super::*;

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
    desktop_settings_sessions::archived_chats::delete_candidates(&rows, root)
}
