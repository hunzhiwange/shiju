//! Persistent catalog records contain specifications only. Host permission is never deserialized.

use std::sync::Arc;

use anyhow::{Result, bail};
use vw_api_transport::id::ProjectScope;

use super::store::SessionStore;
use tool_assembly::catalog::{ToolCatalog, ToolCatalogSnapshot};
use tool_types::Tool;

/// Runtime-owned policy. External requests and model input cannot grant this permission.
#[derive(Debug, Clone, Copy, Default)]
pub struct PersistentToolPolicy {
    allowed: bool,
}

impl PersistentToolPolicy {
    pub fn allows_updates(self) -> bool {
        self.allowed
    }

    /// Only trusted host setup may opt into persistent updates; never derive this from a request.
    pub fn trusted_host() -> Self {
        Self { allowed: true }
    }
}

pub use session_port::PersistedDynamicTools;

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
    /// Copy typed specifications into the new session index, never live capabilities.
    pub fn copy_dynamic_tools(
        &self,
        scope: &ProjectScope,
        source: &str,
        target: &session_info::Info,
    ) -> Result<()> {
        if let Some(record) = self.load_dynamic_tools(scope, source)? {
            // Native creation precedes the UI projection. Preserve the existing ordering.
            self.save_session(
                scope,
                &session_ui_types::ChatSession {
                    id: target.id.clone(),
                    title: target.title.clone(),
                    messages: vec![],
                    message_ids: vec![],
                    calls: vec![],
                    steps: vec![],
                    created_ms: target.time.created,
                    updated_ms: target.time.updated,
                },
            )?;
            self.write_dynamic_tools(
                scope,
                &target.id,
                record.version,
                &serde_json::to_string(&record)?,
            )?;
        }
        Ok(())
    }

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

/// Recreate executability only from current host-owned executors and fresh grants.
/// Missing executors remain unavailable; saved metadata alone never grants a capability.
pub fn rebind(
    record: &PersistedDynamicTools,
    executors: Vec<Arc<dyn Tool>>,
    authorized: impl IntoIterator<Item = String>,
) -> Result<ToolCatalog> {
    let mut selected = Vec::new();
    for executor in executors {
        let current = executor.spec();
        let Some(saved) = record.specs.iter().find(|saved| saved.id == current.id) else {
            bail!(
                "executor is absent from restored dynamic tool specifications: {}",
                current.id
            );
        };
        if serde_json::to_value(saved)? != serde_json::to_value(&current)? {
            bail!("restored dynamic tool specification changed: {}", current.id);
        }
        selected.push(executor);
    }
    ToolCatalog::restored(record.version, selected, authorized)
}

/// Interrupted external effects must never become runnable calls on restore or fork.
pub fn interrupt_part(part: &mut message_types::Part) -> bool {
    use message_types::{Part, PartTime, ToolState, ToolStateError};
    let Part::Tool(tool) = part else { return false };
    let (input, start) = match &tool.state {
        ToolState::Pending(state) => (state.input.clone(), runtime_time::now_ms()),
        ToolState::Running(state) => (state.input.clone(), state.time.start),
        ToolState::Completed(_) | ToolState::Error(_) => return false,
    };
    tool.state = ToolState::Error(ToolStateError {
        input,
        error: "tool call interrupted by session restore; external effects are not replayed".into(),
        metadata: None,
        time: PartTime { start, end: Some(runtime_time::now_ms()) },
    });
    true
}
