use super::*;
use crate::model_relay::RelayBinding;
use acp_client::ApprovalValidity;
use agent_acp::mount_store::MountRecord;
use agent_app_services::mcp::{McpCatalogStore, reviewed_stdio::ReviewedProjectStdio};
use agent_database::run_sync_database;
use std::{path::Path, sync::Arc, time::Instant};
use vw_api_transport::ProjectScope;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReviewedStdioSource {
    Project,
    Mounted,
}

pub(crate) struct ReviewedStdioMount {
    pub(crate) source: ReviewedStdioSource,
    pub(crate) server: McpServer,
    pub(crate) review: Arc<ReviewedProjectStdio>,
    // Share the current check across discovery, approval issuance and dispatch.
    pub(crate) current: Arc<dyn Fn() -> bool + Send + Sync>,
}

impl ReviewedStdioMount {
    pub(crate) fn guard(&self, approval: ApprovalValidity) -> ApprovalValidity {
        let current = self.current.clone();
        Box::new(move || current() && approval())
    }
}

/// Resolve the current session's committed reference or an explicit project selection.
pub(crate) async fn reviewed_linux_stdio(
    launch: Arc<RelayBinding>,
    cwd: &Path,
    project_selected: &[McpServer],
) -> anyhow::Result<Option<ReviewedStdioMount>> {
    let identity = launch.identity();
    let owner = ProjectScope::new(identity.tenant.clone(), identity.project.as_str().into())?;
    let database = || GlobalDatabase::for_home(&crate::global::paths().home);
    let Some(context) = crate::tools::current_tool_use_context() else {
        return resolve(launch, owner, database, project_selected, || Ok(None));
    };
    let session = context.session();
    let Some((session_scope, adapter)) = identity.session.rsplit_once(':') else {
        anyhow::bail!("Invalid reviewed MCP relay owner");
    };
    anyhow::ensure!(
        !adapter.is_empty()
            && session_scope
                .rsplit_once(":acp-source-")
                .is_some_and(|(bound, generation)| bound == session && !generation.is_empty())
            && context.tenant_scope() == identity.tenant,
        "Reviewed MCP relay belongs to another session"
    );
    let scope = SessionScope::new(identity.tenant.clone(), cwd)?;
    let record_scope = run_sync_database(|| {
        session_store::ui_store::load_agent_session_strict(
            owner.tenant_scope(),
            owner.project_id().as_ref(),
            session,
        )
    })?;
    if let Some(record) = &record_scope {
        anyhow::ensure!(
            record.id == session
                && record.project_id == identity.project
                && Path::new(&record.directory).canonicalize()? == scope.directory(),
            "Reviewed MCP session ownership mismatch"
        );
    }
    let read = agent_acp::mount_store::reader(&scope, session).await;
    // A project-only launch can precede publication of the native session.
    let owns_session = record_scope.is_some();
    resolve(launch, owner, database, project_selected, move || {
        let record = read()?;
        if !owns_session && record.is_some() {
            return Err(BackendError::InvalidRequest(
                "Cannot establish reviewed MCP session ownership".into(),
            ));
        }
        Ok(record)
    })
}

fn resolve(
    launch: Arc<RelayBinding>,
    owner: ProjectScope,
    database: impl FnOnce() -> anyhow::Result<GlobalDatabase>,
    project_selected: &[McpServer],
    read: impl Fn() -> Result<Option<MountRecord>, BackendError> + Send + Sync + 'static,
) -> anyhow::Result<Option<ReviewedStdioMount>> {
    let committed = read()?;
    let refs = committed.as_ref().map(|record| record.mcp_catalog.as_slice()).unwrap_or_default();
    if refs.is_empty() && project_selected.is_empty() {
        return Ok(None);
    }
    anyhow::ensure!(
        !committed.as_ref().is_some_and(|record| project_selected
            .iter()
            .any(|server| record.has_mcp_name(mcp_client::server_name(server)))),
        "Reviewed MCP name conflicts across project and session selections"
    );
    let database = database()?;
    let load = |error| anyhow::anyhow!("reviewed MCP selection: {error:?}");
    let store = run_sync_database(|| McpCatalogStore::new(database).map_err(load))?;
    let mut mounted = None;
    for mount in refs {
        let reviewed = run_sync_database(|| {
            store.reviewed_mounted_stdio(&owner, mount).map_or_else(
                |error| match error {
                    vw_api_mcp_management::McpFailureCode::NotFound
                    | vw_api_mcp_management::McpFailureCode::PermissionDenied
                    | vw_api_mcp_management::McpFailureCode::StaleRevision => Ok(None),
                    error => Err(load(error)),
                },
                |service| Ok(Some(service)),
            )
        })?;
        if let Some((proof, server)) = reviewed {
            anyhow::ensure!(
                mounted.is_none(),
                "Only one reviewed session MCP selection is supported"
            );
            mounted = Some((proof, McpServer::Stdio(server), mount.clone()));
        }
    }
    let (source, proof, server, anchor) = if let Some((proof, server, mount)) = mounted {
        anyhow::ensure!(
            project_selected.is_empty(),
            "Only one reviewed MCP selection is supported"
        );
        let revision = committed.as_ref().expect("mount has committed revision").revision;
        (ReviewedStdioSource::Mounted, proof, server, Some((revision, mount)))
    } else {
        let [] = project_selected else {
            let proof = run_sync_database(|| {
                store.reviewed_project_stdio(&owner, project_selected).map_err(load)
            })?;
            let [server] = project_selected else {
                anyhow::bail!("Only one reviewed project MCP selection is supported")
            };
            return bind(
                launch,
                owner,
                store,
                read,
                ReviewedStdioSource::Project,
                proof,
                server.clone(),
                None,
            )
            .map(Some);
        };
        return Ok(None);
    };
    bind(launch, owner, store, read, source, proof, server, anchor).map(Some)
}

fn bind(
    launch: Arc<RelayBinding>,
    owner: ProjectScope,
    store: McpCatalogStore,
    read: impl Fn() -> Result<Option<MountRecord>, BackendError> + Send + Sync + 'static,
    source: ReviewedStdioSource,
    proof: ReviewedProjectStdio,
    server: McpServer,
    anchor: Option<(acp_types::SessionMountRevision, McpCatalogMount)>,
) -> anyhow::Result<ReviewedStdioMount> {
    let service = mcp_client::server_name(&server).to_owned();
    let review = Arc::new(proof);
    let proof = review.clone();
    let current = Arc::new(move || {
        if launch.cancelled().is_cancelled()
            || launch.expires().is_some_and(|expiry| Instant::now() >= expiry)
        {
            return false;
        }
        let Ok(record) = read() else { return false };
        let selected = match &anchor {
            Some((revision, mount)) => record.is_some_and(|record| {
                record.revision == *revision
                    && record
                        .mcp_catalog
                        .iter()
                        .filter(|entry| entry.record.id.0 == service)
                        .eq([mount])
            }),
            None => record.is_none_or(|record| !record.has_mcp_name(&service)),
        };
        selected
            && run_sync_database(|| Ok(proof.is_current(&store, &owner).unwrap_or(false)))
                .unwrap_or(false)
    });
    anyhow::ensure!(current(), "Reviewed MCP selection was revoked");
    Ok(ReviewedStdioMount { source, server, review, current })
}
