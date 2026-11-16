use super::ViewMessage;
use crate::app::message::ProjectMessage;
use crate::app::{App, Message};
use iced::Task;
use desktop_navigation::panel::tabs as domain;
pub fn update(app: &mut App, message: ViewMessage) -> Task<Message> {
    let local = match message.clone() {
        ViewMessage::GoHome => domain::Message::GoHome,
        ViewMessage::HomeAppsBarScrollChanged(x) => domain::Message::HomeAppsBarScrollChanged(x),
        ViewMessage::HomeAppsBarPrev => domain::Message::HomeAppsBarPrev,
        ViewMessage::HomeAppsBarNext => domain::Message::HomeAppsBarNext,
        ViewMessage::TabSelected(id) => domain::Message::TabSelected(id),
        ViewMessage::TabClosed(id) => domain::Message::TabClosed(id),
        ViewMessage::TabHovered(id) => domain::Message::TabHovered(id),
        _ => return Task::none(),
    };
    let previous_project_path = app.project_path.clone();
    let mut state = domain::State {
        project_path: std::mem::take(&mut app.project_path),
        project_preview_tabs: std::mem::take(&mut app.project_preview_tabs),
        preview_tabs: std::mem::take(&mut app.preview_tabs),
        project_preview_active_path: std::mem::take(&mut app.project_preview_active_path),
        active_preview_path: std::mem::take(&mut app.active_preview_path),
        screen: app.screen,
        active_tab_id: std::mem::take(&mut app.active_tab_id),
        preview_tab_menu_path: std::mem::take(&mut app.preview_tab_menu_path),
        preview_tab_menu_pos: std::mem::take(&mut app.preview_tab_menu_pos),
        home_apps_bar_scroll_x: app.home_apps_bar_scroll_x,
        home_apps_bar_scroll_id: app.home_apps_bar_scroll_id.clone(),
        code_projects_menu_open: app.navigation_mode.code_projects_menu_open,
        open_tabs: std::mem::take(&mut app.open_tabs),
        hovered_tab_id: std::mem::take(&mut app.hovered_tab_id),
    };
    let task = domain::update(&mut state, local).map(|()| Message::None);
    app.project_path = state.project_path;
    app.project_preview_tabs = state.project_preview_tabs;
    app.preview_tabs = state.preview_tabs;
    app.project_preview_active_path = state.project_preview_active_path;
    app.active_preview_path = state.active_preview_path;
    app.screen = state.screen;
    app.active_tab_id = state.active_tab_id;
    app.preview_tab_menu_path = state.preview_tab_menu_path;
    app.preview_tab_menu_pos = state.preview_tab_menu_pos;
    app.home_apps_bar_scroll_x = state.home_apps_bar_scroll_x;
    app.home_apps_bar_scroll_id = state.home_apps_bar_scroll_id;
    app.navigation_mode.code_projects_menu_open = state.code_projects_menu_open;
    app.open_tabs = state.open_tabs;
    app.hovered_tab_id = state.hovered_tab_id;
    let effect = match message {
        ViewMessage::GoHome => {
            app.active_menu = None;
            let project_changed = previous_project_path.is_some();
            if project_changed { app.reset_task_board_project_state(); }
            app.switch_project_terminal(previous_project_path, None);
            app.project_id = None;
            Task::batch(vec![
                crate::app::message::terminal::reap_retired_task(app),
                app.reload_sessions_for_project(None),
                if project_changed {
                    app.reload_project_chat_preferences(true)
                } else {
                    Task::none()
                },
            ])
        }
        ViewMessage::TabSelected(id) => {
            if previous_project_path != app.project_path { app.reset_task_board_project_state(); }
            app.switch_project_terminal(previous_project_path.clone(), app.project_path.clone());
            let reap_task = crate::app::message::terminal::reap_retired_task(app);
            if previous_project_path != app.project_path {
                let reload_task = app.reload_sessions_for_project(app.project_path.clone());
                let preferences = app.reload_project_chat_preferences(true);
                if let Some(path) = app.project_path.clone() {
                    let git_task = Task::done(Message::Git(
                        crate::app::message::GitMessage::RefreshGitPanelData,
                    ));
                    if app.has_file_index(&path) {
                        let mut tasks = vec![reap_task, reload_task, git_task, preferences];
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
                            move |result| {
                                Message::Project(ProjectMessage::FileIndexLoaded {
                                    project_path: path,
                                    result,
                                })
                            },
                        );
                        return Task::batch(vec![
                            reap_task,
                            reload_task,
                            index_task,
                            git_task,
                            preferences,
                        ]);
                    }
                }
                return Task::batch([reap_task, reload_task, preferences]);
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
        ViewMessage::TabClosed(_id) => {
            let mut tasks = Vec::new();
            app.switch_project_terminal(previous_project_path.clone(), app.project_path.clone());
            tasks.push(crate::app::message::terminal::reap_retired_task(app));
            if previous_project_path != app.project_path {
                app.reset_task_board_project_state();
                tasks.push(app.reload_sessions_for_project(app.project_path.clone()));
                tasks.push(app.reload_project_chat_preferences(true));
            }
            Task::batch(tasks)
        }
        _ => Task::none(),
    };
    Task::batch([task, effect])
}
#[cfg(test)]
#[path = "tabs_tests.rs"]
mod tabs_tests;
