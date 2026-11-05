//! Persistent catalog records contain specifications only. Host permission is never deserialized.

use std::sync::Arc;

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use vw_api_types::id::ProjectScope;

use super::store::SessionStore;
use crate::tools::catalog::{ToolCatalog, ToolCatalogSnapshot};
use crate::tools::{Tool, ToolSpec};

/// Runtime-owned policy. External requests and model input cannot grant this permission.
#[derive(Debug, Clone, Copy, Default)]
pub struct PersistentToolPolicy {
    allowed: bool,
}

impl PersistentToolPolicy {
    /// Only trusted host setup may opt into persistent updates; never derive this from a request.
    pub fn trusted_host() -> Self {
        Self { allowed: true }
    }
}

/// Complete persisted replacement, including an explicitly empty tool collection.
/// Executors, connection credentials and pending calls are deliberately absent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedDynamicTools {
    pub version: u64,
    pub specs: Vec<ToolSpec>,
}

// Both backends use the same scoped key and JSON representation. A session must already exist.
pub(super) const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS session_dynamic_tools (
    platform_id TEXT NOT NULL CHECK (trim(platform_id) <> ''),
    company_id TEXT NOT NULL CHECK (trim(company_id) <> ''),
    project_id TEXT NOT NULL CHECK (trim(project_id) <> ''),
    session_id TEXT NOT NULL CHECK (trim(session_id) <> ''),
    version BIGINT NOT NULL CHECK (version > 0),
    specs_json TEXT NOT NULL,
    PRIMARY KEY (platform_id, company_id, project_id, session_id),
    FOREIGN KEY (platform_id, company_id, project_id)
        REFERENCES projects(platform_id, company_id, project_id) ON DELETE RESTRICT,
    FOREIGN KEY (platform_id, company_id, project_id, session_id)
        REFERENCES session_index(platform_id, company_id, project_id, session_id) ON DELETE CASCADE
);
"#;

impl SessionStore {
    /// Validate, then serialize and commit under the catalog write lock before publishing.
    /// Concurrent replacements and MCP refreshes cannot pass the commit/publication boundary.
    pub fn replace_dynamic_tools(
        &self,
        scope: &ProjectScope,
        session_id: &str,
        catalog: &ToolCatalog,
        tools: Vec<Arc<dyn Tool>>,
        authorized: impl IntoIterator<Item = String>,
        policy: PersistentToolPolicy,
    ) -> Result<Arc<ToolCatalogSnapshot>> {
        if !policy.allowed {
            bail!("trusted host has disabled persistent dynamic tool updates");
        }
        catalog.replace_persisted(tools, authorized, |snapshot| {
            let record =
                PersistedDynamicTools { version: snapshot.version(), specs: snapshot.specs() };
            // No callbacks or session transport state can enter this representation.
            let payload = serde_json::to_string(&record)?;
            self.write_dynamic_tools(scope, session_id, record.version, &payload)
        })
    }
}
