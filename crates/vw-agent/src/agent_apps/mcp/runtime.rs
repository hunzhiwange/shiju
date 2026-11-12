//! Explicit, revision-pinned catalog selections for the native project runtime.

use std::path::{Path, PathBuf};

use agent_client_protocol::{
    EnvVariable, HttpHeader, McpServer, McpServerHttp, McpServerSse, McpServerStdio,
};
use anyhow::{Context, bail};
use serde::{Deserialize, Serialize};
use vw_api_types::{
    ProjectScope,
    mcp_management::{
        McpCatalogScope, McpFailureCode, McpRecordRef, McpRuntimeSelection, McpTransport,
    },
};

use super::{McpCatalogStore, McpExecutionConnection, revocation::RuntimeSelectionLease};
use crate::{
    global_database::{GlobalDatabase, run_sync_database},
    project::store::ProjectStore,
    tools::mcp_common::server_name,
};

const RUNTIME_CWD: &str = "shiju.managedMcpCwd";

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct StoredSelection {
    pub(super) scope: McpCatalogScope,
    pub(super) record: McpRecordRef,
    #[serde(default)]
    pub(super) disabled: bool,
    #[serde(default)]
    pub(super) generation: u64,
}

impl McpCatalogStore {
    /// Save the exact record revision for a project already authorized by the caller.
    pub fn select_for_runtime(
        &self,
        authorized_project: &ProjectScope,
        authorized_catalog: &McpCatalogScope,
        selection: &McpRuntimeSelection,
    ) -> Result<(), McpFailureCode> {
        let tenant = authorized_project.tenant_scope();
        if authorized_project != &selection.project
            || !scope_belongs_to_project(authorized_catalog, authorized_project)
        {
            return Err(McpFailureCode::PermissionDenied);
        }
        self.resolve_for_native_runtime(authorized_catalog, &selection.record)?;
        let namespace = selection_namespace(authorized_project)?;
        let key = &selection.record.id.0;
        let mut next = StoredSelection {
            scope: authorized_catalog.clone(),
            record: selection.record.clone(),
            disabled: false,
            generation: 0,
        };
        let payload =
            serde_json::to_string(&next).map_err(|_| McpFailureCode::StorageUnavailable)?;
        if self
            .0
            .insert_if_absent(tenant, &namespace, key, &payload)
            .map_err(|_| McpFailureCode::StorageUnavailable)?
        {
            return Ok(());
        }
        let previous = self
            .0
            .get(tenant, &namespace, key)
            .map_err(|_| McpFailureCode::StorageUnavailable)?
            .ok_or(McpFailureCode::StaleRevision)?;
        let current: StoredSelection =
            serde_json::from_str(&previous).map_err(|_| McpFailureCode::StorageUnavailable)?;
        if current.scope != next.scope {
            return Err(McpFailureCode::Conflict);
        }
        if current.record.revision > next.record.revision {
            return Err(McpFailureCode::StaleRevision);
        }
        if current.record == next.record && !current.disabled {
            return Ok(());
        }
        next.generation = current.generation;
        self.save_entry(tenant, &namespace, key, &previous, &next)
    }

    /// Read only this record's selection in the caller-authorized project and catalog.
    pub(crate) fn runtime_usage(
        &self,
        project: &ProjectScope,
        scope: &McpCatalogScope,
        id: &vw_api_types::mcp_management::McpRecordId,
    ) -> Result<vw_api_types::mcp_management::McpRuntimeUsage, McpFailureCode> {
        use vw_api_types::mcp_management::McpRuntimeUsage;
        if !scope_belongs_to_project(scope, project) {
            return Err(McpFailureCode::PermissionDenied);
        }
        let namespace = selection_namespace(project)?;
        let Some(payload) = self
            .0
            .get(project.tenant_scope(), &namespace, &id.0)
            .map_err(|_| McpFailureCode::StorageUnavailable)?
        else {
            return Ok(McpRuntimeUsage::NotSelected);
        };
        let selection: StoredSelection =
            serde_json::from_str(&payload).map_err(|_| McpFailureCode::StorageUnavailable)?;
        if selection.scope != *scope || selection.record.id != *id {
            return Err(McpFailureCode::Conflict);
        }
        let current = match self.connection_for_revision(scope, &selection.record) {
            Ok(_) => true,
            Err(McpFailureCode::NotFound | McpFailureCode::StaleRevision) => false,
            Err(code) => return Err(code),
        };
        Ok(McpRuntimeUsage::Selected {
            record: selection.record,
            disabled: selection.disabled,
            generation: selection.generation,
            current,
        })
    }

    fn runtime_servers(
        &self,
        project: &ProjectScope,
        workspace: &Path,
        existing: &[McpServer],
    ) -> anyhow::Result<Vec<McpServer>> {
        let namespace = selection_namespace(project).map_err(catalog_error)?;
        let rows = self.0.list(project.tenant_scope(), &namespace)?;
        let mut servers = Vec::with_capacity(rows.len());
        for row in rows {
            let selection: StoredSelection = serde_json::from_str(&row.payload_json)
                .map_err(|_| catalog_error(McpFailureCode::StorageUnavailable))?;
            if row.config_key != selection.record.id.0
                || !scope_belongs_to_project(&selection.scope, project)
            {
                return Err(catalog_error(McpFailureCode::PermissionDenied));
            }
            if selection.disabled {
                continue;
            }
            let connection =
                match self.resolve_for_native_runtime(&selection.scope, &selection.record) {
                    Ok(connection) => connection,
                    Err(McpFailureCode::NotFound | McpFailureCode::StaleRevision) => continue,
                    Err(error) => return Err(catalog_error(error)),
                };
            if existing.iter().any(|server| server_name(server) == row.config_key) {
                bail!(
                    "managed MCP runtime server '{}' conflicts with an existing declaration",
                    row.config_key
                );
            }
            let mut server = runtime_server(row.config_key, connection, workspace)?;
            RuntimeSelectionLease::attach(&mut server, project, selection)?;
            servers.push(server);
        }
        Ok(servers)
    }
}

pub(super) fn selection_namespace(project: &ProjectScope) -> Result<String, McpFailureCode> {
    let project = serde_json::to_string(project.project_id())
        .map_err(|_| McpFailureCode::InvalidDefinition)?;
    Ok(format!("agent_apps.mcp.runtime.v1.{project}"))
}

pub(super) fn scope_belongs_to_project(scope: &McpCatalogScope, project: &ProjectScope) -> bool {
    match scope {
        McpCatalogScope::Tenant { scope } => scope == project.tenant_scope(),
        McpCatalogScope::Project { scope } => scope == project,
    }
}

pub(super) fn catalog_error(code: McpFailureCode) -> anyhow::Error {
    anyhow::anyhow!("managed MCP runtime selection: {code:?}")
}

pub(crate) fn selected_runtime_servers(
    project: &ProjectScope,
    workspace: &Path,
    existing: &[McpServer],
) -> anyhow::Result<Vec<McpServer>> {
    let Ok(workspace) = workspace.canonicalize() else {
        return Ok(Vec::new());
    };
    let database = GlobalDatabase::for_home(&crate::global::paths().home)?;
    run_sync_database(|| {
        let Some(info) = ProjectStore::new(database.clone())?.get(project)? else {
            return Ok(Vec::new());
        };
        let in_project = std::iter::once(&info.worktree).chain(&info.sandboxes).any(|root| {
            Path::new(root)
                .canonicalize()
                .is_ok_and(|root| root.parent().is_some() && workspace.starts_with(root))
        });
        if !in_project {
            return Ok(Vec::new());
        }
        McpCatalogStore::new(database)
            .map_err(catalog_error)?
            .runtime_servers(project, &workspace, existing)
    })
}

pub(super) fn runtime_server(
    name: String,
    connection: McpExecutionConnection,
    workspace: &Path,
) -> anyhow::Result<McpServer> {
    match connection {
        McpExecutionConnection::Stdio { command, args, env, cwd } => {
            let env = env.into_iter().map(|(key, value)| EnvVariable::new(key, value)).collect();
            let mut server = McpServerStdio::new(name, command).args(args).env(env);
            if let Some(cwd) = cwd {
                let path =
                    workspace.join(cwd).canonicalize().context("managed MCP cwd is unavailable")?;
                if !path.is_dir() {
                    bail!("managed MCP cwd must be a directory");
                }
                server.meta = Some(serde_json::Map::from_iter([(
                    RUNTIME_CWD.to_owned(),
                    serde_json::to_value(path)?,
                )]));
            }
            Ok(McpServer::Stdio(server))
        }
        McpExecutionConnection::Http { transport, url, headers } => {
            let headers = headers
                .iter()
                .map(|(name, value)| {
                    Ok(HttpHeader::new(name.as_str(), std::str::from_utf8(value.as_bytes())?))
                })
                .collect::<anyhow::Result<_>>()?;
            match transport {
                McpTransport::StreamableHttp => {
                    Ok(McpServer::Http(McpServerHttp::new(name, url).headers(headers)))
                }
                McpTransport::Sse => {
                    Ok(McpServer::Sse(McpServerSse::new(name, url).headers(headers)))
                }
                McpTransport::Stdio => bail!("invalid managed MCP HTTP transport"),
            }
        }
    }
}

pub(crate) fn managed_stdio_cwd(server: &McpServerStdio) -> anyhow::Result<Option<PathBuf>> {
    let Some(value) = server.meta.as_ref().and_then(|meta| meta.get(RUNTIME_CWD)) else {
        return Ok(None);
    };
    let cwd = PathBuf::from(value.as_str().context("invalid managed MCP cwd")?);
    if !cwd.is_absolute() || cwd.canonicalize()? != cwd || !cwd.is_dir() {
        bail!("managed MCP cwd changed after discovery");
    }
    Ok(Some(cwd))
}
