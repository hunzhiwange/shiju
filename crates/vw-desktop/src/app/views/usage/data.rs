//! Desktop selection of active session metadata.
pub use desktop_usage::data::UsageData;
pub fn from_app(app: &crate::app::App) -> UsageData {
    let info = app.active_session_id.as_ref().and_then(|id| app.sessions.iter().find(|s| &s.id == id))
        .or_else(|| app.active_session_id.as_ref().and_then(|id| app.project_sessions.values().flat_map(|s| s.iter()).find(|s| &s.id == id)));
    UsageData::from_parts(&app.usage, app.active_session_id.as_ref(), &app.chat, info,
        &app.active_session_view_state.calls, &app.active_session_view_state.steps,
        app.active_session_view_state.updated_ms, app.desktop_locale)
}
#[cfg(test)]
#[path = "data_tests.rs"]
mod data_tests;
