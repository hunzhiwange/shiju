//! Explicit managed MCP batches on the authorized execution host.

use std::{collections::HashSet, path::PathBuf, sync::Arc};

use tokio_util::sync::CancellationToken;
use vw_agent_apps::mcp::validation::validate_new_name;
use vw_api_types::{
    agent_apps_diagnostics::{
        DiagnosticJobIdentity, DiagnosticJobSnapshot, DiagnosticProbeKind, DiagnosticResult,
    },
    id::TaskId,
    mcp_management::{
        McpCatalogScope, McpConnectionCheckItem, McpConnectionDefinition, McpCredentialRef,
        McpFailureCode, McpRecordId,
    },
};

use super::{
    McpCatalogStore,
    connectivity::{CheckRevision, ConnectionCheckHost},
};
use crate::{
    agent_apps::auth_accounts::{AuthAccountScope, AuthAccountStore},
    gateway::agent_apps_diagnostic_jobs::{
        DiagnosticJobError, DiagnosticJobOwner, DiagnosticJobs, DiagnosticProbeError,
    },
};

pub struct McpProbeTarget {
    pub id: McpRecordId,
    pub connection_revision: u64,
}

pub struct McpProbeBatch {
    pub scope: McpCatalogScope,
    pub generation: u64,
    pub targets: Vec<McpProbeTarget>,
}

/// Submit explicitly authorized records to the host's shared diagnostic scheduler. The
/// gateway supplies the trusted workspace and decides whether to grant each credential.
pub fn start_connection_checks(
    jobs: &DiagnosticJobs,
    owner: DiagnosticJobOwner,
    catalog: Arc<McpCatalogStore>,
    accounts: Arc<AuthAccountStore>,
    workspace: PathBuf,
    batch: McpProbeBatch,
    authorize_credential_use: impl for<'a> Fn(
        &'a DiagnosticJobOwner,
        &McpConnectionDefinition,
        &McpCredentialRef,
    ) -> Option<AuthAccountScope<'a>>
    + Send
    + Sync
    + 'static,
) -> Result<DiagnosticJobIdentity, DiagnosticJobError> {
    let scope_matches = match &batch.scope {
        McpCatalogScope::Tenant { scope } => scope == owner.scope.project.tenant_scope(),
        McpCatalogScope::Project { scope } => scope == &owner.scope.project,
    };
    let mut unique = HashSet::new();
    if !scope_matches
        || !workspace.is_absolute()
        || !batch.targets.iter().all(|target| {
            validate_new_name(&target.id.0, &[]).is_ok_and(|id| id == target.id)
                && target.connection_revision > 0
                && unique.insert(&target.id)
        })
    {
        return Err(DiagnosticJobError::InvalidInput);
    }

    let owner_context = Arc::new(owner.clone());
    let workspace = Arc::new(workspace);
    let authorize = Arc::new(authorize_credential_use);
    let generation = batch.generation;
    let probes = batch
        .targets
        .into_iter()
        .map(|target| {
            let (catalog, accounts) = (catalog.clone(), accounts.clone());
            let (owner, workspace) = (owner_context.clone(), workspace.clone());
            let (scope, authorize) = (batch.scope.clone(), authorize.clone());
            move |cancel: CancellationToken| async move {
                let host = ConnectionCheckHost {
                    scope: &scope,
                    execution_host: &owner.scope.target_host_id,
                    workspace: &workspace,
                    accounts: &accounts,
                };
                let outcome = catalog
                    .check_connection_inner(
                        host,
                        &target.id,
                        CheckRevision::Connection(target.connection_revision),
                        Some(&cancel),
                        |definition, reference| authorize(&owner, definition, reference),
                    )
                    .await;
                if cancel.is_cancelled() {
                    return Err(DiagnosticProbeError::Cancelled);
                }
                Ok(DiagnosticResult::McpConnectionCheck {
                    scope,
                    generation,
                    item: McpConnectionCheckItem {
                        id: target.id,
                        connection_revision: target.connection_revision,
                        outcome,
                    },
                })
            }
        })
        .collect();
    jobs.start(owner, DiagnosticProbeKind::Mcp, probes)
}

impl McpCatalogStore {
    /// Select results for the request currently shown by the caller, from an owner-authorized
    /// diagnostic snapshot. An edit invalidates old connection revisions, including completed jobs.
    pub fn current_connection_check_items(
        &self,
        scope: &McpCatalogScope,
        snapshot: &DiagnosticJobSnapshot,
        operation_id: &TaskId,
        generation: u64,
    ) -> Result<Vec<McpConnectionCheckItem>, McpFailureCode> {
        if snapshot.job.task_id != *operation_id || snapshot.job.kind != DiagnosticProbeKind::Mcp {
            return Ok(Vec::new());
        }
        let mut current = Vec::new();
        for result in snapshot.results.iter().flatten() {
            let DiagnosticResult::McpConnectionCheck {
                scope: result_scope,
                generation: result_generation,
                item,
            } = result
            else {
                continue;
            };
            if result_scope != scope || *result_generation != generation {
                continue;
            }
            match self.current_connection_revision(scope, &item.id) {
                Ok(revision) if revision == item.connection_revision => {
                    current.push(item.clone());
                }
                Ok(_) | Err(McpFailureCode::NotFound) => {}
                Err(code) => return Err(code),
            }
        }
        Ok(current)
    }
}
