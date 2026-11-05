//! Tenant-scoped icon library storage through the shared gateway settings backend.

use anyhow::Result;
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
