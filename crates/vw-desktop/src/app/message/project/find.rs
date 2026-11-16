use super::{ProjectMessage, helpers::run_find_task};
use crate::app::{App, AppTab, Message, Screen};
use desktop_workspace::project_message::find::{FindEffect, FindMessage};
pub(crate) fn handle(app: &mut App, message: ProjectMessage) -> Option<iced::Task<Message>> {
    let message = FindMessage::try_from(message).ok()?;
    let files = if matches!(
        &message,
        FindMessage::FileTreeFindRun(_) | FindMessage::FileTreeFindRefreshActive
    ) {
        app.current_file_index().to_vec()
    } else {
        Vec::new()
    };
    let effects = desktop_workspace::project_message::find::handle(
        &mut app.project_find,
        app.navigation_tabs.project_path.as_deref(),
        files,
        message,
    );
    let mut tasks = Vec::new();
    for effect in effects {
        match effect {
            FindEffect::None => {}
            FindEffect::ShowFileManager => app.file_manager.show_file_manager = true,
            FindEffect::Search {
                tab_id,
                title,
                scope_path,
                query,
                replace_text,
                case_sensitive,
                whole_word,
                use_regex,
                files,
            } => tasks.push(run_find_task(
                tab_id,
                title,
                scope_path,
                query,
                replace_text,
                case_sensitive,
                whole_word,
                use_regex,
                files,
            )),
            FindEffect::Open { id, count } => {
                let open_tab_id = format!("find:{id}");
                if !app.navigation_tabs.open_tabs.iter().any(|t| t.id == open_tab_id) {
                    app.navigation_tabs.open_tabs.push(AppTab {
                        id: open_tab_id.clone(),
                        title: format!("查找结果 {count}"),
                        screen: Screen::Project,
                        project_path: app.navigation_tabs.project_path.clone(),
                    });
                }
                app.navigation_tabs.active_tab_id = Some(open_tab_id);
                app.navigation_tabs.screen = Screen::Project;
                tasks.push(
                    app.save_desktop_preference("show_file_manager", serde_json::Value::Bool(true)),
                );
            }
            FindEffect::Close { id, closing_active_find, active_find } => {
                let open_tab_id = format!("find:{id}");
                let closing_active_open =
                    app.navigation_tabs.active_tab_id.as_deref() == Some(open_tab_id.as_str());
                app.navigation_tabs.open_tabs.retain(|tab| tab.id != open_tab_id);
                if closing_active_find
                    && let Some(active_find_id) = active_find.as_deref()
                    && let Some(tab) = app
                        .navigation_tabs
                        .open_tabs
                        .iter()
                        .find(|tab| tab.id == format!("find:{active_find_id}"))
                {
                    app.navigation_tabs.active_tab_id = Some(tab.id.clone());
                    app.navigation_tabs.screen = tab.screen;
                } else if closing_active_open {
                    app.navigation_tabs.active_tab_id =
                        app.navigation_tabs.open_tabs.last().map(|tab| tab.id.clone());
                    if let Some(tab) = app.navigation_tabs.active_tab_id.as_deref().and_then(|id| {
                        app.navigation_tabs.open_tabs.iter().find(|tab| tab.id == id)
                    }) {
                        app.navigation_tabs.screen = tab.screen;
                    }
                }
            }
            FindEffect::Preview { path, line, column } => {
                app.pending_preview_goto =
                    Some((path.clone(), line.saturating_sub(1), column.saturating_sub(1)));
                tasks.push(iced::Task::done(Message::Preview(
                    crate::app::message::preview::PreviewMessage::Open(path),
                )));
            }
        }
    }
    Some(iced::Task::batch(tasks))
}
#[cfg(test)]
#[path = "find_tests.rs"]
mod find_tests;
