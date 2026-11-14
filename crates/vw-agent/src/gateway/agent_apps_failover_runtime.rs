//! Trusted runtime ownership for ordinary application requests. Session relay
//! requests retain their existing session budget and cannot claim this owner.
use super::settings_store::GatewaySettingsStore;
use crate::global_database::GlobalDatabase;
use std::{num::NonZeroUsize, time::Duration};
use vw_agent_apps::high_availability::health::HealthRegistry;
use vw_api_types::agent_apps_high_availability::{
    CircuitBreakerPolicy, FailoverPolicy, FailoverQueueScope, RetryTimeoutPolicy,
};

pub(crate) struct FailoverRuntime {
    pub(crate) health: HealthRegistry,
    pub(crate) events: parking_lot::RwLock<Option<tokio::sync::broadcast::Sender<serde_json::Value>>>,
}

impl Default for FailoverRuntime {
    fn default() -> Self {
        Self {
            events: parking_lot::RwLock::new(None),
            health: HealthRegistry::new(NonZeroUsize::new(1024).unwrap(), Duration::from_secs(300)),
        }
    }
}

/// One observation per resident grant; replacing or revoking the grant retires
/// its observations too. No client configuration or durable intent is written.
#[derive(Clone, Default)]
pub(crate) struct ActualRoute {
    pub(crate) version: u64,
    pub(crate) provider_id: Option<String>,
    pub(crate) failure_reason: Option<vw_api_types::agent_apps_high_availability::FailoverFailureReason>,
}

impl ActualRoute {
    pub(crate) fn commit(
        &mut self,
        expected: u64,
        provider_id: &str,
        configured: &str,
    ) -> anyhow::Result<Option<String>> {
        if self.version != expected {
            return Ok(None);
        }
        let previous = self.provider_id.as_deref().unwrap_or(configured);
        if previous == provider_id {
            self.provider_id = Some(provider_id.into());
            return Ok(None);
        }
        let next = self
            .version
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("actual route version exhausted"))?;
        let previous = previous.to_owned();
        self.provider_id = Some(provider_id.into());
        self.version = next;
        Ok(Some(previous))
    }
}

/// The app policy and session reliability settings are separate owners. This
/// read never consults client headers or creates another session retry budget.
pub(crate) fn policy(
    database: GlobalDatabase,
    scope: &FailoverQueueScope,
) -> anyhow::Result<FailoverPolicy> {
    if let Some(policy) = saved_policy(database.clone(), scope)? {
        return Ok(policy);
    }
    let timeouts = crate::app_routing::config_apply::RoutingConfigStore::new(
        database, scope.project.tenant_scope().clone(), scope.target_host_id.clone(),
    )?.app_timeouts(&scope.project, scope.app_id)?.unwrap_or_default();
    // Without an explicit failover policy, preserve the existing app timeouts.
    let policy = FailoverPolicy {
        circuit_breaker: CircuitBreakerPolicy {
            failure_threshold: 4,
            success_threshold: 2,
            cooldown_secs: 60,
            error_rate_threshold: 0.6,
            min_requests: 10,
        },
        retry_timeout: RetryTimeoutPolicy {
            max_retries: 3,
            first_byte_timeout_secs: timeouts.first_byte_timeout_secs,
            idle_timeout_secs: timeouts.idle_timeout_secs,
            non_streaming_timeout_secs: timeouts.non_streaming_timeout_secs,
        },
    };
    policy.validate().map_err(anyhow::Error::msg)?;
    Ok(policy)
}

pub(crate) fn saved_policy(
    database: GlobalDatabase,
    scope: &FailoverQueueScope,
) -> anyhow::Result<Option<FailoverPolicy>> {
    let saved = GatewaySettingsStore::new(database)?.get(
        scope.project.tenant_scope(),
        super::agent_apps_high_availability_store::NAMESPACE,
        &policy_key(scope)?,
    )?;
    let policy: Option<FailoverPolicy> = saved.as_deref().map(serde_json::from_str).transpose()?;
    if let Some(policy) = &policy {
        policy.validate().map_err(anyhow::Error::msg)?;
    }
    Ok(policy)
}

pub(crate) fn policy_key(scope: &FailoverQueueScope) -> anyhow::Result<String> {
    // Reuse the queue owner's identity validation and stable scope encoding.
    Ok(format!("policy:{}", super::agent_apps_high_availability_store::config_key(scope)?))
}
