use crate::app::{App, AppTab, Message, Screen};
use super::{ProjectMessage, helpers::run_find_task};
use desktop_workspace::project_message::find::{FindState, FindMessage, FindEffect};
pub(crate) fn handle(app: &mut App, message: ProjectMessage) -> Option<iced::Task<Message>> {
 let message = match message {
ProjectMessage::FileTreeFindScopeChanged(v0, v1) => FindMessage::FileTreeFindScopeChanged(v0, v1),
ProjectMessage::FileTreeFindQueryEditorAction(v0, v1) => FindMessage::FileTreeFindQueryEditorAction(v0, v1),
ProjectMessage::FileTreeFindReplaceEditorAction(v0, v1) => FindMessage::FileTreeFindReplaceEditorAction(v0, v1),
ProjectMessage::FileTreeFindCaseSensitiveToggled(v0, v1) => FindMessage::FileTreeFindCaseSensitiveToggled(v0, v1),
ProjectMessage::FileTreeFindWholeWordToggled(v0, v1) => FindMessage::FileTreeFindWholeWordToggled(v0, v1),
ProjectMessage::FileTreeFindRegexToggled(v0, v1) => FindMessage::FileTreeFindRegexToggled(v0, v1),
ProjectMessage::FileTreeFindCaseSensitiveShortcut => FindMessage::FileTreeFindCaseSensitiveShortcut,
ProjectMessage::FileTreeFindWholeWordShortcut => FindMessage::FileTreeFindWholeWordShortcut,
ProjectMessage::FileTreeFindRegexShortcut => FindMessage::FileTreeFindRegexShortcut,
ProjectMessage::FileTreeFindResultsPageChanged(v0, v1) => FindMessage::FileTreeFindResultsPageChanged(v0, v1),
ProjectMessage::FileTreeFindRun(v0) => FindMessage::FileTreeFindRun(v0),
ProjectMessage::FileTreeFindRefreshActive => FindMessage::FileTreeFindRefreshActive,
ProjectMessage::FileTreeFindInProject => FindMessage::FileTreeFindInProject,
ProjectMessage::FileTreeReplaceInProject => FindMessage::FileTreeReplaceInProject,
ProjectMessage::FileTreeFindCompleted { tab_id, title, scope_path, query, replace_text, case_sensitive, whole_word, use_regex, matches, error, localized_error, limit_reached } => FindMessage::FileTreeFindCompleted { tab_id, title, scope_path, query, replace_text, case_sensitive, whole_word, use_regex, matches, error, localized_error, limit_reached },
ProjectMessage::FileTreeFindMatchSelected { path, line, column } => FindMessage::FileTreeFindMatchSelected { path, line, column },
ProjectMessage::FileTreeFindTabSelected(v0) => FindMessage::FileTreeFindTabSelected(v0),
ProjectMessage::FileTreeFindTabClosed(v0) => FindMessage::FileTreeFindTabClosed(v0),
ProjectMessage::FileTreeFindTabRightClicked(v0, v1, v2) => FindMessage::FileTreeFindTabRightClicked(v0, v1, v2),
ProjectMessage::FileTreeFindTabMenuClose => FindMessage::FileTreeFindTabMenuClose,
ProjectMessage::FileTreeFindTabMenuCloseLeft(v0) => FindMessage::FileTreeFindTabMenuCloseLeft(v0),
ProjectMessage::FileTreeFindTabMenuCloseRight(v0) => FindMessage::FileTreeFindTabMenuCloseRight(v0),
ProjectMessage::FileTreeFindTabMenuCloseAll => FindMessage::FileTreeFindTabMenuCloseAll,
 _ => return None,
 };
 let files = if matches!(&message, FindMessage::FileTreeFindRun(_) | FindMessage::FileTreeFindRefreshActive) { app.current_file_index().to_vec() } else { Vec::new() };
 let mut state = FindState::default();
 state.find_results_tabs = std::mem::take(&mut app.find_results_tabs);
 state.active_find_results_tab_id = app.active_find_results_tab_id.take();
 state.find_results_tab_menu_id = app.find_results_tab_menu_id.take();
 state.find_results_tab_menu_pos = app.find_results_tab_menu_pos.take();
 let effects = desktop_workspace::project_message::find::handle(&mut state, app.project_path.as_deref(), files, message);
 app.find_results_tabs = state.find_results_tabs;
 app.active_find_results_tab_id = state.active_find_results_tab_id;
 app.find_results_tab_menu_id = state.find_results_tab_menu_id;
 app.find_results_tab_menu_pos = state.find_results_tab_menu_pos;
 let mut tasks = Vec::new();
 for effect in effects { match effect {
 FindEffect::None => {},
 FindEffect::ShowFileManager => app.file_manager.show_file_manager = true,
 FindEffect::Search {tab_id,title,scope_path,query,replace_text,case_sensitive,whole_word,use_regex,files} => tasks.push(run_find_task(tab_id,title,scope_path,query,replace_text,case_sensitive,whole_word,use_regex,files)),
 FindEffect::Open {id,count} => {
  let open_tab_id = format!("find:{id}");
  if !app.open_tabs.iter().any(|t| t.id == open_tab_id) {
   app.open_tabs.push(AppTab {id: open_tab_id.clone(), title: format!("查找结果 {count}"), screen: Screen::Project, project_path: app.project_path.clone()});
  }
  app.active_tab_id = Some(open_tab_id); app.screen = Screen::Project;
  tasks.push(app.save_desktop_preference("show_file_manager", serde_json::Value::Bool(true)));
 },
 FindEffect::Close {id,closing_active_find,active_find} => {
  let open_tab_id = format!("find:{id}");
  let closing_active_open = app.active_tab_id.as_deref() == Some(open_tab_id.as_str());
  app.open_tabs.retain(|tab| tab.id != open_tab_id);
  if closing_active_find && let Some(active_find_id) = active_find.as_deref()
   && let Some(tab) = app.open_tabs.iter().find(|tab| tab.id == format!("find:{active_find_id}")) {
    app.active_tab_id = Some(tab.id.clone()); app.screen = tab.screen;
  } else if closing_active_open {
   app.active_tab_id = app.open_tabs.last().map(|tab| tab.id.clone());
   if let Some(tab) = app.active_tab_id.as_deref().and_then(|id| app.open_tabs.iter().find(|tab| tab.id == id)) { app.screen = tab.screen; }
  }
 },
 FindEffect::Preview {path,line,column} => {
 app.pending_preview_goto = Some((path.clone(),line.saturating_sub(1),column.saturating_sub(1)));
 tasks.push(iced::Task::done(Message::Preview(crate::app::message::preview::PreviewMessage::Open(path))));
 }
 }}
 Some(iced::Task::batch(tasks))
}
#[cfg(test)]
#[path = "find_tests.rs"]
mod find_tests;
