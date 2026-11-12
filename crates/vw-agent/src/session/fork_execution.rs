//! Server-owned distinction between inherited fork history and its own execution.
use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::agent_binding::{storage_key, store};

const NAMESPACE: &str = "session.fork-execution";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ForkExecution {
    /// IDs belong to the copied target history, never supplied by clients.
    pub(crate) inherited_message_ids: Vec<String>,
    pub(crate) started: bool,
}

pub(crate) async fn load_scoped(project: &str, session: &str) -> Result<Option<ForkExecution>> {
    let key = storage_key(project, session)?;
    let tenant = super::ui_store::current_tenant_scope();
    crate::global_database::run_isolated(move || {
        store()?
            .get(&tenant, NAMESPACE, &key)?
            .as_deref()
            .map(serde_json::from_str)
            .transpose()
            .map_err(Into::into)
    })
    .await?
}

async fn save(project: &str, session: &str, state: ForkExecution) -> Result<()> {
    let key = storage_key(project, session)?;
    let tenant = super::ui_store::current_tenant_scope();
    let payload = serde_json::to_string(&state)?;
    crate::global_database::run_isolated(move || {
        store()?.upsert(&tenant, NAMESPACE, &key, &payload)
    })
    .await?
}

/// Stage one complete boundary record before publishing the new session row.
pub(crate) async fn create(
    project: &str,
    session: &str,
    inherited_message_ids: Vec<String>,
) -> Result<()> {
    save(project, session, ForkExecution { inherited_message_ids, started: false }).await
}

pub(crate) async fn remove(project: &str, session: &str) -> Result<()> {
    let key = storage_key(project, session)?;
    let tenant = super::ui_store::current_tenant_scope();
    crate::global_database::run_isolated(move || {
        store()?.delete(&tenant, NAMESPACE, &key).map(|_| ())
    })
    .await?
}

/// Call under message::lock_stream_prompt after request validation, before spawning execution.
/// Once set, cancellation and runtime failures must never undo this transition.
pub(crate) async fn start(project: &str, session: &str) -> Result<()> {
    if let Some(mut state) = load_scoped(project, session).await?
        && !state.started
    {
        state.started = true;
        save(project, session, state).await?;
    }
    Ok(())
}

pub(crate) async fn is_selectable(project: &str, session: &str) -> Result<bool> {
    Ok(load_scoped(project, session).await?.is_some_and(|state| !state.started))
}

/// Ignore any serialized projection and reload the authoritative server setting.
pub(crate) async fn project(info: &mut super::session::Info) -> Result<()> {
    info.fork_execution_started = if info.source_slug().starts_with("turn-fork-") {
        load_scoped(&info.project_id, &info.id).await?.map(|state| state.started)
    } else {
        None
    };
    Ok(())
}

#[cfg(test)]
#[path = "fork_execution_tests.rs"]
mod tests;
