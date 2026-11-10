//! Revalidate managed runtime selections before using a cached MCP connection.

use agent_client_protocol::McpServer;
use serde::{Deserialize, Serialize};
use vw_api_types::{
    ProjectScope, TenantScope,
    mcp_management::{McpCatalogScope, McpFailureCode, McpRuntimeSelection},
};

use super::{
    McpCatalogStore,
    runtime::{StoredSelection, catalog_error, scope_belongs_to_project, selection_namespace},
};
use crate::{
    global_database::{GlobalDatabase, run_sync_database},
    tools::mcp_common::server_name,
};

const RUNTIME_SELECTION: &str = "shiju.managedMcpSelection";

#[derive(Serialize, Deserialize)]
pub(super) struct RuntimeSelectionLease {
    project: ProjectScope,
    selection: StoredSelection,
}

impl RuntimeSelectionLease {
    pub(super) fn attach(
        server: &mut McpServer,
        project: &ProjectScope,
        selection: StoredSelection,
    ) -> anyhow::Result<()> {
        let meta = match server {
            McpServer::Stdio(server) => &mut server.meta,
            McpServer::Http(server) => &mut server.meta,
            McpServer::Sse(server) => &mut server.meta,
            _ => anyhow::bail!("unsupported managed MCP transport"),
        };
        let lease = Self { project: project.clone(), selection };
        meta.get_or_insert_with(Default::default)
            .insert(RUNTIME_SELECTION.to_owned(), serde_json::to_value(lease)?);
        Ok(())
    }
}

impl McpCatalogStore {
    /// Disable the selected revision in a project already authorized by the caller.
    pub fn disable_for_runtime(
        &self,
        authorized_project: &ProjectScope,
        authorized_catalog: &McpCatalogScope,
        selection: &McpRuntimeSelection,
    ) -> Result<(), McpFailureCode> {
        if authorized_project != &selection.project
            || !scope_belongs_to_project(authorized_catalog, authorized_project)
        {
            return Err(McpFailureCode::PermissionDenied);
        }
        let tenant = authorized_project.tenant_scope();
        let namespace = selection_namespace(authorized_project)?;
        let key = &selection.record.id.0;
        let previous = self
            .0
            .get(tenant, &namespace, key)
            .map_err(|_| McpFailureCode::StorageUnavailable)?
            .ok_or(McpFailureCode::NotFound)?;
        let mut current: StoredSelection =
            serde_json::from_str(&previous).map_err(|_| McpFailureCode::StorageUnavailable)?;
        if &current.scope != authorized_catalog {
            return Err(McpFailureCode::Conflict);
        }
        if current.record != selection.record {
            return Err(McpFailureCode::StaleRevision);
        }
        if current.disabled {
            return Ok(());
        }
        // A later selection of the same revision must not revive its old executors.
        current.generation =
            current.generation.checked_add(1).ok_or(McpFailureCode::StorageUnavailable)?;
        current.disabled = true;
        self.save_entry(tenant, &namespace, key, &previous, &current)
    }
}

pub(crate) fn runtime_selection_is_current(
    server: &McpServer,
    owner_tenant: Option<&TenantScope>,
) -> anyhow::Result<bool> {
    let meta = match server {
        McpServer::Stdio(server) => server.meta.as_ref(),
        McpServer::Http(server) => server.meta.as_ref(),
        McpServer::Sse(server) => server.meta.as_ref(),
        _ => None,
    };
    let Some(value) = meta.and_then(|meta| meta.get(RUNTIME_SELECTION)) else {
        return Ok(true);
    };
    let RuntimeSelectionLease { project, selection } = serde_json::from_value(value.clone())
        .map_err(|_| catalog_error(McpFailureCode::InvalidDefinition))?;
    let tenant = crate::tools::current_tool_use_context()
        .map(|context| context.tenant_scope())
        .or_else(|| owner_tenant.cloned())
        .unwrap_or_else(crate::session::ui_store::current_tenant_scope);
    let key = &selection.record.id.0;
    if project.tenant_scope() != &tenant
        || owner_tenant.is_some_and(|owner| owner != &tenant)
        || !scope_belongs_to_project(&selection.scope, &project)
        || server_name(server) != key
        || selection.disabled
    {
        return Ok(false);
    }
    let namespace = selection_namespace(&project).map_err(catalog_error)?;
    let database = GlobalDatabase::for_home(&crate::global::paths().home)?;
    run_sync_database(|| {
        let store = McpCatalogStore::new(database).map_err(catalog_error)?;
        let Some(payload) = store.0.get(&tenant, &namespace, key)? else {
            return Ok(false);
        };
        let current: StoredSelection = serde_json::from_str(&payload)
            .map_err(|_| catalog_error(McpFailureCode::StorageUnavailable))?;
        if current != selection {
            return Ok(false);
        }
        match store.connection_for_revision(&selection.scope, &selection.record) {
            Ok(_) => Ok(true),
            Err(McpFailureCode::NotFound | McpFailureCode::StaleRevision) => Ok(false),
            Err(error) => Err(catalog_error(error)),
        }
    })
}
