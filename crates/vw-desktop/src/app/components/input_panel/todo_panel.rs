use crate::app::{App, Message, TodoPanelPlacement, message};
use desktop_chat_input::input_panel::todo_panel as domain;
pub(crate) use domain::TodoPanelSurface;
fn input(app: &App) -> domain::Input<'_> {
 domain::Input { i18n: crate::app::i18n::I18n::new(app.general_settings.desktop_locale),
 active_query: app.current_session_runtime_ref().and_then(|r| r.request_lifecycle.active_agent_request.as_ref().map(|request| request.query.as_str())),
 expanded: app.todo_state.chat_todo_expanded, top_right: app.todo_state.chat_todo_placement == TodoPanelPlacement::ChatTopRight,
 changed_files: app.git.git_changed_files.len() }
}
#[cfg(test)]
pub fn compute_todo_data(app: &App, items: &[todo_types::Todo]) -> TodoPanelData { domain::compute_todo_data(input(app), items) }
pub fn read_todos_for_panel(app: &App) -> Option<(Vec<todo_types::Todo>, Option<String>)> {
 domain::read_todos_for_panel(app.todo_state.chat_todo_session_id == app.session_state.active_session_id, &app.todo_state.chat_todo_items)
}
pub fn todo_panel(app: &App, items: &[todo_types::Todo], anim: u8, surface: TodoPanelSurface) -> iced::Element<'static, Message> {
 domain::todo_panel(input(app), items, anim, surface).map(|message| match message {
 domain::Message::SetPlacement(top) => Message::Chat(message::ChatMessage::SetTodoPanelPlacement(if top { TodoPanelPlacement::ChatTopRight } else { TodoPanelPlacement::InputBottom })),
 domain::Message::Toggle => Message::Chat(message::ChatMessage::ToggleTodoPanel),
 domain::Message::OpenGit => Message::Batch(vec![Message::Project(message::ProjectMessage::FileManagerShowChanges(true)), Message::Git(message::GitMessage::RefreshGitPanelData)]),
 })
}

#[cfg(test)]
pub(crate) use domain::TodoPanelData;
