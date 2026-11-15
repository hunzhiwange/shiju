use iced::widget::text_editor;
use crate::{messages::EnvironmentMessage, state::WorktreeSettingsIdentity};
use vw_gateway_client::GatewayClient;
use vw_api_transport::id::ProjectScope;
use vw_api_project::{ProjectDto, UpdateProjectRequest, CommandsUpdateDto, ListProjectsRequest, ResolveProjectRequest};
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
    gateway_project::GatewayProjectClient::new(client.transport().clone()).project_update(&scope(client, project)?, patch).await.map(|_| ())
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
        let page = gateway_project::GatewayProjectClient::new(client.transport().clone()).project_list(
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
            if let Ok(result) = gateway_project::GatewayProjectClient::new(client.transport().clone()).project_resolve(
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
