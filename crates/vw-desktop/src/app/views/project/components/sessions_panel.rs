use crate::app::{App, Message, message};
pub(super) use desktop_workspace::views::project::components::sessions_panel as domain;
pub(in crate::app::views::project) fn map(event: domain::Message) -> Message {
 match event {
 domain::Message::Open | domain::Message::HeaderToolsToggle => Message::None,
 domain::Message::Project(event) => Message::Project(match event {
domain::events::ProjectMessage::OpenRecentPressed(value) => message::ProjectMessage::OpenRecentPressed(value),
domain::events::ProjectMessage::ProjectToolsMenuToggled(value) => message::ProjectMessage::ProjectToolsMenuToggled(value),
domain::events::ProjectMessage::ProjectEditOpened(value) => message::ProjectMessage::ProjectEditOpened(value),
domain::events::ProjectMessage::RecentRevealPressed(value) => message::ProjectMessage::RecentRevealPressed(value),
domain::events::ProjectMessage::RecentRemovePressed(value) => message::ProjectMessage::RecentRemovePressed(value),
domain::events::ProjectMessage::ProjectLoadSessions(value) => message::ProjectMessage::ProjectLoadSessions(value),
domain::events::ProjectMessage::ProjectLoadMoreSessions(value) => message::ProjectMessage::ProjectLoadMoreSessions(value),
domain::events::ProjectMessage::SessionRenamePressed(value) => message::ProjectMessage::SessionRenamePressed(value),
domain::events::ProjectMessage::SessionCopyPressed(value) => message::ProjectMessage::SessionCopyPressed(value),
domain::events::ProjectMessage::SessionArchivePressed(value) => message::ProjectMessage::SessionArchivePressed(value),
domain::events::ProjectMessage::SessionDeletePressed(value) => message::ProjectMessage::SessionDeletePressed(value),
domain::events::ProjectMessage::ProjectToolsMenuClosed => message::ProjectMessage::ProjectToolsMenuClosed,
domain::events::ProjectMessage::SessionMenuClose => message::ProjectMessage::SessionMenuClose,
domain::events::ProjectMessage::OpenProjectSessionPressed(path, id) => message::ProjectMessage::OpenProjectSessionPressed(path, id),
domain::events::ProjectMessage::SessionRightClicked(id, x, y) => message::ProjectMessage::SessionRightClicked(id, x, y),
domain::events::ProjectMessage::SessionPinPressed(project, id, pinned) => message::ProjectMessage::SessionPinPressed(project, id, pinned),
domain::events::ProjectMessage::ProjectSessionListScrollChanged { project_path, has_vertical_scrollbar } => message::ProjectMessage::ProjectSessionListScrollChanged { project_path, has_vertical_scrollbar },
}),
 domain::Message::CompactSessionRequested { session_id, project_path } => Message::Chat(message::ChatMessage::CompactSessionRequested { session_id, project_path }),
 domain::Message::ToggleBoard => Message::TaskBoard(message::TaskBoardMessage::ToggleBoard),
 domain::Message::NewSession(event) => super::new_session::map(event),
 domain::Message::SessionDetailsHovered { session_id, project_id, directory } => Message::SessionDetailsHovered { session_id, project_id, directory },
 }
}
fn input(app: &App) -> domain::Input<'_> {
 let drafts = super::session_row::running_draft_sessions(app).map(|(path, info)| (path.to_owned(), info)).collect::<Vec<_>>();
 let rows = app.workspace_state.project_sessions.values().flat_map(|sessions| domain::session_tree_projection(sessions, usize::MAX, &app.session_ui_projection.pinned).rows.into_iter().map(|row| row.session)).chain(drafts.iter().map(|(_, info)| info)).map(|info| {
 let mut row = super::session_row::row_input(app, &info.id, &info.title, info.time.updated);
 row.pinned = app.session_ui_projection.pinned.contains(&(info.project_id.clone(), info.id.clone()));
 (info.id.clone(), (row, super::session_row::status_for_session(app, &info.id).into()))
 }).collect();
 domain::Input { desktop_locale: app.general_settings.desktop_locale, pinned: &app.session_ui_projection.pinned,
 session_menu_id: &app.workspace_state.session_menu_id, session_menu_anchor: app.workspace_state.session_menu_anchor, rows, drafts,
 recent_projects: &app.workspace_state.recent_projects, recent_projects_edits: &app.workspace_state.recent_projects_edits,
 project_sessions_loading: &app.workspace_state.project_sessions_loading, project_session_load_counts: &app.workspace_state.project_session_load_counts,
 project_session_has_vertical_scrollbar: &app.workspace_state.project_session_has_vertical_scrollbar, project_sessions: &app.workspace_state.project_sessions,
 project_tools_menu_path: &app.workspace_state.project_tools_menu_path,
 new_session: super::new_session::button_input(app), }
}
pub fn project_sessions_panel(app: &App, path: Option<String>) -> iced::Element<'_, Message> { domain::project_sessions_panel(&input(app), path).map(map) }
#[cfg(test)]
pub fn build_session_menu<'a>(i18n: crate::app::i18n::I18n, id: String, path: String, disabled: bool) -> iced::Element<'a, Message> { domain::build_session_menu(i18n, id, path, disabled).map(map) }
#[cfg(test)]
pub fn project_sessions_header_with_tools<'a>(i18n: crate::app::i18n::I18n, title: String, path: String, clickable: bool, toggle: message::ProjectMessage, show: bool, refreshing: bool) -> iced::Element<'a, Message> { domain::project_sessions_header_with_tools(i18n, title, path, clickable, domain::Message::HeaderToolsToggle, show, refreshing).map(move |event| match event { domain::Message::HeaderToolsToggle => Message::Project(toggle.clone()), other => map(other) }) }

#[cfg(test)]
fn session_items_list<'a>(app: &App, path: &str, sessions: Option<&Vec<session_info::Info>>, title: &str, count: usize, loading: bool) -> iced::Element<'a, Message> {
 let mut input = input(app);
 if let Some(sessions) = sessions { for info in sessions { let mut row = super::session_row::row_input(app, &info.id, &info.title, info.time.updated); row.pinned = app.session_ui_projection.pinned.contains(&(info.project_id.clone(), info.id.clone())); input.rows.insert(info.id.clone(), (row, super::session_row::status_for_session(app, &info.id).into())); } }
 domain::session_items_list(&input, path, sessions, title, count, loading).map(map)
}
#[cfg(test)]
#[path = "sessions_panel_tests.rs"]
mod sessions_panel_tests;
