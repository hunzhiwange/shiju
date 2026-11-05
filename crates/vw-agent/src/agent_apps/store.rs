//! Tenant-scoped icon library storage through the shared gateway settings backend.

use crate::global_database::config::DatabaseProvider;
use anyhow::Result;
use vw_agent_apps::icons::library;
use vw_api_types::agent_apps::{AddCustomIconResult, AgentAppsErrorCode};
use vw_api_types::{agent_apps::CustomIconEntry, id::TenantScope};

use crate::{gateway::settings_store::GatewaySettingsStore, global_database::GlobalDatabase};

pub(super) const ICON_NAMESPACE: &str = "agent_apps.custom_icons";
pub(super) const ICON_KEY: &str = "default";

/// Fixed-key icon storage; callers cannot select arbitrary settings keys.
#[derive(Debug, Clone)]
pub struct IconLibraryStore {
    settings: GatewaySettingsStore,
    pub(super) database: GlobalDatabase,
}

impl IconLibraryStore {
    /// Open the configured SQLite or PostgreSQL gateway settings backend.
    pub fn new(database: GlobalDatabase) -> Result<Self> {
        Ok(Self { settings: GatewaySettingsStore::new(database.clone())?, database })
    }

    /// Add or deduplicate an image. IDs and timestamps are supplied by the caller.
    pub fn add_icon(
        &self,
        tenant: &TenantScope,
        name: &str,
        data: &str,
        id: &str,
        created_at: i64,
    ) -> Result<AddCustomIconResult, AgentAppsErrorCode> {
        self.mutate(tenant, |icons| library::add_icon(icons, name, data, id, created_at))
    }

    /// Rename an existing icon while preserving its image and identity.
    pub fn rename_icon(
        &self,
        tenant: &TenantScope,
        id: &str,
        name: &str,
    ) -> Result<CustomIconEntry, AgentAppsErrorCode> {
        self.mutate(tenant, |icons| {
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
        self.mutate(tenant, |icons| Ok(((), library::delete_icon(icons, id))))
    }

    fn mutate<T>(
        &self,
        tenant: &TenantScope,
        operation: impl FnOnce(
            &[CustomIconEntry],
        ) -> Result<(T, Option<Vec<CustomIconEntry>>), AgentAppsErrorCode>,
    ) -> Result<T, AgentAppsErrorCode> {
        match self.database.provider() {
            DatabaseProvider::Sqlite => self.mutate_sqlite(tenant, operation),
            DatabaseProvider::Postgres => self.mutate_postgres(tenant, operation),
        }
    }

    /// Read in stored order, tolerating malformed arrays and individual entries.
    pub fn read_icons(&self, tenant: &TenantScope) -> Result<Vec<CustomIconEntry>> {
        let Some(raw) = self.settings.get(tenant, ICON_NAMESPACE, ICON_KEY)? else {
            return Ok(Vec::new());
        };
        Ok(parse_icons(&raw))
    }
}

pub(super) fn parse_icons(raw: &str) -> Vec<CustomIconEntry> {
    let Ok(values) = serde_json::from_str::<Vec<serde_json::Value>>(&raw) else {
        tracing::warn!(error_category = "invalid_icon_array", "icon library parse failure");
        return Vec::new();
    };
    values
        .into_iter()
        .filter_map(|value| match serde_json::from_value(value) {
            Ok(icon) => Some(icon),
            Err(_) => {
                tracing::warn!(error_category = "invalid_icon_entry", "icon library parse failure");
                None
            }
        })
        .collect()
}
