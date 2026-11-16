use crate::{messages::EnvironmentMessage, state::WorktreeSettingsIdentity};
use iced::widget::text_editor;
use vw_api_project::{
    CommandsUpdateDto, ListProjectsRequest, ProjectDto, ResolveProjectRequest, UpdateProjectRequest,
};
use vw_api_transport::id::ProjectScope;
use vw_gateway_client::GatewayClient;
pub fn can_add(endpoint: &gateway_endpoint::GatewayEndpoint) -> bool {
    cfg!(not(target_arch = "wasm32")) && endpoint.is_loopback()
}
pub fn scope(client: &GatewayClient, project: &ProjectDto) -> Result<ProjectScope, String> {
    ProjectScope::new(client.endpoint().tenant_scope().clone(), project.id.clone())
        .map_err(|e| e.to_string())
}
pub async fn save_command(
    client: &GatewayClient,
    project: &ProjectDto,
    patch: &UpdateProjectRequest,
) -> Result<(), String> {
    gateway_project::GatewayProjectClient::new(client.transport().clone())
        .project_update(&scope(client, project)?, patch)
        .await
        .map(|_| ())
}
pub async fn load_projects(
    client: &GatewayClient,
    recent: Vec<String>,
) -> Result<Vec<ProjectDto>, String> {
    let mut cursor = None;
    let mut cursors = std::collections::HashSet::new();
    let mut seen = std::collections::HashSet::new();
    let mut projects = Vec::new();
    loop {
        let page = gateway_project::GatewayProjectClient::new(client.transport().clone())
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
            if let Ok(result) =
                gateway_project::GatewayProjectClient::new(client.transport().clone())
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
pub fn apply(
    state: &mut crate::state::WorktreeEnvironmentState,
    current: &WorktreeSettingsIdentity,
    event: EnvironmentMessage,
) -> Option<(ProjectDto, UpdateProjectRequest)> {
    use EnvironmentMessage::*;
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
            if &identity == current && generation == state.generation && state.loading =>
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
            if &identity == current && generation == state.generation && state.saving =>
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

// Native directory selection and editor-cache persistence belong to the shell.
pub enum EnvironmentEffect {
    Pick { identity: WorktreeSettingsIdentity, generation: u64 },
    CommandSaved { path: String, command: Option<String> },
    Toast { success: bool, message: &'static str },
}
pub fn update(
    state: &mut crate::state::WorktreeEnvironmentState,
    event: EnvironmentMessage,
    current: WorktreeSettingsIdentity,
    client: Result<GatewayClient, String>,
    recent: Vec<String>,
    can_pick: bool,
    effects: &mut Vec<EnvironmentEffect>,
) -> iced::Task<EnvironmentMessage> {
    use EnvironmentMessage::*;
    // A project/gateway/tenant switch invalidates every pending editor response.
    if (*state).identity.as_ref().is_some_and(|old| old != &current) {
        let generation = (*state).generation + 1;
        let list_generation = (*state).list_generation + 1;
        (*state) = crate::state::WorktreeEnvironmentState {
            generation,
            list_generation,
            ..Default::default()
        };
    }
    (*state).identity = Some(current.clone());
    match event {
        Refresh if !state.adding => {
            state.list_generation += 1;
            state.projects_loading = true;
            let generation = state.list_generation;
            let recent = recent.clone();
            let client = client.clone();
            return iced::Task::perform(
                async move { load_projects(&client?, recent).await },
                move |result| ProjectsLoaded { identity: current.clone(), generation, result },
            );
        }
        ProjectsLoaded { identity, generation, result } => {
            if identity != current || generation != state.list_generation {
                return iced::Task::none();
            }
            state.projects_loading = false;
            match result {
                Ok(projects) => {
                    state.projects = projects;
                    state.list_error = None;
                }
                Err(error) => {
                    state.list_error = Some(error);
                    return failed(effects);
                }
            }
            return iced::Task::none();
        }
        Add if can_pick && !state.adding && !state.projects_loading => {
            state.adding = true;
            let generation = state.list_generation;
            effects.push(EnvironmentEffect::Pick { identity: current.clone(), generation });
            return iced::Task::none();
        }
        Picked { identity, generation, path } => {
            if identity != current || generation != state.list_generation || !can_pick {
                return iced::Task::none();
            }
            let Some(directory) = path else {
                state.adding = false;
                return iced::Task::none();
            };
            let client = client.clone();
            return iced::Task::perform(
                async move {
                    let client = client?;
                    if !can_add(client.endpoint()) {
                        return Err("远程网关不支持选择本机目录。".into());
                    }
                    gateway_project::GatewayProjectClient::new(client.transport().clone())
                        .project_resolve(
                            client.endpoint().tenant_scope(),
                            &ResolveProjectRequest { directory, create_if_missing: true },
                        )
                        .await
                        .map(|r| r.project)
                },
                move |result| Added { identity: current.clone(), generation, result },
            );
        }
        Added { identity, generation, result } => {
            if identity != current || generation != state.list_generation {
                return iced::Task::none();
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
                    return failed(effects);
                }
            }
            return iced::Task::none();
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
    let patch = apply(state, &current, event);
    if let Some((path, command)) = saved_command {
        let command = if command.trim().is_empty() { None } else { Some(command) };
        effects.push(EnvironmentEffect::CommandSaved { path, command });
    }
    let generation = (*state).generation;
    let client = client.clone();
    if let Some(project) = edit {
        (*state).loaded = false;
        return iced::Task::perform(
            async move {
                let client = client?;
                gateway_project::GatewayProjectClient::new(client.transport().clone())
                    .project_get(&scope(&client, &project)?)
                    .await
                    .map(|r| r.commands.and_then(|c| c.start))
            },
            move |result| Loaded { identity: current.clone(), generation, result },
        );
    }
    if let Some((project, patch)) = patch {
        return iced::Task::perform(
            async move { save_command(&client?, &project, &patch).await },
            move |result| Saved { identity: current.clone(), generation, result },
        );
    }
    if failure {
        return failed(effects);
    }
    if saved {
        effects
            .push(EnvironmentEffect::Toast { success: true, message: "项目环境已保存。" });
        return iced::Task::none();
    }
    iced::Task::none()
}
fn failed(effects: &mut Vec<EnvironmentEffect>) -> iced::Task<EnvironmentMessage> {
    effects.push(EnvironmentEffect::Toast {
        success: false,
        message: "项目环境请求失败，未提交的编辑已保留。",
    });
    iced::Task::none()
}
