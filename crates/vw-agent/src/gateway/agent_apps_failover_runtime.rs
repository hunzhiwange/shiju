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
}

impl Default for FailoverRuntime {
    fn default() -> Self {
        Self {
            health: HealthRegistry::new(NonZeroUsize::new(1024).unwrap(), Duration::from_secs(300)),
        }
    }
}

/// The app policy and session reliability settings are separate owners. This
/// read never consults client headers or creates another session retry budget.
pub(crate) fn policy(
    database: GlobalDatabase,
    scope: &FailoverQueueScope,
) -> anyhow::Result<FailoverPolicy> {
    let key = policy_key(scope)?;
    let saved = GatewaySettingsStore::new(database)?.get(
        scope.project.tenant_scope(),
        super::agent_apps_high_availability_store::NAMESPACE,
        &key,
    )?;
    // Defaults match the fixed upstream circuit configuration. Timeout fields
    // are only wire compatibility: dispatch reads its authoritative AppTimeouts.
    let policy =
        saved.as_deref().map(serde_json::from_str).transpose()?.unwrap_or(FailoverPolicy {
            circuit_breaker: CircuitBreakerPolicy {
                failure_threshold: 4,
                success_threshold: 2,
                cooldown_secs: 60,
                error_rate_threshold: 0.6,
                min_requests: 10,
            },
            retry_timeout: RetryTimeoutPolicy {
                max_retries: 3,
                first_byte_timeout_secs: 60,
                idle_timeout_secs: 120,
                non_streaming_timeout_secs: 600,
            },
        });
    policy.validate().map_err(anyhow::Error::msg)?;
    Ok(policy)
}

pub(crate) fn policy_key(scope: &FailoverQueueScope) -> anyhow::Result<String> {
    // Reuse the queue owner's identity validation and stable scope encoding.
    Ok(format!("policy:{}", super::agent_apps_high_availability_store::config_key(scope)?))
}
