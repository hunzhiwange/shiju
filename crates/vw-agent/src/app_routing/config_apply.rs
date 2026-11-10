//! Runtime settings stored in the gateway's configured database.

use super::RoutingListener;
use crate::{gateway::settings_store::GatewaySettingsStore, global_database::GlobalDatabase};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc};
use vw_agent_apps::providers::validate_identity;
use vw_api_types::{
    agent_apps::AppId,
    id::{ProjectScope, TenantScope},
};

const GLOBAL: &str = "agent_apps.routing.listener.v1";
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct AppTimeouts {
    pub(crate) first_byte_timeout_secs: u16,
    pub(crate) idle_timeout_secs: u16,
    pub(crate) non_streaming_timeout_secs: u16,
}

impl AppTimeouts {
    pub(crate) fn validate(&self) -> Result<()> {
        ensure!(
            (1..=120).contains(&self.first_byte_timeout_secs)
                && self.idle_timeout_secs <= 600
                && (60..=1200).contains(&self.non_streaming_timeout_secs),
            "invalid routing timeouts"
        );
        Ok(())
    }
}

impl Default for AppTimeouts {
    fn default() -> Self {
        Self {
            first_byte_timeout_secs: 60,
            idle_timeout_secs: 120,
            non_streaming_timeout_secs: 600,
        }
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
