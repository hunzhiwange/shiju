//! Project environment commands use the existing project API and never execute on save.
use super::{SettingsMessage, WorktreeSettingsMessage};
use crate::app::{App, Message, state::WorktreeSettingsIdentity};
use iced::{Task, widget::text_editor};
use vw_gateway_client::{
    GatewayClient,
    vw_api_types::{
        id::ProjectScope,
        project::{
            CommandsUpdateDto, ListProjectsRequest, ProjectDto, ResolveProjectRequest,
            UpdateProjectRequest,
        },
    },
};

#[derive(Debug, Clone)]
pub enum EnvironmentMessage {
    Refresh,
    ProjectsLoaded {
        identity: WorktreeSettingsIdentity,
        generation: u64,
        result: Result<Vec<ProjectDto>, String>,
    },
    Add,
    Picked {
        identity: WorktreeSettingsIdentity,
        generation: u64,
        path: Option<String>,
    },
    Added {
        identity: WorktreeSettingsIdentity,
        generation: u64,
        result: Result<ProjectDto, String>,
    },
    Edit(ProjectDto),
    Loaded {
        identity: WorktreeSettingsIdentity,
        generation: u64,
        result: Result<Option<String>, String>,
    },
    Action(text_editor::Action),
    Cancel,
    Save,
    Saved {
        identity: WorktreeSettingsIdentity,
        generation: u64,
        result: Result<(), String>,
    },
}
fn message(event: EnvironmentMessage) -> Message {
    Message::Settings(SettingsMessage::Worktrees(WorktreeSettingsMessage::Environment(event)))
}
pub(crate) fn can_add(endpoint: &vw_gateway_client::GatewayEndpoint) -> bool {
    cfg!(not(target_arch = "wasm32")) && endpoint.is_loopback()
}
fn scope(client: &GatewayClient, project: &ProjectDto) -> Result<ProjectScope, String> {
    ProjectScope::new(client.endpoint().tenant_scope().clone(), project.id.clone())
        .map_err(|e| e.to_string())
}
pub(crate) async fn save_command(
    client: &GatewayClient,
    project: &ProjectDto,
    patch: &UpdateProjectRequest,
) -> Result<(), String> {
    client.project_update(&scope(client, project)?, patch).await.map(|_| ())
}
pub(crate) async fn load_projects(
    client: &GatewayClient,
    recent: Vec<String>,
) -> Result<Vec<ProjectDto>, String> {
    let mut cursor = None;
    let mut cursors = std::collections::HashSet::new();
    let mut seen = std::collections::HashSet::new();
    let mut projects = Vec::new();
    loop {
        let page = client
            .project_list(
                client.endpoint().tenant_scope(),
                &ListProjectsRequest { cursor, limit: Some(100), ..Default::default() },
            )
            .await?;
        for project in page.items {
            if seen.insert(project.id.clone()) {
                projects.push(project);
            }
        }
        cursor = page.next_cursor.filter(|v| !v.is_empty());
        match &cursor {
            None => break,
            Some(value) if !cursors.insert(value.clone()) => {
                return Err("项目分页游标重复，请刷新。".into());
            }
            _ => {}
        }
    }
    if can_add(client.endpoint()) {
        for directory in recent {
            // Resolve canonical gateway identity; never send local recent paths to a remote host.
            if let Ok(result) = client
                .project_resolve(
                    client.endpoint().tenant_scope(),
                    &ResolveProjectRequest { directory, create_if_missing: false },
                )
                .await
                && seen.insert(result.project.id.clone())
            {
                projects.push(result.project);
            }
        }
    }
    Ok(projects)
}
/// Produces only a project patch. All asynchronous effects remain in update.
pub(crate) fn apply(
    app: &mut App,
    event: EnvironmentMessage,
) -> Option<(ProjectDto, UpdateProjectRequest)> {
    use EnvironmentMessage::*;
    let current = WorktreeSettingsIdentity::current(app);
    let state = &mut app.worktree_settings.environment;
    match event {
        Edit(project) if !state.saving => {
            state.generation += 1;
            state.selected = Some(project);
            state.editor = text_editor::Content::new();
            state.loading = true;
            state.loaded = false;
            state.error = None;
        }
        Loaded { identity, generation, result }
            if identity == current && generation == state.generation && state.loading =>
        {
            state.loading = false;
            match result {
                Ok(command) => {
                    state.editor =
                        text_editor::Content::with_text(command.as_deref().unwrap_or(""));
                    state.error = None;
                    state.loaded = true;
                }
                Err(error) => {
                    state.error = Some(error);
                    state.loaded = false;
                }
            }
        }
        Action(action) if !state.saving && !state.loading && state.loaded => {
            state.editor.perform(action)
        }
        Cancel if !state.saving => {
            state.generation += 1;
            state.selected = None;
            state.loaded = false;
            state.loading = false;
            state.error = None;
        }
        Save if !state.saving && !state.loading && state.loaded => {
            let project = state.selected.clone()?;
            state.saving = true;
            state.error = None;
            state.generation += 1;
            let command = state.editor.text();
            let command = if command.trim().is_empty() { None } else { Some(command) };
            return Some((
                project,
                UpdateProjectRequest {
                    commands: Some(CommandsUpdateDto { start: command }),
                    ..Default::default()
                },
            ));
        }
        Saved { identity, generation, result }
            if identity == current && generation == state.generation && state.saving =>
        {
            state.saving = false;
            match result {
                Ok(()) => {
                    state.selected = None;
                    state.loaded = false;
                    state.error = None;
                }
                Err(error) => state.error = Some(error),
            }
        }
        _ => {}
    }
    None
}
pub(crate) fn update(app: &mut App, event: EnvironmentMessage) -> Task<Message> {
    use EnvironmentMessage::*;
    let current = WorktreeSettingsIdentity::current(app);
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
                    client
                        .project_resolve(
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
                client
                    .project_get(&scope(&client, &project)?)
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
