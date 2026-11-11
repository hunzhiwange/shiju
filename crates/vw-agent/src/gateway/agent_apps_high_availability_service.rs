//! Application failover enablement on the authorized execution host.
use super::agent_apps_high_availability_store::{FailoverQueueState, FailoverQueueStore};
use crate::{
    app_providers::AppProviderStore,
    app_routing::{RoutingListener, provider_publication::transactions},
    global_database::GlobalDatabase,
};
use anyhow::{Result, ensure};
use vw_agent_apps::providers::{claude::ClaudeFile, live::commit::Mode};
use vw_api_types::agent_apps_high_availability::{FailoverQueueScope, FailoverQueueVersion};

fn mode_unavailable<T>(_: T) -> anyhow::Error {
    anyhow::anyhow!("application mode is unavailable")
}

/// Caller authorizes provider editing and routing management for this exact
/// project, execution host and application before resolving its home/private root.
/// Lock ordering matches publication: native application, listener, credential owner.
pub fn set_enabled(
    database: GlobalDatabase,
    routing: &RoutingListener,
    home: &std::path::Path,
    root: &std::path::Path,
    scope: &FailoverQueueScope,
    version: FailoverQueueVersion,
    enabled: bool,
) -> Result<FailoverQueueState> {
    let store = FailoverQueueStore::new(database.clone())?;
    if !enabled {
        return store.update(scope, version, |_, enabled| {
            *enabled = false;
            Ok(())
        });
    }
    ensure!(
        scope.app_id == vw_api_types::agent_apps::AppId::Claude,
        "application has no published provider mode"
    );
    let tx = transactions(root, scope)
        .map_err(|_| anyhow::anyhow!("application mode is unavailable"))?;
    let _directory = tx.lock_claude_directory().map_err(mode_unavailable)?;
    let state = tx.state(scope.app_id).map_err(mode_unavailable)?;
    state.require_project(&scope.project).map_err(mode_unavailable)?;
    ensure!(
        tx.pending(scope.app_id).map_err(mode_unavailable)?.is_none(),
        "application mode publication is pending"
    );
    ensure!(!state.stack.enabled, "aggregation mode does not support failover");
    ensure!(
        state.mode.as_ref().is_some_and(|mode| mode.mode == Mode::Proxy && mode.attached),
        "failover requires an attached routing mode"
    );
    let directory = tx.claude_directory(scope.project.tenant_scope()).map_err(mode_unavailable)?;
    let file =
        ClaudeFile::open_selected(home, directory.as_ref(), false).map_err(mode_unavailable)?;
    routing.with_provider(scope, |provider, _generation| {
        ensure!(
            provider.matches_native(&state, file.bytes.as_deref()),
            "application route snapshot changed"
        );
        let configuration = AppProviderStore::new(database)
            .map_err(anyhow::Error::msg)?
            .failover_configuration(&scope.project, scope.app_id)
            .map_err(anyhow::Error::msg)?;
        let current = configuration
            .iter()
            .find(|entry| entry.provider.provider.id == provider.provider_id)
            .ok_or_else(|| anyhow::anyhow!("current provider is missing"))?;
        ensure!(
            provider.matches_record(&current.preimage.payload_json),
            "current provider configuration changed"
        );
        let (providers, preimages): (Vec<_>, Vec<_>) =
            configuration.into_iter().map(|entry| (entry.provider, entry.preimage)).unzip();
        let eligible = vw_agent_apps::high_availability::queue::Queue::new(scope.app_id, vec![])
            .candidates(&providers)
            .map_err(|_| anyhow::anyhow!("unsupported failover application"))?;
        ensure!(
            eligible.iter().any(|entry| entry.id == provider.provider_id),
            "current provider is missing or ineligible"
        );
        store.update_with_preimages(scope, version, &preimages, |queue, enabled| {
            if !queue.members().iter().any(|id| eligible.iter().any(|entry| entry.id == *id)) {
                queue
                    .add(&provider.provider_id, &providers)
                    .map_err(|_| anyhow::anyhow!("current provider is ineligible"))?;
            }
            *enabled = true;
            Ok(())
        })
    })
}

/// Authorized queue edits use the provider owner's snapshot and compare its
/// saved preimages in the same database transaction as the queue revision.
pub(crate) enum QueueChange {
    Add(String),
    Remove(String),
    Reorder(Vec<String>),
}

#[derive(Debug, thiserror::Error)]
#[error("invalid failover queue mutation: {0:?}")]
pub(crate) struct QueueMutationError(pub vw_agent_apps::high_availability::queue::QueueError);

pub(crate) fn update_queue(
    database: GlobalDatabase,
    scope: &FailoverQueueScope,
    version: FailoverQueueVersion,
    change: QueueChange,
) -> Result<FailoverQueueState> {
    let store = FailoverQueueStore::new(database.clone())?;
    match change {
        QueueChange::Add(id) => {
            let configuration = AppProviderStore::new(database)
                .map_err(anyhow::Error::msg)?
                .failover_configuration(&scope.project, scope.app_id)
                .map_err(anyhow::Error::msg)?;
            let (providers, preimages): (Vec<_>, Vec<_>) =
                configuration.into_iter().map(|entry| (entry.provider, entry.preimage)).unzip();
            store.update_with_preimages(scope, version, &preimages, |queue, _| {
                queue.add(&id, &providers).map_err(QueueMutationError)?;
                Ok(())
            })
        }
        QueueChange::Remove(id) => store.update(scope, version, |queue, _| {
            queue.remove(&id).map_err(QueueMutationError)?;
            Ok(())
        }),
        QueueChange::Reorder(ids) => store.update(scope, version, |queue, _| {
            queue.reorder(ids).map_err(QueueMutationError)?;
            Ok(())
        }),
    }
}
