//! Authorized preference migration through the existing single settings owner.
use anyhow::{Result, ensure};
use vw_agent_apps::data_management::{preferences::PreferencesPreview, preview::Decision, source::SourceScope};
use crate::{gateway::settings_store::GatewaySettingsStore, global_database::GlobalDatabase};

/// Executor-local entry: caller holds migration authorization and a durable
/// safety snapshot for this scope. This does not grant authorization from the
/// ordinary preferences API. No host settings or system integrations are written.
pub fn apply(
    database: &GlobalDatabase, scope: &SourceScope, preview: &PreferencesPreview,
    source: &[u8], decision: Option<Decision>,
) -> Result<bool> {
    let store = GatewaySettingsStore::new(database.clone())?;
    let tenant = scope.project().tenant_scope();
    let key = scope.principal_id();
    let before = store.get(tenant, "agent_apps_user", key)?;
    let after = preview.resolve(scope, source, before.as_deref(), decision)
        .map_err(|_| anyhow::anyhow!("preference migration preview is stale or unresolved"))?;
    let Some(after) = after else { return Ok(false); };
    let payload = serde_json::to_string(after)?;
    let changed = match before {
        Some(before) => store.compare_exchange(tenant, "agent_apps_user", key, &before, &payload)?,
        None => store.insert_if_absent(tenant, "agent_apps_user", key, &payload)?,
    };
    ensure!(changed, "preference migration target changed; preview again");
    Ok(true)
}
