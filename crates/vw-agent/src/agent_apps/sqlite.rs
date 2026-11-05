//! Fixed icon mutations within one SQLite immediate transaction.
use super::store::{ICON_KEY, ICON_NAMESPACE, IconLibraryStore, parse_icons};
use crate::global_database::config::DatabaseProvider;
use anyhow::bail;
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use vw_agent_apps::icons::library;
use vw_api_types::{
    agent_apps::{AddCustomIconResult, AgentAppsErrorCode, CustomIconEntry},
    id::TenantScope,
};

impl IconLibraryStore {
    /// Add or deduplicate an image. IDs and timestamps are supplied by the caller.
    pub fn add_icon(
        &self,
        tenant: &TenantScope,
        name: &str,
        data: &str,
        id: &str,
        created_at: i64,
    ) -> Result<AddCustomIconResult, AgentAppsErrorCode> {
        self.mutate_sqlite(tenant, |icons| library::add_icon(icons, name, data, id, created_at))
    }

    /// Rename an existing icon while preserving its image and identity.
    pub fn rename_icon(
        &self,
        tenant: &TenantScope,
        id: &str,
        name: &str,
    ) -> Result<CustomIconEntry, AgentAppsErrorCode> {
        self.mutate_sqlite(tenant, |icons| {
            let updated = library::rename_icon(icons, id, name)?;
            let icon = updated
                .iter()
                .find(|icon| icon.id == id)
                .cloned()
                .ok_or(AgentAppsErrorCode::NotFound)?;
            Ok((icon, Some(updated)))
        })
    }

    /// Delete matching entries; missing IDs are successful no-ops.
    pub fn delete_icon(&self, tenant: &TenantScope, id: &str) -> Result<(), AgentAppsErrorCode> {
        self.mutate_sqlite(tenant, |icons| Ok(((), library::delete_icon(icons, id))))
    }

    fn mutate_sqlite<T>(
        &self,
        tenant: &TenantScope,
        operation: impl FnOnce(
            &[CustomIconEntry],
        ) -> Result<(T, Option<Vec<CustomIconEntry>>), AgentAppsErrorCode>,
    ) -> Result<T, AgentAppsErrorCode> {
        if self.database.provider() != DatabaseProvider::Sqlite {
            return Err(AgentAppsErrorCode::StorageUnavailable);
        }
        let mut business_error = None;
        self.database.with_sqlite_transaction(TransactionBehavior::Immediate, |transaction| {
            let raw: Option<String> = transaction.query_row(
                "SELECT payload_json FROM gateway_settings WHERE platform_id=?1 AND company_id=?2 AND namespace=?3 AND config_key=?4",
                params![tenant.platform_id().as_ref(), tenant.company_id().as_ref(), ICON_NAMESPACE, ICON_KEY], |row| row.get(0),
            ).optional()?;
            let icons = raw.as_deref().map(parse_icons).unwrap_or_default();
            let (result, updated) = match operation(&icons) {
                Ok(value) => value,
                Err(error) => { business_error = Some(error); bail!("icon mutation rejected"); }
            };
            if let Some(updated) = updated {
                let payload = serde_json::to_string(&updated)?;
                let updated_at = i64::try_from(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_millis())?;
                transaction.execute(
                    "INSERT INTO gateway_settings (platform_id,company_id,namespace,config_key,payload_json,updated_at_ms) VALUES (?1,?2,?3,?4,?5,?6) ON CONFLICT (platform_id,company_id,namespace,config_key) DO UPDATE SET payload_json=excluded.payload_json,updated_at_ms=excluded.updated_at_ms",
                    params![tenant.platform_id().as_ref(),tenant.company_id().as_ref(),ICON_NAMESPACE,ICON_KEY,payload,updated_at],
                )?;
            }
            Ok(result)
        }).map_err(|_| business_error.unwrap_or(AgentAppsErrorCode::StorageUnavailable))
    }
}
