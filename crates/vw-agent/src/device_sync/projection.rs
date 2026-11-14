//! Post-commit projection orchestration. Never accepts snapshot bytes or imports.

use std::{future::Future, pin::Pin};

use vw_agent_apps::data_management::safety_snapshot::SafetySnapshotReceipt;

/// The executor builds these jobs from authorized local targets. Each job must
/// await the real owner's complete report and treat partial/cancelled reports as
/// failure: provider native publication, McpCatalogStore::resync_apps,
/// PromptsStore::sync_to_live, or SkillInstallsStore::resync_all_apps. It must read
/// current saved intent on every attempt; never close over an import or preimage.
/// No job may spawn detached work or resolve paths/credentials from remote JSON.
pub struct ProjectionJob<'a> {
    pub target: ProjectionTarget,
    pub refresh: Pin<Box<dyn Future<Output = Result<(), ()>> + Send + 'a>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionTarget {
    Providers,
    Mcp,
    Prompts,
    Skills,
}

/// Configuration is already committed, even when warnings is nonempty. These
/// fixed warnings deliberately exclude owner errors, paths and secret payloads.
#[derive(Debug)]
pub struct ProjectionWarning {
    pub target: ProjectionTarget,
}

pub struct AppliedSnapshot {
    pub receipt: SafetySnapshotReceipt,
    pub changed_records: usize,
    pub warnings: Vec<ProjectionWarning>,
}

impl AppliedSnapshot {
    /// Retry only failed projections. The executor must reacquire the same sync
    /// lock, reauthorize, and construct fresh owner jobs from current saved state.
    /// No import callback exists on this path, so retries cannot replay apply.
    pub async fn retry_projections(
        &mut self,
        _sync_lock: &tokio::sync::OwnedMutexGuard<()>,
        jobs: Vec<ProjectionJob<'_>>,
    ) {
        let requested: Vec<_> = self.warnings.iter().map(|w| w.target).collect();
        let mut attempted = Vec::new();
        for job in jobs {
            if !requested.contains(&job.target) || attempted.contains(&job.target) {
                continue;
            }
            attempted.push(job.target);
            if job.refresh.await.is_ok() {
                self.warnings.retain(|warning| warning.target != job.target);
            }
        }
    }
}

/// Run inline while the executor still holds its operation lock. Missing domains
/// must not be supplied as successful no-ops: the executor supplies jobs only for
/// locally authorized affected domains. Failure never rolls back committed data
/// and never prevents another domain from being refreshed.
pub(super) async fn refresh(jobs: Vec<ProjectionJob<'_>>) -> Vec<ProjectionWarning> {
    let mut warnings = Vec::new();
    let mut attempted = Vec::new();
    for job in jobs {
        if attempted.contains(&job.target) {
            continue;
        }
        attempted.push(job.target);
        if job.refresh.await.is_err() {
            warnings.push(ProjectionWarning { target: job.target });
        }
    }
    warnings
}

/// Adapt the skill owner's saved-binding reconciliation, preserving its partial
/// failure semantics rather than treating an outer Ok as complete success.
pub fn skill_job<'a>(
    store: &'a crate::gateway::agent_apps_skills_store::SkillInstallsStore,
    scope: &'a vw_api_types::id::ProjectScope,
    host: &'a str,
    ssot: &'a std::path::Path,
    targets: &'a [vw_agent_apps::skills::bulk_sync::SkillSyncTarget],
) -> ProjectionJob<'a> {
    ProjectionJob {
        target: ProjectionTarget::Skills,
        refresh: Box::pin(async move {
            let report = store.resync_all_apps(scope, host, ssot, targets).map_err(|_| ())?;
            if report.items.iter().any(|item| {
                item.status == vw_agent_apps::skills::bulk_sync::SkillSyncStatus::Failed
            }) {
                return Err(());
            }
            Ok(())
        }),
    }
}

/// One domain job may reconcile several authorized application targets. Target
/// resolution and authorization stay with the existing host dispatcher.
#[allow(clippy::too_many_arguments)]
pub fn prompt_job<'a>(
    store: &'a crate::agent_apps::prompts_store::PromptsStore,
    scopes: Vec<crate::agent_apps::prompts_store::PromptScope<'a>>,
    executor_host: &'a str,
    home: &'a std::path::Path,
    overrides: &'a std::collections::HashMap<vw_api_types::agent_apps::AppId, String>,
    environment: &'a std::collections::BTreeMap<String, String>,
    transactions: &'a vw_agent_apps::providers::live::commit::ProviderTransactions,
    authorize: impl Fn(crate::agent_apps::prompts_store::PromptScope<'_>)
        -> Result<(), vw_agent_apps::providers::live::write_guard::WriteError> + Send + Sync + 'a,
) -> ProjectionJob<'a> {
    ProjectionJob {
        target: ProjectionTarget::Prompts,
        refresh: Box::pin(async move {
            let mut failed = false;
            for scope in scopes {
                if store.sync_to_live(scope, executor_host, home,
                    overrides.get(&scope.app).map(String::as_str), environment,
                    transactions, || authorize(scope)).is_err() {
                    failed = true;
                }
            }
            if failed { Err(()) } else { Ok(()) }
        }),
    }
}

/// Await the MCP owner's authorized resync call and retain partial/cancelled
/// outcomes as retryable warnings. The future is lazy until apply commits.
pub fn mcp_job<'a>(
    resync: impl Future<Output = Result<
        crate::agent_apps::mcp::McpResyncResult,
        vw_api_types::mcp_management::McpFailureCode,
    >> + Send + 'a,
) -> ProjectionJob<'a> {
    ProjectionJob {
        target: ProjectionTarget::Mcp,
        refresh: Box::pin(async move {
            let report = resync.await.map_err(|_| ())?;
            if report.outcome == vw_api_types::mcp_management::McpBatchOutcome::Succeeded {
                Ok(())
            } else {
                Err(())
            }
        }),
    }
}
