//! Failover admission from a settled, authorized mode snapshot.
use crate::providers::{live::commit::AppState, mode_state::observe};
use vw_api_types::{agent_apps::AppId, app_providers::mode_state::ProviderMode, id::TenantScope};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailoverError {
    UnsupportedApp,
    Forbidden,
    Inactive,
    AggregationConflict,
}

/// Call under the publication owner's lock, after pending recovery and revision
/// validation. This rule does not mutate the saved queue or grant authority.
pub fn admit(app: AppId, tenant: &TenantScope, state: &AppState) -> Result<(), FailoverError> {
    if !crate::routing::supports_app(app) {
        return Err(FailoverError::UnsupportedApp);
    }
    let mode = observe(state, tenant).map_err(|_| FailoverError::Forbidden)?;
    if mode.mode != ProviderMode::Proxy || !mode.attached {
        return Err(FailoverError::Inactive);
    }
    if mode.stack.enabled {
        return Err(FailoverError::AggregationConflict);
    }
    Ok(())
}
