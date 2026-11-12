//! Read committed ACP catalog references after checking the exact session scope.

use serde::Serialize;
use vw_acp::{McpCatalogMount, SessionMountRevision, server::BackendError};
use vw_api_types::{
    ProjectScope,
    mcp_management::{McpCatalogScope, McpFailureCode},
};

use crate::{
    agent_apps::mcp::McpCatalogStore,
    global_database::{GlobalDatabase, run_sync_database},
    session::{
        runtime::{SessionScope, validate_session_scope},
        ui_store,
    },
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogMountSelection {
    pub mount: McpCatalogMount,
    /// None means this exact catalog revision still exists. It does not grant tool execution.
    pub failure: Option<McpFailureCode>,
}

/// A committed mount generation, excluding transport configuration and skill paths.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogMountSnapshot {
    pub project: ProjectScope,
    pub session_id: String,
    pub revision: SessionMountRevision,
    pub catalog: Vec<CatalogMountSelection>,
}

/// The caller must authorize the exact project/session on the current execution host.
/// Directory identity comes only from the scoped server-side session record.
/// This reads the committed generation, without restoring connections or loading history.
pub async fn read_catalog_mounts(
    project: &ProjectScope,
    session_id: &str,
) -> Result<CatalogMountSnapshot, BackendError> {
    let info = crate::global_database::run_isolated({
        let project = project.clone();
        let session_id = session_id.to_owned();
        move || {
            ui_store::load_agent_session_strict(
                project.tenant_scope(),
                project.project_id().as_ref(),
                &session_id,
            )
        }
    })
    .await
    .map_err(|_| unavailable())?
    .map_err(|_| unavailable())?
    .ok_or_else(|| BackendError::NotFound(session_id.to_owned()))?;
    if info.id != session_id || info.project_id != project.project_id().as_ref() {
        return Err(BackendError::OwnershipMismatch(session_id.to_owned()));
    }
    let scope = SessionScope::new(project.tenant_scope().clone(), &info.directory)
        .map_err(|_| BackendError::OwnershipMismatch(session_id.to_owned()))?;
    let verified = validate_session_scope(scope.clone(), session_id)
        .await
        .map_err(|_| BackendError::OwnershipMismatch(session_id.to_owned()))?;
    if verified.project_id != info.project_id {
        return Err(BackendError::OwnershipMismatch(session_id.to_owned()));
    }
    let record = super::mount_store::load(&scope, session_id)
        .await?
        .ok_or_else(|| BackendError::NotFound(session_id.to_owned()))?;
    for mount in &record.mcp_catalog {
        let allowed = match &mount.scope {
            McpCatalogScope::Tenant { scope } => scope == project.tenant_scope(),
            McpCatalogScope::Project { scope } => scope == project,
        };
        if !allowed {
            return Err(BackendError::OwnershipMismatch(session_id.to_owned()));
        }
    }
    let catalog = run_sync_database(|| {
        if record.mcp_catalog.is_empty() {
            return Ok(Vec::new());
        }
        let database = GlobalDatabase::for_home(&crate::global::paths().home)?;
        let store = McpCatalogStore::new(database)
            .map_err(|_| anyhow::anyhow!("MCP catalog unavailable"))?;
        let mut catalog = Vec::new();
        for mount in record.mcp_catalog {
            let failure = match store.list(&mount.scope) {
                Ok(records) => {
                    match records.iter().find(|record| record.record.id == mount.record.id) {
                        Some(record) if record.record == mount.record => None,
                        Some(_) => Some(McpFailureCode::StaleRevision),
                        None => Some(McpFailureCode::NotFound),
                    }
                }
                Err(code) => Some(code),
            };
            catalog.push(CatalogMountSelection { mount, failure });
        }
        Ok(catalog)
    })
    .map_err(|_| unavailable())?;
    Ok(CatalogMountSnapshot {
        project: project.clone(),
        session_id: session_id.to_owned(),
        revision: record.revision,
        catalog,
    })
}

fn unavailable() -> BackendError {
    BackendError::Internal("ACP catalog mounts unavailable".to_owned())
}
