//! Explicit managed MCP batches on the authorized execution host.

use std::{collections::HashSet, path::PathBuf, sync::Arc};

use tokio_util::sync::CancellationToken;
use vw_agent_apps::mcp::validation::validate_new_name;
use {
    vw_api_app_diagnostics::DiagnosticJobIdentity,
    vw_api_app_diagnostics::DiagnosticProbeKind,
    vw_api_app_diagnostics::DiagnosticResult,
    vw_api_mcp_management::McpCatalogScope,
    vw_api_mcp_management::McpConnectionCheckItem,
    vw_api_mcp_management::McpConnectionDefinition,
    vw_api_mcp_management::McpCredentialRef,
    vw_api_mcp_management::McpFailureCode,
    vw_api_mcp_management::McpRecordId,
};

use super::{
    McpCatalogStore,
    connectivity::{CheckRevision, ConnectionCheckHost},
};
use crate::auth_accounts::{AuthAccountScope, AuthAccountStore};
use super::probe_jobs_host::BatchHost;

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
pub fn start_connection_checks<H: BatchHost>(
    jobs: &H::Jobs,
    owner: H::Owner,
    catalog: Arc<McpCatalogStore>,
    accounts: Arc<AuthAccountStore>,
    workspace: PathBuf,
    batch: McpProbeBatch,
    authorize_credential_use: impl for<'a> Fn(
        &'a H::Owner,
        &McpConnectionDefinition,
        &McpCredentialRef,
    ) -> Option<AuthAccountScope<'a>>
    + Send
    + Sync
    + 'static,
) -> Result<DiagnosticJobIdentity, H::JobError> {
    let owner_scope = H::scope(&owner).clone();
    let scope_matches = match &batch.scope {
        McpCatalogScope::Tenant { scope } => scope == owner_scope.project.tenant_scope(),
        McpCatalogScope::Project { scope } => scope == &owner_scope.project,
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
        return Err(H::invalid_input());
    }

    let owner_context = Arc::new(owner.clone());
    let owner_scope = Arc::new(owner_scope);
    let workspace = Arc::new(workspace);
    let authorize = Arc::new(authorize_credential_use);
    let generation = batch.generation;
    let probes = batch
        .targets
        .into_iter()
        .map(|target| {
            let (catalog, accounts) = (catalog.clone(), accounts.clone());
            let (owner, workspace) = (owner_context.clone(), workspace.clone());
            let owner_scope = owner_scope.clone();
            let (scope, authorize) = (batch.scope.clone(), authorize.clone());
            move |cancel: CancellationToken| async move {
                let host = ConnectionCheckHost {
                    scope: &scope,
                    execution_host: &owner_scope.target_host_id,
                    workspace: &workspace,
                    accounts: Some(&accounts),
                    authorize: &|| Ok(()),
                };
                let outcome = catalog
                    .check_connection_inner(
                        &H::runtime(),
host,
                        &target.id,
                        CheckRevision::Connection(target.connection_revision),
                        Some(&cancel),
                        |definition, reference| authorize(&owner, definition, reference),
                    )
                    .await;
                if cancel.is_cancelled() {
                    return Err(H::cancelled());
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
    H::start(jobs, owner, DiagnosticProbeKind::Mcp, probes)
}
