//! Runtime settings stored in the gateway's configured database.

pub(crate) use super::timeouts::AppTimeouts;
use super::{
    RoutingListener,
    outbound_config::{EgressError, RoutingEgress, StoredEgress},
};
use crate::security::SecretStore;
use crate::{gateway::settings_store::GatewaySettingsStore, global_database::GlobalDatabase};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc};
use vw_agent_apps::providers::validate_identity;
use vw_api_types::{
    agent_apps::AppId,
    app_routing::{RoutingEgressStatus, RoutingEgressUpdate},
    id::{ProjectScope, TenantScope},
};

const GLOBAL: &str = "agent_apps.routing.listener.v1";
const EGRESS: &str = "agent_apps.routing.egress.v1";
const APP: &str = "agent_apps.routing.timeouts.v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ListenerConfig {
    pub(crate) enabled: bool,
    pub(crate) bind: SocketAddr,
    pub(crate) logging_enabled: bool,
}

impl ListenerConfig {
    pub(crate) fn validate(&self) -> Result<()> {
        ensure!(self.bind.ip().is_loopback(), "routing requires a loopback address");
        ensure!(self.bind.port() == 0 || self.bind.port() >= 1024, "invalid routing port");
        Ok(())
    }
}

pub(crate) struct RoutingConfigStore {
    settings: GatewaySettingsStore,
    tenant: TenantScope,
    host: String,
}

impl RoutingConfigStore {
    pub(crate) fn new(database: GlobalDatabase, tenant: TenantScope, host: String) -> Result<Self> {
        validate_identity(&host).map_err(anyhow::Error::msg)?;
        Ok(Self { settings: GatewaySettingsStore::new(database)?, tenant, host })
    }

    pub(crate) fn listener(&self) -> Result<Option<ListenerConfig>> {
        let Some(payload) = self.settings.get(&self.tenant, GLOBAL, &self.host)? else {
            return Ok(None);
        };
        decode_listener(&payload).map(Some)
    }

    /// Called after host management authorization. Check persisted attachments
    /// under the same control permit as the change to the client-facing bind.
    pub(crate) async fn apply_listener(
        self: &Arc<Self>,
        routing: &Arc<RoutingListener>,
        policy_default: ListenerConfig,
        config: ListenerConfig,
        check_rebind: impl FnOnce() -> Result<()> + Send + 'static,
    ) -> Result<()> {
        config.validate()?;
        let permit = routing.try_control()?;
        let store = Arc::clone(self);
        let routing = Arc::clone(routing);
        // The blocking worker holds the permit even if its caller disconnects.
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            let saved = store.settings.get(&store.tenant, GLOBAL, &store.host)?;
            let current =
                saved.as_deref().map(decode_listener).transpose()?.unwrap_or(policy_default);
            if current.bind != config.bind {
                check_rebind()?;
            }
            let payload = serde_json::to_string(&config)?;
            routing.apply_config(current, config, || {
                let committed = match saved {
                    Some(previous) => store.settings.compare_exchange(
                        &store.tenant,
                        GLOBAL,
                        &store.host,
                        &previous,
                        &payload,
                    )?,
                    None => store.settings.insert_if_absent(
                        &store.tenant,
                        GLOBAL,
                        &store.host,
                        &payload,
                    )?,
                };
                ensure!(committed, "routing configuration changed during update");
                Ok(())
            })
        })
        .await?
    }

    pub(crate) fn load_egress(&self, secrets: &SecretStore) -> Result<RoutingEgress> {
        let payload = self.settings.get(&self.tenant, EGRESS, &self.host)?;
        StoredEgress::restore(payload.as_deref(), secrets)
    }

    pub(crate) fn apply_egress(
        &self,
        routing: &RoutingListener,
        update: &RoutingEgressUpdate,
        secrets: &SecretStore,
    ) -> std::result::Result<RoutingEgressStatus, EgressError> {
        let _permit = routing.try_control().map_err(EgressError::Listener)?;
        let prepared = RoutingEgress::prepare(update, routing.bound_address())?;
        let payload = StoredEgress::encode(update, secrets).map_err(|_| EgressError::NotApplied)?;
        self.settings
            .upsert(&self.tenant, EGRESS, &self.host, &payload)
            .map_err(|_| EgressError::NotApplied)?;
        let status = prepared.status.clone();
        routing.set_egress(self.tenant.clone(), self.host.clone(), Some(prepared));
        Ok(status)
    }

    pub(crate) fn app_timeouts(
        &self,
        project: &ProjectScope,
        app: AppId,
    ) -> Result<Option<AppTimeouts>> {
        let key = self.app_key(project, app)?;
        let Some(payload) = self.settings.get(&self.tenant, APP, &key)? else {
            return Ok(None);
        };
        let timeouts: AppTimeouts = serde_json::from_str(&payload)?;
        timeouts.validate()?;
        Ok(Some(timeouts))
    }

    pub(crate) fn save_app_timeouts(
        &self,
        project: &ProjectScope,
        app: AppId,
        timeouts: AppTimeouts,
    ) -> Result<()> {
        let key = self.app_key(project, app)?;
        timeouts.validate()?;
        self.settings.upsert(&self.tenant, APP, &key, &serde_json::to_string(&timeouts)?)
    }

    fn app_key(&self, project: &ProjectScope, app: AppId) -> Result<String> {
        ensure!(project.tenant_scope() == &self.tenant, "invalid routing project scope");
        Ok(serde_json::to_string(&(&self.host, project.project_id(), app))?)
    }
}

fn decode_listener(payload: &str) -> Result<ListenerConfig> {
    let config: ListenerConfig = serde_json::from_str(payload)?;
    config.validate()?;
    Ok(config)
}
