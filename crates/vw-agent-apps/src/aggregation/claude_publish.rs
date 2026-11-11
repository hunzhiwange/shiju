//! Claude aggregation publication through the existing native transaction owner.
use super::{
    claude_catalog::{CLAUDE_DEFAULT_WINDOW, ClaudeCatalogMember, ClaudeModelCatalog},
    preview::{self, Catalog, PreviewInput},
};
use crate::providers::{
    claude::patch_with_residue,
    live::{
        commit::*,
        stage::FileChange,
        write_guard::{FilePrivacy, WriteError},
    },
    mode_apply::{CLIENT_TOKEN_PLACEHOLDER, ClaudeMode, ModeApplyError},
    mode_state::{self, ClientConfiguration},
};
use serde_json::{Map, Value, json};
use std::{cell::Cell, ffi::OsStr};
use vw_api_types::{
    agent_apps::AppId,
    app_providers::mode_apply::{ModeApplyResult, TargetPublication},
    id::ProjectScope,
};

/// The executor supplies authorized saved records and restore/failover proofs,
/// then revalidates their revision and project binding inside `unchanged`.
/// The caller holds the native ClaudeFile guard throughout this operation.
/// External route failure is reported as Pending and can be retried idempotently.
pub fn publish(
    mode: &mut ClaudeMode<'_>,
    project: &ProjectScope,
    expected: &AppState,
    claim_unowned: bool,
    input: PreviewInput<'_>,
    unchanged: impl Fn() -> bool,
) -> Result<ModeApplyResult, ModeApplyError> {
    if project.tenant_scope() != mode.scope.tenant {
        return Err(ModeApplyError::Ownership);
    }
    if mode.scope.app != AppId::Claude {
        return Err(ModeApplyError::InvalidInput);
    }
    if mode.transactions.execution_host() != &ExecutionHostId::new(mode.scope.host)? {
        return Err(ModeApplyError::Ownership);
    }
    if crate::providers::claude_login::configuration_has_pending_login(&mode.file.guard)? {
        return Err(WriteError::Conflict.into());
    }
    let state = mode.transactions.state(AppId::Claude)?;
    mode_state::observe(&state, mode.scope.tenant).map_err(|_| ModeApplyError::Ownership)?;
    if let Some(pending) = mode.transactions.pending(AppId::Claude)? {
        return Err(if pending.tenant != *mode.scope.tenant {
            ModeApplyError::Ownership
        } else {
            WriteError::Conflict.into()
        });
    }
    if state.project_scope.as_ref().is_some_and(|owner| owner != project) {
        return Err(ModeApplyError::Ownership);
    }
    if state.project_scope.is_none() && !claim_unowned {
        return Err(ModeApplyError::Ownership);
    }
    if state != *expected || !unchanged() {
        return Err(WriteError::Conflict.into());
    }
    let providers = input.providers;
    let planned =
        preview::preview(AppId::Claude, mode.scope.tenant, &state, input).map_err(|e| match e {
            preview::PreviewError::Forbidden => ModeApplyError::Ownership,
            preview::PreviewError::RecoveryPending => WriteError::Conflict.into(),
            _ => ModeApplyError::InvalidInput,
        })?;
    if planned.issues.iter().any(|issue| issue.blocking) {
        return Err(ModeApplyError::InvalidInput);
    }
    let default = planned.default_provider_id.as_deref().ok_or(ModeApplyError::InvalidInput)?;
    let members = planned
        .members
        .iter()
        .map(|member| {
            let provider = providers
                .iter()
                .find(|p| p.id == member.provider_id)
                .ok_or(ModeApplyError::InvalidInput)?;
            let Catalog::Claude(record) = &provider.catalog else {
                return Err(ModeApplyError::InvalidInput);
            };
            let key = state
                .stack
                .keys
                .iter()
                .find_map(|(key, id)| (id == provider.id).then_some(key.as_str()))
                .ok_or(ModeApplyError::InvalidInput)?;
            Ok(ClaudeCatalogMember { provider_id: provider.id, key, record })
        })
        .collect::<Result<Vec<_>, ModeApplyError>>()?;
    let catalog = ClaudeModelCatalog::new(&members, Some(default))
        .map_err(|_| ModeApplyError::InvalidInput)?;
    let role = catalog.default_model().ok_or(ModeApplyError::InvalidInput)?;
    let runtime = mode.runtime.ok_or(ModeApplyError::Unavailable)?;
    let endpoint = runtime.prepare(&mode.scope)?;
    let mut env = Map::new();
    let route = members
        .iter()
        .find(|member| member.provider_id == default)
        .ok_or(ModeApplyError::InvalidInput)?;
    let saved = route.record.get("settingsConfig").ok_or(ModeApplyError::InvalidInput)?;
    env.extend(
        ClientConfiguration::saved(AppId::Claude, saved, false)
            .map_err(|_| ModeApplyError::InvalidInput)?
            .contract()
            .exclusive,
    );
    merge_switches(&mut env, &members, default);
    env.remove("CLAUDE_CODE_MAX_CONTEXT_TOKENS");
    if let Some(window) = catalog
        .models()
        .iter()
        .filter(|m| !m.supports_1m)
        .map(|m| m.window)
        .min()
        .filter(|window| *window != CLAUDE_DEFAULT_WINDOW)
    {
        env.insert("CLAUDE_CODE_MAX_CONTEXT_TOKENS".into(), json!(window.to_string()));
    }
    env.insert("ANTHROPIC_BASE_URL".into(), json!(endpoint.url));
    env.insert("ANTHROPIC_AUTH_TOKEN".into(), json!(CLIENT_TOKEN_PLACEHOLDER));
    for tier in ["HAIKU", "SONNET", "OPUS", "FABLE"] {
        env.insert(format!("ANTHROPIC_DEFAULT_{tier}_MODEL"), json!(role.id));
        env.insert(format!("ANTHROPIC_DEFAULT_{tier}_MODEL_NAME"), json!(role.name));
    }
    let target = json!({"env":env,"modelPicker":{"replaceBuiltInOptions":true,
        "options":catalog.models().iter().map(|m|json!({"model":m.id,"label":m.label,"description":m.description})).collect::<Vec<_>>()}});
    let contract = ClientConfiguration::saved(AppId::Claude, &target, false)
        .map_err(|_| ModeApplyError::InvalidInput)?
        .contract();
    let next = ModeState {
        mode: Mode::Proxy,
        attached: true,
        proxy_route: Some(default.into()),
        contract: Some(contract),
    };
    let rewrite = state.mode.as_ref() != Some(&next);
    let previous =
        state.mode.as_ref().and_then(|m| m.contract.as_ref()).map(|c| json!({"env":c.exclusive}));
    let changed = Cell::new(false);
    let patch = |pre: Option<&[u8]>| {
        let after = patch_with_residue(pre, previous.as_ref(), &target, false)?;
        changed.set(Some(after.as_slice()) != pre);
        Ok(after)
    };
    let changes = if rewrite {
        vec![FileChange {
            name: OsStr::new(mode.file.name),
            privacy: FilePrivacy::Private,
            patch: &patch,
        }]
    } else {
        vec![]
    };
    let mut stack = state.stack.clone();
    stack.enabled = true;
    if !stack.members.iter().any(|id| id == default) {
        stack.members.insert(0, default.into());
    }
    mode.transactions.commit_project_checked(
        project,
        &mut mode.file.guard,
        &state,
        claim_unowned,
        &changes,
        PendingTarget { mode: Some(next), stack: Some(stack), ..Default::default() },
        || {
            if unchanged() && mode.transactions.state(AppId::Claude)? == state {
                Ok(())
            } else {
                Err(WriteError::Conflict)
            }
        },
    )?;
    let publication = if runtime.publish(&mode.scope, Some(default)).is_ok() {
        TargetPublication::Published
    } else {
        TargetPublication::Pending
    };
    Ok(ModeApplyResult {
        configuration_changed: changed.get(),
        attached: true,
        target_publication: publication,
    })
}

fn switch(value: &Value) -> Option<bool> {
    match value {
        Value::Bool(on) => Some(*on),
        Value::Number(n) => n.as_f64().map(|n| n != 0.0),
        Value::String(s) => match s.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Some(true),
            "0" | "false" | "no" | "off" | "" => Some(false),
            _ => None,
        },
        _ => None,
    }
}
fn merge_switches(
    env: &mut Map<String, Value>,
    members: &[ClaudeCatalogMember<'_>],
    default: &str,
) {
    let others: Vec<_> = members.iter().filter(|m| m.provider_id != default).collect();
    for key in [
        "CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS",
        "CLAUDE_CODE_DISABLE_ARTIFACT",
        "CLAUDE_CODE_AUTO_MODE_SERVER",
    ] {
        let off = key != "CLAUDE_CODE_AUTO_MODE_SERVER";
        if env.get(key).and_then(switch) == Some(off) {
            continue;
        }
        if let Some(value) = others
            .iter()
            .filter_map(|m| m.record.pointer("/settingsConfig/env").and_then(|e| e.get(key)))
            .find(|v| switch(v) == Some(off))
        {
            env.insert(key.into(), value.clone());
        }
    }
    for key in [
        "ENABLE_TOOL_SEARCH",
        "CLAUDE_CODE_ALWAYS_ENABLE_EFFORT",
        "CLAUDE_CODE_ENABLE_FINE_GRAINED_TOOL_STREAMING",
    ] {
        if others.iter().any(|m| {
            m.record.pointer("/settingsConfig/env").and_then(|e| e.get(key)).and_then(switch)
                != Some(true)
        }) {
            env.remove(key);
        }
    }
}
