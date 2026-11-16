//! Server-owned distinction between inherited fork history and its own execution.
use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::binding_store::storage_key;
use std::path::{Path, PathBuf};
use vw_api_transport::TenantScope;

fn store(home: &Path) -> Result<app_settings_store::settings_store::GatewaySettingsStore> {
    let database = agent_database::GlobalDatabase::for_home(home)?;
    app_settings_store::settings_store::GatewaySettingsStore::new(database)
}

const NAMESPACE: &str = "session.fork-execution";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForkExecution {
    /// IDs belong to the copied target history, never supplied by clients.
    pub inherited_message_ids: Vec<String>,
    pub started: bool,
}

pub async fn load_scoped(home: PathBuf, tenant: TenantScope, project: &str, session: &str) -> Result<Option<ForkExecution>> {
    let key = storage_key(project, session)?;
    agent_database::run_isolated(move || {
        store(&home)?
            .get(&tenant, NAMESPACE, &key)?
            .as_deref()
            .map(serde_json::from_str)
            .transpose()
            .map_err(Into::into)
    })
    .await?
}

async fn save(home: PathBuf, tenant: TenantScope, project: &str, session: &str, state: ForkExecution) -> Result<()> {
    let key = storage_key(project, session)?;
    let payload = serde_json::to_string(&state)?;
    agent_database::run_isolated(move || {
        store(&home)?.upsert(&tenant, NAMESPACE, &key, &payload)
    })
    .await?
}

/// Stage one complete boundary record before publishing the new session row.
pub async fn create(
    home: PathBuf, tenant: TenantScope,
    project: &str,
    session: &str,
    inherited_message_ids: Vec<String>,
) -> Result<()> {
    save(home, tenant, project, session, ForkExecution { inherited_message_ids, started: false }).await
}

pub async fn remove(home: PathBuf, tenant: TenantScope, project: &str, session: &str) -> Result<()> {
    let key = storage_key(project, session)?;
    agent_database::run_isolated(move || {
        store(&home)?.delete(&tenant, NAMESPACE, &key).map(|_| ())
    })
    .await?
}

/// Call under message::lock_stream_prompt after request validation, before spawning execution.
/// Once set, cancellation and runtime failures must never undo this transition.
pub async fn start(home: PathBuf, tenant: TenantScope, project: &str, session: &str) -> Result<()> {
    if let Some(mut state) = load_scoped(home.clone(), tenant.clone(), project, session).await?
        && !state.started
    {
        state.started = true;
        save(home, tenant, project, session, state).await?;
    }
    Ok(())
}

pub async fn is_selectable(home: PathBuf, tenant: TenantScope, project: &str, session: &str) -> Result<bool> {
    Ok(load_scoped(home, tenant, project, session).await?.is_some_and(|state| !state.started))
}

/// Reload the authoritative execution state instead of trusting a serialized projection.
pub async fn project(home: PathBuf, tenant: TenantScope, info: &mut session_info::Info) -> Result<()> {
    info.fork_execution_started = if info.source_slug().starts_with("turn-fork-") {
        load_scoped(home, tenant, &info.project_id, &info.id).await?.map(|state| state.started)
    } else {
        None
    };
    Ok(())
}
