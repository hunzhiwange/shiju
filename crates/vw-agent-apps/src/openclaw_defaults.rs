//! OpenClaw defaults publication under the shared provider transaction.
pub use app_openclaw_config_rules::defaults::*;
use crate::providers::{
    live::{commit::{AppState, ProviderTransactions}, write_guard::WriteError},
    openclaw::OpenClawFiles,
};
use vw_api_transport::id::TenantScope;

/// Snapshot, preparation and publication share the provider/model physical lock.
pub fn set_agents_defaults(
    files: &mut OpenClawFiles,
    tx: &ProviderTransactions,
    tenant: &TenantScope,
    expected: &AppState,
    defaults: &OpenClawAgentsDefaults,
    unchanged: impl Fn() -> bool,
) -> Result<(), WriteError> {
    let (name, pre) = files.snapshot()?;
    let next = prepare_agents_defaults(pre.as_deref(), defaults)?;
    files.apply(tx, tenant, expected, name, pre.as_deref(), next, unchanged)
}
