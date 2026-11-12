//! Worktree policy editor events and gateway effects.
use crate::app::state::{WorktreeSettingsIdentity, WorktreeSettingsState, WorktreesTab};
use vw_config_types::worktree::WorktreeSettings;

mod managed;
pub use managed::ManagedMessage;
#[cfg(test)]
pub(crate) use managed::load_groups;

#[derive(Debug, Clone)]
pub enum WorktreeSettingsMessage {
    Managed(ManagedMessage),
    Refresh,
    ContextChanged,
    Edit,
    Cancel,
    Save,
    TabSelected(WorktreesTab),
    RootChanged(String),
    CountChanged(String),
    FetchChanged(bool),
    CleanupChanged(bool),
    AskChanged(bool),
    Loaded {
        identity: WorktreeSettingsIdentity,
        generation: u64,
        result: Result<(WorktreeSettings, String), String>,
    },
    Saved {
        identity: WorktreeSettingsIdentity,
        generation: u64,
        result: Result<WorktreeSettings, String>,
    },
}

pub(crate) fn apply(
    state: &mut WorktreeSettingsState,
    message: WorktreeSettingsMessage,
    current: &WorktreeSettingsIdentity,
) -> Option<WorktreeSettings> {
    use WorktreeSettingsMessage::*;
    match message {
        Loaded { identity, generation, result } => {
            if &identity != current
                || state.identity.as_ref() != Some(&identity)
                || generation != state.generation
                || !state.loading
            {
                return None;
            }
            state.loading = false;
            match result {
                Ok((policy, root)) => {
                    state.saved = Some(policy);
                    state.default_root = root;
                    state.error = None;
                }
                Err(error) => state.error = Some(error),
            }
        }
        Saved { identity, generation, result } => {
            if &identity != current
                || state.identity.as_ref() != Some(&identity)
                || generation != state.generation
                || !state.saving
            {
                if state.save_generation == Some(generation)
                    && state.identity.as_ref() == Some(current)
                    && identity.gateway == current.gateway
                    && identity.tenant == current.tenant
                    && identity.auth_fingerprint == current.auth_fingerprint
                {
                    state.saving = false;
                    state.save_generation = None;
                }
                return None;
            }
            state.saving = false;
            state.save_generation = None;
            match result {
                Ok(policy) => {
                    state.saved = Some(policy);
                    state.draft = None;
                    state.error = None;
                }
                Err(error) => state.error = Some(error),
            }
        }
        ContextChanged => {
            state.generation += 1;
            state.identity = Some(current.clone());
            state.loading = false;
        }
        Refresh => {
            if state.saving {
                return None;
            }
            // A project switch does not change this tenant-global draft. A gateway,
            // credential or tenant switch must never submit it to the new host.
            if state.identity.as_ref().is_some_and(|old| {
                old.gateway != current.gateway
                    || old.tenant != current.tenant
                    || old.auth_fingerprint != current.auth_fingerprint
            }) {
                let generation = state.generation;
                *state = WorktreeSettingsState {
                    generation,
                    managed: crate::app::state::ManagedWorktreesState {
                        generation: state.managed.generation + 1,
                        ..Default::default()
                    },
                    ..Default::default()
                };
            }
            state.identity = Some(current.clone());
            state.generation += 1;
            state.loading = true;
            state.saving = false;
            state.error = None;
        }
        TabSelected(tab) => state.active_tab = tab,
        Edit if !state.saving && state.identity.as_ref() == Some(current) => {
            if state.draft.is_none() {
                state.draft = state.saved.clone().map(Into::into);
            }
        }
        Cancel if !state.saving => {
            state.draft = None;
            state.error = None;
        }
        Save if !state.saving && state.identity.as_ref() == Some(current) => {
            let policy = state.draft.as_ref()?.validated();
            match policy {
                Ok(policy) => {
                    state.generation += 1;
                    state.loading = false;
                    state.saving = true;
                    state.save_generation = Some(state.generation);
                    state.error = None;
                    return Some(policy);
                }
                Err(error) => state.error = Some(error.into()),
            }
        }
        RootChanged(value) if !state.saving => {
            if let Some(draft) = &mut state.draft {
                draft.policy.root = value;
                state.error = None;
            }
        }
        CountChanged(value) if !state.saving => {
            if let Some(draft) = &mut state.draft {
                draft.keep_count = value;
                state.error = None;
            }
        }
        FetchChanged(value) if !state.saving => {
            if let Some(draft) = &mut state.draft {
                draft.policy.fetch_before_create = value;
                state.error = None;
            }
        }
        CleanupChanged(value) if !state.saving => {
            if let Some(draft) = &mut state.draft {
                draft.policy.auto_cleanup_enabled = value;
                state.error = None;
            }
        }
        AskChanged(value) if !state.saving => {
            if let Some(draft) = &mut state.draft {
                draft.policy.ask_before_delete = value;
                state.error = None;
            }
        }
        _ => {}
    }
    None
}

use super::SettingsMessage;
use crate::app::{App, Message};
use iced::Task;

pub(crate) fn update(app: &mut App, event: WorktreeSettingsMessage) -> Task<Message> {
    if let WorktreeSettingsMessage::Managed(event) = event {
        return managed::update(app, event);
    }
    let current = WorktreeSettingsIdentity::current(app);
    if app.worktree_settings.identity.as_ref().is_some_and(|old| {
        old.gateway != current.gateway
            || old.tenant != current.tenant
            || old.auth_fingerprint != current.auth_fingerprint
    }) {
        let generation = app.worktree_settings.generation + 1;
        app.worktree_settings = WorktreeSettingsState {
            generation,
            managed: crate::app::state::ManagedWorktreesState {
                generation: app.worktree_settings.managed.generation + 1,
                ..Default::default()
            },
            ..Default::default()
        };
        return update(app, WorktreeSettingsMessage::Refresh);
    }
    if let WorktreeSettingsMessage::Saved { identity, generation, .. } = &event {
        let same_host = identity.gateway == current.gateway
            && identity.tenant == current.tenant
            && identity.auth_fingerprint == current.auth_fingerprint;
        if same_host
            && app.worktree_settings.save_generation == Some(*generation)
            && (identity != &current || *generation != app.worktree_settings.generation)
        {
            // The write has finished, but its old project response cannot update
            // this view. Keep the draft and reload before allowing another save.
            apply(&mut app.worktree_settings, event, &current);
            return update(app, WorktreeSettingsMessage::Refresh);
        }
    }
    if matches!(event, WorktreeSettingsMessage::ContextChanged) {
        apply(&mut app.worktree_settings, event, &current);
        if !app.worktree_settings.saving
            && app.system_settings_tab
                == crate::app::components::system_settings::SystemTab::Worktrees
        {
            return update(app, WorktreeSettingsMessage::Refresh);
        }
        return Task::none();
    }
    let response = match &event {
        WorktreeSettingsMessage::Loaded { identity, generation, .. } => {
            Some((identity, *generation, app.worktree_settings.loading))
        }
        WorktreeSettingsMessage::Saved { identity, generation, .. } => {
            Some((identity, *generation, app.worktree_settings.saving))
        }
        _ => None,
    };
    if let Some((identity, generation, active)) = response {
        if identity != &current
            || app.worktree_settings.identity.as_ref() != Some(identity)
            || generation != app.worktree_settings.generation
            || !active
        {
            // Reconcile the new project context without accepting the old response.
            if identity != &current && app.worktree_settings.identity.as_ref() == Some(identity) {
                return update(app, WorktreeSettingsMessage::Refresh);
            }
            return Task::none();
        }
    }
    let refresh = matches!(event, WorktreeSettingsMessage::Refresh);
    if refresh && app.worktree_settings.saving {
        return Task::none();
    }
    let failed = matches!(
        &event,
        WorktreeSettingsMessage::Loaded { result: Err(_), .. }
            | WorktreeSettingsMessage::Saved { result: Err(_), .. }
    );
    let saved = matches!(&event, WorktreeSettingsMessage::Saved { result: Ok(_), .. });
    let policy = apply(&mut app.worktree_settings, event, &current);
    let generation = app.worktree_settings.generation;
    let client = crate::app::config::gateway_client();
    if let Some(policy) = policy {
        return Task::perform(
            async move {
                let client = client?;
                client
                    .global_config_patch(&serde_json::json!({"worktree_settings":policy}))
                    .await?;
                // PATCH acknowledged this exact draft. A follow-up GET must not turn
                // a committed save into an apparent failure if the connection drops.
                Ok(policy)
            },
            move |result| {
                Message::Settings(SettingsMessage::Worktrees(WorktreeSettingsMessage::Saved {
                    identity: current.clone(),
                    generation,
                    result,
                }))
            },
        );
    }
    if refresh {
        let managed_refresh = managed::update(app, ManagedMessage::Refresh);
        return Task::batch([
            managed_refresh,
            Task::perform(
                async move {
                    let client = client?;
                    let config = client.global_config_get().await?;
                    let policy = serde_json::from_value(
                        config.get("worktree_settings").cloned().ok_or("网关未返回工作树设置。")?,
                    )
                    .map_err(|e| e.to_string())?;
                    let paths = client.system_path(None).await?;
                    let root = paths
                        .get("task_worktrees_root")
                        .and_then(|v| v.as_str())
                        .filter(|s| !s.is_empty())
                        .ok_or("网关未返回工作树默认目录。")?
                        .to_owned();
                    Ok((policy, root))
                },
                move |result| {
                    Message::Settings(SettingsMessage::Worktrees(WorktreeSettingsMessage::Loaded {
                        identity: current.clone(),
                        generation,
                        result,
                    }))
                },
            ),
        ]);
    }
    if failed {
        return app.show_localized_toast(
            crate::app::state::ToastKind::Error,
            "工作树设置请求失败，未提交的编辑已保留。",
            vec![],
        );
    }
    if saved {
        return app.show_localized_toast(
            crate::app::state::ToastKind::Success,
            "工作树设置已保存。",
            vec![],
        );
    }
    Task::none()
}

pub(crate) fn gateway_changed(app: &mut App) -> Task<Message> {
    let generation = app.worktree_settings.generation + 1;
    app.worktree_settings = WorktreeSettingsState {
        generation,
        managed: crate::app::state::ManagedWorktreesState {
            generation: app.worktree_settings.managed.generation + 1,
            ..Default::default()
        },
        ..Default::default()
    };
    if app.system_settings_tab == crate::app::components::system_settings::SystemTab::Worktrees {
        update(app, WorktreeSettingsMessage::Refresh)
    } else {
        Task::none()
    }
}
