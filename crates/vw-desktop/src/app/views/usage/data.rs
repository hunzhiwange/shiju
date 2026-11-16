//! Desktop selection of active session metadata.
pub use desktop_usage::data::UsageData;
pub fn from_app(app: &crate::app::App) -> UsageData {
    let info = app.session_state.active_session_id.as_ref().and_then(|id| app.session_state.sessions.iter().find(|s| &s.id == id))
        .or_else(|| app.session_state.active_session_id.as_ref().and_then(|id| app.workspace_state.project_sessions.values().flat_map(|s| s.iter()).find(|s| &s.id == id)));
    UsageData::from_parts(&app.chat_view.usage, app.session_state.active_session_id.as_ref(), &app.chat_view.chat, info,
        &app.chat_view.active_session_view_state.calls, &app.chat_view.active_session_view_state.steps,
        app.chat_view.active_session_view_state.updated_ms, app.general_settings.desktop_locale)
}
#[cfg(test)]
#[path = "data_tests.rs"]
mod data_tests;
