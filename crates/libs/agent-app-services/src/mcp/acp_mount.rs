//! Resolve managed MCP references against the scope of an owned ACP session.

#[cfg(all(test, target_os = "linux"))]
#[path = "acp_mount_tests.rs"]
mod tests;

use std::collections::HashSet;

use agent_client_protocol::McpServer;
use acp_types::McpCatalogMount;
use acp_server::server::BackendError;
#[cfg(not(target_arch = "wasm32"))]
use vw_api_transport::ProjectScope;
use {
    vw_api_transport::TenantScope,
    vw_api_mcp_management::McpFailureCode,
};

use super::McpCatalogStore;
#[cfg(not(target_arch = "wasm32"))]
use super::runtime::{runtime_server, scope_belongs_to_project};
use agent_database::GlobalDatabase;
use agent_database::run_sync_database;
use session_port::SessionScope;

#[derive(Default)]
pub struct ResolvedCatalogMounts {
    pub leases: Option<AcpMountLeases>,
    pub servers: Vec<McpServer>,
    pub reviewed_names: Vec<String>,
}

pub fn resolve_mounts(
    scope: &SessionScope,
    session: &str,
    project_id: &str,
    selected: &[McpCatalogMount],
    restoring: bool,
    database: impl FnOnce() -> anyhow::Result<GlobalDatabase>,
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
            if !names.insert(&mount.record.id.0) {
                return Err(mount_error(McpFailureCode::Conflict));
            }
        }

        let store = catalog_store(database).map_err(|error| BackendError::Internal(error.to_string()))?;
        let (mounts, servers, reviewed_names) = run_sync_database(|| {
            let mut mounts = Vec::new();
            let mut servers = Vec::new();
            let reviewed_names = Vec::new();
            for mount in selected {
                match is_reviewed(&store, &project, mount) {
                    Ok(true) => {
                        #[cfg(all(target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))]
                        match store.reviewed_mounted_stdio(&project, mount) {
                            Ok(_) => {
                                reviewed_names.push(mount.record.id.0.clone());
                                mounts.push(mount.clone());
                                continue;
                            }
                            Err(McpFailureCode::PermissionDenied | McpFailureCode::NotFound | McpFailureCode::StaleRevision) if restoring => continue,
                            Err(code) => return Ok(Err(mount_error(code))),
                        }
                        #[cfg(not(all(target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64"))))]
                        return Ok(Err(mount_error(McpFailureCode::PermissionDenied)));
                    }
                    Ok(false) if scope_belongs_to_project(&mount.scope, &project) => {}
                    Ok(false) => return Ok(Err(mount_error(McpFailureCode::PermissionDenied))),
                    Err(code) => return Ok(Err(mount_error(code))),
                }
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
                // Transport materialization can touch the filesystem. Recheck the
                // pinned record before handing the candidate to the session pool.
                match store.connection_for_revision(&mount.scope, &mount.record) {
                    Ok(_) => {}
                    Err(McpFailureCode::NotFound | McpFailureCode::StaleRevision) if restoring => {
                        continue;
                    }
                    Err(code) => return Ok(Err(mount_error(code))),
                }
                servers.push(server);
                mounts.push(mount.clone());
            }
            Ok(Ok((mounts, servers, reviewed_names)))
        })
        .map_err(|_| mount_error(McpFailureCode::StorageUnavailable))??;
        let leases = (!mounts.is_empty()).then(|| AcpMountLeases {
            tenant: scope.tenant().clone(),
            session: session.to_owned(),
            mounts,
            store,
            project,
        });
        Ok(ResolvedCatalogMounts { leases, servers, reviewed_names })
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn mount_error(code: McpFailureCode) -> BackendError {
    BackendError::InvalidRequest(format!("managed MCP mount: {code:?}"))
}

#[cfg(not(target_arch = "wasm32"))]
fn catalog_store(database: impl FnOnce() -> anyhow::Result<GlobalDatabase>) -> anyhow::Result<McpCatalogStore> {
    let unavailable = || anyhow::anyhow!("managed MCP catalog is unavailable");
    let database = database().map_err(|_| unavailable())?;
    McpCatalogStore::new(database).map_err(|_| unavailable())
}

/// The session pool keeps these references apart from client-supplied ACP transport metadata.
pub struct AcpMountLeases {
    tenant: TenantScope,
    session: String,
    pub mounts: Vec<McpCatalogMount>,
    store: McpCatalogStore,
    #[cfg(not(target_arch = "wasm32"))]
    project: ProjectScope,
}

impl AcpMountLeases {
    pub fn is_current(&self, name: &str, context: Option<(TenantScope, String)>) -> anyhow::Result<bool> {
        let Some(mount) = self.mounts.iter().find(|mount| mount.record.id.0 == name) else {
            return Ok(true);
        };
        if context.is_some_and(|(tenant, session)| tenant != self.tenant || session != self.session) {
            return Ok(false);
        }
        run_sync_database(|| self.native_revision_is_current(mount))
    }

    pub fn revoked_names(&self) -> anyhow::Result<HashSet<&str>> {
        run_sync_database(|| {
            let mut revoked = HashSet::new();
            for mount in &self.mounts {
                if !self.native_revision_is_current(mount)? {
                    revoked.insert(mount.record.id.0.as_str());
                }
            }
            Ok(revoked)
        })
    }

    fn native_revision_is_current(&self, mount: &McpCatalogMount) -> anyhow::Result<bool> {
        #[cfg(not(target_arch = "wasm32"))]
        if is_reviewed(&self.store, &self.project, mount)
            .map_err(|code| anyhow::anyhow!("managed MCP mount: {code:?}"))?
        {
            return Ok(false);
        }
        revision_is_current(&self.store, mount)
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn is_reviewed(
    store: &McpCatalogStore,
    project: &ProjectScope,
    mount: &McpCatalogMount,
) -> Result<bool, McpFailureCode> {
    match store.linux_stdio_selection(project, &mount.record.id.0) {
        Ok(selection) => Ok(super::reviewed_stdio::matches_catalog_scope(&selection.scope, &mount.scope)),
        Err(McpFailureCode::NotFound | McpFailureCode::PermissionDenied) => Ok(false),
        Err(code) => Err(code),
    }
}

fn revision_is_current(store: &McpCatalogStore, mount: &McpCatalogMount) -> anyhow::Result<bool> {
    match store.connection_for_revision(&mount.scope, &mount.record) {
        Ok(_) => Ok(true),
        Err(McpFailureCode::NotFound | McpFailureCode::StaleRevision) => Ok(false),
        Err(code) => anyhow::bail!("managed MCP mount: {code:?}"),
    }
}
