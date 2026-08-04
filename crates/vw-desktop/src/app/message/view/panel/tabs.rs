use super::ViewMessage;
use crate::app::message::ProjectMessage;
use crate::app::{App, Message, Screen};
use iced::Task;

pub fn update(app: &mut App, message: ViewMessage) -> Task<Message> {
    match message {
        ViewMessage::GoHome => {
            let previous_project_path = app.project_path.as_deref().map(str::to_owned);
            if let Some(prev_path) = previous_project_path.clone() {
                app.project_preview_tabs
                    .insert(prev_path.clone(), std::mem::take(&mut app.preview_tabs));
                app.project_preview_active_path.insert(prev_path, app.active_preview_path.take());
            }
            app.screen = Screen::Home;
            app.active_menu = None;
            app.active_tab_id = Some("home".to_string());
            app.project_path = None;
            if previous_project_path.is_some() {
                app.reset_task_board_project_state();
            }
            app.preview_tab_menu_path = None;
            app.preview_tab_menu_pos = None;
            app.switch_project_terminal(previous_project_path, None);
            app.project_id = None;
            Task::batch(vec![
                crate::app::message::terminal::reap_retired_task(app),
                app.reload_sessions_for_project(None),
            ])
        }
        ViewMessage::HomeAppsBarScrollChanged(x) => {
            app.home_apps_bar_scroll_x = x;
            Task::none()
        }
        ViewMessage::HomeAppsBarPrev => {
            let x = (app.home_apps_bar_scroll_x - 0.25).max(0.0);
            app.home_apps_bar_scroll_x = x;
            iced::widget::operation::snap_to(
                app.home_apps_bar_scroll_id.clone(),
                iced::widget::scrollable::RelativeOffset { x: Some(x), y: None },
            )
            .map(|_: ()| Message::None)
        }
        ViewMessage::HomeAppsBarNext => {
            let x = (app.home_apps_bar_scroll_x + 0.25).min(1.0);
            app.home_apps_bar_scroll_x = x;
            iced::widget::operation::snap_to(
                app.home_apps_bar_scroll_id.clone(),
                iced::widget::scrollable::RelativeOffset { x: Some(x), y: None },
            )
            .map(|_: ()| Message::None)
        }
        ViewMessage::TabSelected(id) => {
            let previous_project_path = app.project_path.as_deref().map(str::to_owned);
            if let Some(prev_path) = previous_project_path.clone() {
                app.project_preview_tabs
                    .insert(prev_path.clone(), std::mem::take(&mut app.preview_tabs));
                app.project_preview_active_path.insert(prev_path, app.active_preview_path.take());
            }
            app.active_tab_id = Some(id.clone());
            if let Some(tab) = app.open_tabs.iter().find(|t| t.id == id) {
                app.screen = tab.screen;
                if let Some(path) = &tab.project_path {
                    app.project_path = Some(path.clone());
                    let tabs = app.project_preview_tabs.remove(path).unwrap_or_default();
                    let active = app.project_preview_active_path.remove(path).unwrap_or_default();
                    app.preview_tabs = tabs;
                    app.active_preview_path = active;
                } else if id == "home" {
                    app.project_path = None;
                    app.preview_tabs.clear();
                    app.active_preview_path = None;
                }
            } else if let Some(ref path) = app.project_path {
                let tabs = app.project_preview_tabs.remove(path).unwrap_or_default();
                let active = app.project_preview_active_path.remove(path).unwrap_or_default();
                app.preview_tabs = tabs;
                app.active_preview_path = active;
            }
            if previous_project_path != app.project_path {
                app.reset_task_board_project_state();
                app.preview_tab_menu_path = None;
                app.preview_tab_menu_pos = None;
            }
            app.switch_project_terminal(previous_project_path.clone(), app.project_path.clone());
            let reap_task = crate::app::message::terminal::reap_retired_task(app);
            if previous_project_path != app.project_path {
                if let Some(path) = app.project_path.clone() {
                    let reload_task = app.reload_sessions_for_project(app.project_path.clone());
                    let git_task = Task::done(Message::Git(
                        crate::app::message::GitMessage::RefreshGitPanelData,
                    ));
                    if app.has_file_index(&path) {
                        let mut tasks = vec![reap_task, reload_task, git_task];
                        let needs_refresh =
                            app.file_index_cache.get(&path).is_some_and(|files| files.is_empty());
                        if needs_refresh {
                            tasks.push(crate::app::message::project::helpers::refresh_file_index(
                                app,
                            ));
                        }
                        return Task::batch(tasks);
                    } else {
                        let path_clone = path.clone();
                        let index_task = Task::perform(
                            async move {
                                crate::app::message::spawn_blocking_opt(move || {
                                    Some(crate::app::load_file_index(&path_clone))
                                })
                                .await
                                .unwrap_or_default()
                            },
                            |result| Message::Project(ProjectMessage::FileIndexLoaded(result)),
                        );
                        return Task::batch(vec![reap_task, reload_task, index_task, git_task]);
                    }
                }
                return Task::batch(vec![
                    reap_task,
                    app.reload_sessions_for_project(app.project_path.clone()),
                ]);
            }
            if id == crate::apps::workflow::WORKFLOW_TOOL_TAB_ID {
                return Task::batch(vec![
                    reap_task,
                    crate::apps::workflow::ensure_initialized(app),
                ]);
            }
            if id == "knowledge" {
                return Task::batch(vec![
                    reap_task,
                    crate::app::message::knowledge_tool::ensure_loaded(app),
                ]);
            }
            reap_task
        }
        ViewMessage::TabClosed(id) => {
            let previous_project_path = app.project_path.as_deref().map(str::to_owned);
            let mut tasks = Vec::new();
            if let Some(index) = app.open_tabs.iter().position(|t| t.id == id) {
                app.open_tabs.remove(index);

                // If we closed the active tab, switch to another one
                if app.active_tab_id.as_ref() == Some(&id) {
                    if let Some(last_tab) = app.open_tabs.last() {
                        app.active_tab_id = Some(last_tab.id.clone());
                        app.screen = last_tab.screen;
                        if let Some(path) = &last_tab.project_path {
                            if previous_project_path.as_ref() != Some(path) {
                                if let Some(prev_path) = previous_project_path.clone() {
                                    app.project_preview_tabs.insert(
                                        prev_path.clone(),
                                        std::mem::take(&mut app.preview_tabs),
                                    );
                                    app.project_preview_active_path
                                        .insert(prev_path, app.active_preview_path.take());
                                }
                                app.project_path = Some(path.clone());
                                let tabs =
                                    app.project_preview_tabs.remove(path).unwrap_or_default();
                                let active = app
                                    .project_preview_active_path
                                    .remove(path)
                                    .unwrap_or_default();
                                app.preview_tabs = tabs;
                                app.active_preview_path = active;
                            } else {
                                app.project_path = Some(path.clone());
                            }
                        } else if last_tab.id == "home" {
                            if let Some(prev_path) = previous_project_path.clone() {
                                app.project_preview_tabs.insert(
                                    prev_path.clone(),
                                    std::mem::take(&mut app.preview_tabs),
                                );
                                app.project_preview_active_path
                                    .insert(prev_path, app.active_preview_path.take());
                            }
                            app.project_path = None;
                            app.preview_tab_menu_path = None;
                            app.preview_tab_menu_pos = None;
                        }
                    } else {
                        // Should not happen if home tab is always there, but just in case
                        app.active_tab_id = None;
                        app.screen = crate::app::Screen::Home;
                    }
                }
            }
            app.switch_project_terminal(previous_project_path.clone(), app.project_path.clone());
            tasks.push(crate::app::message::terminal::reap_retired_task(app));
            if previous_project_path != app.project_path {
                app.reset_task_board_project_state();
                tasks.push(app.reload_sessions_for_project(app.project_path.clone()));
            }
            Task::batch(tasks)
        }
        ViewMessage::TabHovered(id) => {
            app.hovered_tab_id = id;
            Task::none()
        }
        _ => Task::none(),
    }
}

#[cfg(test)]
#[path = "tabs_tests.rs"]
mod tabs_tests;
