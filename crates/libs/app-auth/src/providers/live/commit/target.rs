use super::*;

pub(in crate::providers::live) fn validate_target(app: AppId, target: &PendingTarget) -> Result<(), WriteError> {
    if let Some(recovery) = &target.aggregation_recovery
        && (!matches!(app, AppId::Claude | AppId::Codex)
            || target.project_scope.is_none()
            || !target.stack.as_ref().is_some_and(|stack| stack.enabled)
            || !target.mode.as_ref().is_some_and(|mode| mode.attached && mode.mode == Mode::Proxy)
            || recovery.principal.trim().is_empty()
            || recovery.principal.chars().any(char::is_control)
            || !recovery.published.is_object())
    {
        return Err(WriteError::InvalidResource);
    }

    if let Some(selection) = &target.omo {
        if app != AppId::OpenCode || target.project_scope.is_none() {
            return Err(WriteError::InvalidResource);
        }
        match selection {
            OmoSelection::Standard(id) | OmoSelection::Slim(id) => {
                crate::providers::validate_identity(id).map_err(|_| WriteError::InvalidResource)?;
            }
            OmoSelection::Disabled => {}
        }
    }
    if target.codex_aggregation_settings.is_some()
        && (app != AppId::Codex || target.project_scope.is_none())
    {
        return Err(WriteError::InvalidResource);
    }
    if let Some(login) = &target.claude_managed_login {
        if app != AppId::Claude
            || target.kimi_binding.is_some()
            || target.stack.is_some()
            || login.principal.trim().is_empty()
            || login.principal.chars().any(char::is_control)
            || !target.mode.as_ref().is_some_and(|mode| mode.mode == Mode::Direct && !mode.attached)
        {
            return Err(WriteError::InvalidResource);
        }
        crate::providers::validate_identity(&login.account_id)
            .map_err(|_| WriteError::InvalidResource)?;
    }
    if let Some(binding) = &target.kimi_binding {
        if app != AppId::Kimi
            || target.pointer.is_some()
            || target.stack.is_some()
            || !target.mode.as_ref().is_some_and(|m| m.mode == Mode::Direct && !m.attached)
            || binding.principal.trim().is_empty()
            || binding.principal.chars().any(char::is_control)
        {
            return Err(WriteError::InvalidResource);
        }
        crate::providers::kimi::validate_identity(&binding.provider_id)?;
        crate::providers::validate_identity(&binding.account_id)
            .map_err(|_| WriteError::InvalidResource)?;
    }
    for id in target.pointer.iter().chain(target.mode.iter().filter_map(|m| m.proxy_route.as_ref()))
    {
        crate::providers::validate_identity(id).map_err(|_| WriteError::InvalidResource)?;
    }
    if let Some(stack) = &target.stack {
        let mut members = std::collections::HashSet::new();
        for id in &stack.members {
            crate::providers::validate_identity(id).map_err(|_| WriteError::InvalidResource)?;
            if !members.insert(id) {
                return Err(WriteError::InvalidResource);
            }
        }
        for (key, id) in &stack.keys {
            crate::providers::validate_identity(key).map_err(|_| WriteError::InvalidResource)?;
            crate::providers::validate_identity(id).map_err(|_| WriteError::InvalidResource)?;
        }
    }
    // Keep the original control-state bound. Only the frozen projection may use
    // the remaining private journal budget; all persisted files remain <= 1 MiB.
    let mut control = target.clone();
    control.aggregation_recovery = None;
    if serde_json::to_vec(&control).map_err(|_| WriteError::InvalidResource)?.len() > 65536
        || serde_json::to_vec(target).map_err(|_| WriteError::InvalidResource)?.len() > 900_000
    {
        return Err(WriteError::InvalidResource);
    }
    Ok(())
}
/// Stack keys are permanent reservations: removal and reassignment are forbidden.
/// The stack has its own tenant owner, independent of direct-pointer changes.
pub(in crate::providers::live) fn validate_stack_update(
    state: &AppState,
    tenant: &TenantScope,
    target: &PendingTarget,
) -> Result<(), WriteError> {
    if let Some(stack) = &target.stack
        && (state.stack_tenant.as_ref().is_some_and(|t| t != tenant)
            || !state.stack.is_empty() && state.stack_tenant.as_ref() != Some(tenant)
            || state.stack.keys.iter().any(|(key, id)| stack.keys.get(key) != Some(id)))
    {
        return Err(WriteError::Conflict);
    }
    Ok(())
}
pub(in crate::providers::live) fn apply_target(state: &mut AppState, tenant: &TenantScope, target: &PendingTarget) {
    if let Some(recovery) = &target.aggregation_recovery {
        state.aggregation_recovery = Some(recovery.clone());
    }

    if let Some(selection) = &target.omo {
        state.omo = Some(selection.clone());
    }
    if let Some(settings) = &target.codex_aggregation_settings {
        state.codex_aggregation_settings = Some(settings.clone());
    }
    if let Some(scope) = &target.project_scope {
        state.project_scope = Some(scope.clone());
    }
    if let Some(id) = &target.pointer {
        state.pointer = Some(ScopedProvider { tenant: tenant.clone(), id: id.clone() });
    }
    if let Some(stack) = &target.stack {
        state.stack = stack.clone();
        state.stack_tenant = Some(tenant.clone());
    }
    if let Some(login) = &target.claude_managed_login {
        state.claude_managed_login = Some(login.clone());
    }
    if let Some(binding) = &target.kimi_binding {
        state.kimi_bindings.insert(binding.provider_id.clone(), binding.clone());
    }
    if let Some(mode) = &target.mode {
        state.mode = Some(mode.clone());
        state.mode_tenant = Some(tenant.clone());
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Intent,
    Marked,
    BeforePublish,
    FilePublished,
    Published,
    Settled,
}
#[cfg(any(test, feature = "test-support"))]
type CheckpointHook = Box<dyn FnMut(&ProviderTransactions, AppId, Phase) -> Result<(), WriteError>>;
#[cfg(any(test, feature = "test-support"))]
thread_local! { pub static HOOK: std::cell::RefCell<Option<CheckpointHook>> = const { std::cell::RefCell::new(None) }; }
