//! Resolve managed MCP references against the scope of an owned ACP session.

use std::collections::HashSet;

use agent_client_protocol::McpServer;
use vw_acp::{McpCatalogMount, server::BackendError};
#[cfg(not(target_arch = "wasm32"))]
use vw_api_types::ProjectScope;
use vw_api_types::{TenantScope, mcp_management::McpFailureCode};

use super::McpCatalogStore;
#[cfg(not(target_arch = "wasm32"))]
use super::runtime::{runtime_server, scope_belongs_to_project};
#[cfg(not(target_arch = "wasm32"))]
use crate::global_database::GlobalDatabase;
use crate::{global_database::run_sync_database, session::runtime::SessionScope};

#[derive(Default)]
pub(crate) struct ResolvedCatalogMounts {
    pub(crate) leases: Option<AcpMountLeases>,
    pub(crate) servers: Vec<McpServer>,
}

pub(crate) fn resolve_mounts(
    scope: &SessionScope,
    session: &str,
    project_id: &str,
    selected: &[McpCatalogMount],
    restoring: bool,
) -> Result<ResolvedCatalogMounts, BackendError> {
    if selected.is_empty() {
        return Ok(ResolvedCatalogMounts::default());
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (scope, session, project_id, restoring);
        Err(BackendError::InvalidRequest("managed MCP mounts are unavailable".to_owned()))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let project = ProjectScope::new(scope.tenant().clone(), project_id.into())
            .map_err(|_| mount_error(McpFailureCode::PermissionDenied))?;
        let mut names = HashSet::new();
        for mount in selected {
            if !scope_belongs_to_project(&mount.scope, &project) {
                return Err(mount_error(McpFailureCode::PermissionDenied));
            }
            if !names.insert(&mount.record.id.0) {
                return Err(mount_error(McpFailureCode::Conflict));
            }
        }

        let store = catalog_store().map_err(|error| BackendError::Internal(error.to_string()))?;
        let (mounts, servers) = run_sync_database(|| {
            let mut mounts = Vec::new();
            let mut servers = Vec::new();
            for mount in selected {
                let connection = match store.resolve_for_native_runtime(&mount.scope, &mount.record)
                {
                    Ok(connection) => connection,
                    Err(McpFailureCode::NotFound | McpFailureCode::StaleRevision) if restoring => {
                        continue;
                    }
                    Err(code) => return Ok(Err(mount_error(code))),
                };
                let server = match runtime_server(
                    mount.record.id.0.clone(),
                    connection,
                    scope.directory(),
                ) {
                    Ok(server) => server,
                    Err(_) => return Ok(Err(mount_error(McpFailureCode::InvalidDefinition))),
                };
                servers.push(server);
                mounts.push(mount.clone());
            }
            Ok(Ok((mounts, servers)))
        })
        .map_err(|_| mount_error(McpFailureCode::StorageUnavailable))??;
        let leases = (!mounts.is_empty()).then(|| AcpMountLeases {
            tenant: scope.tenant().clone(),
            session: session.to_owned(),
            mounts,
            store,
        });
        Ok(ResolvedCatalogMounts { leases, servers })
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn mount_error(code: McpFailureCode) -> BackendError {
    BackendError::InvalidRequest(format!("managed MCP mount: {code:?}"))
}

#[cfg(not(target_arch = "wasm32"))]
fn catalog_store() -> anyhow::Result<McpCatalogStore> {
    let unavailable = || anyhow::anyhow!("managed MCP catalog is unavailable");
    let database =
        GlobalDatabase::for_home(&crate::global::paths().home).map_err(|_| unavailable())?;
    McpCatalogStore::new(database).map_err(|_| unavailable())
}

/// The session pool keeps these references apart from client-supplied ACP transport metadata.
pub(crate) struct AcpMountLeases {
    tenant: TenantScope,
    session: String,
    pub(crate) mounts: Vec<McpCatalogMount>,
    store: McpCatalogStore,
}

impl AcpMountLeases {
    pub(crate) fn is_current(&self, name: &str) -> anyhow::Result<bool> {
        let Some(mount) = self.mounts.iter().find(|mount| mount.record.id.0 == name) else {
            return Ok(true);
        };
        if crate::tools::current_tool_use_context().is_some_and(|context| {
            context.tenant_scope() != self.tenant || context.session() != self.session
        }) {
            return Ok(false);
        }
        run_sync_database(|| revision_is_current(&self.store, mount))
    }

    pub(crate) fn revoked_names(&self) -> anyhow::Result<HashSet<&str>> {
        run_sync_database(|| {
            let mut revoked = HashSet::new();
            for mount in &self.mounts {
                if !revision_is_current(&self.store, mount)? {
                    revoked.insert(mount.record.id.0.as_str());
                }
            }
            Ok(revoked)
        })
    }
}

fn revision_is_current(store: &McpCatalogStore, mount: &McpCatalogMount) -> anyhow::Result<bool> {
    match store.connection_for_revision(&mount.scope, &mount.record) {
        Ok(_) => Ok(true),
        Err(McpFailureCode::NotFound | McpFailureCode::StaleRevision) => Ok(false),
        Err(code) => anyhow::bail!("managed MCP mount: {code:?}"),
    }
}
