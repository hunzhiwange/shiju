//! Project environment commands use the existing project API and never execute on save.
use super::{SettingsMessage, WorktreeSettingsMessage};
use crate::app::{App, Message};
use iced::Task;
use {
    vw_api_project::ProjectDto,
    vw_api_project::ResolveProjectRequest,
    vw_api_project::UpdateProjectRequest,
};

pub use desktop_settings_workspace::messages::EnvironmentMessage;

fn message(event: EnvironmentMessage) -> Message {
    Message::Settings(SettingsMessage::Worktrees(WorktreeSettingsMessage::Environment(event)))
}
pub(crate) use desktop_settings_workspace::environment::{can_add, save_command, load_projects, scope};
pub(crate) fn apply(app: &mut App, event: EnvironmentMessage) -> Option<(ProjectDto, UpdateProjectRequest)> {
    let current = crate::app::state::worktree_identity(app);
    desktop_settings_workspace::environment::apply(&mut app.worktree_settings.environment, &current, event)
}
pub(crate) fn update(app: &mut App, event: EnvironmentMessage) -> Task<Message> {
    use EnvironmentMessage::*;
    let current = crate::app::state::worktree_identity(app);
    // A project/gateway/tenant switch invalidates every pending editor response.
    if app.worktree_settings.environment.identity.as_ref().is_some_and(|old| old != &current) {
        let generation = app.worktree_settings.environment.generation + 1;
        let list_generation = app.worktree_settings.environment.list_generation + 1;
        app.worktree_settings.environment = crate::app::state::WorktreeEnvironmentState {
            generation,
            list_generation,
            ..Default::default()
        };
    }
    app.worktree_settings.environment.identity = Some(current.clone());
    let state = &mut app.worktree_settings.environment;
    match event {
        Refresh if !state.adding => {
            state.list_generation += 1;
            state.projects_loading = true;
            let generation = state.list_generation;
            let recent = app.recent_projects.clone();
            let client = crate::app::gateway_client();
            return Task::perform(
                async move { load_projects(&client?, recent).await },
                move |result| {
                    message(ProjectsLoaded { identity: current.clone(), generation, result })
                },
            );
        }
        ProjectsLoaded { identity, generation, result } => {
            if identity != current || generation != state.list_generation {
                return Task::none();
            }
            state.projects_loading = false;
            match result {
                Ok(projects) => {
                    state.projects = projects;
                    state.list_error = None;
                }
                Err(error) => {
                    state.list_error = Some(error);
                    return failed(app);
                }
            }
            return Task::none();
        }
        Add if can_add(&crate::app::gateway_client_endpoint())
            && !state.adding
            && !state.projects_loading =>
        {
            state.adding = true;
            let generation = state.list_generation;
            return Task::perform(
                async {
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        rfd::AsyncFileDialog::new()
                            .pick_folder()
                            .await
                            .map(|f| f.path().to_string_lossy().into_owned())
                    }
                    #[cfg(target_arch = "wasm32")]
                    {
                        None
                    }
                },
                move |path| message(Picked { identity: current.clone(), generation, path }),
            );
        }
        Picked { identity, generation, path } => {
            if identity != current
                || generation != state.list_generation
                || !can_add(&crate::app::gateway_client_endpoint())
            {
                return Task::none();
            }
            let Some(directory) = path else {
                state.adding = false;
                return Task::none();
            };
            let client = crate::app::gateway_client();
            return Task::perform(
                async move {
                    let client = client?;
                    if !can_add(client.endpoint()) {
                        return Err("远程网关不支持选择本机目录。".into());
                    }
                    gateway_project::GatewayProjectClient::new(client.transport().clone()).project_resolve(
                            client.endpoint().tenant_scope(),
                            &ResolveProjectRequest { directory, create_if_missing: true },
                        )
                        .await
                        .map(|r| r.project)
                },
                move |result| message(Added { identity: current.clone(), generation, result }),
            );
        }
        Added { identity, generation, result } => {
            if identity != current || generation != state.list_generation {
                return Task::none();
            }
            state.adding = false;
            match result {
                Ok(project) => {
                    if !state.projects.iter().any(|p| p.id == project.id) {
                        state.projects.push(project);
                    }
                    state.list_error = None;
                }
                Err(error) => {
                    state.list_error = Some(error);
                    return failed(app);
                }
            }
            return Task::none();
        }
        _ => {}
    }
    let edit = match &event {
        Edit(project) if !state.saving => Some(project.clone()),
        _ => None,
    };
    let accepted = match &event {
        Loaded { identity, generation, .. } => {
            identity == &current && *generation == state.generation && state.loading
        }
        Saved { identity, generation, .. } => {
            identity == &current && *generation == state.generation && state.saving
        }
        _ => false,
    };
    let failure =
        accepted && matches!(&event, Loaded { result: Err(_), .. } | Saved { result: Err(_), .. });
    let saved = accepted && matches!(&event, Saved { result: Ok(()), .. });
    let saved_command = if saved {
        state.selected.as_ref().map(|p| (p.directory.clone(), state.editor.text()))
    } else {
        None
    };
    let patch = apply(app, event);
    if let Some((path, command)) = saved_command {
        let command = if command.trim().is_empty() { None } else { Some(command) };
        super::super::projects::environment_command_saved(app, &path, command);
    }
    let generation = app.worktree_settings.environment.generation;
    let client = crate::app::gateway_client();
    if let Some(project) = edit {
        app.worktree_settings.environment.loaded = false;
        return Task::perform(
            async move {
                let client = client?;
                gateway_project::GatewayProjectClient::new(client.transport().clone()).project_get(&scope(&client, &project)?)
                    .await
                    .map(|r| r.commands.and_then(|c| c.start))
            },
            move |result| message(Loaded { identity: current.clone(), generation, result }),
        );
    }
    if let Some((project, patch)) = patch {
        return Task::perform(
            async move { save_command(&client?, &project, &patch).await },
            move |result| message(Saved { identity: current.clone(), generation, result }),
        );
    }
    if failure {
        return failed(app);
    }
    if saved {
        return app.show_localized_toast(
            crate::app::state::ToastKind::Success,
            "项目环境已保存。",
            vec![],
        );
    }
    Task::none()
}
fn failed(app: &mut App) -> Task<Message> {
    app.show_localized_toast(
        crate::app::state::ToastKind::Error,
        "项目环境请求失败，未提交的编辑已保留。",
        vec![],
    )
}
