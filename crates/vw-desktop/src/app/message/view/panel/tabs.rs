use super::ViewMessage;
use crate::app::message::ProjectMessage;
use crate::app::{App, Message};
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
    let update = domain::update(
        &mut app.navigation_tabs,
        &mut app.navigation_mode.code_projects_menu_open,
        &mut app.navigation_panels,
        local,
    );
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
    Task::batch([task, effect])
}
#[cfg(test)]
#[path = "tabs_tests.rs"]
mod tabs_tests;
