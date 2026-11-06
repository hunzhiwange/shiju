//! Persistence adapter for the single application projection dispatcher.
use super::*;
use vw_agent_apps::providers::{universal, universal_sync};
use vw_api_types::app_providers::universal_sync::*;
impl AppProviderStore {
    pub fn universal_sync(
        &self,
        tenant: &TenantScope,
        id: &str,
        request: &UniversalSyncRequest,
    ) -> Result<Vec<UniversalSyncResult>, String> {
        validate_identity(id)?;
        universal_sync::validate_request(request)?;
        let raw = self
            .settings
            .get(tenant, "app_providers.universal", id)
            .map_err(|_| "application provider storage unavailable")?
            .ok_or("application provider not found")?;
        let parent: serde_json::Value = serde_json::from_str(&raw)
            .map_err(|_| "invalid stored application provider profile")?;
        universal::summary(id, &parent)
            .map_err(|_| "invalid stored application provider profile")?;
        Ok(request
            .apps
            .iter()
            .map(|&app| {
                let child = universal_sync::child_id(app, id);
                let result = child.as_ref().map_err(|e| *e).and_then(|child| {
                    self.project_universal_child(tenant, id, &parent, app, child)
                });
                UniversalSyncResult {
                    app_id: app,
                    provider_id: child.ok(),
                    catalog: match result {
                        Ok(()) => UniversalCatalogResult::Updated,
                        Err(code) => UniversalCatalogResult::Failed { code },
                    },
                    live: UniversalLiveResult::NotAttempted,
                }
            })
            .collect())
    }
    fn project_universal_child(
        &self,
        tenant: &TenantScope,
        id: &str,
        parent: &serde_json::Value,
        app: AppId,
        child: &str,
    ) -> Result<(), UniversalSyncError> {
        let key = Self::key(app, child).map_err(|_| UniversalSyncError::InvalidTargetIdentity)?;
        let before = self
            .settings
            .get(tenant, NAMESPACE, &key)
            .map_err(|_| UniversalSyncError::StorageUnavailable)?;
        let stored = before
            .as_ref()
            .map(|raw| serde_json::from_str(raw).map_err(|_| UniversalSyncError::InvalidProfile))
            .transpose()?;
        let projected = universal_sync::project(id, parent, app, stored.as_ref())?;
        let after =
            serde_json::to_string(&projected).map_err(|_| UniversalSyncError::InvalidProfile)?;
        let changed = match before {
            Some(before) => {
                self.settings.compare_exchange(tenant, NAMESPACE, &key, &before, &after)
            }
            None => self.settings.insert_if_absent(tenant, NAMESPACE, &key, &after),
        }
        .map_err(|_| UniversalSyncError::StorageUnavailable)?;
        if !changed {
            return Err(UniversalSyncError::ChangedDuringUpdate);
        }
        Ok(())
    }
}
