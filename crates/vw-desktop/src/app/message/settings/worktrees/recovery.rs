//! Confirmation and recovery use only the gateway's managed lifecycle endpoints.
use super::{ManagedMessage, SettingsMessage, WorktreeSettingsMessage};
use crate::app::{
    App, Message,
    state::{WorktreeRecoveryProject, WorktreeSettingsIdentity},
};
use iced::Task;
use vw_gateway_client::vw_api_types::{
    id::ProjectScope, project::ListProjectsRequest, worktree::*,
};

#[derive(Debug, Clone)]
pub enum RecoveryMessage {
    Refresh,
    Loaded {
        identity: WorktreeSettingsIdentity,
        generation: u64,
        result: Result<Vec<WorktreeRecoveryProject>, String>,
    },
    Cancel,
    Ignored(bool),
    Confirm,
    Deleted {
        identity: WorktreeSettingsIdentity,
        generation: u64,
        target: (String, String),
        result: Result<DeleteManagedWorktreeResponse, String>,
    },
    Restore {
        project: String,
        snapshot: String,
    },
    Restored {
        identity: WorktreeSettingsIdentity,
        generation: u64,
        target: (String, String),
        result: Result<RestoreWorktreeResponse, String>,
    },
}
fn message(event: RecoveryMessage) -> Message {
    Message::Settings(SettingsMessage::Worktrees(WorktreeSettingsMessage::Recovery(event)))
}
/// A navigation boundary revokes confirmations and ignores all outstanding responses.
pub(crate) fn invalidate(app: &mut App) {
    let state = &mut app.worktree_settings;
    state.recovery = crate::app::state::WorktreeRecoveryState {
        generation: state.recovery.generation + 1,
        ..Default::default()
    };
    state.managed.generation += 1;
    state.managed.preflight = None;
    state.managed.preflight_target = None;
}
fn toast(app: &mut App, success: bool) -> Task<Message> {
    app.show_localized_toast(
        if success {
            crate::app::state::ToastKind::Success
        } else {
            crate::app::state::ToastKind::Error
        },
        if success {
            "工作树操作已完成。"
        } else {
            "工作树操作失败，请查看详情并重试。"
        },
        vec![],
    )
}
fn accepts(app: &App, identity: &WorktreeSettingsIdentity, generation: u64) -> bool {
    identity == &WorktreeSettingsIdentity::current(app)
        && app.worktree_settings.recovery.identity.as_ref() == Some(identity)
        && app.worktree_settings.recovery.generation == generation
}
pub(crate) fn update(app: &mut App, event: RecoveryMessage) -> Task<Message> {
    let current = WorktreeSettingsIdentity::current(app);
    match event {
        RecoveryMessage::Cancel => {
            if app.worktree_settings.recovery.pending.is_none() {
                let state = &mut app.worktree_settings;
                state.managed.generation += 1;
                state.managed.preflight = None;
                state.managed.preflight_target = None;
                state.recovery.ignored_acknowledged = false;
            }
            Task::none()
        }
        RecoveryMessage::Ignored(value) => {
            if app.worktree_settings.recovery.pending.is_none() {
                app.worktree_settings.recovery.ignored_acknowledged = value;
            }
            Task::none()
        }
        RecoveryMessage::Confirm => {
            let state = &mut app.worktree_settings;
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
                return toast(app, false);
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
            let client = crate::app::gateway_client();
            let scope = ProjectScope::new(current.tenant.clone(), target.0.clone().into())
                .map_err(|e| e.to_string());
            let worktree = target.1.clone();
            Task::perform(
                async move { client?.managed_worktree_delete(&scope?, &worktree, &request).await },
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
            if !accepts(app, &identity, generation)
                || app.worktree_settings.recovery.pending.as_ref() != Some(&target)
            {
                return Task::none();
            }
            let success = result.is_ok();
            let state = &mut app.worktree_settings;
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
            let feedback = toast(app, success);
            if success {
                Task::batch([
                    feedback,
                    super::managed::update(app, ManagedMessage::Refresh),
                    update(app, RecoveryMessage::Refresh),
                ])
            } else {
                feedback
            }
        }
        RecoveryMessage::Refresh => {
            let state = &mut app.worktree_settings.recovery;
            if state.pending.is_some() || state.loading {
                return Task::none();
            }
            state.generation += 1;
            state.identity = Some(current.clone());
            state.loading = true;
            state.error = None;
            let generation = state.generation;
            let client = crate::app::gateway_client();
            Task::perform(
                async move {
                    let client = client?;
                    let mut cursor = None;
                    let mut seen = std::collections::HashSet::new();
                    let mut projects = Vec::new();
                    loop {
                        let page = client
                            .project_list(
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
                                client.worktree_snapshots(&scope).await.map(|list| list.items);
                            let proposal = client.worktree_cleanup_proposal(&scope).await;
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
            if !accepts(app, &identity, generation) || !app.worktree_settings.recovery.loading {
                return Task::none();
            }
            let state = &mut app.worktree_settings.recovery;
            state.loading = false;
            match result {
                Ok(projects) => {
                    let failed =
                        projects.iter().any(|p| p.snapshots.is_err() || p.proposal.is_err());
                    state.projects = projects;
                    state.loaded = true;
                    if failed {
                        return toast(app, false);
                    }
                }
                Err(error) => {
                    state.error = Some(error);
                    return toast(app, false);
                }
            }
            Task::none()
        }
        RecoveryMessage::Restore { project, snapshot } => {
            let state = &mut app.worktree_settings.recovery;
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
            let client = crate::app::gateway_client();
            Task::perform(
                async move { client?.worktree_snapshot_restore(&scope?, &snapshot).await },
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
            if !accepts(app, &identity, generation)
                || app.worktree_settings.recovery.pending.as_ref() != Some(&target)
            {
                return Task::none();
            }
            app.worktree_settings.recovery.pending = None;
            match result {
                Ok(value) if value.worktree.worktree.project_id.as_ref() == target.0 => {
                    app.worktree_settings.recovery.restored = Some((
                        target.0,
                        value.worktree.worktree.id.as_ref().to_owned(),
                        value.worktree.worktree.directory,
                    ));
                    let feedback = toast(app, true);
                    Task::batch([
                        feedback,
                        super::managed::update(app, ManagedMessage::Refresh),
                        update(app, RecoveryMessage::Refresh),
                    ])
                }
                result => {
                    app.worktree_settings.recovery.error =
                        Some(result.err().unwrap_or_else(|| "恢复响应项目身份不匹配。".into()));
                    toast(app, false)
                }
            }
        }
    }
}
