//! Sole saved-model storage owner. Immutable deployment revisions prevent mixed provider/catalog saves.
use crate::{gateway::settings_store::GatewaySettingsStore, global_database::GlobalDatabase};
use sha2::{Digest, Sha256};
use vw_api_types::{id::TenantScope, model_catalog::*};
const NAMESPACE: &str = "model_catalog.saved";
pub(crate) struct ModelCatalogStore(GatewaySettingsStore);
impl ModelCatalogStore {
    pub fn new(db: GlobalDatabase) -> Result<Self, String> {
        GatewaySettingsStore::new(db).map(Self).map_err(|_| "model catalog unavailable".into())
    }
    fn key(identity: &ModelCatalogIdentity, revision: &str) -> Result<String, String> {
        vw_agent_apps::models::validate(identity, None)?;
        if revision.len() != 64 || !revision.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("invalid catalog revision".into());
        }
        serde_json::to_string(&(identity, revision)).map_err(|_| "invalid model identity".into())
    }
    pub fn read(
        &self,
        tenant: &TenantScope,
        identity: &ModelCatalogIdentity,
        revision: Option<&str>,
    ) -> Result<Option<SavedCodexCatalog>, String> {
        let Some(revision) = revision else { return Ok(None) };
        let raw = self
            .0
            .get(tenant, NAMESPACE, &Self::key(identity, revision)?)
            .map_err(|_| "model catalog unavailable")?
            .ok_or("saved model catalog missing")?;
        if hex::encode(Sha256::digest(raw.as_bytes())) != revision {
            return Err("saved model catalog changed".into());
        }
        let catalog: SavedCodexCatalog =
            serde_json::from_str(&raw).map_err(|_| "invalid saved model catalog")?;
        vw_agent_apps::models::validate(identity, Some(&catalog))?;
        Ok(Some(catalog))
    }
    pub fn save(
        &self,
        tenant: &TenantScope,
        identity: &ModelCatalogIdentity,
        catalog: Option<&SavedCodexCatalog>,
    ) -> Result<Option<String>, String> {
        vw_agent_apps::models::validate(identity, catalog)?;
        let Some(catalog) = catalog else { return Ok(None) };
        let raw = serde_json::to_string(catalog).map_err(|_| "invalid model catalog")?;
        let revision = hex::encode(Sha256::digest(raw.as_bytes()));
        let key = Self::key(identity, &revision)?;
        if !self
            .0
            .insert_if_absent(tenant, NAMESPACE, &key, &raw)
            .map_err(|_| "model catalog unavailable")?
            && self
                .0
                .get(tenant, NAMESPACE, &key)
                .map_err(|_| "model catalog unavailable")?
                .as_ref()
                != Some(&raw)
        {
            return Err("saved model catalog changed".into());
        }
        Ok(Some(revision))
    }
}
