//! Managed checkout browsing effects; all operations keep the gateway scope.
use crate::{messages::{ManagedMessage, WorktreeSettingsMessage}, state::*};
use crate::lifecycle::Effect;
use iced::Task;
use {
    vw_gateway_client::GatewayClient,
    vw_api_transport::id::ProjectScope,
    vw_api_project::ListProjectsRequest,
    vw_api_project::ProjectDto,
    vw_api_worktree::ManagedWorktreeDto,
    vw_api_worktree::WorktreeStatus,
};


fn message(event: ManagedMessage) -> WorktreeSettingsMessage {
    WorktreeSettingsMessage::Managed(event)
}
fn same_host(a: &WorktreeSettingsIdentity, b: &WorktreeSettingsIdentity) -> bool {
    a.gateway == b.gateway && a.tenant == b.tenant && a.auth_fingerprint == b.auth_fingerprint
}
fn scope(identity: &WorktreeSettingsIdentity, project: &str) -> Result<ProjectScope, String> {
    ProjectScope::new(identity.tenant.clone(), project.into()).map_err(|e| e.to_string())
}

pub async fn load_groups(
    client: &GatewayClient,
) -> Result<Vec<ManagedWorktreeGroup>, String> {
    let mut cursor = None;
    let mut cursors = std::collections::HashSet::new();
    let mut seen = std::collections::HashSet::new();
    let mut groups = Vec::new();
    loop {
        let page = gateway_project::GatewayProjectClient::new(client.transport().clone()).project_list(
                client.endpoint().tenant_scope(),
                &ListProjectsRequest { cursor, limit: Some(100), ..Default::default() },
            )
            .await?;
        for project in page.items {
            if !seen.insert(project.id.as_ref().to_owned()) {
                continue;
            }
            let result = gateway_project::GatewayProjectClient::new(client.transport().clone()).managed_worktrees(
                    &ProjectScope::new(
                        client.endpoint().tenant_scope().clone(),
                        project.id.clone(),
                    )
                    .map_err(|e| e.to_string())?,
                )
                .await;
            let (items, error) = match result {
                Ok(list) => (list.items, None),
                Err(error) => (vec![], Some(error)),
            };
            groups.push(ManagedWorktreeGroup { project, items, error, collapsed: false });
        }
        cursor = page.next_cursor.filter(|c| !c.is_empty());
        match &cursor {
            None => return Ok(groups),
            Some(value) if !cursors.insert(value.clone()) => {
                return Err("项目分页游标重复，请刷新。".into());
            }
            _ => {}
        }
    }
}
fn selected(settings: &WorktreeSettingsState, project: &str, worktree: &str) -> Option<(ProjectDto, ManagedWorktreeDto)> {
    let group = settings
        .managed
        .groups
        .iter()
        .find(|g| g.project.id.as_ref() == project && g.error.is_none())?;
    let item = group.items.iter().find(|w| {
        w.worktree.id.as_ref() == worktree
            && w.worktree.project_id == group.project.id
            && w.worktree.directory != group.project.directory
    })?;
    Some((group.project.clone(), item.clone()))
}
fn fail(settings: &mut WorktreeSettingsState, effects: &mut Vec<Effect>, error: String) -> Task<WorktreeSettingsMessage> {
    settings.managed.error = Some(error);
    effects.push(Effect::Toast { success: false, message: "工作树操作失败，请刷新后重试。" });
    Task::none()
}
pub fn update(
    settings: &mut WorktreeSettingsState,
    event: ManagedMessage,
    current: WorktreeSettingsIdentity,
    navigation_generation: u64,
    client: Result<vw_gateway_client::GatewayClient, String>,
    effects: &mut Vec<Effect>,
) -> Task<WorktreeSettingsMessage> {
    let state = &mut settings.managed;
    if state.identity.as_ref().is_some_and(|old| !same_host(old, &current)) {
        let generation = state.generation + 1;
        *state = crate::state::ManagedWorktreesState { generation, ..Default::default() };
    }
    match event {
        ManagedMessage::Retry { project, worktree } => {
            if state.retrying.is_some() {
                return Task::none();
            }
            let Some((_, item)) = selected(settings, &project, &worktree) else {
                return Task::none();
            };
            if item.worktree.status != WorktreeStatus::Error {
                return Task::none();
            }
            let target = (project.clone(), worktree.clone());
            settings.managed.retrying = Some(target.clone());
            let client = client.clone();
            let scope = scope(&current, &project);
            Task::perform(
                async move { gateway_project::GatewayProjectClient::new(client?.transport().clone()).managed_worktree_retry(&scope?, &worktree).await.map(|_| ()) },
                move |result| {
                    message(ManagedMessage::Retried {
                        identity: current.clone(),
                        target: target.clone(),
                        result,
                    })
                },
            )
        }
        ManagedMessage::Retried { identity, target, result } => {
            if !same_host(&identity, &current) || state.retrying.as_ref() != Some(&target) {
                return Task::none();
            }
            state.retrying = None;
            match result {
                Ok(()) => update(settings, ManagedMessage::Refresh, current, navigation_generation, client, effects),
                Err(error) => fail(settings, effects, error),
            }
        }
        ManagedMessage::Refresh => {
            state.generation += 1;
            state.identity = Some(current.clone());
            state.loading = true;
            state.error = None;
            state.preflight = None;
            state.preflight_target = None;
            let generation = state.generation;
            let client = client.clone();
            Task::perform(async move { load_groups(&client?).await }, move |result| {
                message(ManagedMessage::Loaded { identity: current.clone(), generation, result })
            })
        }
        ManagedMessage::Loaded { identity, generation, result } => {
            if !same_host(&identity, &current) || generation != state.generation || !state.loading {
                return Task::none();
            }
            state.loading = false;
            match result {
                Ok(groups) => state.replace_groups(groups),
                Err(error) => return fail(settings, effects, error),
            }
            Task::none()
        }
        ManagedMessage::Toggle(id) => {
            if let Some(group) = state.groups.iter_mut().find(|g| g.project.id.as_ref() == id) {
                group.collapsed = !group.collapsed;
            }
            Task::none()
        }
        ManagedMessage::Open { project, worktree } => {
            if state.opening.is_some() {
                return Task::none();
            }
            let Some((_, item)) = selected(settings, &project, &worktree) else {
                return Task::none();
            };
            if item.worktree.status != WorktreeStatus::Ready {
                return Task::none();
            }
            let target = (project.clone(), worktree.clone());
            settings.managed.opening = Some(target.clone());
            settings.managed.error = None;
            let navigation = navigation_generation;
            let client = client.clone();
            Task::perform(
                async move {
                    let client = client?;
                    let scope =
                        ProjectScope::new(client.endpoint().tenant_scope().clone(), project.into())
                            .map_err(|e| e.to_string())?;
                    let project = gateway_project::GatewayProjectClient::new(client.transport().clone()).project_get(&scope).await?.project;
                    let item = gateway_project::GatewayProjectClient::new(client.transport().clone()).managed_worktrees(&scope)
                        .await?
                        .items
                        .into_iter()
                        .find(|w| {
                            w.worktree.id.as_ref() == worktree
                                && w.worktree.project_id == project.id
                                && w.worktree.status == WorktreeStatus::Ready
                                && w.worktree.directory != project.directory
                        })
                        .ok_or("工作树已不可用，请刷新。")?;
                    Ok((project, item.worktree.directory))
                },
                move |result| {
                    message(ManagedMessage::Prepared {
                        identity: current.clone(),
                        navigation,
                        target: target.clone(),
                        result,
                    })
                },
            )
        }
        ManagedMessage::Prepared { identity, navigation, target, result } => {
            if !same_host(&identity, &current) || state.opening.as_ref() != Some(&target) {
                return Task::none();
            }
            state.opening = None;
            if identity.project != current.project
                || navigation != navigation_generation
            {
                return Task::none();
            }
            match result {
                Err(error) => fail(settings, effects, error),
                Ok((project, directory)) => {
                    if project.id.as_ref() != target.0 {
                        return fail(settings, effects, "项目身份已变化，请刷新。".into());
                    }
                    effects.push(Effect::Open { project, directory });
                    let client = client.clone();
                    let scope = scope(&current, &target.0);
                    Task::perform(
                        async move {
                            gateway_project::GatewayProjectClient::new(client?.transport().clone()).managed_worktree_record_use(&scope?, &target.1)
                                .await
                                .map(|_| ())
                        },
                        move |result| {
                            message(ManagedMessage::Used { identity: current.clone(), result })
                        },
                    )
                }
            }
        }
        ManagedMessage::Preflight { project, worktree } => {
            if state.loading || settings.recovery.pending.is_some() {
                return Task::none();
            }

            let Some((_, item)) = selected(settings, &project, &worktree) else {
                return Task::none();
            };
            if !item.protection_reasons.is_empty() || item.worktree.status != WorktreeStatus::Ready
            {
                return Task::none();
            }
            let target = (project.clone(), worktree.clone());
            settings.recovery.ignored_acknowledged = false;
            let state = &mut settings.managed;
            state.generation += 1;
            state.identity = Some(current.clone());
            state.preflight_target = Some(target.clone());
            state.preflight = None;
            state.error = None;
            let generation = state.generation;
            let scope = scope(&current, &project);
            let client = client.clone();
            Task::perform(
                async move { gateway_project::GatewayProjectClient::new(client?.transport().clone()).managed_worktree_deletion_preflight(&scope?, &worktree).await },
                move |result| {
                    message(ManagedMessage::PreflightLoaded {
                        identity: current.clone(),
                        generation,
                        target: target.clone(),
                        result,
                    })
                },
            )
        }
        ManagedMessage::PreflightLoaded { identity, generation, target, result } => {
            if identity != current
                || generation != state.generation
                || state.preflight_target.as_ref() != Some(&target)
            {
                return Task::none();
            }
            match result {
                Ok(value) => state.preflight = Some(value),
                Err(error) => {
                    state.preflight_target = None;
                    return fail(settings, effects, error);
                }
            }
            Task::none()
        }
        ManagedMessage::Used { identity, result } => {
            if same_host(&identity, &current)
                && let Err(error) = result
            {
                return fail(settings, effects, error);
            }
            Task::none()
        }
    }
}
