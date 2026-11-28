use super::ViewMessage;
use crate::app::message::{ProjectMessage, task_board};
use crate::app::{App, FocusArea, Message, Screen};
use desktop_navigation::panel::tabs as domain;
use iced::Task;
pub fn update(app: &mut App, message: ViewMessage) -> Task<Message> {
    let local = match message {
        ViewMessage::GoHome => domain::Message::GoHome,
        ViewMessage::HomeAppsBarScrollChanged(x) => domain::Message::HomeAppsBarScrollChanged(x),
        ViewMessage::HomeAppsBarPrev => domain::Message::HomeAppsBarPrev,
        ViewMessage::HomeAppsBarNext => domain::Message::HomeAppsBarNext,
        ViewMessage::TabSelected(id) => domain::Message::TabSelected(id),
        ViewMessage::TabClosed(id) => domain::Message::TabClosed(id),
        ViewMessage::TabHovered(id) => domain::Message::TabHovered(id),
        _ => return Task::none(),
    };
    let selects_tab = match &local {
        domain::Message::TabSelected(_) => true,
        domain::Message::TabClosed(id) => app.navigation_tabs.active_tab_id.as_ref() == Some(id),
        _ => false,
    };
    let was_task_board = app.navigation_tabs.screen == Screen::TaskBoard;
    let closes_task_board =
        matches!(&local, domain::Message::TabClosed(id) if id == task_board::TAB_ID);
    let previous_screen = app.navigation_tabs.screen;
    let previous_tab = app.navigation_tabs.active_tab_id.clone();
    let update = domain::update(
        &mut app.navigation_tabs,
        &mut app.navigation_mode.code_projects_menu_open,
        &mut app.navigation_panels,
        local,
    );
    if app.navigation_tabs.screen != previous_screen || app.navigation_tabs.active_tab_id != previous_tab {
        app.session_state.session_view_cache.navigation_generation += 1;
    }
    let task_board_active = app.navigation_tabs.screen == Screen::TaskBoard;
    if was_task_board || task_board_active || closes_task_board {
        app.task_board.show_task_board = task_board_active;
    }
    if closes_task_board {
        task_board::close_tab(app);
    }
    let task_board_project_changed = match &update.effect {
        Some(
            domain::Effect::TabSelected { previous_project_path, .. }
            | domain::Effect::TabClosed { previous_project_path },
        ) => previous_project_path != &app.navigation_tabs.project_path,
        _ => false,
    };
    if selects_tab
        && app.navigation_tabs.screen == Screen::Preview
        && let Some(path) = app.navigation_tabs.active_preview_path.clone()
    {
        app.focus_area = FocusArea::Preview;
        #[cfg(not(target_arch = "wasm32"))]
        crate::app::message::preview::lsp::sync_lsp_for_path(app, &path);
        #[cfg(target_arch = "wasm32")]
        let _ = path;
    }
    let task = update.task.map(|()| Message::None);
    let effect = match update.effect {
        Some(domain::Effect::GoHome { previous_project_path }) => {
            let project_changed = previous_project_path.is_some();
            if project_changed {
                app.reset_task_board_project_state();
            }
            app.switch_project_terminal(previous_project_path, None);
            app.workspace_state.project_id = None;
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
        Some(domain::Effect::TabSelected { id, previous_project_path }) => {
            if previous_project_path != app.navigation_tabs.project_path {
                app.reset_task_board_project_state();
            }
            app.switch_project_terminal(
                previous_project_path.clone(),
                app.navigation_tabs.project_path.clone(),
            );
            let reap_task = crate::app::message::terminal::reap_retired_task(app);
            if previous_project_path != app.navigation_tabs.project_path {
                let reload_task =
                    app.reload_sessions_for_project(app.navigation_tabs.project_path.clone());
                let preferences = app.reload_project_chat_preferences(true);
                if let Some(path) = app.navigation_tabs.project_path.clone() {
                    let git_task = Task::done(Message::Git(
                        crate::app::message::GitMessage::RefreshGitPanelData,
                    ));
                    if app.has_file_index(&path) {
                        let mut tasks = vec![reap_task, reload_task, git_task, preferences];
                        let needs_refresh =
                            app.workspace_state.file_index_cache.get(&path).is_some_and(|files| files.is_empty());
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
        Some(domain::Effect::TabClosed { previous_project_path }) => {
            let mut tasks = Vec::new();
            app.switch_project_terminal(
                previous_project_path.clone(),
                app.navigation_tabs.project_path.clone(),
            );
            tasks.push(crate::app::message::terminal::reap_retired_task(app));
            if previous_project_path != app.navigation_tabs.project_path {
                app.reset_task_board_project_state();
                tasks.push(
                    app.reload_sessions_for_project(app.navigation_tabs.project_path.clone()),
                );
                tasks.push(app.reload_project_chat_preferences(true));
            }
            Task::batch(tasks)
        }
        _ => Task::none(),
    };
    if selects_tab && task_board_active {
        return Task::batch([
            task,
            effect,
            task_board::activate_tab(app, task_board_project_changed),
        ]);
    }
    #[cfg(not(target_arch = "wasm32"))]
    if selects_tab && app.navigation_tabs.screen == Screen::GitWorkbench {
        return Task::batch([task, effect, crate::app::git_workbench::activate(app)]);
    }
    #[cfg(not(target_arch = "wasm32"))]
    if selects_tab && app.navigation_tabs.screen == Screen::Projects {
        return Task::batch([
            task,
            effect,
            crate::app::workbench_projects::refresh_catalog(app, false),
        ]);
    }
    Task::batch([task, effect])
}
#[cfg(test)]
#[path = "tabs_tests.rs"]
mod tabs_tests;
