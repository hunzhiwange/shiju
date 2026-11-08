//! Persist catalog edits using the same scoped conditional update as other provider fields.
use super::{AppProviderStore, NAMESPACE};
use serde_json::Value;
use vw_agent_apps::models::edits;
use vw_api_types::{
    id::TenantScope,
    model_catalog::{
        ModelCatalogEditRequest, ModelCatalogIdentity, ModelCatalogReadResponse,
        ModelCatalogWriteResponse,
    },
};

impl AppProviderStore {
    /// Read an already authorized deployment from its tenant's provider record.
    pub fn read_model_catalog(
        &self,
        tenant: &TenantScope,
        deployment: &ModelCatalogIdentity,
    ) -> Result<ModelCatalogReadResponse, String> {
        let key = Self::key(deployment.app_id, &deployment.provider_id)?;
        let (_, record) = self.model_catalog_record(tenant, &key)?;
        edits::read(tenant, deployment, &record)
    }

    /// Save edits for an already authorized deployment without updating live CLI files.
    pub fn edit_model_catalog(
        &self,
        tenant: &TenantScope,
        request: &ModelCatalogEditRequest,
    ) -> Result<ModelCatalogWriteResponse, String> {
        let deployment = &request.deployment;
        let key = Self::key(deployment.app_id, &deployment.provider_id)?;
        let (before, record) = self.model_catalog_record(tenant, &key)?;
        edits::edit(tenant, &record, request, |updated| {
            self.settings
                .compare_exchange(tenant, NAMESPACE, &key, &before, &updated.to_string())
                .map_err(|_| "application provider storage unavailable".into())
        })
    }

    fn model_catalog_record(
        &self,
        tenant: &TenantScope,
        key: &str,
    ) -> Result<(String, Value), String> {
        let raw = self
            .settings
            .get(tenant, NAMESPACE, key)
            .map_err(|_| "application provider storage unavailable")?
            .ok_or("application provider not found")?;
        let record = serde_json::from_str(&raw)
            .map_err(|_| "invalid stored application provider profile")?;
        Ok((raw, record))
    }
}

#[cfg(test)]
#[path = "model_edits_tests.rs"]
mod tests;
