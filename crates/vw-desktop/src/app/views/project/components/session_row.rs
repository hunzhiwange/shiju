use crate::app::{App, Message};
use crate::app::i18n::I18n;
use session_info::Info;
use desktop_workspace::views::project::components::session_row as domain;
pub use domain::SessionRowDetails;
pub use domain::SessionRowStatus;
pub(in crate::app::views::project) fn status_for_session(app: &App, id: &str) -> SessionRowStatus {
 let runtime = app.session_state.session_runtime_states.get(id);
 domain::SessionRowStatus::from_activity(
 runtime.is_some_and(|r| r.request_lifecycle.is_requesting),
 runtime.map_or(0, |r| r.request_lifecycle.queue.len()),
 runtime.is_some_and(|r| r.has_unseen_success),
 )
}
/// Build sidebar-only rows for local drafts whose first request is in flight.
pub(in crate::app::views::project) fn running_draft_sessions(
    app: &App,
) -> impl Iterator<Item = (&str, Info)> {
    app.session_state.session_runtime_states.iter().filter_map(move |(id, runtime)| {
        let draft = runtime.draft.as_ref()?;
        if !runtime.request_lifecycle.is_requesting {
            return None;
        }
        let info = domain::running_draft_session(
            id, &draft.directory,
            runtime.request_lifecycle.active_agent_request.as_ref().map(|request| request.query.as_str()),
            runtime.request_lifecycle.request_started_ms, app.general_settings.desktop_locale,
        );
        Some((draft.project_path.as_str(), info))
    })
}


pub(in crate::app::views::project) fn row_input(app: &App, id: &str, title: &str, updated: u64) -> domain::Input {
 let i18n = I18n::new(app.general_settings.desktop_locale);
    let metadata = app.session_ui_projection.hover_details.get(id);
    let workspace_kind = metadata.and_then(|details| details.primary_workspace);
    let binding = metadata.and_then(|details| details.agent.as_ref());
    let request = app.session_state.session_runtime_states
        .get(id)
        .filter(|runtime| runtime.request_lifecycle.is_requesting)
        .and_then(|runtime| runtime.request_lifecycle.active_agent_request.as_deref());
    let agent = if let Some(request) = request {
        domain::agent_description(
            i18n,
            request.agent.as_deref(),
            None,
            request.acp_test.then(|| request.acp_agent.as_deref().unwrap_or("")),
        )
    } else if let Some(binding) = binding {
        domain::agent_description(
            i18n,
            Some(&binding.agent),
            binding.label.as_deref(),
            binding.acp_agent.as_deref(),
        )
    } else {
        "—".to_owned()
    };

 domain::Input {
 desktop_locale: app.general_settings.desktop_locale,
 active: app.session_state.active_session_id.as_deref() == Some(id),
 title: if title.trim().is_empty() && !app.session_state.session_previews.contains_key(id) { i18n.__("未命名任务").to_owned() } else { crate::app::i18n::other::session_title(app, id, title) },
 pinned: false,
 status_animation_frame: app.status_animation_frame,
 menu_open: app.workspace_state.session_menu_id.is_some(), workspace_kind, agent,
 relative: crate::app::i18n::chat::relative_time(i18n, crate::app::components::chat_panel::utils::relative_time_bucket(updated, crate::app::time::now_ms())),
 }
}
pub fn session_row(app: &App, id: &str, details: &SessionRowDetails, status: SessionRowStatus, open: Message) -> iced::Element<'static, Message> {
 let mut input = row_input(app, id, details.title(), details.updated);
 input.pinned = app.session_ui_projection.pinned.contains(&(details.project_id().to_owned(), id.to_owned()));
 domain::session_row(&input, id, details, status.into(), super::sessions_panel::domain::Message::Open).map(move |event| match event { super::sessions_panel::domain::Message::Open => open.clone(), other => super::sessions_panel::map(other) })
}

#[cfg(test)]
#[path = "session_row_tests.rs"]
mod session_row_tests;
