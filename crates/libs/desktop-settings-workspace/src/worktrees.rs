use crate::{messages::WorktreeSettingsMessage, state::*};
use config_worktree::WorktreeSettings;
pub fn apply(
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
                    environment: crate::state::WorktreeEnvironmentState {
                        generation: state.environment.generation + 1,
                        list_generation: state.environment.list_generation + 1,
                        ..Default::default()
                    },
                    recovery: crate::state::WorktreeRecoveryState {
                        generation: state.recovery.generation + 1,
                        ..Default::default()
                    },
                    managed: crate::state::ManagedWorktreesState {
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

/// Persist the exact draft acknowledged by PATCH without requiring a follow-up GET.
pub async fn save_policy(
    client: &vw_gateway_client::GatewayClient,
    policy: WorktreeSettings,
) -> Result<WorktreeSettings, String> {
    gateway_config::GatewayConfigClient::new(client.transport().clone())
        .global_config_patch(&serde_json::json!({"worktree_settings":policy}))
        .await?;
    // PATCH acknowledged this exact draft. A follow-up GET must not turn
    // a committed save into an apparent failure if the connection drops.
    Ok(policy)
}

/// Read tenant policy and the gateway-owned default worktree root.
pub async fn load_policy(
    client: &vw_gateway_client::GatewayClient,
) -> Result<(WorktreeSettings, String), String> {
    let config = gateway_config::GatewayConfigClient::new(client.transport().clone())
        .global_config_get()
        .await?;
    let policy = serde_json::from_value(
        config.get("worktree_settings").cloned().ok_or("网关未返回工作树设置。")?,
    )
    .map_err(|e| e.to_string())?;
    let paths = gateway_system::GatewaySystemClient::new(client.transport().clone())
        .system_path(None)
        .await?;
    let root = paths
        .get("task_worktrees_root")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or("网关未返回工作树默认目录。")?
        .to_owned();
    Ok((policy, root))
}

/// The shell performs only the cross-page refresh and toast effects selected here.
pub enum UpdatePlan {
    Ignore,
    Refresh,
    Environment,
    Apply { policy: Option<WorktreeSettings>, refresh: bool, failed: bool, saved: bool },
}
pub fn prepare(
    state: &mut WorktreeSettingsState,
    event: WorktreeSettingsMessage,
    current: WorktreeSettingsIdentity,
    visible: bool,
) -> UpdatePlan {
    if matches!(event, WorktreeSettingsMessage::TabSelected(WorktreesTab::General)) {
        state.active_tab = WorktreesTab::General;
        return UpdatePlan::Refresh;
    }
    if matches!(event, WorktreeSettingsMessage::TabSelected(WorktreesTab::Environment)) {
        state.active_tab = WorktreesTab::Environment;
        return UpdatePlan::Environment;
    }
    if state.identity.as_ref().is_some_and(|old| {
        old.gateway != current.gateway
            || old.tenant != current.tenant
            || old.auth_fingerprint != current.auth_fingerprint
    }) {
        let generation = state.generation + 1;
        *state = WorktreeSettingsState {
            generation,
            recovery: crate::state::WorktreeRecoveryState {
                generation: state.recovery.generation + 1,
                ..Default::default()
            },
            environment: crate::state::WorktreeEnvironmentState {
                generation: state.environment.generation + 1,
                list_generation: state.environment.list_generation + 1,
                ..Default::default()
            },
            managed: crate::state::ManagedWorktreesState {
                generation: state.managed.generation + 1,
                ..Default::default()
            },
            ..Default::default()
        };
        return UpdatePlan::Refresh;
    }
    if let WorktreeSettingsMessage::Saved { identity, generation, .. } = &event {
        let same_host = identity.gateway == current.gateway
            && identity.tenant == current.tenant
            && identity.auth_fingerprint == current.auth_fingerprint;
        if same_host
            && state.save_generation == Some(*generation)
            && (identity != &current || *generation != state.generation)
        {
            // The write has finished, but its old project response cannot update
            // this view. Keep the draft and reload before allowing another save.
            apply(state, event, &current);
            return UpdatePlan::Refresh;
        }
    }
    if matches!(event, WorktreeSettingsMessage::ContextChanged) {
        apply(state, event, &current);
        if !state.saving && visible {
            return UpdatePlan::Refresh;
        }
        return UpdatePlan::Ignore;
    }
    let response = match &event {
        WorktreeSettingsMessage::Loaded { identity, generation, .. } => {
            Some((identity, *generation, state.loading))
        }
        WorktreeSettingsMessage::Saved { identity, generation, .. } => {
            Some((identity, *generation, state.saving))
        }
        _ => None,
    };
    if let Some((identity, generation, active)) = response {
        if identity != &current
            || state.identity.as_ref() != Some(identity)
            || generation != state.generation
            || !active
        {
            // Reconcile the new project context without accepting the old response.
            if identity != &current && state.identity.as_ref() == Some(identity) {
                return UpdatePlan::Refresh;
            }
            return UpdatePlan::Ignore;
        }
    }
    let refresh = matches!(event, WorktreeSettingsMessage::Refresh);
    if refresh && state.saving {
        return UpdatePlan::Ignore;
    }
    let failed = matches!(
        &event,
        WorktreeSettingsMessage::Loaded { result: Err(_), .. }
            | WorktreeSettingsMessage::Saved { result: Err(_), .. }
    );
    let saved = matches!(&event, WorktreeSettingsMessage::Saved { result: Ok(_), .. });
    let policy = apply(state, event, &current);
    UpdatePlan::Apply { policy, refresh, failed, saved }
}

/// Reset every request generation when gateway identity changes.
pub fn gateway_changed(state: &mut crate::state::WorktreeSettingsState) {
    let generation = state.generation + 1;
    *state = crate::state::WorktreeSettingsState {
        generation,
        recovery: crate::state::WorktreeRecoveryState {
            generation: state.recovery.generation + 1,
            ..Default::default()
        },
        environment: crate::state::WorktreeEnvironmentState {
            generation: state.environment.generation + 1,
            list_generation: state.environment.list_generation + 1,
            ..Default::default()
        },
        managed: crate::state::ManagedWorktreesState {
            generation: state.managed.generation + 1,
            ..Default::default()
        },
        ..Default::default()
    };
}
