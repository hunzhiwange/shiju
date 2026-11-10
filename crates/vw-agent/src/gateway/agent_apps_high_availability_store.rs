//! Persistence for an authorized application's ordered failover queue.

use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use vw_agent_apps::{
    high_availability::queue::Queue, providers::validate_identity, routing::supports_app,
};
use vw_api_types::agent_apps_high_availability::{FailoverQueueScope, FailoverQueueVersion};

use super::settings_store::GatewaySettingsStore;
use crate::global_database::{GlobalDatabase, config::DatabaseProvider};

pub(super) const NAMESPACE: &str = "agent_apps.high_availability";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FailoverQueueState {
    pub version: FailoverQueueVersion,
    pub enabled: bool,
    pub members: Vec<String>,
}

impl Default for FailoverQueueState {
    fn default() -> Self {
        Self { version: FailoverQueueVersion(0), enabled: false, members: Vec::new() }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("failover queue version conflict")]
pub struct FailoverQueueVersionConflict;

pub struct FailoverQueueStore(GatewaySettingsStore);

impl FailoverQueueStore {
    pub fn new(database: GlobalDatabase) -> Result<Self> {
        ensure!(database.provider() == DatabaseProvider::Sqlite, "SQLite failover queue required");
        Ok(Self(GatewaySettingsStore::new(database)?))
    }

    /// The caller must resolve and authorize the project, host and app before reading.
    pub fn get(&self, scope: &FailoverQueueScope) -> Result<FailoverQueueState> {
        let key = config_key(scope)?;
        let payload = self.0.get(scope.project.tenant_scope(), NAMESPACE, &key)?;
        decode(payload.as_deref())
    }

    /// The caller must authorize queue management for this scope.
    /// The change runs with the SQLite write lock held and should only mutate the in-memory queue.
    pub fn update(
        &self,
        scope: &FailoverQueueScope,
        expected_version: FailoverQueueVersion,
        change: impl FnOnce(&mut Queue, &mut bool) -> Result<()>,
    ) -> Result<FailoverQueueState> {
        let key = config_key(scope)?;
        self.0.update_sqlite(scope.project.tenant_scope(), NAMESPACE, &key, |payload| {
            let mut state = decode(payload)?;
            if state.version != expected_version {
                bail!(FailoverQueueVersionConflict);
            }

            let mut queue = Queue::new(scope.app_id, state.members.clone());
            let mut enabled = state.enabled;
            change(&mut queue, &mut enabled)?;
            if queue.members() == state.members && enabled == state.enabled {
                return Ok((state, None));
            }

            state.version.0 =
                state.version.0.checked_add(1).context("failover queue version exhausted")?;
            state.members = queue.members().to_vec();
            state.enabled = enabled;
            let replacement = serde_json::to_string(&state)?;
            Ok((state, Some(replacement)))
        })
    }
}

pub(super) fn config_key(scope: &FailoverQueueScope) -> Result<String> {
    ensure!(supports_app(scope.app_id), "application does not support failover");
    validate_identity(&scope.target_host_id)
        .map_err(|_| anyhow::anyhow!("invalid failover queue host identity"))?;
    Ok(serde_json::to_string(&(scope.project.project_id(), &scope.target_host_id, scope.app_id))?)
}

fn decode(payload: Option<&str>) -> Result<FailoverQueueState> {
    payload
        .map(|json| serde_json::from_str(json).context("invalid saved failover queue"))
        .transpose()
        .map(Option::unwrap_or_default)
}
