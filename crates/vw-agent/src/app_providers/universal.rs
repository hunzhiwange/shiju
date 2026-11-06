//! Universal records use the existing tenant settings backend, one atomic row per identity.
use super::*;
use vw_agent_apps::providers::universal;
use vw_api_types::app_providers::universal::{UniversalProviderSummary, UniversalProviderWrite};
const UNIVERSAL: &str = "app_providers.universal";
impl AppProviderStore {
    pub fn universal_save(
        &self,
        tenant: &TenantScope,
        id: &str,
        request: &UniversalProviderWrite,
    ) -> Result<UniversalProviderSummary, String> {
        validate_identity(id)?;
        let before = self
            .settings
            .get(tenant, UNIVERSAL, id)
            .map_err(|_| "application provider storage unavailable")?;
        let stored = before
            .as_ref()
            .map(|v| serde_json::from_str(v).map_err(|_| "invalid stored universal provider"))
            .transpose()?;
        let updated = universal::save(id, stored.as_ref(), request)?;
        let summary = universal::summary(id, &updated)?;
        let after =
            serde_json::to_string(&updated).map_err(|_| "invalid stored universal provider")?;
        let changed = match before {
            Some(before) => self.settings.compare_exchange(tenant, UNIVERSAL, id, &before, &after),
            None => self.settings.insert_if_absent(tenant, UNIVERSAL, id, &after),
        }
        .map_err(|_| "application provider storage unavailable")?;
        if !changed {
            return Err("application provider changed during update".into());
        }
        Ok(summary)
    }
    pub fn universal_get(
        &self,
        tenant: &TenantScope,
        id: &str,
    ) -> Result<UniversalProviderSummary, String> {
        validate_identity(id)?;
        let raw = self
            .settings
            .get(tenant, UNIVERSAL, id)
            .map_err(|_| "application provider storage unavailable")?
            .ok_or("application provider not found")?;
        let stored = serde_json::from_str(&raw).map_err(|_| "invalid stored universal provider")?;
        universal::summary(id, &stored)
    }
    pub fn universal_list(
        &self,
        tenant: &TenantScope,
    ) -> Result<Vec<UniversalProviderSummary>, String> {
        self.settings
            .list(tenant, UNIVERSAL)
            .map_err(|_| "application provider storage unavailable")?
            .into_iter()
            .map(|row| {
                let stored = serde_json::from_str(&row.payload_json)
                    .map_err(|_| "invalid stored universal provider")?;
                universal::summary(&row.config_key, &stored)
            })
            .collect()
    }
    /// Delete only this saved parent. Child projection lifecycle belongs to universal_sync.
    pub fn universal_delete(&self, tenant: &TenantScope, id: &str) -> Result<bool, String> {
        validate_identity(id)?;
        self.settings
            .delete(tenant, UNIVERSAL, id)
            .map_err(|_| "application provider storage unavailable".into())
    }
}
