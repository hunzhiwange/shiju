//! Confirmation and recovery use only the gateway's managed lifecycle endpoints.
use crate::{messages::{ManagedMessage, RecoveryMessage, WorktreeSettingsMessage}, state::*};
use crate::lifecycle::Effect;
use iced::Task;
use {
    vw_api_transport::id::ProjectScope,
    vw_api_project::ListProjectsRequest,
    vw_api_worktree::*,
};


fn message(event: RecoveryMessage) -> WorktreeSettingsMessage {
    WorktreeSettingsMessage::Recovery(event)
}
/// A navigation boundary revokes confirmations and ignores all outstanding responses.
pub fn invalidate(settings: &mut WorktreeSettingsState) {
    let state = &mut *settings;
    state.recovery = crate::state::WorktreeRecoveryState {
        generation: state.recovery.generation + 1,
        ..Default::default()
    };
    state.managed.generation += 1;
    state.managed.preflight = None;
    state.managed.preflight_target = None;
}
fn toast(effects: &mut Vec<Effect>, success: bool) -> Task<WorktreeSettingsMessage> {
    effects.push(Effect::Toast { success, message: if success { "工作树操作已完成。" } else { "工作树操作失败，请查看详情并重试。" } });
    Task::none()
}
fn accepts(settings: &WorktreeSettingsState, current: &WorktreeSettingsIdentity, identity: &WorktreeSettingsIdentity, generation: u64) -> bool {
    identity == current
        && settings.recovery.identity.as_ref() == Some(identity)
        && settings.recovery.generation == generation
}
pub fn update(
    settings: &mut WorktreeSettingsState,
    event: RecoveryMessage,
    current: WorktreeSettingsIdentity,
    navigation_generation: u64,
    client: Result<vw_gateway_client::GatewayClient, String>,
    effects: &mut Vec<Effect>,
) -> Task<WorktreeSettingsMessage> {
    match event {
        RecoveryMessage::Cancel => {
            if settings.recovery.pending.is_none() {
                let state = &mut *settings;
                state.managed.generation += 1;
                state.managed.preflight = None;
                state.managed.preflight_target = None;
                state.recovery.ignored_acknowledged = false;
            }
            Task::none()
        }
        RecoveryMessage::Ignored(value) => {
            if settings.recovery.pending.is_none() {
                settings.recovery.ignored_acknowledged = value;
            }
            Task::none()
        }
        RecoveryMessage::Confirm => {
            let state = &mut *settings;
            if state.recovery.pending.is_some() || state.managed.identity.as_ref() != Some(&current)
            {
                return Task::none();
            }
            let Some(target) = state.managed.preflight_target.clone() else {
                return Task::none();
            };
            let Some(preflight) = state.managed.preflight.as_ref() else {
                return Task::none();
            };
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis();
            if u128::from(preflight.expires_at_ms) <= now
                || preflight.fingerprint.is_empty()
                || !preflight.protection_reasons.is_empty()
                || (preflight.has_ignored_files && !state.recovery.ignored_acknowledged)
            {
                state.recovery.error =
                    Some("预检已过期、工作树受保护或尚未确认忽略文件丢失；请重新预检。".into());
                return toast(effects, false);
            }
            let request = DeleteManagedWorktreeRequest {
                fingerprint: preflight.fingerprint.clone(),
                confirmed: true,
                confirm_ignored_loss: state.recovery.ignored_acknowledged,
            };
            state.recovery.generation += 1;
            state.recovery.loading = false;
            state.recovery.pending = Some(target.clone());
            state.recovery.identity = Some(current.clone());
            state.recovery.error = None;
            let generation = state.recovery.generation;
            let client = client.clone();
            let scope = ProjectScope::new(current.tenant.clone(), target.0.clone().into())
                .map_err(|e| e.to_string());
            let worktree = target.1.clone();
            Task::perform(
                async move { gateway_project::GatewayProjectClient::new(client?.transport().clone()).managed_worktree_delete(&scope?, &worktree, &request).await },
                move |result| {
                    message(RecoveryMessage::Deleted {
                        identity: current.clone(),
                        generation,
                        target: target.clone(),
                        result,
                    })
                },
            )
        }
        RecoveryMessage::Deleted { identity, generation, target, result } => {
            if !accepts(settings, &current, &identity, generation)
                || settings.recovery.pending.as_ref() != Some(&target)
            {
                return Task::none();
            }
            let success = result.is_ok();
            let state = &mut *settings;
            state.recovery.pending = None;
            state.recovery.error = result.as_ref().err().cloned();
            state.recovery.outcomes.push((
                target.0,
                target.1,
                result.map(|value| value.snapshot_id),
            ));
            // A failed deletion also consumes the local confirmation: always re-preflight.
            state.managed.preflight = None;
            state.managed.preflight_target = None;
            state.recovery.ignored_acknowledged = false;
            let feedback = toast(effects, success);
            if success {
                Task::batch([
                    feedback,
                    super::managed::update(settings, ManagedMessage::Refresh, current.clone(), navigation_generation, client.clone(), effects),
                    update(settings, RecoveryMessage::Refresh, current, navigation_generation, client, effects),
                ])
            } else {
                feedback
            }
        }
        RecoveryMessage::Refresh => {
            let state = &mut settings.recovery;
            if state.pending.is_some() || state.loading {
                return Task::none();
            }
            state.generation += 1;
            state.identity = Some(current.clone());
            state.loading = true;
            state.error = None;
            let generation = state.generation;
            let client = client.clone();
            Task::perform(
                async move {
                    let client = client?;
                    let mut cursor = None;
                    let mut seen = std::collections::HashSet::new();
                    let mut projects = Vec::new();
                    loop {
                        let page = gateway_project::GatewayProjectClient::new(client.transport().clone()).project_list(
                                client.endpoint().tenant_scope(),
                                &ListProjectsRequest {
                                    cursor,
                                    limit: Some(100),
                                    ..Default::default()
                                },
                            )
                            .await?;
                        for project in page.items {
                            if projects
                                .iter()
                                .any(|p: &WorktreeRecoveryProject| p.project.id == project.id)
                            {
                                continue;
                            }
                            let scope = ProjectScope::new(
                                client.endpoint().tenant_scope().clone(),
                                project.id.clone(),
                            )
                            .map_err(|e| e.to_string())?;
                            let snapshots =
                                gateway_project::GatewayProjectClient::new(client.transport().clone()).worktree_snapshots(&scope).await.map(|list| list.items);
                            let proposal = gateway_project::GatewayProjectClient::new(client.transport().clone()).worktree_cleanup_proposal(&scope).await;
                            projects.push(WorktreeRecoveryProject { project, snapshots, proposal });
                        }
                        cursor = page.next_cursor.filter(|c| !c.is_empty());
                        match &cursor {
                            None => return Ok(projects),
                            Some(c) if !seen.insert(c.clone()) => {
                                return Err("项目分页游标重复，请刷新。".into());
                            }
                            _ => {}
                        }
                    }
                },
                move |result| {
                    message(RecoveryMessage::Loaded {
                        identity: current.clone(),
                        generation,
                        result,
                    })
                },
            )
        }
        RecoveryMessage::Loaded { identity, generation, result } => {
            if !accepts(settings, &current, &identity, generation) || !settings.recovery.loading {
                return Task::none();
            }
            let state = &mut settings.recovery;
            state.loading = false;
            match result {
                Ok(projects) => {
                    let failed =
                        projects.iter().any(|p| p.snapshots.is_err() || p.proposal.is_err());
                    state.projects = projects;
                    state.loaded = true;
                    if failed {
                        return toast(effects, false);
                    }
                }
                Err(error) => {
                    state.error = Some(error);
                    return toast(effects, false);
                }
            }
            Task::none()
        }
        RecoveryMessage::Restore { project, snapshot } => {
            let state = &mut settings.recovery;
            if state.pending.is_some() || state.loading || state.identity.as_ref() != Some(&current)
            {
                return Task::none();
            }
            let valid = state.projects.iter().any(|p| {
                p.project.id.as_ref() == project
                    && p.snapshots
                        .as_ref()
                        .is_ok_and(|rows| rows.iter().any(|s| s.id == snapshot && !s.restored))
            });
            if !valid {
                return Task::none();
            }
            let target = (project.clone(), snapshot.clone());
            state.generation += 1;
            state.pending = Some(target.clone());
            state.error = None;
            let generation = state.generation;
            let scope = ProjectScope::new(current.tenant.clone(), project.into())
                .map_err(|e| e.to_string());
            let client = client.clone();
            Task::perform(
                async move { gateway_project::GatewayProjectClient::new(client?.transport().clone()).worktree_snapshot_restore(&scope?, &snapshot).await },
                move |result| {
                    message(RecoveryMessage::Restored {
                        identity: current.clone(),
                        generation,
                        target: target.clone(),
                        result,
                    })
                },
            )
        }
        RecoveryMessage::Restored { identity, generation, target, result } => {
            if !accepts(settings, &current, &identity, generation)
                || settings.recovery.pending.as_ref() != Some(&target)
            {
                return Task::none();
            }
            settings.recovery.pending = None;
            match result {
                Ok(value) if value.worktree.worktree.project_id.as_ref() == target.0 => {
                    settings.recovery.restored = Some((
                        target.0,
                        value.worktree.worktree.id.as_ref().to_owned(),
                        value.worktree.worktree.directory,
                    ));
                    let feedback = toast(effects, true);
                    Task::batch([
                        feedback,
                        super::managed::update(settings, ManagedMessage::Refresh, current.clone(), navigation_generation, client.clone(), effects),
                        update(settings, RecoveryMessage::Refresh, current, navigation_generation, client, effects),
                    ])
                }
                result => {
                    settings.recovery.error =
                        Some(result.err().unwrap_or_else(|| "恢复响应项目身份不匹配。".into()));
                    toast(effects, false)
                }
            }
        }
    }
}
